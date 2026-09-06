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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BiquadError {
    InvalidSampleRate,
    InvalidFrequency,
    InvalidQ,
    InvalidChannels,
    InvalidBand,
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

impl Coefficients {
    const fn zero() -> Self {
        Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl std::ops::Sub for Coefficients {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            b0: self.b0 - rhs.b0,
            b1: self.b1 - rhs.b1,
            b2: self.b2 - rhs.b2,
            a1: self.a1 - rhs.a1,
            a2: self.a2 - rhs.a2,
        }
    }
}

impl std::ops::Add for Coefficients {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            b0: self.b0 + rhs.b0,
            b1: self.b1 + rhs.b1,
            b2: self.b2 + rhs.b2,
            a1: self.a1 + rhs.a1,
            a2: self.a2 + rhs.a2,
        }
    }
}

impl std::ops::Div<f32> for Coefficients {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            b0: self.b0 / rhs,
            b1: self.b1 / rhs,
            b2: self.b2 / rhs,
            a1: self.a1 / rhs,
            a2: self.a2 / rhs,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Biquad {
    params: BiquadParams,
    coefficients: Coefficients,
    target_coefficients: Coefficients,
    ramp_step: Coefficients,
    ramp_remaining: usize,
    channels: usize,
    z1: [f32; 2],
    z2: [f32; 2],
}

/// Fixed-capacity eight-band parametric EQ. Band state is constructed before
/// processing; the audio method only visits enabled filters and allocates
/// nothing.
#[derive(Clone, Debug)]
pub struct ParametricEq {
    bands: [Option<Biquad>; 8],
    channels: usize,
}

pub const GRAPHIC_EQ_FREQUENCIES_HZ: [f32; 10] = [
    31.5, 63.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

/// Fixed ten-band graphic EQ using the same biquad coefficient path as the
/// parametric EQ. Gains are constrained to the M04 +/-18 dB contract.
#[derive(Clone, Debug)]
pub struct GraphicEq {
    bands: [Option<Biquad>; 10],
    gains_db: [f32; 10],
    sample_rate: f32,
    channels: usize,
}

impl GraphicEq {
    pub fn new(
        gains_db: [f32; 10],
        sample_rate: f32,
        channels: usize,
    ) -> Result<Self, BiquadError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(BiquadError::InvalidSampleRate);
        }
        if channels == 0 || channels > 2 {
            return Err(BiquadError::InvalidChannels);
        }
        let mut bands = [None, None, None, None, None, None, None, None, None, None];
        for (index, gain_db) in gains_db.into_iter().enumerate() {
            if !gain_db.is_finite() || !(-18.0..=18.0).contains(&gain_db) {
                return Err(BiquadError::InvalidQ);
            }
            if GRAPHIC_EQ_FREQUENCIES_HZ[index] >= sample_rate * 0.5 {
                return Err(BiquadError::InvalidFrequency);
            }
            bands[index] = Some(Biquad::new(
                BiquadParams {
                    kind: FilterKind::Peaking,
                    frequency_hz: GRAPHIC_EQ_FREQUENCIES_HZ[index],
                    q: 1.4,
                    gain_db,
                    sample_rate,
                },
                channels,
            )?);
        }
        Ok(Self {
            bands,
            gains_db,
            sample_rate,
            channels,
        })
    }

    pub fn gains_db(&self) -> &[f32; 10] {
        &self.gains_db
    }

    pub fn set_gain_db(&mut self, index: usize, gain_db: f32) -> Result<(), BiquadError> {
        if !gain_db.is_finite() || !(-18.0..=18.0).contains(&gain_db) {
            return Err(BiquadError::InvalidQ);
        }
        let band = self.bands.get_mut(index).ok_or(BiquadError::InvalidBand)?;
        *band = Some(Biquad::new(
            BiquadParams {
                kind: FilterKind::Peaking,
                frequency_hz: GRAPHIC_EQ_FREQUENCIES_HZ[index],
                q: 1.4,
                gain_db,
                sample_rate: self.sample_rate,
            },
            self.channels,
        )?);
        self.gains_db[index] = gain_db;
        Ok(())
    }

