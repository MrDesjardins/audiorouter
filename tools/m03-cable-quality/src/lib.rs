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

/// Read PCM16 or PCM24 stored left-aligned in a 32-bit WAVEFORMATEXTENSIBLE
/// container. The returned values are normalized by 2^(valid_bits - 1).
struct PcmWavFormat {
    tag: u16,
    channels: u16,
    rate: u32,
    bits: u16,
    valid_bits: u16,
    subformat: Option<[u8; 16]>,
}

pub fn read_pcm_wav(path: &std::path::Path) -> Result<(u32, u16, u16, Vec<f32>), String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("expected a RIFF/WAVE file".into());
    }
    let mut fmt: Option<PcmWavFormat> = None;
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
                let tag = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
                let channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
                let rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
                let bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
                let (valid_bits, subformat) = if tag == 0xfffe && size >= 40 {
                    let extension_size =
                        u16::from_le_bytes(bytes[start + 16..start + 18].try_into().unwrap());
                    if extension_size < 22 {
                        return Err("invalid extensible WAV format size".into());
                    }
                    let valid =
                        u16::from_le_bytes(bytes[start + 18..start + 20].try_into().unwrap());
                    let mut guid = [0u8; 16];
                    guid.copy_from_slice(&bytes[start + 24..start + 40]);
                    (valid, Some(guid))
                } else {
                    (bits, None)
                };
                fmt = Some(PcmWavFormat {
                    tag,
                    channels,
                    rate,
                    bits,
                    valid_bits,
                    subformat,
                });
            }
            b"data" => data = Some(&bytes[start..end]),
            _ => {}
        }
        cursor = end + (size & 1);
    }
    let PcmWavFormat {
        tag,
        channels,
        rate,
        bits,
        valid_bits,
        subformat,
    } = fmt.ok_or("WAV fmt chunk is missing")?;
    if channels == 0 || rate == 0 {
        return Err("WAV channel count and sample rate must be positive".into());
    }
    let pcm_guid = [1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113];
    let (bytes_per_sample, shift) = match (tag, bits, valid_bits, subformat) {
        (1, 16, 16, None) => (2usize, 0),
        (0xfffe, 32, 24, Some(guid)) if guid == pcm_guid => (4usize, 8),
        _ => return Err("expected PCM16 or PCM24-in-32 WAVE data".into()),
    };
    let raw = data.ok_or("WAV data chunk is missing")?;
    if raw.len() % bytes_per_sample != 0
        || raw.len() % (usize::from(channels) * bytes_per_sample) != 0
    {
        return Err("WAV data is not a whole number of frames".into());
    }
    let samples = if shift == 0 {
        raw.chunks_exact(2)
            .map(|chunk| i16::from_le_bytes(chunk.try_into().unwrap()) as f32 / 32768.0)
            .collect()
    } else {
        raw.chunks_exact(4)
            .map(|chunk| {
                let packed = i32::from_le_bytes(chunk.try_into().unwrap());
                if packed & 0xff != 0 {
                    return Err("PCM24-in-32 sample has nonzero padding bits".into());
                }
                Ok::<f32, String>((packed >> 8) as f32 / 8_388_608.0)
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok((rate, channels, valid_bits, samples))
}

/// Write PCM16 or left-aligned PCM24-in-32 WAV data for portable fixtures.
pub fn write_pcm_wav(
    path: &std::path::Path,
    sample_rate: u32,
    channels: u16,
    valid_bits: u16,
    samples: &[f32],
) -> Result<(), String> {
    if sample_rate == 0
        || channels == 0
        || samples.is_empty()
        || !samples.len().is_multiple_of(usize::from(channels))
    {
        return Err("PCM WAV requires a positive rate, channels, and whole nonempty frames".into());
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err("PCM samples must be finite".into());
    }
    let pcm24 = valid_bits == 24;
    if valid_bits != 16 && !pcm24 {
        return Err("only PCM16 and PCM24-in-32 are supported".into());
    }
    let (tag, bits, block_align, fmt_size) = if pcm24 {
        (
            0xfffeu16,
            32u16,
            channels.checked_mul(4).ok_or("channel count overflow")?,
            40u32,
        )
    } else {
        (
            1u16,
            16u16,
            channels.checked_mul(2).ok_or("channel count overflow")?,
            16u32,
        )
    };
    let sample_bytes = if pcm24 { 4usize } else { 2usize };
    let data_size = samples
        .len()
        .checked_mul(sample_bytes)
        .ok_or("PCM WAV data size overflow")?;
    let riff_size = 4usize + 8 + fmt_size as usize + 8 + data_size;
    let riff_size = u32::try_from(riff_size).map_err(|_| "PCM WAV exceeds RIFF size limit")?;
    let data_size = u32::try_from(data_size).map_err(|_| "PCM WAV exceeds RIFF size limit")?;
    let mut out = Vec::with_capacity(riff_size as usize + 8);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&fmt_size.to_le_bytes());
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(
        &(sample_rate
            .checked_mul(u32::from(block_align))
            .ok_or("PCM WAV byte rate overflow")?)
        .to_le_bytes(),
    );
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    if pcm24 {
        out.extend_from_slice(&22u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&[1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
    }
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
    for sample in samples {
        if pcm24 {
            let value =
                ((*sample as f64 * 8_388_608.0).round() as i64).clamp(-8_388_608, 8_388_607) as i32;
            out.extend_from_slice(&(value << 8).to_le_bytes());
        } else {
            let value = ((*sample as f64 * 32_768.0).round() as i64).clamp(-32_768, 32_767) as i16;
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    std::fs::write(path, out).map_err(|error| format!("{}: {error}", path.display()))
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

pub fn analyze_pcm_pair_wav(
    reference_path: &std::path::Path,
    capture_path: &std::path::Path,
    max_offset: usize,
    valid_bits: u16,
) -> Result<serde_json::Value, String> {
    if !matches!(valid_bits, 16 | 24) {
        return Err("PCM measurement supports 16 or 24 valid bits".into());
    }
    let mut report = analyze_wav_pair(reference_path, capture_path, max_offset, None)?;
    let max_error = report["maxAbsoluteError"]
        .as_f64()
        .ok_or("missing maximum sample error")?;
    let lsb = 1.0 / 2f64.powi(i32::from(valid_bits) - 1);
    report["requirement"] = serde_json::json!("VCAB-21");
    report["target"] =
        serde_json::json!(format!("PCM{valid_bits} round-trip error <= 1 LSB ({lsb})"));
    report["pass"] = serde_json::json!(max_error <= lsb);
    report["validBits"] = serde_json::json!(valid_bits);
    report["maximumErrorLsb"] = serde_json::json!(max_error / lsb);
    Ok(report)
}

pub fn analyze_silence_wav(
    capture_path: &std::path::Path,
    max_peak_dbfs: f64,
) -> Result<serde_json::Value, String> {
    if !max_peak_dbfs.is_finite() || max_peak_dbfs > 0.0 {
        return Err("maximum peak threshold must be a finite dBFS value at or below 0".into());
    }
    let (sample_rate, channels, samples) = read_float_wav(capture_path)?;
    if samples.is_empty() {
        return Err("WAV files contain no samples".into());
    }
    let observed = metrics(&samples);
    let peak_dbfs = if observed.peak == 0.0 {
        -1000.0
    } else {
        20.0 * observed.peak.log10()
    };
    let pass = peak_dbfs <= max_peak_dbfs;
    Ok(serde_json::json!({
        "requirement": "VCAB-26",
        "target": format!("unrouted cable isolation peak <= {max_peak_dbfs} dBFS"),
        "pass": pass,
        "format": {"sampleRateHz": sample_rate, "channels": channels, "encoding": "float32"},
        "frames": observed.frames / usize::from(channels),
        "peak": observed.peak,
        "peakDbfs": peak_dbfs,
        "rms": observed.rms,
        "dc": observed.dc,
        "capture": capture_path.file_name().and_then(|name| name.to_str())
    }))
}

pub fn analyze_tone_wav(
    capture_path: &std::path::Path,
    frequency_hz: f64,
    max_thdn_db: f64,
) -> Result<serde_json::Value, String> {
    let (sample_rate, channels, samples) = read_float_wav(capture_path)?;
    analyze_tone_samples(
        capture_path,
        sample_rate,
        channels,
        &samples,
        frequency_hz,
        max_thdn_db,
        None,
    )
}

pub fn analyze_tone_pair_wav(
    reference_path: &std::path::Path,
    capture_path: &std::path::Path,
    frequency_hz: f64,
    max_thdn_db: f64,
) -> Result<serde_json::Value, String> {
    let (reference_rate, reference_channels, reference) = read_float_wav(reference_path)?;
    let (capture_rate, capture_channels, capture) = read_float_wav(capture_path)?;
    if reference_rate != capture_rate || reference_channels != capture_channels {
        return Err("reference and capture WAV formats do not match".into());
    }
    let channels = usize::from(reference_channels);
    let reference_frames = reference.len() / channels;
    let first_reference = reference
        .iter()
        .step_by(channels)
        .copied()
        .collect::<Vec<_>>();
    let first_capture = capture
        .iter()
        .step_by(channels)
        .copied()
        .collect::<Vec<_>>();
    let (offset, _) = align(&first_reference, &first_capture, 96_000)
        .ok_or("could not align tone reference and capture")?;
    let end = offset
        .checked_add(reference_frames)
        .ok_or("tone capture range overflow")?;
    if end > capture.len() / channels {
        return Err("capture ended before the reference tone finished".into());
    }
    let samples = &capture[offset * channels..end * channels];
    analyze_tone_samples(
        capture_path,
        capture_rate,
        capture_channels,
        samples,
        frequency_hz,
        max_thdn_db,
        Some(offset),
    )
}

/// Compare a captured logarithmic sweep to its reference using local
/// Hann-windowed single-frequency projections. This measures relative
/// passband magnitude only; aliasing and phase response need separate cases.
pub fn analyze_sweep_pair_wav(
    reference_path: &std::path::Path,
    capture_path: &std::path::Path,
    start_hz: f64,
    end_hz: f64,
    max_deviation_db: f64,
) -> Result<serde_json::Value, String> {
    if !start_hz.is_finite()
        || !end_hz.is_finite()
        || start_hz <= 0.0
        || end_hz <= start_hz
        || !max_deviation_db.is_finite()
        || max_deviation_db <= 0.0
    {
        return Err("sweep range and positive response tolerance must be finite and valid".into());
    }
    let (rate, channels, reference) = read_float_wav(reference_path)?;
    let (capture_rate, capture_channels, capture) = read_float_wav(capture_path)?;
    if rate != capture_rate || channels != capture_channels {
        return Err("reference and capture WAV formats do not match".into());
    }
    if end_hz >= f64::from(rate) / 2.0 {
        return Err("sweep end frequency must be below Nyquist".into());
    }
    let channel_count = usize::from(channels);
    let reference_frames = reference.len() / channel_count;
    let capture_frames = capture.len() / channel_count;
    let window = (rate as usize / 4).max(256);
    if reference_frames < window * 2 {
        return Err("sweep WAV is too short for response analysis".into());
    }
    let first_reference = reference
        .iter()
        .step_by(channel_count)
        .copied()
        .collect::<Vec<_>>();
    let first_capture = capture
        .iter()
        .step_by(channel_count)
        .copied()
        .collect::<Vec<_>>();
    let (offset, _) = align(&first_reference, &first_capture, 96_000)
        .ok_or("could not align sweep reference and capture")?;
    let half = window / 2;
    if offset
        .checked_add(reference_frames)
        .is_none_or(|end| end > capture_frames)
    {
        return Err("capture ended before the reference sweep finished".into());
    }
    let duration = reference_frames as f64 / f64::from(rate);
    let logarithmic_range = (end_hz / start_hz).ln();
    let first_time = half as f64 / f64::from(rate);
    let last_time = (reference_frames - half) as f64 / f64::from(rate);
    let first_hz = start_hz * (logarithmic_range * first_time / duration).exp();
    let last_hz = start_hz * (logarithmic_range * last_time / duration).exp();
    let points = 48usize;
    let frequencies = (0..points)
        .map(|index| first_hz * (last_hz / first_hz).powf(index as f64 / (points - 1) as f64))
        .collect::<Vec<_>>();
    let mut per_channel = Vec::with_capacity(channel_count);
    let mut worst = 0.0f64;
    for channel in 0..channel_count {
        let mut response = Vec::with_capacity(points);
        let mut channel_worst = 0.0f64;
        for &frequency in &frequencies {
            let center_time = duration * (frequency / start_hz).ln() / logarithmic_range;
            let center = (center_time * f64::from(rate)).round() as usize;
            let begin = center.saturating_sub(half);
            let ref_mono = (begin..begin + window)
                .map(|frame| reference[frame * channel_count + channel])
                .collect::<Vec<_>>();
            let capture_begin = offset + begin;
            let captured_mono = (capture_begin..capture_begin + window)
                .map(|frame| capture[frame * channel_count + channel])
                .collect::<Vec<_>>();
            let reference_amplitude = projected_amplitude(&ref_mono, rate, frequency);
            let capture_amplitude = projected_amplitude(&captured_mono, rate, frequency);
            if reference_amplitude <= f64::MIN_POSITIVE || capture_amplitude <= f64::MIN_POSITIVE {
                return Err("sweep response contains silence at an analysis frequency".into());
            }
            let deviation_db = 20.0 * (capture_amplitude / reference_amplitude).log10();
            channel_worst = channel_worst.max(deviation_db.abs());
            response
                .push(serde_json::json!({"frequencyHz": frequency, "deviationDb": deviation_db}));
        }
        worst = worst.max(channel_worst);
        per_channel.push(serde_json::json!({"channel": channel, "maxAbsoluteDeviationDb": channel_worst, "points": response}));
    }
    Ok(serde_json::json!({
        "requirement": "VCAB-22",
        "target": format!("sweep passband magnitude deviation <= {max_deviation_db} dB"),
        "pass": worst <= max_deviation_db,
        "format": {"sampleRateHz": rate, "channels": channels, "encoding": "float32"},
        "frames": reference_frames,
        "alignmentOffsetFrames": offset,
        "frequencyRangeHz": {"start": start_hz, "end": end_hz},
        "analysisPointsPerChannel": points,
        "maxDeviationDb": max_deviation_db,
        "worstDeviationDb": worst,
        "channelsMetrics": per_channel,
        "scope": "relative passband magnitude only; aliasing and phase response are not measured"
    }))
}

fn projected_amplitude(samples: &[f32], sample_rate: u32, frequency_hz: f64) -> f64 {
    let mut real = 0.0;
    let mut imaginary = 0.0;
    let mut window_sum = 0.0;
    let length = samples.len();
    for (index, sample) in samples.iter().enumerate() {
        let window = 0.5 - 0.5 * (std::f64::consts::TAU * index as f64 / (length - 1) as f64).cos();
        let phase = std::f64::consts::TAU * frequency_hz * index as f64 / f64::from(sample_rate);
        real += f64::from(*sample) * window * phase.cos();
        imaginary += f64::from(*sample) * window * phase.sin();
        window_sum += window;
    }
    2.0 * real.hypot(imaginary) / window_sum
}

fn analyze_tone_samples(
    capture_path: &std::path::Path,
    sample_rate: u32,
    channels: u16,
    samples: &[f32],
    frequency_hz: f64,
    max_thdn_db: f64,
    alignment_offset: Option<usize>,
) -> Result<serde_json::Value, String> {
    if !frequency_hz.is_finite()
        || frequency_hz <= 0.0
        || !max_thdn_db.is_finite()
        || max_thdn_db > 0.0
    {
        return Err("frequency must be positive and the THD+N target must be a finite dB value at or below 0".into());
    }
    if frequency_hz >= f64::from(sample_rate) / 2.0 {
        return Err("tone frequency must be below Nyquist".into());
    }
    let channel_count = usize::from(channels);
    let frames = samples.len() / channel_count;
    if frames < 8 {
        return Err("WAV is too short for a tone fit".into());
    }
    let mut channel_values = Vec::with_capacity(channel_count);
    let mut worst = f64::NEG_INFINITY;
    for channel in 0..channel_count {
        let mono = (0..frames)
            .map(|frame| samples[frame * channel_count + channel])
            .collect::<Vec<_>>();
        let thdn =
            thdn_db(&mono, sample_rate, frequency_hz).ok_or("could not fit the requested tone")?;
        if !thdn.is_finite() {
            return Err("tone fit produced a non-finite THD+N value".into());
        }
        worst = worst.max(thdn);
        channel_values.push(serde_json::json!({"channel": channel, "thdnDb": thdn}));
    }
    let pass = worst <= max_thdn_db;
    Ok(serde_json::json!({
        "requirement": "VCAB-22/23",
        "target": format!("{frequency_hz} Hz THD+N <= {max_thdn_db} dB"),
        "pass": pass,
        "format": {"sampleRateHz": sample_rate, "channels": channels, "encoding": "float32"},
        "frames": frames,
        "alignmentOffsetFrames": alignment_offset,
        "frequencyHz": frequency_hz,
        "maxThdnDb": max_thdn_db,
        "worstThdnDb": worst,
        "channelsMetrics": channel_values,
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
    fn wav_pair_analysis_aligns_all_required_channel_counts_together() {
        let root =
            std::env::temp_dir().join(format!("m03-quality-channel-matrix-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let reference_path = root.join("reference.wav");
        let capture_path = root.join("capture.wav");
        for channels in [1u16, 2, 8] {
            let mono = (0..channels)
                .map(|channel| seeded_noise(2048, 0x1234 + u64::from(channel)))
                .collect::<Vec<_>>();
            let mut reference = Vec::with_capacity(2048 * usize::from(channels));
            for frame in 0..2048 {
                for channel in &mono {
                    reference.push(channel[frame]);
                }
            }
            let mut capture = vec![0.0; 5 * usize::from(channels)];
            capture.extend_from_slice(&reference);
            write_test_wav(&reference_path, channels, &reference);
            write_test_wav(&capture_path, channels, &capture);
            let report = analyze_wav_pair(&reference_path, &capture_path, 16, None).unwrap();
            assert_eq!(report["alignmentOffsetFrames"], 5);
            assert_eq!(report["bitExact"], true);
            assert_eq!(
                report["channelsMetrics"].as_array().unwrap().len(),
                usize::from(channels)
            );
            let mut corrupted = capture;
            corrupted[(5 + 1500) * usize::from(channels) + usize::from(channels - 1)] += 0.01;
            write_test_wav(&capture_path, channels, &corrupted);
            let report = analyze_wav_pair(&reference_path, &capture_path, 16, None).unwrap();
            assert_eq!(report["bitExact"], false);
            assert!(
                report["channelsMetrics"][usize::from(channels - 1)]["maxAbsoluteError"]
                    .as_f64()
                    .unwrap()
                    > 0.0
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wav_decoder_rejects_non_finite_samples() {
        let path = std::env::temp_dir().join(format!("m03-quality-nan-{}.wav", std::process::id()));
        write_test_wav(&path, 1, &[f32::NAN]);
        assert!(read_float_wav(&path).unwrap_err().contains("non-finite"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn pcm16_and_pcm24_in_32_round_trip_with_separate_channels() {
        let root = std::env::temp_dir().join(format!("m03-quality-pcm-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let input = [-1.0, 0.0, 0.25, 0.89, -0.89, 0.5];
        for valid_bits in [16, 24] {
            let path = root.join(format!("pcm{valid_bits}.wav"));
            write_pcm_wav(&path, 48_000, 2, valid_bits, &input).unwrap();
            let (rate, channels, decoded_bits, decoded) = read_pcm_wav(&path).unwrap();
            assert_eq!((rate, channels, decoded_bits), (48_000, 2, valid_bits));
            let lsb = 1.0 / 2f64.powi(i32::from(valid_bits) - 1);
            for (source, captured) in input.iter().zip(decoded.iter()) {
                assert!((f64::from(*source) - f64::from(*captured)).abs() <= lsb);
            }
            assert_ne!(decoded[2], decoded[3]);
        }
        let pcm24 = root.join("pcm24.wav");
        let mut bytes = std::fs::read(&pcm24).unwrap();
        let data = bytes
            .windows(4)
            .position(|window| window == b"data")
            .unwrap()
            + 8;
        bytes[data] = 1;
        std::fs::write(&pcm24, bytes).unwrap();
        assert!(read_pcm_wav(&pcm24).unwrap_err().contains("padding bits"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pcm_writer_rejects_non_finite_or_partial_frames() {
        let path = std::env::temp_dir().join(format!(
            "m03-quality-pcm-invalid-{}.wav",
            std::process::id()
        ));
        assert!(write_pcm_wav(&path, 48_000, 2, 16, &[f32::NAN, 0.0]).is_err());
        assert!(write_pcm_wav(&path, 48_000, 2, 16, &[0.0]).is_err());
        assert!(!path.exists());
    }

    #[test]
    fn pcm_pair_verdict_uses_the_requested_lsb_limit() {
        let root =
            std::env::temp_dir().join(format!("m03-quality-pcm-pair-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let reference_path = root.join("reference.wav");
        let capture_path = root.join("capture.wav");
        let reference = tone(4096, 48_000, 997.0, 0.7);
        let captured = reference
            .iter()
            .map(|sample| (*sample * 32768.0).round() / 32768.0)
            .collect::<Vec<_>>();
        write_test_wav(&reference_path, 1, &reference);
        write_test_wav(&capture_path, 1, &captured);
        let report = analyze_pcm_pair_wav(&reference_path, &capture_path, 32, 16).unwrap();
        assert_eq!(report["requirement"], "VCAB-21");
        assert_eq!(report["pass"], true);
        assert!(report["maximumErrorLsb"].as_f64().unwrap() <= 1.0);
        let mut outside = captured;
        outside[100] += 2.0 / 32768.0;
        write_test_wav(&capture_path, 1, &outside);
        let report = analyze_pcm_pair_wav(&reference_path, &capture_path, 32, 16).unwrap();
        assert_eq!(report["pass"], false);
        assert!(analyze_pcm_pair_wav(&reference_path, &capture_path, 32, 32).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sweep_response_metric_reports_passband_gain_and_tolerance() {
        let root = std::env::temp_dir().join(format!("m03-quality-sweep-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let reference_path = root.join("reference.wav");
        let capture_path = root.join("capture.wav");
        let mono = log_sweep(48_000 * 4, 48_000, 10.0, 20_000.0, 0.8);
        let mut reference = Vec::with_capacity(mono.len() * 2);
        let mut capture = vec![0.0; 26];
        let gain = 10f64.powf(0.03 / 20.0) as f32;
        for sample in &mono {
            reference.extend([*sample, *sample]);
            capture.extend([*sample * gain, *sample * gain]);
        }
        write_test_wav(&reference_path, 2, &reference);
        write_test_wav(&capture_path, 2, &capture);
        let report =
            analyze_sweep_pair_wav(&reference_path, &capture_path, 20.0, 20_000.0, 0.05).unwrap();
        assert_eq!(report["requirement"], "VCAB-22");
        assert_eq!(report["pass"], true);
        assert_eq!(report["alignmentOffsetFrames"], 13);
        assert_eq!(report["channelsMetrics"].as_array().unwrap().len(), 2);
        let gain = 10f64.powf(0.10 / 20.0) as f32;
        let outside = mono
            .iter()
            .flat_map(|sample| [*sample * gain, *sample * gain])
            .collect::<Vec<_>>();
        write_test_wav(&capture_path, 2, &outside);
        let report =
            analyze_sweep_pair_wav(&reference_path, &capture_path, 20.0, 20_000.0, 0.05).unwrap();
        assert_eq!(report["pass"], false);
        assert!(
            analyze_sweep_pair_wav(&reference_path, &capture_path, 20.0, 24_000.0, 0.05).is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn silence_metric_applies_peak_threshold_in_dbfs() {
        let path =
            std::env::temp_dir().join(format!("m03-quality-silence-{}.wav", std::process::id()));
        write_test_wav(&path, 2, &[0.0, 0.0, 1.0e-8, -1.0e-8]);
        let report = analyze_silence_wav(&path, -140.0).unwrap();
        assert_eq!(report["requirement"], "VCAB-26");
        assert_eq!(report["pass"], true);
        assert!(report["peakDbfs"].as_f64().unwrap() <= -140.0);
        let report = analyze_silence_wav(&path, -170.0).unwrap();
        assert_eq!(report["pass"], false);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tone_metric_reports_per_channel_thdn_verdict() {
        let path =
            std::env::temp_dir().join(format!("m03-quality-tone-{}.wav", std::process::id()));
        let reference_path = std::env::temp_dir().join(format!(
            "m03-quality-tone-reference-{}.wav",
            std::process::id()
        ));
        let clean = tone(48_000, 48_000, 997.0, 0.89);
        let mut stereo = Vec::with_capacity(clean.len() * 2);
        for sample in &clean {
            stereo.extend([*sample, *sample]);
        }
        write_test_wav(&path, 2, &stereo);
        let report = analyze_tone_wav(&path, 997.0, -100.0).unwrap();
        assert_eq!(report["pass"], true);
        assert_eq!(report["channelsMetrics"].as_array().unwrap().len(), 2);
        let distorted = clean
            .iter()
            .enumerate()
            .flat_map(|(i, sample)| {
                let harmonic = (0.001
                    * (std::f64::consts::TAU * 2.0 * 997.0 * i as f64 / 48_000.0).sin())
                    as f32;
                [*sample, *sample + harmonic]
            })
            .collect::<Vec<_>>();
        write_test_wav(&path, 2, &distorted);
        let report = analyze_tone_wav(&path, 997.0, -100.0).unwrap();
        assert_eq!(report["pass"], false);
        assert!(report["worstThdnDb"].as_f64().unwrap() > -70.0);
        assert!(analyze_tone_wav(&path, 24_000.0, -100.0).is_err());
        write_test_wav(&reference_path, 2, &stereo);
        let mut captured = vec![0.0; 14];
        captured.extend_from_slice(&distorted);
        captured.extend([0.0; 16]);
        write_test_wav(&path, 2, &captured);
        let report = analyze_tone_pair_wav(&reference_path, &path, 997.0, -100.0).unwrap();
        assert_eq!(report["alignmentOffsetFrames"], 7);
        assert_eq!(report["pass"], false);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(reference_path).unwrap();
    }
}
