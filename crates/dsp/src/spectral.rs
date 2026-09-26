//! Short-time Fourier transform noise reduction: learned-profile Denoise and
//! adaptive Speech Denoise.
//!
//! Frames are 1024 samples with a 256-sample hop (75 % overlap), square-root
//! Hann analysis and synthesis windows, and weighted overlap-add. Output is
//! delayed by [`SPECTRAL_LATENCY`] samples. All buffers and FFT tables are
//! allocated in the constructors; `process` never allocates, locks, or waits.

use std::f32::consts::PI;

/// Frame length in samples.
pub const SPECTRAL_FRAME: usize = 1024;
/// Hop between frames in samples.
pub const SPECTRAL_HOP: usize = 256;
/// Disclosed processing delay in samples (one frame).
pub const SPECTRAL_LATENCY: usize = SPECTRAL_FRAME;
/// Number of positive-frequency bins (DC through Nyquist).
pub const SPECTRAL_BINS: usize = SPECTRAL_FRAME / 2 + 1;
/// Bands in a serialized noise profile.
pub const NOISE_PROFILE_BANDS: usize = 64;

/// In-place iterative radix-2 complex FFT of a fixed power-of-two size.
#[derive(Clone, Debug)]
struct Fft {
    size: usize,
    cos: Vec<f32>,
    sin: Vec<f32>,
    reverse: Vec<usize>,
}

impl Fft {
    fn new(size: usize) -> Self {
        debug_assert!(size.is_power_of_two());
        let bits = size.trailing_zeros();
        let reverse = (0..size)
            .map(|index| index.reverse_bits() >> (usize::BITS - bits))
            .collect();
        let cos = (0..size / 2).map(|k| (2.0 * PI * k as f32 / size as f32).cos()).collect();
        let sin = (0..size / 2).map(|k| (2.0 * PI * k as f32 / size as f32).sin()).collect();
        Self { size, cos, sin, reverse }
    }

    /// Forward transform when `inverse` is false; unnormalized inverse otherwise.
    fn transform(&self, real: &mut [f32], imag: &mut [f32], inverse: bool) {
        for index in 0..self.size {
            let target = self.reverse[index];
            if target > index {
                real.swap(index, target);
                imag.swap(index, target);
            }
        }
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut length = 2;
        while length <= self.size {
            let half = length / 2;
            let stride = self.size / length;
            for start in (0..self.size).step_by(length) {
                for offset in 0..half {
                    let (c, s) = (self.cos[offset * stride], sign * self.sin[offset * stride]);
                    let (a, b) = (start + offset, start + offset + half);
                    let tr = real[b] * c - imag[b] * s;
                    let ti = real[b] * s + imag[b] * c;
                    real[b] = real[a] - tr;
                    imag[b] = imag[a] - ti;
                    real[a] += tr;
                    imag[a] += ti;
                }
            }
            length *= 2;
        }
    }
}

/// Per-bin gain rule applied to each frame's power spectrum.
trait SpectralGain {
    /// Fill `gains` (length [`SPECTRAL_BINS`]) from the frame's `power`.
    fn gains(&mut self, power: &[f32], gains: &mut [f32]);
}

/// Mono STFT analysis/resynthesis around a [`SpectralGain`] rule.
#[derive(Clone, Debug)]
struct Stft {
    fft: Fft,
    window: Vec<f32>,
    input: Vec<f32>,
    output: Vec<f32>,
    input_fill: usize,
    output_read: usize,
    real: Vec<f32>,
    imag: Vec<f32>,
    power: Vec<f32>,
    gains: Vec<f32>,
    primed: usize,
}

