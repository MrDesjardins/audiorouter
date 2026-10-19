//! Fun Voice Changer (DSP-20): pitch correction, pitch and formant shifting
//! and character effects for one voice.
//!
//! The voice is processed in mono. Every buffer is allocated by
//! [`VoiceChanger::new`]; `process_block` performs no allocation, locking or
//! I/O. The output is delayed by exactly [`VOICE_CHANGER_LATENCY_SAMPLES`]
//! whatever the settings (the pitch core uses 976 samples and the safety
//! limiter's look-ahead the other 48), so switching presets never moves the
//! voice in time. With the pitch stages on, TD-PSOLA places each grain up to
//! about half a period from its source, so voiced audio is aligned to within
//! that fraction of a period.

use crate::{Biquad, BiquadParams, FilterKind, LimiterParams, PeakLimiter};
use microfft::real::rfft_2048;

/// Fixed output delay of [`VoiceChanger`], in samples (21.3 ms at 48 kHz).
pub const VOICE_CHANGER_LATENCY_SAMPLES: usize = 1_024;
/// Frames per `process_block` call.
pub const VOICE_CHANGER_BLOCK_FRAMES: usize = 128;

const SAMPLE_RATE: f32 = 48_000.0;
const LIMITER_LOOKAHEAD: usize = 48;
/// Delay of the pitch core; the limiter's look-ahead makes up the rest.
const CORE_DELAY: usize = VOICE_CHANGER_LATENCY_SAMPLES - LIMITER_LOOKAHEAD;
const RING: usize = 4_096;
const RING_MASK: usize = RING - 1;
const ANALYSIS_WINDOW: usize = 1_024;
const ANALYSIS_HOP: usize = 256;
const FFT_SIZE: usize = 2_048;
/// Shortest analysed period (1 kHz) and longest (70 Hz), in samples.
const MIN_LAG: usize = 48;
const MAX_LAG: usize = 686;
/// Grain spacing used for unvoiced sound, which has no pitch (5 ms).
const UNVOICED_PERIOD: f32 = 240.0;
const MAX_MARKS: usize = 64;
/// Stage, preset and enable crossfades (30 ms).
const FADE_STEP: f32 = 1.0 / 1_440.0;
/// Fade-in after a reset (10 ms).
const RESET_FADE_STEP: f32 = 1.0 / 480.0;
/// One-pole parameter glide per block: 1 - exp(-128 / 1440), a 30 ms
/// time constant.
const BLOCK_GLIDE: f32 = 0.085_06;
const LPC_ORDER: usize = 16;
const LPC_WINDOW: usize = 512;
const MOD_RING: usize = 2_048;
const FDN_LINES: usize = 8;
const FDN_CAPACITY: usize = 2_400;
/// Mutually prime feedback-delay line lengths (23–47 ms at full size).
const FDN_LENGTHS: [f32; FDN_LINES] = [
    1_123.0, 1_291.0, 1_453.0, 1_621.0, 1_789.0, 1_951.0, 2_111.0, 2_273.0,
];
const FDN_SIGNS: [f32; FDN_LINES] = [1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0];
/// A target note changes only once the voice is this many semitones closer
/// to the new note than to the held one (30 cents past the midpoint).
const NOTE_HYSTERESIS: f32 = 0.6;
/// Smallest value kept in feedback paths; below it, state snaps to zero.
const DENORMAL: f32 = 1.0e-20;

/// Notes of a pitch-correction scale, in semitones above the key.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VoiceScale {
    #[default]
    Chromatic,
    Major,
    Minor,
    MajorPentatonic,
    MinorPentatonic,
    Blues,
    /// Only the key note, at the octave nearest the voice (robot voice).
    Monotone,
}

impl VoiceScale {
    fn steps(self) -> &'static [u8] {
        match self {
            Self::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Self::Major => &[0, 2, 4, 5, 7, 9, 11],
            Self::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Self::MajorPentatonic => &[0, 2, 4, 7, 9],
            Self::MinorPentatonic => &[0, 3, 5, 7, 10],
            Self::Blues => &[0, 3, 5, 6, 7, 10],
            Self::Monotone => &[0],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ModulationType {
    #[default]
    Chorus,
    Flanger,
}

/// Settings of one Fun Voice Changer node. [`Default`] is the neutral
/// setting: every effect off, so the output is the input, delayed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceChangerParams {
    pub gate_db: f32,
    pub correct_enabled: bool,
    /// Key of the scale, 0 = C … 11 = B.
    pub key: u8,
    pub scale: VoiceScale,
    pub retune_ms: f32,
    pub humanize_percent: f32,
    pub shift_enabled: bool,
    pub pitch_semitones: f32,
    pub pitch_cents: f32,
    pub formant_semitones: f32,
    pub ring_enabled: bool,
    pub ring_hz: f32,
    pub ring_depth_percent: f32,
    pub crush_enabled: bool,
    pub crush_bits: u8,
    pub crush_downsample: u8,
    pub drive_enabled: bool,
    pub drive_percent: f32,
    pub whisper_percent: f32,
    pub tone_enabled: bool,
    pub low_cut_hz: f32,
    pub high_cut_hz: f32,
    pub peak_hz: f32,
    pub peak_db: f32,
    pub mod_enabled: bool,
    pub mod_type: ModulationType,
    pub mod_rate_hz: f32,
    pub mod_depth_percent: f32,
    pub mod_feedback_percent: f32,
    pub space_enabled: bool,
    pub space_size_percent: f32,
    pub space_decay_ms: f32,
    pub space_level_percent: f32,
    pub mix_percent: f32,
    pub output_db: f32,
}

impl Default for VoiceChangerParams {
    fn default() -> Self {
        Self {
            gate_db: -90.0,
            correct_enabled: false,
            key: 0,
            scale: VoiceScale::Chromatic,
            retune_ms: 50.0,
            humanize_percent: 0.0,
            shift_enabled: false,
            pitch_semitones: 0.0,
            pitch_cents: 0.0,
            formant_semitones: 0.0,
            ring_enabled: false,
            ring_hz: 50.0,
            ring_depth_percent: 100.0,
            crush_enabled: false,
            crush_bits: 16,
            crush_downsample: 1,
            drive_enabled: false,
            drive_percent: 0.0,
            whisper_percent: 0.0,
            tone_enabled: false,
            low_cut_hz: 20.0,
            high_cut_hz: 20_000.0,
            peak_hz: 1_000.0,
            peak_db: 0.0,
            mod_enabled: false,
            mod_type: ModulationType::Chorus,
            mod_rate_hz: 0.5,
            mod_depth_percent: 50.0,
            mod_feedback_percent: 0.0,
            space_enabled: false,
            space_size_percent: 50.0,
            space_decay_ms: 1_000.0,
            space_level_percent: 30.0,
            mix_percent: 100.0,
            output_db: 0.0,
        }
    }
}

fn bounded(value: f32, minimum: f32, maximum: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        fallback
    }
}

impl VoiceChangerParams {
    /// The same settings with every value inside its range; a non-finite
    /// value takes the neutral default. The backend validates the ranges
    /// first; this keeps the audio path safe regardless.
    pub fn sanitized(self) -> Self {
        let neutral = Self::default();
        Self {
            gate_db: bounded(self.gate_db, -90.0, -20.0, neutral.gate_db),
            key: self.key.min(11),
            retune_ms: bounded(self.retune_ms, 0.0, 400.0, neutral.retune_ms),
            humanize_percent: bounded(self.humanize_percent, 0.0, 100.0, 0.0),
            pitch_semitones: bounded(self.pitch_semitones, -12.0, 12.0, 0.0),
            pitch_cents: bounded(self.pitch_cents, -100.0, 100.0, 0.0),
            formant_semitones: bounded(self.formant_semitones, -12.0, 12.0, 0.0),
            ring_hz: bounded(self.ring_hz, 20.0, 2_000.0, neutral.ring_hz),
            ring_depth_percent: bounded(self.ring_depth_percent, 0.0, 100.0, 100.0),
            crush_bits: self.crush_bits.clamp(4, 16),
            crush_downsample: self.crush_downsample.clamp(1, 12),
            drive_percent: bounded(self.drive_percent, 0.0, 100.0, 0.0),
            whisper_percent: bounded(self.whisper_percent, 0.0, 100.0, 0.0),
            low_cut_hz: bounded(self.low_cut_hz, 20.0, 2_000.0, neutral.low_cut_hz),
            high_cut_hz: bounded(self.high_cut_hz, 1_000.0, 20_000.0, neutral.high_cut_hz),
            peak_hz: bounded(self.peak_hz, 200.0, 8_000.0, neutral.peak_hz),
            peak_db: bounded(self.peak_db, -12.0, 12.0, 0.0),
            mod_rate_hz: bounded(self.mod_rate_hz, 0.05, 5.0, neutral.mod_rate_hz),
            mod_depth_percent: bounded(self.mod_depth_percent, 0.0, 100.0, 50.0),
            mod_feedback_percent: bounded(self.mod_feedback_percent, -90.0, 90.0, 0.0),
            space_size_percent: bounded(self.space_size_percent, 0.0, 100.0, 50.0),
            space_decay_ms: bounded(self.space_decay_ms, 200.0, 4_000.0, 1_000.0),
            space_level_percent: bounded(self.space_level_percent, 0.0, 100.0, 30.0),
            mix_percent: bounded(self.mix_percent, 0.0, 100.0, 100.0),
            output_db: bounded(self.output_db, -24.0, 12.0, 0.0),
            ..self
        }
    }

