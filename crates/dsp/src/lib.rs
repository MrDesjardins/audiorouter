//! Allocation-free built-in DSP primitives for M04.

use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterKind {
    Peaking,
    LowShelf,
    HighShelf,
    LowPass,
    HighPass,
    Notch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EqPresetId {
    VoiceNeutral,
    Hum50Hz,
    Hum60Hz,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqPreset {
    pub id: EqPresetId,
    pub bands: [Option<BiquadParams>; 8],
}

/// Returns the versioned, explainable starting points used by the built-in EQ.
/// Disabled bands are `None`; callers still validate and instantiate each band
/// through `Biquad::new` before adding it to a prepared graph.
pub fn eq_preset(id: EqPresetId, sample_rate: f32) -> Result<EqPreset, BiquadError> {
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Err(BiquadError::InvalidSampleRate);
    }
    let empty = [None; 8];
    let band = |frequency_hz, q| {
        Some(BiquadParams {
            kind: FilterKind::Notch,
            frequency_hz,
            q,
            gain_db: 0.0,
            sample_rate,
        })
    };
    let bands = match id {
        EqPresetId::VoiceNeutral => empty,
        EqPresetId::Hum50Hz => {
            let mut bands = empty;
            bands[0] = band(50.0, 8.0);
            bands
        }
        EqPresetId::Hum60Hz => {
            let mut bands = empty;
            bands[0] = band(60.0, 8.0);
            bands
        }
    };
    if bands
        .iter()
        .flatten()
        .any(|params| params.frequency_hz >= sample_rate * 0.5)
    {
        return Err(BiquadError::InvalidFrequency);
    }
    Ok(EqPreset { id, bands })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiquadParams {
    pub kind: FilterKind,
    pub frequency_hz: f32,
    pub q: f32,
    pub gain_db: f32,
    pub sample_rate: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BiquadError {
    InvalidSampleRate,
    InvalidFrequency,
    InvalidQ,
    InvalidChannels,
    NonFiniteParameter,
}

#[derive(Clone, Copy, Debug)]
struct Coefficients {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Clone, Debug)]
pub struct Biquad {
    params: BiquadParams,
    coefficients: Coefficients,
    channels: usize,
    z1: [f32; 2],
    z2: [f32; 2],
}

impl Biquad {
    pub fn new(params: BiquadParams, channels: usize) -> Result<Self, BiquadError> {
        validate(params, channels)?;
        Ok(Self {
            coefficients: coefficients(params),
            params,
            channels,
            z1: [0.0; 2],
            z2: [0.0; 2],
        })
    }

    pub fn params(&self) -> BiquadParams {
        self.params
    }

    pub fn set_params(&mut self, params: BiquadParams) -> Result<(), BiquadError> {
        validate(params, self.channels)?;
        self.params = params;
        self.coefficients = coefficients(params);
        Ok(())
    }

    pub fn reset(&mut self) {
        self.z1 = [0.0; 2];
        self.z2 = [0.0; 2];
    }

    /// Returns the magnitude response in dB from the exact coefficients used
    /// by `process_interleaved`. This is a control-plane calculation and does
    /// not inspect or mutate filter state.
    pub fn magnitude_db_at(&self, frequency_hz: f32) -> Result<f32, BiquadError> {
        if !frequency_hz.is_finite() {
            return Err(BiquadError::NonFiniteParameter);
        }
        if !(0.0..=self.params.sample_rate * 0.5).contains(&frequency_hz) {
            return Err(BiquadError::InvalidFrequency);
        }
        let omega = 2.0 * PI * frequency_hz / self.params.sample_rate;
        let cos = omega.cos();
        let sin = omega.sin();
        let cos2 = (2.0 * omega).cos();
        let sin2 = (2.0 * omega).sin();
        let numerator_real =
            self.coefficients.b0 + self.coefficients.b1 * cos + self.coefficients.b2 * cos2;
        let numerator_imag = -self.coefficients.b1 * sin - self.coefficients.b2 * sin2;
        let denominator_real = 1.0 + self.coefficients.a1 * cos + self.coefficients.a2 * cos2;
        let denominator_imag = -self.coefficients.a1 * sin - self.coefficients.a2 * sin2;
        let numerator = numerator_real.hypot(numerator_imag);
        let denominator = denominator_real.hypot(denominator_imag);
        if denominator <= f32::EPSILON {
            return Err(BiquadError::InvalidFrequency);
        }
        Ok(20.0 * (numerator / denominator).max(f32::MIN_POSITIVE).log10())
    }

    /// Processes interleaved mono/stereo samples in place without allocation.
    /// Non-finite input is repaired to silence and output is kept finite.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for (index, sample) in samples.iter_mut().enumerate() {
            let channel = index % self.channels;
            let input = if sample.is_finite() { *sample } else { 0.0 };
            let output = self.coefficients.b0 * input + self.z1[channel];
            self.z1[channel] =
                self.coefficients.b1 * input - self.coefficients.a1 * output + self.z2[channel];
            self.z2[channel] = self.coefficients.b2 * input - self.coefficients.a2 * output;
            *sample = if output.is_finite() { output } else { 0.0 };
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_db: f32,
    pub makeup_db: f32,
    pub sample_rate: f32,
}

/// A stereo-linked feed-forward compressor. Its state is scalar by design so
/// a stereo image receives the same gain reduction on both channels.
#[derive(Clone, Debug)]
pub struct Compressor {
    params: CompressorParams,
    channels: usize,
    envelope_db: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GateParams {
    pub threshold_db: f32,
    pub hysteresis_db: f32,
    pub ratio: f32,
    pub range_db: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
    pub sample_rate: f32,
}

/// A stereo-linked downward gate/expander with explicit hold and hysteresis.
#[derive(Clone, Debug)]
pub struct Gate {
    params: GateParams,
    channels: usize,
    gain_db: f32,
    open: bool,
    hold_frames: usize,
}

impl Gate {
    pub fn new(params: GateParams, channels: usize) -> Result<Self, BiquadError> {
        validate_gate(params, channels)?;
        Ok(Self {
            params,
            channels,
            gain_db: -params.range_db,
            open: false,
            hold_frames: 0,
        })
    }

    pub fn params(&self) -> GateParams {
        self.params
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn reset(&mut self) {
        self.gain_db = -self.params.range_db;
        self.open = false;
        self.hold_frames = 0;
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        let attack = (-1.0 / (self.params.attack_ms * 0.001 * self.params.sample_rate)).exp();
        let release = (-1.0 / (self.params.release_ms * 0.001 * self.params.sample_rate)).exp();
        let hold = (self.params.hold_ms * 0.001 * self.params.sample_rate) as usize;
        for frame in samples.chunks_exact_mut(self.channels) {
            let peak = frame
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0_f32, f32::max);
            let level_db = 20.0 * peak.max(1.0e-6).log10();
            if self.open {
                if level_db < self.params.threshold_db - self.params.hysteresis_db {
                    if self.hold_frames == 0 {
                        self.hold_frames = hold;
                    }
                    if self.hold_frames > 0 {
                        self.hold_frames -= 1;
                    } else {
                        self.open = false;
                    }
                } else {
                    self.hold_frames = 0;
                }
            } else if level_db >= self.params.threshold_db {
                self.open = true;
                self.hold_frames = 0;
            }
            let target_db = if self.open {
                0.0
            } else {
                -((self.params.threshold_db - level_db) * (self.params.ratio - 1.0))
                    .clamp(0.0, self.params.range_db)
            };
            let coefficient = if target_db > self.gain_db {
                attack
            } else {
                release
            };
            self.gain_db = coefficient * self.gain_db + (1.0 - coefficient) * target_db;
            let gain = 10.0_f32.powf(self.gain_db / 20.0);
            for sample in frame {
                let input = if sample.is_finite() { *sample } else { 0.0 };
                let output = input * gain;
                *sample = if output.is_finite() { output } else { 0.0 };
            }
        }
    }
}

fn validate_gate(params: GateParams, channels: usize) -> Result<(), BiquadError> {
    if channels == 0 || channels > 2 {
        return Err(BiquadError::InvalidChannels);
    }
    if !params.threshold_db.is_finite()
        || !params.hysteresis_db.is_finite()
        || !params.ratio.is_finite()
        || !params.range_db.is_finite()
        || !params.attack_ms.is_finite()
        || !params.hold_ms.is_finite()
        || !params.release_ms.is_finite()
        || !params.sample_rate.is_finite()
    {
        return Err(BiquadError::NonFiniteParameter);
    }
    if !(44_100.0..=192_000.0).contains(&params.sample_rate) {
        return Err(BiquadError::InvalidSampleRate);
    }
    if !(-80.0..=0.0).contains(&params.threshold_db)
        || !(0.0..=12.0).contains(&params.hysteresis_db)
        || !(1.0..=20.0).contains(&params.ratio)
        || !(0.0..=80.0).contains(&params.range_db)
        || !(0.1..=100.0).contains(&params.attack_ms)
        || !(0.0..=1_000.0).contains(&params.hold_ms)
        || !(10.0..=2_000.0).contains(&params.release_ms)
    {
        return Err(BiquadError::InvalidQ);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LimiterParams {
    pub ceiling_db: f32,
}

/// A sample-peak safety limiter. It deliberately makes no true-peak or
/// lookahead claim; the ceiling is enforced on every emitted sample.
#[derive(Clone, Copy, Debug)]
pub struct PeakLimiter {
    ceiling_linear: f32,
}

impl PeakLimiter {
    pub fn new(params: LimiterParams) -> Result<Self, BiquadError> {
        if !params.ceiling_db.is_finite() {
            return Err(BiquadError::NonFiniteParameter);
        }
        if !(-12.0..=0.0).contains(&params.ceiling_db) {
            return Err(BiquadError::InvalidQ);
        }
        Ok(Self {
            ceiling_linear: 10.0_f32.powf(params.ceiling_db / 20.0),
        })
    }

    pub fn ceiling_db(&self) -> f32 {
        20.0 * self.ceiling_linear.log10()
    }

    pub fn process_interleaved(&self, samples: &mut [f32]) {
        for sample in samples {
            let input = if sample.is_finite() { *sample } else { 0.0 };
            *sample = input.clamp(-self.ceiling_linear, self.ceiling_linear);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DelayError {
    InvalidSampleRate,
    InvalidChannels,
    InvalidMaximum,
    InvalidDelay,
    NonFiniteParameter,
}

/// A bounded interleaved delay line. The ring is allocated once at
/// construction and parameter changes only alter read positions.
#[derive(Clone, Debug)]
pub struct DelayLine {
    sample_rate: f32,
    channels: usize,
    capacity_frames: usize,
    delay_frames: usize,
    buffer: Vec<f32>,
    write_frame: usize,
}

impl DelayLine {
    pub fn new(max_delay_ms: f32, sample_rate: f32, channels: usize) -> Result<Self, DelayError> {
        if !max_delay_ms.is_finite() || max_delay_ms < 0.0 {
            return Err(DelayError::NonFiniteParameter);
        }
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(DelayError::InvalidSampleRate);
        }
        if channels == 0 || channels > 2 {
            return Err(DelayError::InvalidChannels);
        }
        if max_delay_ms > 1_000.0 {
            return Err(DelayError::InvalidMaximum);
        }
        let max_frames = (max_delay_ms * 0.001 * sample_rate).ceil() as usize;
        let capacity_frames = max_frames.saturating_add(1);
        Ok(Self {
            sample_rate,
            channels,
            capacity_frames,
            delay_frames: 0,
            buffer: vec![0.0; capacity_frames * channels],
            write_frame: 0,
        })
    }

    pub fn delay_ms(&self) -> f32 {
        self.delay_frames as f32 * 1_000.0 / self.sample_rate
    }

    pub fn set_delay_ms(&mut self, delay_ms: f32) -> Result<(), DelayError> {
        if !delay_ms.is_finite() {
            return Err(DelayError::NonFiniteParameter);
        }
        if delay_ms < 0.0 {
            return Err(DelayError::InvalidDelay);
        }
        let frames = (delay_ms * 0.001 * self.sample_rate).round() as usize;
        if frames >= self.capacity_frames {
            return Err(DelayError::InvalidDelay);
        }
        self.delay_frames = frames;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_frame = 0;
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(self.channels) {
            let read_frame = (self.write_frame + self.capacity_frames - self.delay_frames)
                % self.capacity_frames;
            for (channel, sample) in frame.iter_mut().enumerate() {
                let input = if sample.is_finite() { *sample } else { 0.0 };
                let index = self.write_frame * self.channels + channel;
                let delayed = self.buffer[read_frame * self.channels + channel];
                self.buffer[index] = input;
                *sample = if self.delay_frames == 0 {
                    input
                } else if delayed.is_finite() {
                    delayed
                } else {
                    0.0
                };
            }
            self.write_frame = (self.write_frame + 1) % self.capacity_frames;
        }
    }
}

impl Compressor {
    pub fn new(params: CompressorParams, channels: usize) -> Result<Self, BiquadError> {
        validate_compressor(params, channels)?;
        Ok(Self {
            params,
            channels,
            envelope_db: -120.0,
        })
    }

    pub fn params(&self) -> CompressorParams {
        self.params
    }

    pub fn reset(&mut self) {
        self.envelope_db = -120.0;
    }

    /// Applies linked detection and compression in place without allocation.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        let attack = (-1.0 / (self.params.attack_ms * 0.001 * self.params.sample_rate)).exp();
        let release = (-1.0 / (self.params.release_ms * 0.001 * self.params.sample_rate)).exp();
        for frame in samples.chunks_exact_mut(self.channels) {
            let peak = frame
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0_f32, f32::max);
            let level_db = 20.0 * peak.max(1.0e-6).log10();
            let coefficient = if level_db > self.envelope_db {
                attack
            } else {
                release
            };
            self.envelope_db = coefficient * self.envelope_db + (1.0 - coefficient) * level_db;
            let reduction_db = compression_reduction(
                self.envelope_db,
                self.params.threshold_db,
                self.params.ratio,
                self.params.knee_db,
            );
            let gain = 10.0_f32.powf((self.params.makeup_db - reduction_db) / 20.0);
            for sample in frame {
                let input = if sample.is_finite() { *sample } else { 0.0 };
                let output = input * gain;
                *sample = if output.is_finite() { output } else { 0.0 };
            }
        }
    }
}

fn validate_compressor(params: CompressorParams, channels: usize) -> Result<(), BiquadError> {
    if channels == 0 || channels > 2 {
        return Err(BiquadError::InvalidChannels);
    }
    if !params.threshold_db.is_finite()
        || !params.ratio.is_finite()
        || !params.attack_ms.is_finite()
        || !params.release_ms.is_finite()
        || !params.knee_db.is_finite()
        || !params.makeup_db.is_finite()
        || !params.sample_rate.is_finite()
    {
        return Err(BiquadError::NonFiniteParameter);
    }
    if !(44_100.0..=192_000.0).contains(&params.sample_rate) {
        return Err(BiquadError::InvalidSampleRate);
    }
    if !(-60.0..=0.0).contains(&params.threshold_db)
        || !(1.0..=20.0).contains(&params.ratio)
        || !(0.1..=200.0).contains(&params.attack_ms)
        || !(10.0..=2_000.0).contains(&params.release_ms)
        || !(0.0..=24.0).contains(&params.knee_db)
        || !(0.0..=24.0).contains(&params.makeup_db)
    {
        return Err(BiquadError::InvalidQ);
    }
    Ok(())
}

fn compression_reduction(level_db: f32, threshold_db: f32, ratio: f32, knee_db: f32) -> f32 {
    let over = level_db - threshold_db;
    if knee_db > 0.0 && over > -knee_db * 0.5 && over < knee_db * 0.5 {
        let distance = over + knee_db * 0.5;
        distance * distance * (1.0 - 1.0 / ratio) / (2.0 * knee_db)
    } else if over > 0.0 {
        over * (1.0 - 1.0 / ratio)
    } else {
        0.0
    }
}

fn validate(params: BiquadParams, channels: usize) -> Result<(), BiquadError> {
    if channels == 0 || channels > 2 {
        return Err(BiquadError::InvalidChannels);
    }
    if !params.sample_rate.is_finite()
        || !params.frequency_hz.is_finite()
        || !params.q.is_finite()
        || !params.gain_db.is_finite()
    {
        return Err(BiquadError::NonFiniteParameter);
    }
    if params.sample_rate <= 0.0 {
        return Err(BiquadError::InvalidSampleRate);
    }
    if !(0.0..params.sample_rate * 0.5).contains(&params.frequency_hz) {
        return Err(BiquadError::InvalidFrequency);
    }
    if !(0.1..=20.0).contains(&params.q) {
        return Err(BiquadError::InvalidQ);
    }
    Ok(())
}

fn coefficients(params: BiquadParams) -> Coefficients {
    let omega = 2.0 * PI * params.frequency_hz / params.sample_rate;
    let sin = omega.sin();
    let cos = omega.cos();
    let alpha = sin / (2.0 * params.q);
    let amplitude = 10.0_f32.powf(params.gain_db / 40.0);
    let (b0, b1, b2, a0, a1, a2) = match params.kind {
        FilterKind::Peaking => (
            1.0 + alpha * amplitude,
            -2.0 * cos,
            1.0 - alpha * amplitude,
            1.0 + alpha / amplitude,
            -2.0 * cos,
            1.0 - alpha / amplitude,
        ),
        FilterKind::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        FilterKind::LowPass => (
            (1.0 - cos) / 2.0,
            1.0 - cos,
            (1.0 - cos) / 2.0,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        ),
        FilterKind::HighPass => (
            (1.0 + cos) / 2.0,
            -(1.0 + cos),
            (1.0 + cos) / 2.0,
            1.0 + alpha,
            -2.0 * cos,
            1.0 - alpha,
        ),
        FilterKind::LowShelf | FilterKind::HighShelf => {
            let two_sqrt = 2.0 * amplitude.sqrt() * alpha;
            let sign = if params.kind == FilterKind::LowShelf {
                1.0
            } else {
                -1.0
            };
            let beta = 2.0 * amplitude.sqrt() * alpha;
            if sign > 0.0 {
                (
                    amplitude * ((amplitude + 1.0) - (amplitude - 1.0) * cos + beta),
                    2.0 * amplitude * ((amplitude - 1.0) - (amplitude + 1.0) * cos),
                    amplitude * ((amplitude + 1.0) - (amplitude - 1.0) * cos - beta),
                    (amplitude + 1.0) + (amplitude - 1.0) * cos + two_sqrt,
                    -2.0 * ((amplitude - 1.0) + (amplitude + 1.0) * cos),
                    (amplitude + 1.0) + (amplitude - 1.0) * cos - two_sqrt,
                )
            } else {
                (
                    amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos + beta),
                    -2.0 * amplitude * ((amplitude - 1.0) + (amplitude + 1.0) * cos),
                    amplitude * ((amplitude + 1.0) + (amplitude - 1.0) * cos - beta),
                    (amplitude + 1.0) - (amplitude - 1.0) * cos + two_sqrt,
                    2.0 * ((amplitude - 1.0) - (amplitude + 1.0) * cos),
                    (amplitude + 1.0) - (amplitude - 1.0) * cos - two_sqrt,
                )
            }
        }
    };
    Coefficients {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(kind: FilterKind) -> BiquadParams {
        BiquadParams {
            kind,
            frequency_hz: 1_000.0,
            q: 0.707,
            gain_db: 6.0,
            sample_rate: 48_000.0,
        }
    }

    #[test]
    fn rejects_unsafe_parameters() {
        assert!(matches!(
            Biquad::new(params(FilterKind::Peaking), 3),
            Err(BiquadError::InvalidChannels)
        ));
        assert!(matches!(
            Biquad::new(
                BiquadParams {
                    frequency_hz: 30_000.0,
                    ..params(FilterKind::Notch)
                },
                1
            ),
            Err(BiquadError::InvalidFrequency)
        ));
    }

    #[test]
    fn flat_peaking_filter_is_neutral_and_repairs_nonfinite() {
        let mut filter = Biquad::new(
            BiquadParams {
                gain_db: 0.0,
                ..params(FilterKind::Peaking)
            },
            2,
        )
        .unwrap();
        let mut samples = [0.1, -0.2, f32::NAN, f32::INFINITY];
        filter.process_interleaved(&mut samples);
        assert!((samples[0] - 0.1).abs() < 1e-6);
        assert!((samples[1] + 0.2).abs() < 1e-6);
        assert_eq!(samples[2], 0.0);
        assert_eq!(samples[3], 0.0);
    }

    #[test]
    fn all_shapes_remain_finite_and_reset_clears_state() {
        for kind in [
            FilterKind::Peaking,
            FilterKind::LowShelf,
            FilterKind::HighShelf,
            FilterKind::LowPass,
            FilterKind::HighPass,
            FilterKind::Notch,
        ] {
            let mut filter = Biquad::new(params(kind), 2).unwrap();
            let mut samples = [1.0; 128];
            filter.process_interleaved(&mut samples);
            assert!(samples.iter().all(|sample| sample.is_finite()));
            filter.reset();
            let mut impulse = [0.0; 2];
            filter.process_interleaved(&mut impulse);
            assert_eq!(impulse, [0.0; 2]);
        }
    }

    #[test]
    fn parameter_update_changes_response_without_reallocation_api() {
        let mut filter = Biquad::new(params(FilterKind::Peaking), 1).unwrap();
        let mut boosted = [1.0; 8];
        filter.process_interleaved(&mut boosted);
        filter
            .set_params(BiquadParams {
                gain_db: -6.0,
                ..params(FilterKind::Peaking)
            })
            .unwrap();
        filter.reset();
        let mut cut = [1.0; 8];
        filter.process_interleaved(&mut cut);
        assert_ne!(boosted, cut);
    }

    #[test]
    fn response_curve_uses_processing_coefficients() {
        let flat = Biquad::new(
            BiquadParams {
                gain_db: 0.0,
                ..params(FilterKind::Peaking)
            },
            1,
        )
        .unwrap();
        assert!(flat.magnitude_db_at(1_000.0).unwrap().abs() < 1e-4);

        let boosted = Biquad::new(params(FilterKind::Peaking), 1).unwrap();
        assert!((boosted.magnitude_db_at(1_000.0).unwrap() - 6.0).abs() < 0.02);

        let notch = Biquad::new(
            BiquadParams {
                kind: FilterKind::Notch,
                gain_db: 0.0,
                ..params(FilterKind::Notch)
            },
            1,
        )
        .unwrap();
        assert!(notch.magnitude_db_at(1_000.0).unwrap() < -60.0);
        assert!(matches!(
            notch.magnitude_db_at(30_000.0),
            Err(BiquadError::InvalidFrequency)
        ));
    }

    #[test]
    fn eq_presets_are_bounded_and_explainable() {
        let neutral = eq_preset(EqPresetId::VoiceNeutral, 48_000.0).unwrap();
        assert_eq!(neutral.id, EqPresetId::VoiceNeutral);
        assert!(neutral.bands.iter().all(Option::is_none));

        for (id, frequency) in [(EqPresetId::Hum50Hz, 50.0), (EqPresetId::Hum60Hz, 60.0)] {
            let preset = eq_preset(id, 48_000.0).unwrap();
            let band = preset.bands[0].unwrap();
            assert_eq!(band.kind, FilterKind::Notch);
            assert_eq!(band.frequency_hz, frequency);
            let filter = Biquad::new(band, 1).unwrap();
            assert!(filter.magnitude_db_at(frequency).unwrap() < -40.0);
        }
        assert!(matches!(
            eq_preset(EqPresetId::Hum50Hz, f32::NAN),
            Err(BiquadError::InvalidSampleRate)
        ));
    }

    #[test]
    fn compressor_is_neutral_below_threshold_and_reduces_linked_peak() {
        let base = CompressorParams {
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 0.1,
            release_ms: 10.0,
            knee_db: 0.0,
            makeup_db: 0.0,
            sample_rate: 48_000.0,
        };
        let mut quiet = Compressor::new(base, 2).unwrap();
        let mut quiet_samples = [0.1, 0.1, 0.1, 0.1];
        quiet.process_interleaved(&mut quiet_samples);
        assert!(quiet_samples
            .iter()
            .all(|sample| (*sample - 0.1).abs() < 1e-4));

        let mut loud = Compressor::new(base, 2).unwrap();
        let mut loud_samples = [0.0; 128];
        for frame in loud_samples.chunks_exact_mut(2) {
            frame[0] = 1.0;
            frame[1] = 0.25;
        }
        loud.process_interleaved(&mut loud_samples);
        assert!(loud_samples[126] < 1.0);
        assert!((loud_samples[126] / 1.0 - loud_samples[127] / 0.25).abs() < 1e-5);
    }

    #[test]
    fn compressor_rejects_out_of_contract_values_and_repairs_nonfinite() {
        let params = CompressorParams {
            threshold_db: -18.0,
            ratio: 21.0,
            attack_ms: 10.0,
            release_ms: 150.0,
            knee_db: 6.0,
            makeup_db: 0.0,
            sample_rate: 48_000.0,
        };
        assert!(matches!(
            Compressor::new(params, 1),
            Err(BiquadError::InvalidQ)
        ));
        let mut compressor = Compressor::new(
            CompressorParams {
                ratio: 1.0,
                ..params
            },
            1,
        )
        .unwrap();
        let mut samples = [f32::NAN, f32::INFINITY];
        compressor.process_interleaved(&mut samples);
        assert_eq!(samples, [0.0, 0.0]);
    }

    #[test]
    fn gate_attenuates_quiet_signal_and_opens_for_linked_loud_signal() {
        let params = GateParams {
            threshold_db: -30.0,
            hysteresis_db: 3.0,
            ratio: 4.0,
            range_db: 60.0,
            attack_ms: 0.1,
            hold_ms: 0.0,
            release_ms: 10.0,
            sample_rate: 48_000.0,
        };
        let mut gate = Gate::new(params, 2).unwrap();
        let mut quiet = [0.01, 0.01];
        gate.process_interleaved(&mut quiet);
        assert!(quiet[0] < 0.01);
        assert!(!gate.is_open());
        let mut loud = [0.0; 128];
        for frame in loud.chunks_exact_mut(2) {
            frame[0] = 1.0;
            frame[1] = 0.25;
        }
        gate.process_interleaved(&mut loud);
        assert!(gate.is_open());
        assert!((loud[126] / 1.0 - loud[127] / 0.25).abs() < 1e-5);
    }

    #[test]
    fn gate_hysteresis_holds_open_below_threshold_until_release_condition() {
        let params = GateParams {
            threshold_db: -20.0,
            hysteresis_db: 6.0,
            ratio: 2.0,
            range_db: 40.0,
            attack_ms: 0.1,
            hold_ms: 0.0,
            release_ms: 10.0,
            sample_rate: 48_000.0,
        };
        let mut gate = Gate::new(params, 1).unwrap();
        let mut loud = [1.0];
        gate.process_interleaved(&mut loud);
        assert!(gate.is_open());
        let mut just_below = [0.11];
        gate.process_interleaved(&mut just_below);
        assert!(gate.is_open());
        let mut quiet = [0.01];
        gate.process_interleaved(&mut quiet);
        assert!(!gate.is_open());
    }

    #[test]
    fn peak_limiter_enforces_declared_sample_ceiling_and_repairs_nonfinite() {
        let limiter = PeakLimiter::new(LimiterParams { ceiling_db: -1.0 }).unwrap();
        let mut samples = [2.0, -2.0, f32::NAN, f32::INFINITY];
        limiter.process_interleaved(&mut samples);
        let ceiling = 10.0_f32.powf(-1.0 / 20.0);
        assert_eq!(samples, [ceiling, -ceiling, 0.0, 0.0]);
        assert!((limiter.ceiling_db() + 1.0).abs() < 1e-5);
    }

    #[test]
    fn delay_line_is_bounded_and_preserves_channel_order() {
        let mut delay = DelayLine::new(10.0, 1_000.0, 2).unwrap();
        delay.set_delay_ms(2.0).unwrap();
        let mut samples = [1.0, 10.0, 2.0, 20.0, 3.0, 30.0];
        delay.process_interleaved(&mut samples);
        assert_eq!(samples, [0.0, 0.0, 0.0, 0.0, 1.0, 10.0]);
        assert!(matches!(
            delay.set_delay_ms(12.0),
            Err(DelayError::InvalidDelay)
        ));
        delay.reset();
        let mut zero_delay = [4.0, 5.0];
        delay.set_delay_ms(0.0).unwrap();
        delay.process_interleaved(&mut zero_delay);
        assert_eq!(zero_delay, [4.0, 5.0]);
    }
}