impl Stft {
    fn new() -> Self {
        // sqrt-Hann analysis and synthesis; their product (Hann) overlaps to a
        // constant 2 at 75 % overlap, removed by the 0.5 scale on synthesis.
        let window = (0..SPECTRAL_FRAME)
            .map(|n| (0.5 - 0.5 * (2.0 * PI * n as f32 / SPECTRAL_FRAME as f32).cos()).sqrt())
            .collect();
        Self {
            fft: Fft::new(SPECTRAL_FRAME),
            window,
            input: vec![0.0; SPECTRAL_FRAME],
            output: vec![0.0; SPECTRAL_FRAME],
            input_fill: SPECTRAL_FRAME - SPECTRAL_HOP,
            output_read: 0,
            real: vec![0.0; SPECTRAL_FRAME],
            imag: vec![0.0; SPECTRAL_FRAME],
            power: vec![0.0; SPECTRAL_BINS],
            gains: vec![1.0; SPECTRAL_BINS],
            primed: 0,
        }
    }

    fn reset(&mut self) {
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.input_fill = SPECTRAL_FRAME - SPECTRAL_HOP;
        self.output_read = 0;
        self.primed = 0;
    }

    fn process(&mut self, samples: &mut [f32], rule: &mut impl SpectralGain) {
        for sample in samples.iter_mut() {
            let value = if sample.is_finite() { *sample } else { 0.0 };
            self.input[self.input_fill] = value;
            self.input_fill += 1;
            // The output FIFO holds one frame; the first frame's worth of
            // output is silence (the disclosed latency).
            *sample = if self.primed >= SPECTRAL_LATENCY {
                self.output[self.output_read]
            } else {
                self.primed += 1;
                0.0
            };
            self.output_read += 1;
            if self.input_fill == SPECTRAL_FRAME {
                self.frame(rule);
            }
        }
    }

    fn frame(&mut self, rule: &mut impl SpectralGain) {
        for index in 0..SPECTRAL_FRAME {
            self.real[index] = self.input[index] * self.window[index];
            self.imag[index] = 0.0;
        }
        self.fft.transform(&mut self.real, &mut self.imag, false);
        for bin in 0..SPECTRAL_BINS {
            self.power[bin] = self.real[bin] * self.real[bin] + self.imag[bin] * self.imag[bin];
        }
        rule.gains(&self.power, &mut self.gains);
        for bin in 0..SPECTRAL_BINS {
            let gain = if self.gains[bin].is_finite() { self.gains[bin].clamp(0.0, 1.0) } else { 0.0 };
            self.real[bin] *= gain;
            self.imag[bin] *= gain;
            if bin != 0 && bin != SPECTRAL_FRAME / 2 {
                // Keep the spectrum Hermitian so the inverse stays real.
                let mirror = SPECTRAL_FRAME - bin;
                self.real[mirror] = self.real[bin];
                self.imag[mirror] = -self.imag[bin];
            }
        }
        self.fft.transform(&mut self.real, &mut self.imag, true);
        // Shift the output FIFO by one hop, then overlap-add the new frame.
        self.output.copy_within(SPECTRAL_HOP.., 0);
        self.output[SPECTRAL_FRAME - SPECTRAL_HOP..].fill(0.0);
        let scale = 0.5 / SPECTRAL_FRAME as f32;
        for index in 0..SPECTRAL_FRAME {
            self.output[index] += self.real[index] * self.window[index] * scale;
        }
        self.output_read -= SPECTRAL_HOP;
        self.input.copy_within(SPECTRAL_HOP.., 0);
        self.input_fill = SPECTRAL_FRAME - SPECTRAL_HOP;
    }
}

/// Learned-profile noise reduction rule (Audio Hijack "Denoise").
#[derive(Clone, Debug)]
struct LearnedRule {
    noise: Vec<f32>,
    learned_frames: u32,
    learning: bool,
    over_subtraction: f32,
    floor: f32,
    previous: Vec<f32>,
    smoothed: Vec<f32>,
}