    pub fn set_gain_db_ramped(
        &mut self,
        index: usize,
        gain_db: f32,
        frames: usize,
    ) -> Result<(), BiquadError> {
        if !gain_db.is_finite() || !(-18.0..=18.0).contains(&gain_db) {
            return Err(BiquadError::InvalidQ);
        }
        let band = self.bands.get_mut(index).ok_or(BiquadError::InvalidBand)?;
        let band = band.as_mut().ok_or(BiquadError::InvalidBand)?;
        let params = BiquadParams {
            kind: FilterKind::Peaking,
            frequency_hz: GRAPHIC_EQ_FREQUENCIES_HZ[index],
            q: 1.4,
            gain_db,
            sample_rate: self.sample_rate,
        };
        band.set_params_ramped(params, frames)?;
        self.gains_db[index] = gain_db;
        Ok(())
    }

    pub fn magnitude_db_at(&self, frequency_hz: f32) -> Result<f32, BiquadError> {
        let mut magnitude_db = 0.0;
        for band in self.bands.iter().flatten() {
            magnitude_db += band.magnitude_db_at(frequency_hz)?;
        }
        Ok(magnitude_db)
    }

    pub fn reset(&mut self) {
        for band in self.bands.iter_mut().flatten() {
            band.reset();
        }
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for band in self.bands.iter_mut().flatten() {
            band.process_interleaved(samples);
        }
    }
}

impl ParametricEq {
    pub fn new(
        band_params: [Option<BiquadParams>; 8],
        channels: usize,
    ) -> Result<Self, BiquadError> {
        if channels == 0 || channels > 2 {
            return Err(BiquadError::InvalidChannels);
        }
        let mut bands = [None, None, None, None, None, None, None, None];
        for (index, params) in band_params.into_iter().enumerate() {
            bands[index] = params
                .map(|params| Biquad::new(params, channels))
                .transpose()?;
        }
        Ok(Self { bands, channels })
    }

    pub fn from_preset(preset: EqPreset, channels: usize) -> Result<Self, BiquadError> {
        Self::new(preset.bands, channels)
    }

    pub fn active_bands(&self) -> usize {
        self.bands.iter().filter(|band| band.is_some()).count()
    }

    pub fn set_band(
        &mut self,
        index: usize,
        params: Option<BiquadParams>,
    ) -> Result<(), BiquadError> {
        let band = params
            .map(|params| Biquad::new(params, self.channels))
            .transpose()?;
        let slot = self.bands.get_mut(index).ok_or(BiquadError::InvalidBand)?;
        *slot = band;
        Ok(())
    }

    pub fn reset(&mut self) {
        for band in self.bands.iter_mut().flatten() {
            band.reset();
        }
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for band in self.bands.iter_mut().flatten() {
            band.process_interleaved(samples);
        }
    }
}