    fn pitch_active(&self) -> bool {
        self.correct_enabled || self.shift_enabled
    }
}

/// Live pitch reading for the inspector's tuner strip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VoiceChangerTelemetry {
    pub voiced: bool,
    /// Detected fundamental, or 0 when unvoiced or the pitch stages are off.
    pub f0_hz: f32,
    /// Nearest MIDI note to the detected pitch (69 = A4).
    pub note: u8,
    /// Distance of the detected pitch from that note.
    pub cents: f32,
}

/// A value that glides toward its target once per block and can be read
/// linearly interpolated inside the block, so gains never step.
#[derive(Clone, Copy, Debug)]
struct Glide {
    value: f32,
    previous: f32,
}

impl Glide {
    fn new(value: f32) -> Self {
        Self {
            value,
            previous: value,
        }
    }

    fn step(&mut self, target: f32) {
        self.previous = self.value;
        let difference = target - self.value;
        self.value = if difference.abs() <= 1.0e-6 * (1.0 + target.abs()) {
            target
        } else {
            self.value + difference * BLOCK_GLIDE
        };
    }

    fn snap(&mut self, target: f32) {
        self.value = target;
        self.previous = target;
    }

    fn at(&self, frame: usize) -> f32 {
        self.previous
            + (self.value - self.previous) * (frame + 1) as f32 / VOICE_CHANGER_BLOCK_FRAMES as f32
    }
}

/// Linear 0…1 crossfade for a stage's enable switch.
#[derive(Clone, Copy, Debug, Default)]
struct Fade {
    value: f32,
}

impl Fade {
    fn advance(&mut self, on: bool) -> f32 {
        self.value = if on {
            (self.value + FADE_STEP).min(1.0)
        } else {
            (self.value - FADE_STEP).max(0.0)
        };
        self.value
    }

    fn snap(&mut self, on: bool) {
        self.value = if on { 1.0 } else { 0.0 };
    }

    fn silent(&self) -> bool {
        self.value <= 0.0
    }
}

fn flush(value: f32) -> f32 {
    if value.abs() < DENORMAL {
        0.0
    } else {
        value
    }
}

fn ring_read(ring: &[f32; RING], position: f64) -> f32 {
    let floor = position.floor();
    let fraction = (position - floor) as f32;
    let index = floor as i64 as usize;
    let a = ring[index & RING_MASK];
    let b = ring[index.wrapping_add(1) & RING_MASK];
    a + (b - a) * fraction
}

/// McLeod pitch method (normalized square difference) with voicing
/// hysteresis and octave-jump rejection.
#[derive(Debug)]
struct PitchTracker {
    signal: Box<[f32; FFT_SIZE]>,
    power: Box<[f32; FFT_SIZE]>,
    nsdf: Box<[f32; MAX_LAG + 2]>,
    voiced: bool,
    period: f32,
    clarity: f32,
    history: [f32; 5],
    history_len: usize,
    history_next: usize,
    rejections: u8,
}

impl PitchTracker {
    fn new() -> Self {
        Self {
            signal: Box::new([0.0; FFT_SIZE]),
            power: Box::new([0.0; FFT_SIZE]),
            nsdf: Box::new([0.0; MAX_LAG + 2]),
            voiced: false,
            period: UNVOICED_PERIOD,
            clarity: 0.0,
            history: [0.0; 5],
            history_len: 0,
            history_next: 0,
            rejections: 0,
        }
    }

    fn reset(&mut self) {
        self.voiced = false;
        self.period = UNVOICED_PERIOD;
        self.clarity = 0.0;
        self.history_len = 0;
        self.history_next = 0;
        self.rejections = 0;
    }

    fn f0_hz(&self) -> f32 {
        if self.voiced {
            SAMPLE_RATE / self.period
        } else {
            0.0
        }
    }

    /// Analyse the window ending at `end` (inclusive). `floor` is the level
    /// below which the window counts as silence.
    fn analyse(&mut self, ring: &[f32; RING], end: u64, floor: f32) {
        let start = end as usize + 1 + RING - ANALYSIS_WINDOW;
        let mut mean = 0.0_f32;
        for index in 0..ANALYSIS_WINDOW {
            let value = ring[(start + index) & RING_MASK];
            self.signal[index] = value;
            mean += value;
        }
        mean /= ANALYSIS_WINDOW as f32;
        let mut energy = 0.0_f32;
        for value in &mut self.signal[..ANALYSIS_WINDOW] {
            *value -= mean;
            energy += *value * *value;
        }
        self.signal[ANALYSIS_WINDOW..].fill(0.0);
        let rms = (energy / ANALYSIS_WINDOW as f32).sqrt();
        let estimate = if rms > floor {
            self.estimate(energy)
        } else {
            None
        };
        self.accept(estimate);
    }

    /// Returns (period in samples, clarity) of the strongest periodicity.
    fn estimate(&mut self, energy: f32) -> Option<(f32, f32)> {
        // Autocorrelation by Wiener–Khinchin: the forward real FFT of the
        // (real, even) power spectrum is N times the autocorrelation.
        let squares: [f32; ANALYSIS_WINDOW] = {
            let mut squares = [0.0_f32; ANALYSIS_WINDOW];
            for (square, value) in squares.iter_mut().zip(self.signal.iter()) {
                *square = value * value;
            }
            squares
        };
        let spectrum = rfft_2048(&mut self.signal);
        let dc = spectrum[0].re * spectrum[0].re;
        let nyquist = spectrum[0].im * spectrum[0].im;
        self.power[0] = dc;
        self.power[FFT_SIZE / 2] = nyquist;
        for (bin, value) in spectrum.iter().enumerate().skip(1) {
            let value = value.norm_sqr();
            self.power[bin] = value;
            self.power[FFT_SIZE - bin] = value;
        }
        let correlation = rfft_2048(&mut self.power);
        let scale = 1.0 / FFT_SIZE as f32;
        // m(τ) = Σ x[j]² + x[j+τ]² over the overlap, updated incrementally.
        let mut m = 2.0 * energy;
        for lag in 0..=MAX_LAG + 1 {
            if lag > 0 {
                m -= squares[lag - 1] + squares[ANALYSIS_WINDOW - lag];
            }
            self.nsdf[lag] = if m > 1.0e-12 {
                2.0 * correlation[lag].re * scale / m
            } else {
                0.0
            };
        }
        // Key maxima: the highest point of each positive lobe after the
        // first negative-going zero crossing.
        let mut best: Option<(usize, f32)> = None;
        let mut highest = 0.0_f32;
        let mut keys = [(0_usize, 0.0_f32); 24];
        let mut key_count = 0;
        let mut lag = 1;
        while lag <= MAX_LAG && self.nsdf[lag] > 0.0 {
            lag += 1;
        }
        let mut lobe: Option<(usize, f32)> = None;
        while lag <= MAX_LAG {
            let value = self.nsdf[lag];
            if value > 0.0 {
                if lobe.is_none_or(|(_, peak)| value > peak) {
                    lobe = Some((lag, value));
                }
            } else if let Some(peak) = lobe.take() {
                if key_count < keys.len() {
                    keys[key_count] = peak;
                    key_count += 1;
                }
            }
            lag += 1;
        }
        if let Some(peak) = lobe {
            if key_count < keys.len() {
                keys[key_count] = peak;
                key_count += 1;
            }
        }
        for &(_, value) in &keys[..key_count] {
            highest = highest.max(value);
        }
        for &(lag, value) in &keys[..key_count] {
            if lag >= MIN_LAG && value >= 0.9 * highest {
                best = Some((lag, value));
                break;
            }
        }
        let (lag, value) = best?;
        // Parabolic interpolation of the peak.
        let left = self.nsdf[lag - 1];
        let right = self.nsdf[lag + 1];
        let denominator = left - 2.0 * value + right;
        let offset = if denominator.abs() > 1.0e-9 {
            (0.5 * (left - right) / denominator).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let clarity = value - 0.25 * (left - right) * offset;
        Some((lag as f32 + offset, clarity.min(1.0)))
    }

    fn accept(&mut self, estimate: Option<(f32, f32)>) {
        let Some((period, clarity)) = estimate else {
            self.voiced = false;
            self.clarity = 0.0;
            return;
        };
        self.clarity = clarity;
        let threshold = if self.voiced { 0.6 } else { 0.75 };
        if clarity < threshold {
            self.voiced = false;
            return;
        }
        if self.history_len >= 3 {
            let mut sorted = [0.0_f32; 5];
            sorted[..self.history_len].copy_from_slice(&self.history[..self.history_len]);
            sorted[..self.history_len].sort_by(f32::total_cmp);
            let median = sorted[self.history_len / 2];
            if (period / median).log2().abs() > 0.8 && self.rejections < 3 {
                // Keep the previous estimate through a suspected octave error.
                self.rejections += 1;
                return;
            }
        }
        self.rejections = 0;
        self.history[self.history_next] = period;
        self.history_next = (self.history_next + 1) % self.history.len();
        self.history_len = (self.history_len + 1).min(self.history.len());
        self.period = period;
        self.voiced = true;
    }
}

/// Target-note choice and the gliding correction for pitch correction.
#[derive(Clone, Copy, Debug, Default)]
struct Corrector {
    note: Option<f32>,
    correction: f32,
    held_hops: u32,
}

impl Corrector {
    fn nearest(scale: VoiceScale, key: u8, midi: f32) -> f32 {
        let mut best = f32::MAX;
        let mut best_note = midi.round();
        let octave = (midi / 12.0).floor() as i32;
        for octave in octave - 1..=octave + 1 {
            for &step in scale.steps() {
                let note = (octave * 12 + i32::from(key) + i32::from(step)) as f32;
                let distance = (midi - note).abs();
                if distance < best {
                    best = distance;
                    best_note = note;
                }
            }
        }
        best_note
    }