impl SpectralGain for LearnedRule {
    fn gains(&mut self, power: &[f32], gains: &mut [f32]) {
        if self.learning {
            // Running mean of the noise power while only noise is present.
            self.learned_frames = self.learned_frames.saturating_add(1);
            let weight = 1.0 / self.learned_frames.min(200) as f32;
            for (noise, power) in self.noise.iter_mut().zip(power) {
                *noise += (power - *noise) * weight;
            }
            gains.fill(1.0);
            return;
        }
        for bin in 0..SPECTRAL_BINS {
            self.smoothed[bin] = 0.5 * self.smoothed[bin] + 0.5 * power[bin];
            let estimate = self.smoothed[bin].max(power[bin] * 0.5);
            let snr_gain = if estimate > 0.0 {
                (1.0 - self.over_subtraction * self.noise[bin] / estimate).max(0.0).sqrt()
            } else {
                0.0
            };
            // Smooth decreases to limit "musical noise"; increases are immediate.
            let smoothed = snr_gain.max(self.previous[bin] * 0.5);
            self.previous[bin] = smoothed;
            gains[bin] = smoothed.max(self.floor);
        }
    }
}

/// Mono learned-profile denoiser.
#[derive(Clone, Debug)]
pub struct Denoiser {
    stft: Stft,
    rule: LearnedRule,
}

impl Denoiser {
    /// `reduction_percent` 0–100 scales over-subtraction (0 to 3×);
    /// `floor_percent` 0–100 is the remaining noise floor as linear gain
    /// (0 % removes everything the profile describes, 100 % leaves it).
    /// `profile` is a serialized [`NOISE_PROFILE_BANDS`] profile or `None`.
    pub fn new(reduction_percent: f32, floor_percent: f32, profile: Option<&str>, learning: bool) -> Self {
        let clamp = |value: f32| if value.is_finite() { value.clamp(0.0, 100.0) } else { 50.0 };
        let mut noise = vec![0.0; SPECTRAL_BINS];
        if let Some(encoded) = profile {
            decode_noise_profile(encoded, &mut noise);
        }
        Self {
            stft: Stft::new(),
            rule: LearnedRule {
                noise,
                learned_frames: 0,
                learning,
                over_subtraction: clamp(reduction_percent) / 100.0 * 3.0,
                floor: clamp(floor_percent) / 100.0,
                previous: vec![1.0; SPECTRAL_BINS],
                smoothed: vec![0.0; SPECTRAL_BINS],
            },
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        self.stft.process(samples, &mut self.rule);
    }

    pub fn reset(&mut self) {
        self.stft.reset();
        self.rule.previous.fill(1.0);
    }

    pub fn is_learning(&self) -> bool {
        self.rule.learning
    }

    /// Serialize the current noise estimate (control thread only).
    pub fn noise_profile(&self) -> String {
        encode_noise_profile(&self.rule.noise)
    }
}

/// Adaptive speech-focused noise suppression (Audio Hijack "Speech
/// Denoise" equivalent, without a machine-learning model): minimum-statistics
/// noise tracking, a decision-directed Wiener gain, and extra attenuation
/// outside the 80 Hz – 8 kHz speech band.
#[derive(Clone, Debug)]
struct SpeechRule {
    smoothed: Vec<f32>,
    noise: Vec<f32>,
    previous_clean: Vec<f32>,
    floor: f32,
    band_floor: f32,
    speech_low: usize,
    speech_high: usize,
    frames: u32,
}

impl SpectralGain for SpeechRule {
    fn gains(&mut self, power: &[f32], gains: &mut [f32]) {
        self.frames = self.frames.saturating_add(1);
        for bin in 0..SPECTRAL_BINS {
            let current = power[bin].max(1.0e-12);
            self.smoothed[bin] = 0.8 * self.smoothed[bin] + 0.2 * current;
            // Track directly while the smoothed estimate settles, then follow
            // drops immediately and rise slowly (about 75 % per second at a
            // 256-sample hop and 48 kHz) so speech is not mistaken for noise.
            if self.frames <= 20 || self.smoothed[bin] < self.noise[bin] {
                self.noise[bin] = self.smoothed[bin];
            } else {
                self.noise[bin] *= 1.003;
            }
            // The minimum of a smoothed noisy power underestimates its mean;
            // compensate before using it as the noise level.
            let noise = (self.noise[bin] * MINIMUM_BIAS).max(1.0e-12);
            let posterior = current / noise;
            let prior = (0.98 * self.previous_clean[bin] / noise + 0.02 * (posterior - 1.0).max(0.0)).max(1.0e-3);
            let wiener = prior / (1.0 + prior);
            self.previous_clean[bin] = wiener * wiener * current;
            let floor = if (self.speech_low..=self.speech_high).contains(&bin) { self.floor } else { self.band_floor };
            gains[bin] = wiener.sqrt().max(floor);
        }
    }
}

const MINIMUM_BIAS: f32 = 2.5;

/// Mono adaptive speech denoiser.
#[derive(Clone, Debug)]
pub struct SpeechDenoiser {
    stft: Stft,
    rule: SpeechRule,
}

impl SpeechDenoiser {
    /// `strength_percent` 0–100: 0 leaves audio unchanged, 100 suppresses
    /// noise down to about −30 dB inside and −40 dB outside the speech band.
    pub fn new(strength_percent: f32, sample_rate: f32) -> Self {
        let strength = if strength_percent.is_finite() { strength_percent.clamp(0.0, 100.0) / 100.0 } else { 0.5 };
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 { sample_rate } else { 48_000.0 };
        let bin_of = |hz: f32| ((hz / sample_rate) * SPECTRAL_FRAME as f32).round() as usize;
        Self {
            stft: Stft::new(),
            rule: SpeechRule {
                smoothed: vec![0.0; SPECTRAL_BINS],
                noise: vec![0.0; SPECTRAL_BINS],
                previous_clean: vec![0.0; SPECTRAL_BINS],
                floor: 10.0_f32.powf(-30.0 * strength / 20.0),
                band_floor: 10.0_f32.powf(-40.0 * strength / 20.0),
                speech_low: bin_of(80.0).max(1),
                speech_high: bin_of(8_000.0).min(SPECTRAL_BINS - 1),
                frames: 0,
            },
        }
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        self.stft.process(samples, &mut self.rule);
    }

