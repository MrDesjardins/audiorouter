//! Portable signal generation and analysis used by the Windows cable runner.
//! This crate deliberately contains no device I/O: measurements become
//! acceptance evidence only when a capture from the real route is analyzed.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SignalMetrics {
    pub frames: usize,
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub thdn_db: Option<f64>,
}

pub fn seeded_noise(frames: usize, seed: u64) -> Vec<f32> {
    let mut state = if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    };
    (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (((state >> 40) as i32 - (1 << 23)) as f64 / (1 << 23) as f64) as f32
        })
        .collect()
}

pub fn tone(frames: usize, sample_rate: u32, frequency_hz: f64, peak: f64) -> Vec<f32> {
    (0..frames)
        .map(|i| {
            (peak * (std::f64::consts::TAU * frequency_hz * i as f64 / sample_rate as f64).sin())
                as f32
        })
        .collect()
}

pub fn log_sweep(
    frames: usize,
    sample_rate: u32,
    start_hz: f64,
    end_hz: f64,
    peak: f64,
) -> Vec<f32> {
    if frames < 2 || sample_rate == 0 || start_hz <= 0.0 || end_hz <= start_hz {
        return vec![0.0; frames];
    }
    let duration = frames as f64 / sample_rate as f64;
    let ratio = end_hz / start_hz;
    (0..frames)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            let phase = std::f64::consts::TAU * start_hz * duration / ratio.ln()
                * ((ratio.powf(t / duration)) - 1.0);
            (peak * phase.sin()) as f32
        })
        .collect()
}

pub fn impulse_train(frames: usize, period: usize, peak: f32) -> Vec<f32> {
    if period == 0 {
        return vec![0.0; frames];
    }
    (0..frames)
        .map(|i| if i % period == 0 { peak } else { 0.0 })
        .collect()
}

/// Finds the integer sample offset that minimizes squared error against a
/// reference. The bounded search avoids accidental quadratic work on long runs.
pub fn align(reference: &[f32], capture: &[f32], max_offset: usize) -> Option<(usize, usize)> {
    if reference.is_empty() || capture.is_empty() {
        return None;
    }
    let mut best: Option<(f64, usize, usize)> = None;
    for offset in 0..=max_offset.min(capture.len().saturating_sub(1)) {
        let count = reference.len().min(capture.len() - offset);
        if count == 0 {
            continue;
        }
        let error = reference
            .iter()
            .zip(&capture[offset..offset + count])
            .map(|(a, b)| {
                let d = *a as f64 - *b as f64;
                d * d
            })
            .sum::<f64>()
            / count as f64;
        if best.is_none_or(|(current, _, _)| error < current) {
            best = Some((error, offset, count));
        }
    }
    best.map(|(_, offset, count)| (offset, count))
}

pub fn compare(
    reference: &[f32],
    capture: &[f32],
    offset: usize,
    frames: usize,
) -> Option<(f64, f64)> {
    if frames == 0 || offset.checked_add(frames)? > capture.len() || frames > reference.len() {
        return None;
    }
    let mut max_error = 0.0f64;
    let mut rms_error = 0.0f64;
    for (a, b) in reference[..frames]
        .iter()
        .zip(&capture[offset..offset + frames])
    {
        let error = (*a as f64 - *b as f64).abs();
        max_error = max_error.max(error);
        rms_error += error * error;
    }
    Some((max_error, (rms_error / frames as f64).sqrt()))
}

pub fn metrics(samples: &[f32]) -> SignalMetrics {
    let n = samples.len().max(1) as f64;
    let dc = samples.iter().map(|x| *x as f64).sum::<f64>() / n;
    let peak = samples
        .iter()
        .map(|x| (*x as f64).abs())
        .fold(0.0, f64::max);
    let rms = (samples
        .iter()
        .map(|x| {
            let v = *x as f64;
            v * v
        })
        .sum::<f64>()
        / n)
        .sqrt();
    SignalMetrics {
        frames: samples.len(),
        peak,
        rms,
        dc,
        thdn_db: None,
    }
}

/// Single-bin sine fit. THD+N is the residual after least-squares fitting the
/// fundamental's sine/cosine coefficients, divided by fitted fundamental RMS.
pub fn thdn_db(samples: &[f32], sample_rate: u32, frequency_hz: f64) -> Option<f64> {
    if samples.len() < 8 || sample_rate == 0 || frequency_hz <= 0.0 {
        return None;
    }
    let (mut ss, mut cc, mut sc, mut ys, mut yc) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, sample) in samples.iter().enumerate() {
        let phase = std::f64::consts::TAU * frequency_hz * i as f64 / sample_rate as f64;
        let (s, c, y) = (phase.sin(), phase.cos(), *sample as f64);
        ss += s * s;
        cc += c * c;
        sc += s * c;
        ys += y * s;
        yc += y * c;
    }
    let det = ss * cc - sc * sc;
    if det.abs() < f64::EPSILON {
        return None;
    }
    let a = (ys * cc - yc * sc) / det;
    let b = (yc * ss - ys * sc) / det;
    let fundamental_rms = (a * a + b * b).sqrt() / std::f64::consts::SQRT_2;
    if fundamental_rms <= f64::MIN_POSITIVE {
        return None;
    }
    let residual = samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            let phase = std::f64::consts::TAU * frequency_hz * i as f64 / sample_rate as f64;
            let error = *sample as f64 - a * phase.sin() - b * phase.cos();
            error * error
        })
        .sum::<f64>();
    let noise_rms = (residual / samples.len() as f64).sqrt();
    Some(20.0 * (noise_rms / fundamental_rms).log10())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_signals_are_reproducible_and_bounded() {
        assert_eq!(seeded_noise(1024, 42), seeded_noise(1024, 42));
        assert!(seeded_noise(1024, 42)
            .iter()
            .all(|x| (-1.0..1.0).contains(x)));
        assert_eq!(
            impulse_train(10, 3, 1.0),
            [1., 0., 0., 1., 0., 0., 1., 0., 0., 1.]
        );
        assert!(log_sweep(1024, 48_000, 20.0, 20_000.0, 0.5)
            .iter()
            .all(|x| x.abs() <= 0.5));
    }

    #[test]
    fn alignment_and_error_metrics_detect_shift_and_corruption() {
        let signal = seeded_noise(4096, 17);
        let mut captured = vec![0.0; 7];
        captured.extend_from_slice(&signal);
        let (offset, count) = align(&signal, &captured, 16).unwrap();
        assert_eq!((offset, count), (7, signal.len()));
        assert_eq!(compare(&signal, &captured, offset, count), Some((0.0, 0.0)));
        captured[100] = 0.25;
        assert!(compare(&signal, &captured, offset, count).unwrap().0 > 0.0);
    }

    #[test]
    fn sine_fit_reports_distortion_and_exact_sine_is_low() {
        let pure = tone(48_000, 48_000, 997.0, 0.89);
        assert!(thdn_db(&pure[100..47_900], 48_000, 997.0).unwrap() < -100.0);
        let distorted: Vec<_> = pure
            .iter()
            .enumerate()
            .map(|(i, x)| {
                *x + (0.01 * (std::f64::consts::TAU * 2.0 * 997.0 * i as f64 / 48_000.0).sin())
                    as f32
            })
            .collect();
        assert!(thdn_db(&distorted[100..47_900], 48_000, 997.0).unwrap() > -70.0);
    }
}
