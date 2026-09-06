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
}