    pub fn reset(&mut self) {
        self.stft.reset();
        self.rule.frames = 0;
        self.rule.previous_clean.fill(0.0);
    }
}

/// Block length of [`Convolver`]; also its disclosed latency in samples.
pub const CONVOLUTION_BLOCK: usize = 512;
/// Longest accepted impulse response: 2 s at 48 kHz.
pub const MAX_IMPULSE_RESPONSE: usize = 96_000;
const CONVOLUTION_FFT: usize = 2 * CONVOLUTION_BLOCK;
const CONVOLUTION_BINS: usize = CONVOLUTION_BLOCK + 1;

/// Mono FIR filter by uniformly partitioned overlap-save convolution (Audio
/// Hijack "FIR Filter" equivalent). The impulse response is split into
/// [`CONVOLUTION_BLOCK`]-sample partitions whose spectra are precomputed;
/// each block costs one forward and one inverse 1024-point FFT plus one
/// complex multiply-add per partition. Output lags input by one block.
#[derive(Clone, Debug)]
pub struct Convolver {
    fft: Fft,
    partitions: usize,
    /// Impulse-response partition spectra, `partitions × CONVOLUTION_BINS`.
    response_real: Vec<f32>,
    response_imag: Vec<f32>,
    /// Frequency-domain delay line of past input spectra (same layout).
    history_real: Vec<f32>,
    history_imag: Vec<f32>,
    newest: usize,
    window: Vec<f32>,
    input_fill: usize,
    output: Vec<f32>,
    real: Vec<f32>,
    imag: Vec<f32>,
    wet: f32,
    dry_delay: Vec<f32>,
}

impl Convolver {
    /// `response` is truncated to [`MAX_IMPULSE_RESPONSE`] and normalized to
    /// unit energy, then scaled by `gain`. `wet_percent` 0–100 blends the
    /// filtered signal with the (equally delayed) dry signal.
    pub fn new(response: &[f32], gain: f32, wet_percent: f32) -> Self {
        let length = response.len().clamp(1, MAX_IMPULSE_RESPONSE);
        let energy: f32 = response[..response.len().min(length)].iter().map(|sample| sample * sample).sum();
        let scale = if energy.is_finite() && energy > 1.0e-12 { gain / energy.sqrt() } else { 0.0 };
        let partitions = length.div_ceil(CONVOLUTION_BLOCK);
        let fft = Fft::new(CONVOLUTION_FFT);
        let mut response_real = vec![0.0; partitions * CONVOLUTION_BINS];
        let mut response_imag = vec![0.0; partitions * CONVOLUTION_BINS];
        let mut real = vec![0.0; CONVOLUTION_FFT];
        let mut imag = vec![0.0; CONVOLUTION_FFT];
        for partition in 0..partitions {
            real.fill(0.0);
            imag.fill(0.0);
            let start = partition * CONVOLUTION_BLOCK;
            for (offset, sample) in response.iter().skip(start).take(CONVOLUTION_BLOCK.min(length - start)).enumerate() {
                real[offset] = if sample.is_finite() { sample * scale } else { 0.0 };
            }
            fft.transform(&mut real, &mut imag, false);
            let base = partition * CONVOLUTION_BINS;
            response_real[base..base + CONVOLUTION_BINS].copy_from_slice(&real[..CONVOLUTION_BINS]);
            response_imag[base..base + CONVOLUTION_BINS].copy_from_slice(&imag[..CONVOLUTION_BINS]);
        }
        let wet = if wet_percent.is_finite() { wet_percent.clamp(0.0, 100.0) / 100.0 } else { 1.0 };
        Self {
            fft,
            partitions,
            response_real,
            response_imag,
            history_real: vec![0.0; partitions * CONVOLUTION_BINS],
            history_imag: vec![0.0; partitions * CONVOLUTION_BINS],
            newest: 0,
            window: vec![0.0; CONVOLUTION_FFT],
            input_fill: 0,
            output: vec![0.0; CONVOLUTION_BLOCK],
            real,
            imag,
            wet,
            dry_delay: vec![0.0; CONVOLUTION_BLOCK],
        }
    }