    /// Update once per analysis hop. Returns the correction in semitones.
    fn update(&mut self, params: &VoiceChangerParams, midi: Option<f32>) -> f32 {
        let target = match (params.correct_enabled, midi) {
            (true, Some(midi)) => {
                let nearest = Self::nearest(params.scale, params.key, midi);
                let note = match self.note {
                    Some(held)
                        if held != nearest
                            && (midi - held).abs() < (midi - nearest).abs() + NOTE_HYSTERESIS =>
                    {
                        held
                    }
                    _ => nearest,
                };
                if self.note == Some(note) {
                    self.held_hops = self.held_hops.saturating_add(1);
                } else {
                    self.note = Some(note);
                    self.held_hops = 0;
                }
                note - midi
            }
            (true, None) => {
                // Unvoiced: keep the last correction; it does not affect
                // noise, which is never pitch shifted.
                return self.correction;
            }
            (false, _) => {
                self.note = None;
                0.0
            }
        };
        let held_ms = self.held_hops as f32 * ANALYSIS_HOP as f32 / 48.0;
        let humanize = params.humanize_percent / 100.0;
        let mut time_ms = params.retune_ms;
        if params.correct_enabled && held_ms > 250.0 {
            time_ms = time_ms * (1.0 + 4.0 * humanize) + 60.0 * humanize;
        }
        if time_ms <= 0.0 {
            self.correction = target;
        } else {
            let coefficient = 1.0 - (-(ANALYSIS_HOP as f32) / (time_ms * 48.0)).exp();
            self.correction += (target - self.correction) * coefficient;
        }
        self.correction
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Mark {
    position: u64,
    period: f32,
    voiced: bool,
}

/// Time-domain pitch-synchronous overlap-add with formant resampling.
#[derive(Debug)]
struct Psola {
    marks: [Mark; MAX_MARKS],
    mark_count: usize,
    mark_next: usize,
    last_mark: u64,
    next_synthesis: f64,
    accumulator: Box<[f32; RING]>,
    weights: Box<[f32; RING]>,
    warmup: usize,
    /// Window weight per output sample, averaged over about 10 ms.
    density: f32,
}

impl Psola {
    fn new() -> Self {
        Self {
            marks: [Mark::default(); MAX_MARKS],
            mark_count: 0,
            mark_next: 0,
            last_mark: 0,
            next_synthesis: 0.0,
            accumulator: Box::new([0.0; RING]),
            weights: Box::new([0.0; RING]),
            warmup: 0,
            density: 1.0,
        }
    }

    /// Start (or restart) at input sample `now`.
    fn start(&mut self, now: u64) {
        self.mark_count = 0;
        self.mark_next = 0;
        self.last_mark = now.saturating_sub((CORE_DELAY + 512) as u64);
        self.next_synthesis = -1.0;
        self.accumulator.fill(0.0);
        self.weights.fill(0.0);
        self.warmup = CORE_DELAY + 512;
        self.density = 1.0;
    }

    fn push_mark(&mut self, mark: Mark) {
        self.marks[self.mark_next] = mark;
        self.mark_next = (self.mark_next + 1) % MAX_MARKS;
        self.mark_count = (self.mark_count + 1).min(MAX_MARKS);
        self.last_mark = mark.position;
    }

    /// Place analysis marks one period apart, on the waveform peak while
    /// voiced, as far as the input `now` allows.
    fn place_marks(&mut self, ring: &[f32; RING], now: u64, voiced: bool, period: f32) {
        let period = if voiced { period } else { UNVOICED_PERIOD };
        let reach = if voiced { (period * 0.25) as u64 } else { 0 };
        loop {
            let candidate = self.last_mark + period.round().max(1.0) as u64;
            if candidate + reach > now {
                break;
            }
            let position = if voiced {
                let earliest = (self.last_mark + (period * 0.5) as u64).max(candidate - reach);
                let mut best = earliest;
                let mut best_value = f32::MIN;
                for position in earliest..=candidate + reach {
                    let value = ring[position as usize & RING_MASK];
                    if value > best_value {
                        best_value = value;
                        best = position;
                    }
                }
                best
            } else {
                candidate
            };
            self.push_mark(Mark {
                position,
                period,
                voiced,
            });
        }
    }

    /// The mark nearest `target` whose grain fits in the input up to `now`.
    fn choose_mark(&self, target: f64, now: u64, reach: impl Fn(&Mark) -> f32) -> Option<Mark> {
        let mut best: Option<Mark> = None;
        let mut best_distance = f64::MAX;
        for offset in 0..self.mark_count {
            let index = (self.mark_next + MAX_MARKS - 1 - offset) % MAX_MARKS;
            let mark = self.marks[index];
            let reach = f64::from(reach(&mark));
            if mark.position as f64 + reach > now as f64 {
                continue;
            }
            let distance = (mark.position as f64 - target).abs();
            if distance < best_distance {
                best_distance = distance;
                best = Some(mark);
            } else if (mark.position as f64) < target {
                break;
            }
        }
        best
    }

    /// Place every grain due before output position `out` is emitted, then
    /// return that position's sample.
    fn render(&mut self, ring: &[f32; RING], now: u64, out: u64, ratio: f32, formant: f32) -> f32 {
        let place_reach = (CORE_DELAY as f32 / (1.0 + formant)).floor();
        if self.next_synthesis < 0.0 {
            // First grain: start on a mark so ratio 1 reproduces the input.
            let target = out as f64 + f64::from(place_reach);
            self.next_synthesis = self
                .choose_mark(target, now, |_| 0.0)
                .map_or(target, |mark| mark.position as f64)
                .max(out as f64 + 1.0);
        }
        while self.next_synthesis <= out as f64 + f64::from(place_reach) {
            let synthesis = self.next_synthesis;
            // A grain spans one source period either side of its mark
            // (shortened if the delay budget requires it); resampling it by
            // the formant factor makes it 1/formant as long at the output.
            let source_half = |mark: &Mark| mark.period.min(formant * place_reach).max(16.0);
            let Some(mark) = self.choose_mark(synthesis, now, source_half) else {
                // No usable mark yet: skip ahead one unvoiced period.
                self.next_synthesis += f64::from(UNVOICED_PERIOD);
                continue;
            };
            let half = source_half(&mark) / formant;
            let center = synthesis.round() as i64;
            let span = half as i64;
            let step = std::f32::consts::PI / half;
            for offset in -span..=span {
                let position = center + offset;
                if position < out as i64 {
                    continue;
                }
                let weight = 0.5 + 0.5 * (step * offset as f32).cos();
                let source = mark.position as f64 + offset as f64 * f64::from(formant);
                let value = ring_read(ring, source);
                let index = position as usize & RING_MASK;
                self.accumulator[index] += weight * value;
                self.weights[index] += weight;
            }
            let grain_ratio = if mark.voiced { ratio } else { 1.0 };
            self.next_synthesis = if (grain_ratio - 1.0).abs() < 1.0e-4 && formant == 1.0 {
                // Unshifted: follow the marks so grains sit where they came from.
                mark.position as f64 + f64::from(mark.period)
            } else {
                synthesis + f64::from(mark.period / grain_ratio)
            }
            .max(synthesis + 1.0);
        }
        let index = out as usize & RING_MASK;
        // Normalise by the average grain overlap, not the instantaneous one:
        // dividing by a weight that ripples within each period would reshape
        // the waveform (and the formants). Unshifted grains are copies of one
        // another and add in amplitude; shifted ones partly cancel and add
        // closer to in power, so the exponent follows the shift.
        self.density += (self.weights[index] - self.density) * (1.0 / 480.0);
        let shift = (ratio.log2().abs() * 2.0).min(1.0);
        let value = self.accumulator[index] / self.density.max(0.25).powf(1.0 - 0.5 * shift);
        self.accumulator[index] = 0.0;
        self.weights[index] = 0.0;
        value
    }
}

/// Order-16 linear prediction for the whisper effect: the voice's spectral
/// envelope excited by noise of the residual's level.
#[derive(Debug)]
struct Whisper {
    window: Box<[f32; LPC_WINDOW]>,
    history: Box<[f32; LPC_WINDOW]>,
    written: usize,
    coefficients: [f32; LPC_ORDER],
    input: [f32; LPC_ORDER],
    output: [f32; LPC_ORDER],
    level: f32,
    noise: u32,
}

impl Whisper {
    fn new() -> Self {
        let mut window = Box::new([0.0; LPC_WINDOW]);
        for (index, value) in window.iter_mut().enumerate() {
            *value =
                0.5 - 0.5 * (std::f32::consts::TAU * index as f32 / (LPC_WINDOW - 1) as f32).cos();
        }
        Self {
            window,
            history: Box::new([0.0; LPC_WINDOW]),
            written: 0,
            coefficients: [0.0; LPC_ORDER],
            input: [0.0; LPC_ORDER],
            output: [0.0; LPC_ORDER],
            level: 0.0,
            noise: 0x2545_f491,
        }
    }

    fn reset(&mut self) {
        self.history.fill(0.0);
        self.written = 0;
        self.coefficients = [0.0; LPC_ORDER];
        self.input = [0.0; LPC_ORDER];
        self.output = [0.0; LPC_ORDER];
        self.level = 0.0;
    }

    fn analyse(&mut self) {
        let mut correlation = [0.0_f64; LPC_ORDER + 1];
        let mut windowed = [0.0_f32; LPC_WINDOW];
        for (index, value) in windowed.iter_mut().enumerate() {
            *value = self.history[(self.written + index) % LPC_WINDOW] * self.window[index];
        }
        for (lag, sum) in correlation.iter_mut().enumerate() {
            *sum = windowed[lag..]
                .iter()
                .zip(windowed.iter())
                .map(|(a, b)| f64::from(*a) * f64::from(*b))
                .sum();
        }
        if correlation[0] < 1.0e-10 {
            self.coefficients = [0.0; LPC_ORDER];
            return;
        }
        // White-noise correction keeps the recursion well conditioned.
        correlation[0] *= 1.0001;
        let mut a = [0.0_f64; LPC_ORDER + 1];
        let mut error = correlation[0];
        for order in 1..=LPC_ORDER {
            let mut accumulator = correlation[order];
            for index in 1..order {
                accumulator -= a[index] * correlation[order - index];
            }
            let reflection = accumulator / error;
            if !reflection.is_finite() || reflection.abs() >= 1.0 {
                break;
            }
            let previous = a;
            a[order] = reflection;
            for index in 1..order {
                a[index] = previous[index] - reflection * previous[order - index];
            }
            error *= 1.0 - reflection * reflection;
            if error <= 0.0 {
                break;
            }
        }
        for (coefficient, value) in self.coefficients.iter_mut().zip(&a[1..]) {
            *coefficient = *value as f32;
        }
    }

    fn noise(&mut self) -> f32 {
        self.noise ^= self.noise << 13;
        self.noise ^= self.noise >> 17;
        self.noise ^= self.noise << 5;
        // Uniform in [-1, 1) scaled to unit variance.
        (self.noise as f32 / u32::MAX as f32 * 2.0 - 1.0) * 1.732_050_8
    }

    fn process(&mut self, sample: f32) -> f32 {
        self.history[self.written] = sample;
        self.written = (self.written + 1) % LPC_WINDOW;
        if self.written % ANALYSIS_HOP == 0 {
            self.analyse();
        }
        let prediction: f32 = self
            .coefficients
            .iter()
            .zip(self.input.iter())
            .map(|(a, x)| a * x)
            .sum();
        let residual = sample - prediction;
        self.input.rotate_right(1);
        self.input[0] = sample;
        // About 10 ms residual-energy follower.
        self.level = flush(self.level + (residual * residual - self.level) * 0.002_08);
        let excitation = self.noise() * self.level.sqrt();
        let feedback: f32 = self
            .coefficients
            .iter()
            .zip(self.output.iter())
            .map(|(a, y)| a * y)
            .sum();
        let mut value = excitation + feedback;
        if !value.is_finite() || value.abs() > 4.0 {
            self.output = [0.0; LPC_ORDER];
            value = 0.0;
        }
        self.output.rotate_right(1);
        self.output[0] = flush(value);
        value
    }
}

/// Chorus or flanger: one modulated fractional delay.
#[derive(Debug)]
struct Modulation {
    line: Box<[f32; MOD_RING]>,
    written: usize,
    phase: f32,
    last: f32,
    dirty: bool,
}

impl Modulation {
    fn new() -> Self {
        Self {
            line: Box::new([0.0; MOD_RING]),
            written: 0,
            phase: 0.0,
            last: 0.0,
            dirty: false,
        }
    }

    fn reset(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        self.line.fill(0.0);
        self.written = 0;
        self.phase = 0.0;
        self.last = 0.0;
    }

    fn process(
        &mut self,
        sample: f32,
        kind: ModulationType,
        rate: f32,
        depth: f32,
        feedback: f32,
    ) -> f32 {
        self.dirty = true;
        self.phase += rate / SAMPLE_RATE;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let sweep = 0.5 + 0.5 * (std::f32::consts::TAU * self.phase).sin();
        let (base, range, feedback, wet_gain) = match kind {
            ModulationType::Chorus => (720.0, 384.0, 0.0, 0.707),
            ModulationType::Flanger => (96.0, 96.0, feedback, 0.5),
        };
        let delay = base + range * depth * sweep;
        self.line[self.written] = flush(sample + feedback * self.last);
        let position = self.written as f32 + MOD_RING as f32 - delay;
        let floor = position.floor();
        let fraction = position - floor;
        let index = floor as usize;
        let a = self.line[index % MOD_RING];
        let b = self.line[(index + 1) % MOD_RING];
        let wet = a + (b - a) * fraction;
        self.last = flush(wet);
        self.written = (self.written + 1) % MOD_RING;
        wet_gain * (sample + wet)
    }
}

/// Small room or cave: an eight-line feedback delay network with a
/// Householder mix and damping in the loop.
#[derive(Debug)]
struct Space {
    lines: Box<[[f32; FDN_CAPACITY]; FDN_LINES]>,
    written: usize,
    lengths: [f32; FDN_LINES],
    gains: [f32; FDN_LINES],
    damping: [f32; FDN_LINES],
    dirty: bool,
}

impl Space {
    fn new() -> Self {
        Self {
            lines: Box::new([[0.0; FDN_CAPACITY]; FDN_LINES]),
            written: 0,
            lengths: FDN_LENGTHS,
            gains: [0.0; FDN_LINES],
            damping: [0.0; FDN_LINES],
            dirty: false,
        }
    }

    fn reset(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        for line in self.lines.iter_mut() {
            line.fill(0.0);
        }
        self.written = 0;
        self.damping = [0.0; FDN_LINES];
    }

    fn configure(&mut self, size_percent: f32, decay_ms: f32) {
        let scale = 0.35 + 0.65 * size_percent / 100.0;
        for ((length, gain), base) in self
            .lengths
            .iter_mut()
            .zip(&mut self.gains)
            .zip(FDN_LENGTHS)
        {
            // Glide line lengths so a size change does not click.
            *length += (base * scale - *length) * BLOCK_GLIDE;
            *gain = 10.0_f32.powf(-3.0 * *length / (decay_ms * 0.001 * SAMPLE_RATE));
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let mut values = [0.0_f32; FDN_LINES];
        self.dirty = true;
        let mut sum = 0.0;
        for (line, value) in values.iter_mut().enumerate() {
            let position = self.written as f32 + FDN_CAPACITY as f32 - self.lengths[line];
            let floor = position.floor();
            let fraction = position - floor;
            let index = floor as usize;
            let a = self.lines[line][index % FDN_CAPACITY];
            let b = self.lines[line][(index + 1) % FDN_CAPACITY];
            let read = a + (b - a) * fraction;
            self.damping[line] = flush(self.damping[line] + (read - self.damping[line]) * 0.65);
            *value = self.damping[line];
            sum += *value;
        }
        let mut wet = 0.0;
        for (line, value) in values.iter().enumerate() {
            let feedback = self.gains[line] * (value - 0.25 * sum);
            self.lines[line][self.written] = flush(feedback + 0.35 * FDN_SIGNS[line] * sample);
            wet += FDN_SIGNS[line] * value;
        }
        self.written = (self.written + 1) % FDN_CAPACITY;
        0.25 * wet
    }
}

/// The Fun Voice Changer processor (DSP-20).
///
/// ```
/// use audiorouter_dsp::voice_changer::{VoiceChanger, VoiceChangerParams};
/// let mut changer = VoiceChanger::new();
/// let mut block = [0.0_f32; 128];
/// changer.process_block(&VoiceChangerParams::default(), &mut block);
/// assert!(block.iter().all(|sample| sample.is_finite()));
/// ```
#[derive(Debug)]
pub struct VoiceChanger {
    raw: Box<[f32; RING]>,
    gated: Box<[f32; RING]>,
    written: u64,
    primed: bool,
    // Gate.
    gate_envelope: f32,
    gate_open: bool,
    gate_gain: f32,
    // Pitch.
    tracker: PitchTracker,
    corrector: Corrector,
    correction: f32,
    psola: Psola,
    psola_active: bool,
    pitch_fade: Fade,
    pitch: Glide,
    formant: Glide,
    // Character and effects.
    ring_fade: Fade,
    ring_hz: Glide,
    ring_depth: Glide,
    ring_phase: f32,
    crush_fade: Fade,
    crush_hold: f32,
    crush_count: u8,
    drive_fade: Fade,
    drive: Glide,
    whisper: Whisper,
    whisper_amount: Glide,
    tone_fade: Fade,
    tone: [Biquad; 3],
    tone_settings: [f32; 4],
    modulation: Modulation,
    mod_fade: Fade,
    mod_type: ModulationType,
    mod_rate: Glide,
    mod_depth: Glide,
    mod_feedback: Glide,
    space: Space,
    space_fade: Fade,
    space_level: Glide,
    // Output.
    mix: Glide,
    output_gain: Glide,
    limiter: PeakLimiter,
    reset_fade: f32,
    non_finite: u64,
}

fn tone_filters(low_cut: f32, high_cut: f32, peak_hz: f32, peak_db: f32) -> [BiquadParams; 3] {
    [
        BiquadParams {
            kind: FilterKind::HighPass,
            frequency_hz: low_cut,
            q: std::f32::consts::FRAC_1_SQRT_2,
            gain_db: 0.0,
            sample_rate: SAMPLE_RATE,
        },
        BiquadParams {
            kind: FilterKind::LowPass,
            frequency_hz: high_cut,
            q: std::f32::consts::FRAC_1_SQRT_2,
            gain_db: 0.0,
            sample_rate: SAMPLE_RATE,
        },
        BiquadParams {
            kind: FilterKind::Peaking,
            frequency_hz: peak_hz,
            q: 1.0,
            gain_db: peak_db,
            sample_rate: SAMPLE_RATE,
        },
    ]
}

impl Default for VoiceChanger {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceChanger {
    pub fn new() -> Self {
        let neutral = VoiceChangerParams::default();
        let [low, high, peak] = tone_filters(
            neutral.low_cut_hz,
            neutral.high_cut_hz,
            neutral.peak_hz,
            neutral.peak_db,
        );
        let filter = |params| Biquad::new(params, 1).expect("tone filter defaults are valid");
        Self {
            raw: Box::new([0.0; RING]),
            gated: Box::new([0.0; RING]),
            written: 0,
            primed: false,
            gate_envelope: 0.0,
            gate_open: true,
            gate_gain: 1.0,
            tracker: PitchTracker::new(),
            corrector: Corrector::default(),
            correction: 0.0,
            psola: Psola::new(),
            psola_active: false,
            pitch_fade: Fade::default(),
            pitch: Glide::new(0.0),
            formant: Glide::new(0.0),
            ring_fade: Fade::default(),
            ring_hz: Glide::new(neutral.ring_hz.ln()),
            ring_depth: Glide::new(1.0),
            ring_phase: 0.0,
            crush_fade: Fade::default(),
            crush_hold: 0.0,
            crush_count: 0,
            drive_fade: Fade::default(),
            drive: Glide::new(0.0),
            whisper: Whisper::new(),
            whisper_amount: Glide::new(0.0),
            tone_fade: Fade::default(),
            tone: [filter(low), filter(high), filter(peak)],
            tone_settings: [
                neutral.low_cut_hz.ln(),
                neutral.high_cut_hz.ln(),
                neutral.peak_hz.ln(),
                neutral.peak_db,
            ],
            modulation: Modulation::new(),
            mod_fade: Fade::default(),
            mod_type: ModulationType::Chorus,
            mod_rate: Glide::new(neutral.mod_rate_hz),
            mod_depth: Glide::new(0.5),
            mod_feedback: Glide::new(0.0),
            space: Space::new(),
            space_fade: Fade::default(),
            space_level: Glide::new(0.3),
            mix: Glide::new(1.0),
            output_gain: Glide::new(1.0),
            limiter: PeakLimiter::new_at_sample_rate(
                LimiterParams {
                    ceiling_db: -1.0,
                    lookahead_ms: 1.0,
                    release_ms: 50.0,
                },
                SAMPLE_RATE,
                1,
            )
            .expect("safety limiter settings are valid"),
            reset_fade: 1.0,
            non_finite: 0,
        }
    }

    /// Clear all audio state (allocation-free). The next block takes its
    /// settings directly and fades in over 10 ms, so no audio from before
    /// the reset is heard.
    pub fn reset(&mut self) {
        self.raw.fill(0.0);
        self.gated.fill(0.0);
        self.gate_envelope = 0.0;
        self.gate_open = true;
        self.gate_gain = 1.0;
        self.tracker.reset();
        self.corrector = Corrector::default();
        self.correction = 0.0;
        self.psola_active = false;
        self.ring_phase = 0.0;
        self.crush_hold = 0.0;
        self.crush_count = 0;
        self.whisper.reset();
        for filter in &mut self.tone {
            filter.reset();
        }
        self.modulation.reset();
        self.space.reset();
        self.limiter.reset();
        self.primed = false;
        self.reset_fade = 0.0;
    }

    /// Live pitch reading (control thread, between blocks).
    pub fn telemetry(&self) -> VoiceChangerTelemetry {
        if !self.psola_active || !self.tracker.voiced {
            return VoiceChangerTelemetry::default();
        }
        let f0 = self.tracker.f0_hz();
        let midi = 69.0 + 12.0 * (f0 / 440.0).log2();
        let note = midi.round();
        VoiceChangerTelemetry {
            voiced: true,
            f0_hz: f0,
            note: note.clamp(0.0, 127.0) as u8,
            cents: (midi - note) * 100.0,
        }
    }

    /// How many blocks produced a non-finite value and were silenced.
    pub fn non_finite_blocks(&self) -> u64 {
        self.non_finite
    }

    fn update_tone(&mut self, params: &VoiceChangerParams, snap: bool) {
        let targets = [
            params.low_cut_hz.ln(),
            params.high_cut_hz.ln(),
            params.peak_hz.ln(),
            params.peak_db,
        ];
        let mut changed = false;
        for (current, target) in self.tone_settings.iter_mut().zip(targets) {
            let next = if snap {
                target
            } else {
                let difference = target - *current;
                if difference.abs() < 1.0e-4 {
                    target
                } else {
                    *current + difference * BLOCK_GLIDE
                }
            };
            if next != *current {
                *current = next;
                changed = true;
            }
        }
        if changed {
            let settings = tone_filters(
                self.tone_settings[0].exp(),
                self.tone_settings[1].exp(),
                self.tone_settings[2].exp(),
                self.tone_settings[3],
            );
            let frames = if snap { 0 } else { VOICE_CHANGER_BLOCK_FRAMES };
            for (filter, settings) in self.tone.iter_mut().zip(settings) {
                // A rejected setting keeps the previous filter.
                let _ = filter.set_params_ramped(settings, frames);
            }
        }
    }

    fn update_settings(&mut self, params: &VoiceChangerParams) {
        let snap = !self.primed;
        let pitch_target = params.pitch_semitones + params.pitch_cents / 100.0;
        let pitch_target = if params.shift_enabled {
            pitch_target
        } else {
            0.0
        };
        let formant_target = if params.shift_enabled {
            params.formant_semitones
        } else {
            0.0
        };
        let ring_target = params.ring_hz.ln();
        let ring_depth = params.ring_depth_percent / 100.0;
        let drive = params.drive_percent / 100.0;
        let whisper = params.whisper_percent / 100.0;
        let mod_depth = params.mod_depth_percent / 100.0;
        let mod_feedback = params.mod_feedback_percent / 100.0;
        let space_level = params.space_level_percent / 100.0;
        let mix = params.mix_percent / 100.0;
        let output = 10.0_f32.powf(params.output_db / 20.0);
        let glides: [(&mut Glide, f32); 11] = [
            (&mut self.pitch, pitch_target),
            (&mut self.formant, formant_target),
            (&mut self.ring_hz, ring_target),
            (&mut self.ring_depth, ring_depth),
            (&mut self.drive, drive),
            (&mut self.whisper_amount, whisper),
            (&mut self.mod_rate, params.mod_rate_hz),
            (&mut self.mod_depth, mod_depth),
            (&mut self.mod_feedback, mod_feedback),
            (&mut self.space_level, space_level),
            (&mut self.mix, mix),
        ];
        for (glide, target) in glides {
            if snap {
                glide.snap(target);
            } else {
                glide.step(target);
            }
        }
        if snap {
            self.output_gain.snap(output);
            self.pitch_fade.snap(params.pitch_active());
            self.ring_fade.snap(params.ring_enabled);
            self.crush_fade.snap(params.crush_enabled);
            self.drive_fade.snap(params.drive_enabled);
            self.tone_fade.snap(params.tone_enabled);
            self.mod_fade.snap(params.mod_enabled);
            self.mod_type = params.mod_type;
            self.space_fade.snap(params.space_enabled);
            if params.pitch_active() {
                self.psola_active = true;
                self.psola.start(self.written);
            }
        } else {
            self.output_gain.step(output);
        }
        self.update_tone(params, snap);
        self.space
            .configure(params.space_size_percent, params.space_decay_ms);
        if snap {
            for _ in 0..64 {
                self.space
                    .configure(params.space_size_percent, params.space_decay_ms);
            }
        }
    }

    /// Process one 128-frame mono block in place. `params` is the node's
    /// current target; values glide toward it, so a changed setting or
    /// preset never clicks.
    pub fn process_block(
        &mut self,
        params: &VoiceChangerParams,
        samples: &mut [f32; VOICE_CHANGER_BLOCK_FRAMES],
    ) {
        let params = params.sanitized();
        self.update_settings(&params);
        self.primed = true;
        let gate_off = params.gate_db <= -89.9;
        let open_level = 10.0_f32.powf(params.gate_db / 20.0);
        let close_level = open_level * 0.707_946; // 3 dB hysteresis
        let shift_ratio_semitones = self.pitch.value;
        let formant = 2.0_f32.powf(self.formant.value / 12.0);
        let silence_floor = open_level.max(0.001);
        let pitch_wanted = params.pitch_active();
        let ring_increment = self.ring_hz.value.exp() / SAMPLE_RATE;
        let bits = f32::from(params.crush_bits);
        let crush_step = 2.0 / 2.0_f32.powf(bits);
        let mod_wanted = params.mod_enabled && self.mod_type == params.mod_type;
        for (frame, sample) in samples.iter_mut().enumerate() {
            let input = if sample.is_finite() { *sample } else { 0.0 };
            let now = self.written;
            let slot = now as usize & RING_MASK;
            self.raw[slot] = input;
            // 1. Gate.
            let level = input.abs();
            let coefficient = if level > self.gate_envelope {
                0.004_158 // 5 ms
            } else {
                0.000_173 // 120 ms
            };
            self.gate_envelope =
                flush(self.gate_envelope + (level - self.gate_envelope) * coefficient);
            let gated = if gate_off {
                self.gate_open = true;
                self.gate_gain = 1.0;
                input
            } else {
                if self.gate_open && self.gate_envelope < close_level {
                    self.gate_open = false;
                } else if !self.gate_open && self.gate_envelope > open_level {
                    self.gate_open = true;
                }
                let target = if self.gate_open { 1.0 } else { 0.0 };
                let coefficient = if self.gate_open { 0.004_158 } else { 0.000_173 };
                self.gate_gain = flush(self.gate_gain + (target - self.gate_gain) * coefficient);
                input * self.gate_gain
            };
            self.gated[slot] = gated;
            self.written += 1;
            let delayed = now.wrapping_sub(CORE_DELAY as u64);
            let dry_core = self.gated[delayed as usize & RING_MASK];
            let dry_raw = self.raw[delayed as usize & RING_MASK];
            // 2–4. Pitch analysis, correction and PSOLA.
            if pitch_wanted && !self.psola_active {
                self.psola_active = true;
                self.psola.start(now);
                self.tracker.reset();
                self.corrector = Corrector::default();
            }
            let mut core = dry_core;
            if self.psola_active {
                if (now + 1) % ANALYSIS_HOP as u64 == 0 {
                    self.tracker.analyse(&self.gated, now, silence_floor);
                    let midi = self
                        .tracker
                        .voiced
                        .then(|| 69.0 + 12.0 * (self.tracker.f0_hz() / 440.0).log2());
                    self.correction = self.corrector.update(&params, midi);
                }
                self.psola
                    .place_marks(&self.gated, now, self.tracker.voiced, self.tracker.period);
                let ratio = 2.0_f32.powf((shift_ratio_semitones + self.correction) / 12.0);
                let shifted = if now >= CORE_DELAY as u64 {
                    self.psola.render(&self.gated, now, delayed, ratio, formant)
                } else {
                    0.0
                };
                let fade = if self.psola.warmup > 0 {
                    self.psola.warmup -= 1;
                    self.pitch_fade.value
                } else {
                    self.pitch_fade.advance(pitch_wanted)
                };
                core = dry_core + fade * (shifted - dry_core);
                if !pitch_wanted && self.pitch_fade.silent() {
                    self.psola_active = false;
                }
            } else {
                self.pitch_fade.snap(false);
            }
            // 5. Character: ring modulator, bit crusher, saturation.
            let ring = self.ring_fade.advance(params.ring_enabled);
            if !self.ring_fade.silent() {
                self.ring_phase += ring_increment;
                if self.ring_phase >= 1.0 {
                    self.ring_phase -= 1.0;
                }
                let depth = ring * self.ring_depth.at(frame);
                let carrier = (std::f32::consts::TAU * self.ring_phase).sin();
                core *= 1.0 - depth + depth * carrier;
            }
            let crush = self.crush_fade.advance(params.crush_enabled);
            if !self.crush_fade.silent() {
                if self.crush_count == 0 {
                    self.crush_hold = (core / crush_step).round() * crush_step;
                }
                self.crush_count = (self.crush_count + 1) % params.crush_downsample.max(1);
                core += crush * (self.crush_hold - core);
            }
            let drive_fade = self.drive_fade.advance(params.drive_enabled);
            if !self.drive_fade.silent() {
                let drive_db = 36.0 * self.drive.at(frame);
                let pre = 10.0_f32.powf(drive_db / 20.0);
                let post = 10.0_f32.powf(-drive_db / 40.0);
                core += drive_fade * ((core * pre).tanh() * post - core);
            }
            // 6. Whisper.
            let whisper = self.whisper_amount.at(frame);
            if whisper > 0.0 {
                let breath = self.whisper.process(core);
                core += whisper * (breath - core);
            } else if self.whisper_amount.previous > 0.0 {
                self.whisper.reset();
            }
            // 7. Tone.
            let tone = self.tone_fade.advance(params.tone_enabled);
            if !self.tone_fade.silent() {
                let mut filtered = [core];
                for filter in &mut self.tone {
                    filter.process_interleaved(&mut filtered);
                }
                core += tone * (filtered[0] - core);
            }
            // 8. Modulation; a type change fades out, switches, fades in.
            let modulation = self.mod_fade.advance(mod_wanted);
            if self.mod_fade.silent() {
                if self.mod_type != params.mod_type {
                    self.mod_type = params.mod_type;
                    self.modulation.reset();
                }
            } else {
                let wet = self.modulation.process(
                    core,
                    self.mod_type,
                    self.mod_rate.at(frame),
                    self.mod_depth.at(frame),
                    self.mod_feedback.at(frame),
                );
                core += modulation * (wet - core);
            }
            // 9. Space.
            let space = self.space_fade.advance(params.space_enabled);
            if !self.space_fade.silent() {
                let wet = self.space.process(core);
                core += space * self.space_level.at(frame) * wet;
            }
            // 10. Mix and output gain (the limiter runs on the whole block).
            let mix = self.mix.at(frame);
            let mut output = (dry_raw + mix * (core - dry_raw)) * self.output_gain.at(frame);
            if self.reset_fade < 1.0 {
                self.reset_fade = (self.reset_fade + RESET_FADE_STEP).min(1.0);
                output *= self.reset_fade;
            }
            *sample = output;
        }
        if self.space_fade.silent() && !params.space_enabled {
            self.space.reset();
        }
        if self.mod_fade.silent() && !params.mod_enabled {
            self.modulation.reset();
        }
        self.limiter.process_channel(0, samples);
        if samples.iter().any(|sample| !sample.is_finite()) {
            self.non_finite += 1;
            samples.fill(0.0);
            self.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: usize = VOICE_CHANGER_BLOCK_FRAMES;

    /// Synthetic vowel: a Rosenberg glottal pulse train through three
    /// formant resonators (/a/: 700, 1220, 2600 Hz). `f0` may vary per sample.
    struct Vowel {
        phase: f32,
        state: [[f32; 2]; 3],
        formants: [(f32, f32); 3],
        scale: f32,
    }

    impl Vowel {
        fn new() -> Self {
            Self::with_formants([(700.0, 110.0), (1_220.0, 120.0), (2_600.0, 160.0)])
        }

        fn with_formants(formants: [(f32, f32); 3]) -> Self {
            Self {
                phase: 0.0,
                state: [[0.0; 2]; 3],
                formants,
                scale: 0.02,
            }
        }

        fn next(&mut self, f0: f32) -> f32 {
            self.phase += f0 / SAMPLE_RATE;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let phase = self.phase;
            let pulse = if phase < 0.4 {
                0.5 * (1.0 - (std::f32::consts::PI * phase / 0.4).cos())
            } else if phase < 0.56 {
                (std::f32::consts::PI * (phase - 0.4) / 0.32).cos()
            } else {
                0.0
            };
            let mut value = pulse - 0.3;
            for (state, (frequency, bandwidth)) in self.state.iter_mut().zip(self.formants) {
                let radius = (-std::f32::consts::PI * bandwidth / SAMPLE_RATE).exp();
                let theta = std::f32::consts::TAU * frequency / SAMPLE_RATE;
                let output = (1.0 - radius) * value + 2.0 * radius * theta.cos() * state[0]
                    - radius * radius * state[1];
                state[1] = state[0];
                state[0] = output;
                value = output;
            }
            value * self.scale * 40.0
        }
    }

    fn vowel(frames: usize, f0: impl Fn(usize) -> f32) -> Vec<f32> {
        let mut source = Vowel::new();
        let mut samples: Vec<f32> = (0..frames).map(|frame| source.next(f0(frame))).collect();
        // Peak at -6 dBFS, below the safety limiter's ceiling.
        let peak = samples
            .iter()
            .fold(0.0_f32, |peak, value| peak.max(value.abs()));
        for value in &mut samples {
            *value *= 0.5 / peak.max(1.0e-9);
        }
        samples
    }

    fn run(changer: &mut VoiceChanger, params: &VoiceChangerParams, input: &[f32]) -> Vec<f32> {
        let mut output = Vec::with_capacity(input.len());
        for chunk in input.chunks(BLOCK) {
            let mut block = [0.0_f32; BLOCK];
            block[..chunk.len()].copy_from_slice(chunk);
            changer.process_block(params, &mut block);
            output.extend_from_slice(&block[..chunk.len()]);
        }
        output
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
    }

    /// Fundamental of `samples` by the same MPM tracker, median over hops.
    fn measured_f0(samples: &[f32]) -> f32 {
        let mut tracker = PitchTracker::new();
        let mut ring = Box::new([0.0_f32; RING]);
        let mut estimates = Vec::new();
        for (index, value) in samples.iter().enumerate() {
            ring[index & RING_MASK] = *value;
            if index >= ANALYSIS_WINDOW && (index + 1) % ANALYSIS_HOP == 0 {
                tracker.analyse(&ring, index as u64, 0.001);
                if tracker.voiced {
                    estimates.push(tracker.f0_hz());
                }
            }
        }
        assert!(!estimates.is_empty(), "no voiced frames");
        estimates.sort_by(f32::total_cmp);
        estimates[estimates.len() / 2]
    }

    fn cents(a: f32, b: f32) -> f32 {
        1_200.0 * (a / b).log2()
    }

    #[test]
    fn detects_steady_vowels_within_one_percent() {
        for f0 in [90.0, 150.0, 220.0, 400.0] {
            let samples = vowel(24_000, |_| f0);
            let mut tracker = PitchTracker::new();
            let mut ring = Box::new([0.0_f32; RING]);
            let mut checked = 0;
            for (index, value) in samples.iter().enumerate() {
                ring[index & RING_MASK] = *value;
                if index >= 4_096 && (index + 1) % ANALYSIS_HOP == 0 {
                    tracker.analyse(&ring, index as u64, 0.001);
                    assert!(tracker.voiced, "{f0} Hz not voiced at {index}");
                    let error = (tracker.f0_hz() - f0).abs() / f0;
                    assert!(error < 0.01, "{f0} Hz read as {}", tracker.f0_hz());
                    checked += 1;
                }
            }
            assert!(checked > 50);
        }
    }

    #[test]
    fn noise_and_silence_are_unvoiced() {
        let mut tracker = PitchTracker::new();
        let mut ring = Box::new([0.0_f32; RING]);
        let mut seed = 0x1234_5678_u32;
        for index in 0..16_384 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            ring[index & RING_MASK] = (seed as f32 / u32::MAX as f32 - 0.5) * 0.4;
            if index >= 4_096 && (index + 1) % ANALYSIS_HOP == 0 {
                tracker.analyse(&ring, index as u64, 0.001);
                assert!(!tracker.voiced, "white noise voiced at {index}");
            }
        }
        ring.fill(0.0);
        tracker.analyse(&ring, 20_000, 0.001);
        assert!(!tracker.voiced);
    }

    #[test]
    fn neutral_settings_pass_the_input_through_delayed() {
        let input = vowel(9_600, |_| 140.0);
        let mut changer = VoiceChanger::new();
        let output = run(&mut changer, &VoiceChangerParams::default(), &input);
        for frame in VOICE_CHANGER_LATENCY_SAMPLES..input.len() {
            let expected = input[frame - VOICE_CHANGER_LATENCY_SAMPLES];
            assert!(
                (output[frame] - expected).abs() <= 1.0e-6,
                "frame {frame}: {} vs {expected}",
                output[frame]
            );
        }
        assert!(output[..VOICE_CHANGER_LATENCY_SAMPLES]
            .iter()
            .all(|s| *s == 0.0));
    }

    #[test]
    fn an_impulse_arrives_after_the_declared_latency_for_effects_without_pitch() {
        let mut settings = vec![VoiceChangerParams::default()];
        settings.push(VoiceChangerParams {
            tone_enabled: true,
            low_cut_hz: 300.0,
            high_cut_hz: 3_000.0,
            ..VoiceChangerParams::default()
        });
        settings.push(VoiceChangerParams {
            ring_enabled: true,
            ring_hz: 50.0,
            ..VoiceChangerParams::default()
        });
        for params in settings {
            let mut input = vec![0.0_f32; 8_192];
            input[4_000] = 0.5;
            let mut changer = VoiceChanger::new();
            let output = run(&mut changer, &params, &input);
            let (peak, _) = output
                .iter()
                .enumerate()
                .fold((0, 0.0_f32), |best, (index, value)| {
                    if value.abs() > best.1 {
                        (index, value.abs())
                    } else {
                        best
                    }
                });
            assert!(
                (peak as i64 - 4_000 - VOICE_CHANGER_LATENCY_SAMPLES as i64).abs() <= 8,
                "{params:?}: peak at {peak}"
            );
        }
    }

    #[test]
    fn unshifted_pitch_stage_keeps_the_voice_at_the_declared_latency() {
        // Vibrato and syllable-like level changes, so only the true lag
        // correlates best (a steady vowel matches every period).
        let input: Vec<f32> = vowel(48_000, |frame| {
            130.0 + 12.0 * (std::f32::consts::TAU * 0.9 * frame as f32 / SAMPLE_RATE).sin()
        })
        .iter()
        .enumerate()
        .map(|(frame, value)| {
            let syllable = (std::f32::consts::TAU * 3.0 * frame as f32 / SAMPLE_RATE).sin();
            value * (0.6 + 0.4 * syllable)
        })
        .collect();
        let mut changer = VoiceChanger::new();
        let params = VoiceChangerParams {
            shift_enabled: true,
            ..VoiceChangerParams::default()
        };
        let output = run(&mut changer, &params, &input);
        // Cross-correlate after warmup: the best lag is the latency, within
        // half a period.
        let start = 12_000;
        let length = 24_000;
        let mut best = (0_i64, f32::MIN);
        for lag in 600..1_500 {
            let score: f32 = (start..start + length)
                .map(|frame| output[frame] * input[frame - lag])
                .sum();
            if score > best.1 {
                best = (lag as i64, score);
            }
        }
        let half_period = (SAMPLE_RATE / 130.0 / 2.0) as i64;
        assert!(
            (best.0 - VOICE_CHANGER_LATENCY_SAMPLES as i64).abs() <= half_period,
            "best alignment lag {}",
            best.0
        );
        let level = rms(&output[start..start + length]) / rms(&input[start..start + length]);
        assert!((0.7..1.4).contains(&level), "level ratio {level}");
    }

    #[test]
    fn pitch_shift_moves_the_fundamental_by_the_requested_interval() {
        for (f0, semitones) in [(120.0, -5.0), (120.0, 7.0), (200.0, -12.0), (150.0, 12.0)] {
            let input = vowel(48_000, |_| f0);
            let mut changer = VoiceChanger::new();
            let params = VoiceChangerParams {
                shift_enabled: true,
                pitch_semitones: semitones,
                ..VoiceChangerParams::default()
            };
            let output = run(&mut changer, &params, &input);
            let measured = measured_f0(&output[16_000..]);
            let expected = f0 * 2.0_f32.powf(semitones / 12.0);
            assert!(
                cents(measured, expected).abs() < 10.0,
                "{f0} Hz {semitones:+} st: {measured} Hz, expected {expected}"
            );
            assert_eq!(output.len(), input.len());
        }
    }

    #[test]
    fn pitch_correction_snaps_a_detuned_voice_to_the_scale() {
        // 30 cents above A3 (220 Hz): chromatic correction should land on A3.
        let detuned = 220.0 * 2.0_f32.powf(30.0 / 1_200.0);
        let input = vowel(48_000, |_| detuned);
        for retune_ms in [0.0, 50.0] {
            let mut changer = VoiceChanger::new();
            let params = VoiceChangerParams {
                correct_enabled: true,
                retune_ms,
                ..VoiceChangerParams::default()
            };
            let output = run(&mut changer, &params, &input);
            let measured = measured_f0(&output[24_000..]);
            assert!(
                cents(measured, 220.0).abs() < 10.0,
                "retune {retune_ms} ms: {measured} Hz"
            );
        }
        // C major has no A#: a voice at A#3 + 10 cents lands on A or B.
        let between = 233.08 * 2.0_f32.powf(10.0 / 1_200.0);
        let mut changer = VoiceChanger::new();
        let params = VoiceChangerParams {
            correct_enabled: true,
            retune_ms: 0.0,
            scale: VoiceScale::Major,
            ..VoiceChangerParams::default()
        };
        let output = run(&mut changer, &params, &vowel(48_000, |_| between));
        let measured = measured_f0(&output[24_000..]);
        let to_a = cents(measured, 220.0).abs();
        let to_b = cents(measured, 246.94).abs();
        assert!(
            to_a.min(to_b) < 10.0,
            "major-scale correction gave {measured} Hz"
        );
    }

    #[test]
    fn target_note_does_not_warble_at_a_boundary() {
        let mut corrector = Corrector::default();
        let params = VoiceChangerParams {
            correct_enabled: true,
            retune_ms: 0.0,
            ..VoiceChangerParams::default()
        };
        // Wobble ±20 cents around the 60/61 boundary (60.5).
        let mut notes = Vec::new();
        for hop in 0..200 {
            let midi = 60.5 + 0.2 * (hop as f32 * 0.7).sin();
            let correction = corrector.update(&params, Some(midi));
            notes.push((midi + correction).round() as i32);
        }
        notes.dedup();
        assert_eq!(notes.len(), 1, "target changed: {notes:?}");
    }

    /// A 125 Hz pulse train through one 700 Hz resonance: its spectral
    /// peak is the formant, read off the harmonics around the strongest.
    fn resonance(frames: usize) -> Vec<f32> {
        let radius = (-std::f32::consts::PI * 80.0 / SAMPLE_RATE).exp();
        let theta = std::f32::consts::TAU * 700.0 / SAMPLE_RATE;
        let mut state = [0.0_f32; 2];
        (0..frames)
            .map(|frame| {
                let pulse = if frame % 384 == 0 { 1.0 } else { 0.0 };
                let value = (1.0 - radius) * pulse + 2.0 * radius * theta.cos() * state[0]
                    - radius * radius * state[1];
                state[1] = state[0];
                state[0] = value;
                value * 8.0
            })
            .collect()
    }

    fn formant_peak_hz(samples: &[f32], f0: f64) -> f64 {
        let magnitudes: Vec<f64> = (1..20)
            .map(|harmonic| {
                let frequency = f0 * harmonic as f64;
                let (mut re, mut im) = (0.0_f64, 0.0_f64);
                for (index, value) in samples.iter().enumerate() {
                    let window = 0.5
                        - 0.5
                            * (std::f64::consts::TAU * index as f64 / (samples.len() - 1) as f64)
                                .cos();
                    let angle =
                        std::f64::consts::TAU * frequency * index as f64 / f64::from(SAMPLE_RATE);
                    re += f64::from(*value) * window * angle.cos();
                    im -= f64::from(*value) * window * angle.sin();
                }
                re.hypot(im)
            })
            .collect();
        let strongest = (1..magnitudes.len() - 1)
            .max_by(|a, b| magnitudes[*a].total_cmp(&magnitudes[*b]))
            .unwrap();
        let around = &magnitudes[strongest - 1..=strongest + 1];
        let weighted: f64 = around
            .iter()
            .enumerate()
            .map(|(offset, magnitude)| magnitude * f0 * (strongest + offset) as f64)
            .sum();
        weighted / around.iter().sum::<f64>()
    }

    #[test]
    fn formant_shift_moves_the_resonance_without_changing_pitch() {
        let input = resonance(48_000);
        let peak_for = |semitones: f32| {
            let params = VoiceChangerParams {
                shift_enabled: true,
                formant_semitones: semitones,
                ..VoiceChangerParams::default()
            };
            let output = run(&mut VoiceChanger::new(), &params, &input);
            let measured = measured_f0(&output[16_000..]);
            assert!(
                cents(measured, 125.0).abs() < 15.0,
                "formant {semitones:+}: pitch moved to {measured} Hz"
            );
            formant_peak_hz(&output[20_000..27_680], 125.0)
        };
        let unshifted = peak_for(0.0);
        for semitones in [5.0_f32, -5.0] {
            let ratio = peak_for(semitones) / unshifted;
            let expected = 2.0_f64.powf(f64::from(semitones) / 12.0);
            assert!(
                (ratio / expected - 1.0).abs() < 0.08,
                "formant {semitones:+}: resonance moved by {ratio}, expected {expected}"
            );
        }
    }

    #[test]
    fn duration_is_preserved_over_a_minute() {
        let frames = 48_000 * 60;
        let mut changer = VoiceChanger::new();
        let params = VoiceChangerParams {
            shift_enabled: true,
            pitch_semitones: 12.0,
            ..VoiceChangerParams::default()
        };
        let mut source = Vowel::new();
        let mut produced = 0;
        let mut block = [0.0_f32; BLOCK];
        let (mut early, mut late, mut input_energy) = (0.0_f32, 0.0_f32, 0.0_f32);
        while produced < frames {
            for value in &mut block {
                *value = source.next(150.0);
            }
            let energy_in: f32 = block.iter().map(|value| value * value).sum();
            changer.process_block(&params, &mut block);
            let energy: f32 = block.iter().map(|value| value * value).sum();
            if (48_000..96_000).contains(&produced) {
                early += energy;
                input_energy += energy_in;
            }
            if produced >= frames - 48_000 {
                late += energy;
            }
            produced += BLOCK;
        }
        // Block-in, block-out keeps the duration exact by construction; the
        // grain scheduler must not drift, so the last second sounds like the
        // second one and keeps the input's level.
        assert_eq!(produced % BLOCK, 0);
        let drift = (late / early).sqrt();
        assert!((0.9..1.1).contains(&drift), "level drifted by {drift}");
        let level = (early / input_energy).sqrt();
        assert!((0.6..1.6).contains(&level), "level ratio {level}");
    }

    #[test]
    fn hostile_input_never_produces_non_finite_or_loud_output() {
        let ceiling = 10.0_f32.powf(-1.0 / 20.0);
        let everything = VoiceChangerParams {
            gate_db: -60.0,
            correct_enabled: true,
            retune_ms: 0.0,
            shift_enabled: true,
            pitch_semitones: -12.0,
            formant_semitones: 12.0,
            ring_enabled: true,
            crush_enabled: true,
            crush_bits: 4,
            crush_downsample: 12,
            drive_enabled: true,
            drive_percent: 100.0,
            whisper_percent: 50.0,
            tone_enabled: true,
            peak_db: 12.0,
            mod_enabled: true,
            mod_type: ModulationType::Flanger,
            mod_feedback_percent: 90.0,
            space_enabled: true,
            space_decay_ms: 4_000.0,
            space_level_percent: 100.0,
            output_db: 12.0,
            ..VoiceChangerParams::default()
        };
        let mut changer = VoiceChanger::new();
        let inputs: [fn(usize) -> f32; 5] = [
            |frame| if (frame / 50) % 2 == 0 { 1.0 } else { -1.0 },
            |_| 1.0,
            |_| 0.0,
            |frame| if frame % 7 == 0 { f32::NAN } else { 0.5 },
            |frame| {
                if frame % 11 == 0 {
                    f32::INFINITY
                } else {
                    1.0e-30
                }
            },
        ];
        for input in inputs {
            for block_index in 0..200 {
                let mut block = [0.0_f32; BLOCK];
                for (offset, value) in block.iter_mut().enumerate() {
                    *value = input(block_index * BLOCK + offset);
                }
                changer.process_block(&everything, &mut block);
                for value in block {
                    assert!(value.is_finite());
                    assert!(value.abs() <= ceiling + 1.0e-6, "peak {value}");
                }
            }
        }
    }

    #[test]
    fn a_preset_change_mid_vowel_does_not_click() {
        let input = vowel(48_000, |_| 140.0);
        let mut changer = VoiceChanger::new();
        let first = VoiceChangerParams::default();
        let second = VoiceChangerParams {
            ring_enabled: true,
            ring_hz: 300.0,
            ring_depth_percent: 40.0,
            tone_enabled: true,
            low_cut_hz: 300.0,
            high_cut_hz: 3_000.0,
            drive_enabled: true,
            drive_percent: 30.0,
            ..VoiceChangerParams::default()
        };
        let largest_step = |samples: &[f32]| {
            samples
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).abs())
                .fold(0.0_f32, f32::max)
        };
        // Steady-state steps of each setting on its own, after warmup.
        let steady_second = run(&mut VoiceChanger::new(), &second, &input);
        let steady = run(&mut changer, &first, &input[..24_064]);
        let largest_step =
            largest_step(&steady[2_000..]).max(largest_step(&steady_second[2_000..]));
        let changed = run(&mut changer, &second, &input[24_064..]);
        let mut joined = steady[steady.len() - 1..].to_vec();
        joined.extend_from_slice(&changed[..4_800]);
        let worst = joined
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            worst <= largest_step * 2.0,
            "step {worst} vs steady {largest_step}"
        );
    }

    #[test]
    fn reset_drops_earlier_audio() {
        let mut changer = VoiceChanger::new();
        let params = VoiceChangerParams {
            space_enabled: true,
            space_level_percent: 100.0,
            ..VoiceChangerParams::default()
        };
        run(&mut changer, &params, &vowel(9_600, |_| 150.0));
        changer.reset();
        let output = run(&mut changer, &params, &vec![0.0; 9_600]);
        assert!(output.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn telemetry_reports_the_voice_while_pitch_stages_run() {
        let mut changer = VoiceChanger::new();
        assert!(!changer.telemetry().voiced);
        let params = VoiceChangerParams {
            correct_enabled: true,
            ..VoiceChangerParams::default()
        };
        run(&mut changer, &params, &vowel(24_000, |_| 220.0));
        let telemetry = changer.telemetry();
        assert!(telemetry.voiced);
        assert_eq!(telemetry.note, 57);
        assert!(telemetry.cents.abs() < 10.0, "{telemetry:?}");
    }

    #[test]
    fn whisper_keeps_the_level_and_removes_the_pitch() {
        let input = vowel(48_000, |_| 150.0);
        let mut changer = VoiceChanger::new();
        let params = VoiceChangerParams {
            whisper_percent: 100.0,
            ..VoiceChangerParams::default()
        };
        let output = run(&mut changer, &params, &input);
        let level = rms(&output[12_000..]) / rms(&input[12_000..]);
        assert!((0.3..2.0).contains(&level), "whisper level ratio {level}");
        let mut tracker = PitchTracker::new();
        let mut ring = Box::new([0.0_f32; RING]);
        let mut voiced = 0;
        let mut frames = 0;
        for (index, value) in output.iter().enumerate().skip(12_000) {
            ring[index & RING_MASK] = *value;
            if (index + 1) % ANALYSIS_HOP == 0 && index > 14_000 {
                tracker.analyse(&ring, index as u64, 0.001);
                voiced += usize::from(tracker.voiced);
                frames += 1;
            }
        }
        assert!(
            voiced * 4 < frames,
            "{voiced} of {frames} frames still voiced"
        );
    }
}