impl Biquad {
    pub fn new(params: BiquadParams, channels: usize) -> Result<Self, BiquadError> {
        validate(params, channels)?;
        Ok(Self {
            coefficients: coefficients(params),
            target_coefficients: coefficients(params),
            ramp_step: Coefficients::zero(),
            ramp_remaining: 0,
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
        self.target_coefficients = self.coefficients;
        self.ramp_step = Coefficients::zero();
        self.ramp_remaining = 0;
        Ok(())
    }

    /// Schedules a coefficient transition over a fixed number of samples.
    /// The transition is prepared on the control thread and applied without
    /// allocation during processing.
    pub fn set_params_ramped(
        &mut self,
        params: BiquadParams,
        frames: usize,
    ) -> Result<(), BiquadError> {
        validate(params, self.channels)?;
        self.params = params;
        self.target_coefficients = coefficients(params);
        if frames == 0 {
            self.coefficients = self.target_coefficients;
            self.ramp_step = Coefficients::zero();
            self.ramp_remaining = 0;
        } else {
            self.ramp_step = (self.target_coefficients - self.coefficients) / frames as f32;
            self.ramp_remaining = frames;
        }
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
            self.advance_ramp();
            let channel = index % self.channels;
            let input = if sample.is_finite() { *sample } else { 0.0 };
            let output = self.coefficients.b0 * input + self.z1[channel];
            self.z1[channel] =
                self.coefficients.b1 * input - self.coefficients.a1 * output + self.z2[channel];
            self.z2[channel] = self.coefficients.b2 * input - self.coefficients.a2 * output;
            *sample = if output.is_finite() { output } else { 0.0 };
        }
    }

    fn advance_ramp(&mut self) {
        if self.ramp_remaining != 0 {
            self.coefficients = self.coefficients + self.ramp_step;
            self.ramp_remaining -= 1;
            if self.ramp_remaining == 0 {
                self.coefficients = self.target_coefficients;
                self.ramp_step = Coefficients::zero();
            }
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
    gain_reduction_db: f32,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeterSnapshot {
    pub peak: [f32; 2],
    pub rms: [f32; 2],
    pub peak_db: [f32; 2],
    pub rms_db: [f32; 2],
    pub clipping: u64,
}

/// Per-channel block meter. Peak/RMS values are linear and dB values use a
/// -120 dBFS floor for silence rather than representing negative infinity.
#[derive(Clone, Copy, Debug)]
pub struct SignalMeter {
    channels: usize,
    peak: [f32; 2],
    sum_squares: [f64; 2],
    samples: u64,
    clipping: u64,
}

impl SignalMeter {
    pub fn new(channels: usize) -> Result<Self, BiquadError> {
        if channels == 0 || channels > 2 {
            return Err(BiquadError::InvalidChannels);
        }
        Ok(Self {
            channels,
            peak: [0.0; 2],
            sum_squares: [0.0; 2],
            samples: 0,
            clipping: 0,
        })
    }

    pub fn reset(&mut self) {
        self.peak = [0.0; 2];
        self.sum_squares = [0.0; 2];
        self.samples = 0;
        self.clipping = 0;
    }

    pub fn process_interleaved(&mut self, samples: &[f32]) {
        for frame in samples.chunks_exact(self.channels) {
            for (channel, sample) in frame.iter().enumerate() {
                let value = if sample.is_finite() { *sample } else { 0.0 };
                let magnitude = value.abs();
                self.peak[channel] = self.peak[channel].max(magnitude);
                self.sum_squares[channel] += f64::from(value) * f64::from(value);
                if magnitude >= 1.0 {
                    self.clipping = self.clipping.saturating_add(1);
                }
            }
            self.samples = self.samples.saturating_add(1);
        }
    }

    pub fn snapshot(&self) -> MeterSnapshot {
        let mut rms = [0.0; 2];
        if self.samples != 0 {
            let count = self.samples as f64;
            for (channel, value) in rms.iter_mut().enumerate().take(self.channels) {
                *value = (self.sum_squares[channel] / count).sqrt() as f32;
            }
        }
        MeterSnapshot {
            peak: self.peak,
            rms,
            peak_db: [db_floor(self.peak[0]), db_floor(self.peak[1])],
            rms_db: [db_floor(rms[0]), db_floor(rms[1])],
            clipping: self.clipping,
        }
    }
}

fn db_floor(value: f32) -> f32 {
    20.0 * value.max(1.0e-6).log10()
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug)]
pub struct VoiceChainConfig {
    pub sample_rate: f32,
    pub eq: Option<EqPresetId>,
    pub gate: Option<GateParams>,
    pub compressor: Option<CompressorParams>,
    pub delay_max_ms: Option<f32>,
    pub delay_ms: f32,
    pub limiter: LimiterParams,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceChainError {
    Biquad(BiquadError),
    Delay(DelayError),
}

impl From<BiquadError> for VoiceChainError {
    fn from(error: BiquadError) -> Self {
        Self::Biquad(error)
    }
}

impl From<DelayError> for VoiceChainError {
    fn from(error: DelayError) -> Self {
        Self::Delay(error)
    }
}

/// Prepared built-in processing order for a worker-side voice branch:
/// EQ, gate, compressor, delay, limiter, and telemetry. All optional state is
/// constructed before processing and the process method allocates nothing.
#[derive(Clone, Debug)]
pub struct VoiceChain {
    eq: Option<ParametricEq>,
    gate: Option<Gate>,
    compressor: Option<Compressor>,
    delay: Option<DelayLine>,
    limiter: PeakLimiter,
    meter: SignalMeter,
}

impl VoiceChain {
    pub fn new(config: VoiceChainConfig, channels: usize) -> Result<Self, VoiceChainError> {
        let eq = config
            .eq
            .map(|id| eq_preset(id, config.sample_rate))
            .transpose()?
            .map(|preset| ParametricEq::from_preset(preset, channels))
            .transpose()?;
        let gate = config
            .gate
            .map(|params| Gate::new(params, channels))
            .transpose()?;
        let compressor = config
            .compressor
            .map(|params| Compressor::new(params, channels))
            .transpose()?;
        let delay = config
            .delay_max_ms
            .map(|max_ms| {
                let mut delay = DelayLine::new(max_ms, config.sample_rate, channels)?;
                delay.set_delay_ms(config.delay_ms)?;
                Ok::<_, DelayError>(delay)
            })
            .transpose()?;
        Ok(Self {
            eq,
            gate,
            compressor,
            delay,
            limiter: PeakLimiter::new(config.limiter)?,
            meter: SignalMeter::new(channels)?,
        })
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if let Some(eq) = &mut self.eq {
            eq.process_interleaved(samples);
        }
        if let Some(gate) = &mut self.gate {
            gate.process_interleaved(samples);
        }
        if let Some(compressor) = &mut self.compressor {
            compressor.process_interleaved(samples);
        }
        if let Some(delay) = &mut self.delay {
            delay.process_interleaved(samples);
        }
        self.limiter.process_interleaved(samples);
        self.meter.process_interleaved(samples);
    }

    pub fn meter(&self) -> MeterSnapshot {
        self.meter.snapshot()
    }

    pub fn reset(&mut self) {
        if let Some(eq) = &mut self.eq {
            eq.reset();
        }
        if let Some(gate) = &mut self.gate {
            gate.reset();
        }
        if let Some(compressor) = &mut self.compressor {
            compressor.reset();
        }
        if let Some(delay) = &mut self.delay {
            delay.reset();
        }
        self.meter.reset();
    }
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
            gain_reduction_db: 0.0,
        })
    }

    pub fn params(&self) -> CompressorParams {
        self.params
    }

    pub fn reset(&mut self) {
        self.envelope_db = -120.0;
        self.gain_reduction_db = 0.0;
    }

    pub fn gain_reduction_db(&self) -> f32 {
        self.gain_reduction_db
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
            self.gain_reduction_db = reduction_db;
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
    fn biquad_parameter_ramp_reaches_target_without_discontinuity_api() {
        let mut filter = Biquad::new(
            BiquadParams {
                gain_db: 0.0,
                ..params(FilterKind::Peaking)
            },
            1,
        )
        .unwrap();
        let initial = filter.magnitude_db_at(1_000.0).unwrap();
        filter
            .set_params_ramped(params(FilterKind::Peaking), 8)
            .unwrap();
        let mut samples = [1.0; 8];
        filter.process_interleaved(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!((filter.magnitude_db_at(1_000.0).unwrap() - 6.0).abs() < 0.02);
        assert!(samples
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.5));
        assert!((initial - 0.0).abs() < 1e-4);
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
    fn parametric_eq_prebuilds_eight_bands_and_processes_without_growth() {
        let preset = eq_preset(EqPresetId::Hum50Hz, 48_000.0).unwrap();
        let mut eq = ParametricEq::from_preset(preset, 2).unwrap();
        assert_eq!(eq.active_bands(), 1);
        let mut samples = [0.25, -0.25, f32::NAN, f32::INFINITY];
        eq.process_interleaved(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        eq.set_band(1, Some(params(FilterKind::Peaking))).unwrap();
        assert_eq!(eq.active_bands(), 2);
        eq.set_band(0, None).unwrap();
        assert_eq!(eq.active_bands(), 1);
        assert!(matches!(
            eq.set_band(8, None),
            Err(BiquadError::InvalidBand)
        ));
        eq.reset();
    }

    #[test]
    fn graphic_eq_has_ten_bounded_bands_and_flat_default() {
        let mut gains = [0.0; 10];
        let mut eq = GraphicEq::new(gains, 48_000.0, 1).unwrap();
        assert_eq!(eq.gains_db(), &gains);
        let mut flat = [0.25, -0.25];
        eq.process_interleaved(&mut flat);
        assert!((flat[0] - 0.25).abs() < 1e-4);
        assert!((flat[1] + 0.25).abs() < 1e-4);
        assert!(eq.magnitude_db_at(1_000.0).unwrap().abs() < 1e-3);
        eq.set_gain_db(9, 18.0).unwrap();
        gains[9] = 18.0;
        assert_eq!(eq.gains_db(), &gains);
        assert!((eq.magnitude_db_at(16_000.0).unwrap() - 18.0).abs() < 0.1);
        eq.set_gain_db_ramped(9, -18.0, 16).unwrap();
        gains[9] = -18.0;
        let mut ramped = [0.25; 32];
        eq.process_interleaved(&mut ramped);
        assert!(ramped.iter().all(|sample| sample.is_finite()));
        assert_eq!(eq.gains_db(), &gains);
        assert!(matches!(
            eq.set_gain_db(10, 0.0),
            Err(BiquadError::InvalidBand)
        ));
        assert!(matches!(
            GraphicEq::new([19.0; 10], 48_000.0, 1),
            Err(BiquadError::InvalidQ)
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
        assert_eq!(quiet.gain_reduction_db(), 0.0);

        let mut loud = Compressor::new(base, 2).unwrap();
        let mut loud_samples = [0.0; 128];
        for frame in loud_samples.chunks_exact_mut(2) {
            frame[0] = 1.0;
            frame[1] = 0.25;
        }
        loud.process_interleaved(&mut loud_samples);
        assert!(loud_samples[126] < 1.0);
        assert!((loud_samples[126] / 1.0 - loud_samples[127] / 0.25).abs() < 1e-5);
        assert!(loud.gain_reduction_db() > 0.0);
        loud.reset();
        assert_eq!(loud.gain_reduction_db(), 0.0);
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
    fn signal_meter_reports_separate_finite_peak_rms_and_clipping() {
        let mut meter = SignalMeter::new(2).unwrap();
        meter.process_interleaved(&[1.0, 0.5, -1.0, f32::NAN]);
        let snapshot = meter.snapshot();
        assert_eq!(snapshot.peak, [1.0, 0.5]);
        assert!((snapshot.rms[0] - 1.0).abs() < 1e-6);
        assert!((snapshot.rms[1] - 0.35355338).abs() < 1e-5);
        assert!(snapshot.peak_db[0].abs() < 1e-5);
        assert!(snapshot.rms_db[1] < -8.0);
        assert_eq!(snapshot.clipping, 2);
        meter.reset();
        let silence = meter.snapshot();
        assert_eq!(silence.peak, [0.0, 0.0]);
        assert_eq!(silence.rms_db, [-120.0, -120.0]);
    }

    #[test]
    fn voice_chain_prepares_optional_stages_and_reports_finite_output() {
        let mut chain = VoiceChain::new(
            VoiceChainConfig {
                sample_rate: 48_000.0,
                eq: Some(EqPresetId::Hum50Hz),
                gate: Some(GateParams {
                    threshold_db: -45.0,
                    hysteresis_db: 3.0,
                    ratio: 4.0,
                    range_db: 60.0,
                    attack_ms: 5.0,
                    hold_ms: 50.0,
                    release_ms: 150.0,
                    sample_rate: 48_000.0,
                }),
                compressor: Some(CompressorParams {
                    threshold_db: -18.0,
                    ratio: 3.0,
                    attack_ms: 10.0,
                    release_ms: 150.0,
                    knee_db: 6.0,
                    makeup_db: 0.0,
                    sample_rate: 48_000.0,
                }),
                delay_max_ms: Some(5.0),
                delay_ms: 0.0,
                limiter: LimiterParams { ceiling_db: -1.0 },
            },
            1,
        )
        .unwrap();
        let mut samples = [0.0, 0.5, -0.5, f32::NAN];
        chain.process_interleaved(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(chain.meter().peak[0] <= 10.0_f32.powf(-1.0 / 20.0));
        chain.reset();
        assert_eq!(chain.meter().peak, [0.0, 0.0]);
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