    pub fn reset(&mut self) {
        self.history_real.fill(0.0);
        self.history_imag.fill(0.0);
        self.window.fill(0.0);
        self.output.fill(0.0);
        self.dry_delay.fill(0.0);
        self.input_fill = 0;
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            let input = if sample.is_finite() { *sample } else { 0.0 };
            let position = self.input_fill;
            // Overlap-save window: previous block in the first half, the
            // block being filled in the second half.
            self.window[CONVOLUTION_BLOCK + position] = input;
            let dry = self.dry_delay[position];
            self.dry_delay[position] = input;
            *sample = self.wet * self.output[position] + (1.0 - self.wet) * dry;
            self.input_fill += 1;
            if self.input_fill == CONVOLUTION_BLOCK {
                self.block();
                self.input_fill = 0;
            }
        }
    }

    fn block(&mut self) {
        self.real.copy_from_slice(&self.window);
        self.imag.fill(0.0);
        self.fft.transform(&mut self.real, &mut self.imag, false);
        self.newest = (self.newest + self.partitions - 1) % self.partitions;
        let base = self.newest * CONVOLUTION_BINS;
        self.history_real[base..base + CONVOLUTION_BINS].copy_from_slice(&self.real[..CONVOLUTION_BINS]);
        self.history_imag[base..base + CONVOLUTION_BINS].copy_from_slice(&self.imag[..CONVOLUTION_BINS]);
        // Y = Σ X(newest + k) · H(k): partition k meets the input from k blocks ago.
        self.real[..CONVOLUTION_BINS].fill(0.0);
        self.imag[..CONVOLUTION_BINS].fill(0.0);
        for partition in 0..self.partitions {
            let input_base = ((self.newest + partition) % self.partitions) * CONVOLUTION_BINS;
            let response_base = partition * CONVOLUTION_BINS;
            for bin in 0..CONVOLUTION_BINS {
                let (xr, xi) = (self.history_real[input_base + bin], self.history_imag[input_base + bin]);
                let (hr, hi) = (self.response_real[response_base + bin], self.response_imag[response_base + bin]);
                self.real[bin] += xr * hr - xi * hi;
                self.imag[bin] += xr * hi + xi * hr;
            }
        }
        for bin in 1..CONVOLUTION_BLOCK {
            self.real[CONVOLUTION_FFT - bin] = self.real[bin];
            self.imag[CONVOLUTION_FFT - bin] = -self.imag[bin];
        }
        self.fft.transform(&mut self.real, &mut self.imag, true);
        let scale = 1.0 / CONVOLUTION_FFT as f32;
        for index in 0..CONVOLUTION_BLOCK {
            let value = self.real[CONVOLUTION_BLOCK + index] * scale;
            self.output[index] = if value.is_finite() { value } else { 0.0 };
        }
        self.window.copy_within(CONVOLUTION_BLOCK.., 0);
    }
}

