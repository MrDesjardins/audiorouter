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
    let limit = max_offset.min(capture.len().saturating_sub(1));
    // Prefer an exact 32-sample signature. This is linear in the search range
    // and preserves exact frame offsets for deterministic noise and impulses.
    let signature_len = reference.len().min(32);
    if signature_len > 0 {
        let signature_limit = max_offset.min(capture.len().saturating_sub(signature_len));
        for offset in 0..=signature_limit {
            if capture[offset..offset + signature_len] == reference[..signature_len] {
                return Some((offset, reference.len().min(capture.len() - offset)));
            }
        }
    }
    let coarse_step = 16usize;
    let coarse_stride = 16usize;
    let window = reference.len().min(capture.len()).min(4096);
    let error_at = |offset: usize, stride: usize| {
        let count = window.min(capture.len().saturating_sub(offset));
        let mut error = 0.0;
        let mut samples = 0usize;
        for index in (0..count).step_by(stride) {
            let difference = f64::from(reference[index]) - f64::from(capture[offset + index]);
            error += difference * difference;
            samples += 1;
        }
        if samples == 0 {
            f64::INFINITY
        } else {
            error / samples as f64
        }
    };
    let mut coarse_best = 0usize;
    let mut coarse_error = f64::INFINITY;
    for offset in (0..=limit).step_by(coarse_step) {
        let error = error_at(offset, coarse_stride);
        if error < coarse_error {
            coarse_best = offset;
            coarse_error = error;
        }
    }
    let refine_start = coarse_best.saturating_sub(coarse_step);
    let refine_end = coarse_best.saturating_add(coarse_step).min(limit);
    let mut best_offset = coarse_best;
    let mut best_error = f64::INFINITY;
    for offset in refine_start..=refine_end {
        let error = error_at(offset, 1);
        if error < best_error {
            best_offset = offset;
            best_error = error;
        }
    }
    let count = reference
        .len()
        .min(capture.len().saturating_sub(best_offset));
    (count > 0).then_some((best_offset, count))
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

/// Read the canonical IEEE float32 WAV format emitted by the native WASAPI
/// probe. Reject extensible/PCM WAVs rather than silently mis-decoding them.
pub fn read_float_wav(path: &std::path::Path) -> Result<(u32, u16, Vec<f32>), String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("expected a RIFF/WAVE file".into());
    }
    let mut format = None;
    let mut data = None;
    let mut cursor = 12usize;
    while cursor.checked_add(8).is_some_and(|end| end <= bytes.len()) {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let start = cursor + 8;
        let end = start.checked_add(size).ok_or("WAV chunk size overflow")?;
        if end > bytes.len() {
            return Err("truncated WAV chunk".into());
        }
        match id {
            b"fmt " if size >= 16 => {
                format = Some((
                    u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                    u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
                ))
            }
            b"data" => data = Some(&bytes[start..end]),
            _ => {}
        }
        cursor = end + (size & 1);
    }
    let (tag, channels, rate, bits) = format.ok_or("WAV fmt chunk is missing")?;
    if tag != 3 || channels == 0 || rate == 0 || bits != 32 {
        return Err("expected interleaved IEEE float32 WAV".into());
    }
    let raw = data.ok_or("WAV data chunk is missing")?;
    if raw.len() % 4 != 0 || raw.len() % (usize::from(channels) * 4) != 0 {
        return Err("WAV data is not a whole number of frames".into());
    }
    let samples: Vec<f32> = raw
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
        .collect();
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err("WAV contains non-finite samples".into());
    }
    Ok((rate, channels, samples))
}