fn band_edges() -> [usize; NOISE_PROFILE_BANDS + 1] {
    // Log-spaced from bin 1 to Nyquist; band 0 also covers DC.
    let mut edges = [0; NOISE_PROFILE_BANDS + 1];
    for (band, edge) in edges.iter_mut().enumerate() {
        let position = band as f32 / NOISE_PROFILE_BANDS as f32;
        *edge = (((SPECTRAL_BINS - 1) as f32).powf(position)).round() as usize;
    }
    edges[0] = 0;
    edges[NOISE_PROFILE_BANDS] = SPECTRAL_BINS;
    for band in 1..=NOISE_PROFILE_BANDS {
        edges[band] = edges[band].max(edges[band - 1] + 1).min(SPECTRAL_BINS);
    }
    edges
}

/// Encode per-bin noise power as 64 log-spaced bands, 1 dB per step from
/// −160 dB, two hex digits each (128 characters).
pub fn encode_noise_profile(noise: &[f32]) -> String {
    let edges = band_edges();
    let mut encoded = String::with_capacity(NOISE_PROFILE_BANDS * 2);
    for band in 0..NOISE_PROFILE_BANDS {
        let bins = &noise[edges[band].min(noise.len())..edges[band + 1].min(noise.len())];
        let mean = if bins.is_empty() { 0.0 } else { bins.iter().sum::<f32>() / bins.len() as f32 };
        let db = 10.0 * mean.max(1.0e-30).log10();
        let step = (db + 160.0).round().clamp(0.0, 255.0) as u8;
        encoded.push_str(&format!("{step:02x}"));
    }
    encoded
}

/// Decode a profile from [`encode_noise_profile`] into per-bin power.
/// Returns false (leaving `noise` unchanged) for malformed input.
pub fn decode_noise_profile(encoded: &str, noise: &mut [f32]) -> bool {
    if encoded.len() != NOISE_PROFILE_BANDS * 2 || noise.len() != SPECTRAL_BINS {
        return false;
    }
    let mut steps = [0u8; NOISE_PROFILE_BANDS];
    for (band, step) in steps.iter_mut().enumerate() {
        let Ok(value) = u8::from_str_radix(&encoded[band * 2..band * 2 + 2], 16) else {
            return false;
        };
        *step = value;
    }
    let edges = band_edges();
    for band in 0..NOISE_PROFILE_BANDS {
        let power = 10.0_f32.powf((f32::from(steps[band]) - 160.0) / 10.0);
        for bin in edges[band]..edges[band + 1] {
            noise[bin] = power;
        }
    }
    true
}

/// Whether a string is a well-formed serialized noise profile.
pub fn is_noise_profile(encoded: &str) -> bool {
    encoded.len() == NOISE_PROFILE_BANDS * 2 && encoded.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise(frames: usize, seed: u32, amplitude: f32) -> Vec<f32> {
        let mut state = seed;
        (0..frames)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                amplitude * ((state >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0)
            })
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
    }

    fn sine(frames: usize, frequency: f32, amplitude: f32) -> Vec<f32> {
        (0..frames)
            .map(|frame| amplitude * (2.0 * PI * frequency * frame as f32 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn fft_round_trips() {
        let fft = Fft::new(16);
        let original: Vec<f32> = (0..16).map(|index| (index as f32 * 0.7).sin()).collect();
        let mut real = original.clone();
        let mut imag = vec![0.0; 16];
        fft.transform(&mut real, &mut imag, false);
        fft.transform(&mut real, &mut imag, true);
        for (restored, original) in real.iter().zip(&original) {
            assert!((restored / 16.0 - original).abs() < 1e-5);
        }
    }

    #[test]
    fn stft_with_unity_gain_reconstructs_after_the_latency() {
        struct Unity;
        impl SpectralGain for Unity {
            fn gains(&mut self, _: &[f32], gains: &mut [f32]) {
                gains.fill(1.0);
            }
        }
        let input = sine(9_600, 440.0, 0.5);
        let mut output = input.clone();
        let mut stft = Stft::new();
        for chunk in output.chunks_mut(128) {
            stft.process(chunk, &mut Unity);
        }
        for frame in 2_048..input.len() - SPECTRAL_LATENCY {
            assert!((output[frame + SPECTRAL_LATENCY] - input[frame]).abs() < 1e-3, "frame {frame}");
        }
    }

    #[test]
    fn learned_denoise_removes_profiled_noise_but_keeps_a_tone() {
        let hiss = noise(96_000, 7, 0.05);
        let mut denoiser = Denoiser::new(100.0, 0.0, None, true);
        let mut learning = hiss[..48_000].to_vec();
        denoiser.process(&mut learning);
        let profile = denoiser.noise_profile();
        assert!(is_noise_profile(&profile));
        let mut applied = Denoiser::new(100.0, 0.0, Some(&profile), false);
        assert!(!applied.is_learning());
        let tone = sine(48_000, 1_000.0, 0.3);
        let mut mixed: Vec<f32> = tone.iter().zip(&hiss[48_000..]).map(|(t, n)| t + n).collect();
        applied.process(&mut mixed);
        let mut noise_only = hiss[48_000..].to_vec();
        let mut again = Denoiser::new(100.0, 0.0, Some(&profile), false);
        again.process(&mut noise_only);
        let tail = 24_000..48_000;
        assert!(rms(&noise_only[tail.clone()]) < 0.05 / 3.0_f32.sqrt() * 0.3, "noise reduced");
        assert!(rms(&mixed[tail]) > 0.3 / 2.0_f32.sqrt() * 0.8, "tone kept");
    }

    #[test]
    fn speech_denoise_suppresses_steady_noise_and_passes_a_tone() {
        let mut denoiser = SpeechDenoiser::new(100.0, 48_000.0);
        let mut steady = noise(96_000, 3, 0.05);
        denoiser.process(&mut steady);
        assert!(rms(&steady[72_000..]) < 0.05 / 3.0_f32.sqrt() * 0.3);
        // Speech-like: a 500 Hz tone gated on/off at a syllable rate (4 Hz)
        // over the same noise, measured during the "on" halves only.
        let mut denoiser = SpeechDenoiser::new(100.0, 48_000.0);
        let hiss = noise(96_000, 11, 0.05);
        let tone = sine(96_000, 500.0, 0.3);
        let gate = |frame: usize| (frame / 6_000) % 2 == 0;
        let mut speech: Vec<f32> = (0..96_000).map(|frame| if gate(frame) { tone[frame] } else { 0.0 } + hiss[frame]).collect();
        denoiser.process(&mut speech);
        let voiced: Vec<f32> = (72_000..96_000 - SPECTRAL_LATENCY)
            .filter(|frame| gate(*frame) && frame % 6_000 > 1_500 && frame % 6_000 < 4_500)
            .map(|frame| speech[frame + SPECTRAL_LATENCY])
            .collect();
        assert!(rms(&voiced) > 0.3 / 2.0_f32.sqrt() * 0.7);
        let mut silent = SpeechDenoiser::new(0.0, 48_000.0);
        let mut samples = noise(4_096, 5, 0.1);
        samples[10] = f32::NAN;
        silent.process(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn convolver_matches_direct_convolution_after_one_block() {
        let response: Vec<f32> = noise(1_500, 21, 0.3);
        let energy: f32 = response.iter().map(|sample| sample * sample).sum();
        let normalized: Vec<f32> = response.iter().map(|sample| sample / energy.sqrt()).collect();
        let input = noise(6_000, 9, 0.5);
        let mut output = input.clone();
        let mut convolver = Convolver::new(&response, 1.0, 100.0);
        for chunk in output.chunks_mut(128) {
            convolver.process(chunk);
        }
        for frame in [1_600, 2_345, 4_000, 5_000] {
            let direct: f32 = (0..normalized.len().min(frame + 1)).map(|tap| normalized[tap] * input[frame - tap]).sum();
            let actual = output[frame + CONVOLUTION_BLOCK];
            assert!((actual - direct).abs() < 1e-3, "frame {frame}: {actual} vs {direct}");
        }
    }

    #[test]
    fn convolver_impulse_delays_and_mix_blends_dry() {
        let mut response = vec![0.0; 100];
        response[40] = 0.25;
        let input = sine(4_096, 440.0, 0.5);
        let mut wet = input.clone();
        Convolver::new(&response, 1.0, 100.0).process(&mut wet);
        for frame in 1_000..3_000 {
            assert!((wet[frame + CONVOLUTION_BLOCK + 40] - input[frame]).abs() < 1e-4);
        }
        let mut half = input.clone();
        Convolver::new(&response, 1.0, 50.0).process(&mut half);
        for frame in 1_000..3_000 {
            let expected = 0.5 * input[frame - 40] + 0.5 * input[frame];
            assert!((half[frame + CONVOLUTION_BLOCK] - expected).abs() < 1e-4);
        }
        let mut silent = vec![f32::NAN; 1_024];
        Convolver::new(&[0.0; 10], 1.0, 100.0).process(&mut silent);
        assert!(silent.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn noise_profile_round_trips_and_rejects_malformed_input() {
        let mut noise = vec![0.0; SPECTRAL_BINS];
        for (bin, value) in noise.iter_mut().enumerate() {
            *value = 1.0e-4 * (1.0 + bin as f32 / 100.0);
        }
        let encoded = encode_noise_profile(&noise);
        let mut decoded = vec![0.0; SPECTRAL_BINS];
        assert!(decode_noise_profile(&encoded, &mut decoded));
        for (original, restored) in noise.iter().zip(&decoded).skip(8) {
            let ratio_db = 10.0 * (restored / original).log10();
            assert!(ratio_db.abs() < 2.5, "profile error {ratio_db} dB");
        }
        assert!(!decode_noise_profile("zz", &mut decoded));
        assert!(!is_noise_profile(&"g".repeat(128)));
    }
}