pub fn analyze_wav_pair(
    reference_path: &std::path::Path,
    capture_path: &std::path::Path,
    max_offset: usize,
    frequency_hz: Option<f64>,
) -> Result<serde_json::Value, String> {
    let (reference_rate, reference_channels, reference) = read_float_wav(reference_path)?;
    let (capture_rate, capture_channels, capture) = read_float_wav(capture_path)?;
    if reference_rate != capture_rate || reference_channels != capture_channels {
        return Err("reference and capture WAV formats do not match".into());
    }
    let channels = usize::from(reference_channels);
    let reference_frames = reference.len() / channels;
    let capture_frames = capture.len() / channels;
    if reference_frames == 0 || capture_frames == 0 {
        return Err("WAV files contain no samples".into());
    }
    // Align on channel zero, then compare all channels at that same frame
    // offset so channel swaps and isolated channel corruption stay visible.
    let reference_first = reference
        .iter()
        .step_by(channels)
        .copied()
        .collect::<Vec<_>>();
    let capture_first = capture
        .iter()
        .step_by(channels)
        .copied()
        .collect::<Vec<_>>();
    let (offset, aligned_frames) = align(&reference_first, &capture_first, max_offset)
        .ok_or("could not align reference and capture")?;
    let mut channel_metrics = Vec::with_capacity(channels);
    let mut equal = true;
    let mut max_error = 0.0f64;
    let mut squared_error = 0.0f64;
    for channel in 0..channels {
        let expected = (0..aligned_frames)
            .map(|frame| reference[frame * channels + channel])
            .collect::<Vec<_>>();
        let actual = (0..aligned_frames)
            .map(|frame| capture[(offset + frame) * channels + channel])
            .collect::<Vec<_>>();
        let (channel_max, channel_rms) = compare(&expected, &actual, 0, aligned_frames)
            .ok_or("aligned channel range is invalid")?;
        equal &= channel_max == 0.0;
        max_error = max_error.max(channel_max);
        squared_error += channel_rms * channel_rms * aligned_frames as f64;
        channel_metrics.push(serde_json::json!({
            "channel": channel,
            "maxAbsoluteError": channel_max,
            "rmsError": channel_rms,
            "thdnDb": frequency_hz.and_then(|frequency| thdn_db(&actual, reference_rate, frequency))
        }));
    }
    let rms_error = (squared_error / (aligned_frames * channels) as f64).sqrt();
    Ok(serde_json::json!({
        "format": {"sampleRateHz": reference_rate, "channels": reference_channels, "encoding": "float32"},
        "referenceFrames": reference_frames, "captureFrames": capture_frames,
        "alignmentOffsetFrames": offset, "alignedFrames": aligned_frames,
        "maxAbsoluteError": max_error, "rmsError": rms_error,
        "requirement": "VCAB-20",
        "target": "float32 samples equal after frame alignment",
        "bitExact": equal, "pass": equal, "channelsMetrics": channel_metrics,
        "reference": reference_path.file_name().and_then(|name| name.to_str()),
        "capture": capture_path.file_name().and_then(|name| name.to_str())
    }))
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

    fn write_test_wav(path: &std::path::Path, channels: u16, samples: &[f32]) {
        let data_len = (samples.len() * 4) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(data_len + 36).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&48_000u32.to_le_bytes());
        bytes.extend_from_slice(&(48_000u32 * u32::from(channels) * 4).to_le_bytes());
        bytes.extend_from_slice(&(channels * 4).to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn wav_pair_analysis_keeps_channels_separate() {
        let unique = format!(
            "m03-quality-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&root).unwrap();
        let reference_path = root.join("reference.wav");
        let capture_path = root.join("capture.wav");
        let left = tone(512, 48_000, 997.0, 0.4);
        let right = tone(512, 48_000, 997.0, 0.2);
        let mut reference = Vec::with_capacity(1024);
        for (a, b) in left.iter().zip(right.iter()) {
            reference.extend([*a, *b]);
        }
        let mut shifted = vec![0.0, 0.0, 0.0, 0.0];
        shifted.extend_from_slice(&reference);
        write_test_wav(&reference_path, 2, &reference);
        write_test_wav(&capture_path, 2, &shifted);
        let report = analyze_wav_pair(&reference_path, &capture_path, 8, Some(997.0)).unwrap();
        assert_eq!(report["alignmentOffsetFrames"], 2);
        assert_eq!(report["bitExact"], true);
        assert_eq!(report["channelsMetrics"].as_array().unwrap().len(), 2);
        let mut broken = shifted;
        broken[(2 + 100) * 2 + 1] += 0.01;
        write_test_wav(&capture_path, 2, &broken);
        let report = analyze_wav_pair(&reference_path, &capture_path, 8, None).unwrap();
        assert_eq!(report["bitExact"], false);
        assert!(
            report["channelsMetrics"][1]["maxAbsoluteError"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wav_decoder_rejects_non_finite_samples() {
        let path = std::env::temp_dir().join(format!("m03-quality-nan-{}.wav", std::process::id()));
        write_test_wav(&path, 1, &[f32::NAN]);
        assert!(read_float_wav(&path).unwrap_err().contains("non-finite"));
        std::fs::remove_file(path).unwrap();
    }
}
