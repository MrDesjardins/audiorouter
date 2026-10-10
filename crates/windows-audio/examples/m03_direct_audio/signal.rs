//! Offline diagnostic, not a replacement for the specification's quality gates.
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use serde_json::{json, Value};

use super::{write_json, RATE, STRIDE};

pub fn write_wav(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let data = u32::try_from(bytes.len()).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(data + 36).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16u32.to_le_bytes());
    header.extend_from_slice(&3u16.to_le_bytes());
    header.extend_from_slice(&2u16.to_le_bytes());
    header.extend_from_slice(&(RATE as u32).to_le_bytes());
    header.extend_from_slice(&((RATE * STRIDE) as u32).to_le_bytes());
    header.extend_from_slice(&(STRIDE as u16).to_le_bytes());
    header.extend_from_slice(&32u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data.to_le_bytes());
    file.write_all(&header)
        .and_then(|_| file.write_all(bytes))
        .map_err(|e| e.to_string())
}

fn decode_wav(bytes: &[u8]) -> Result<Vec<[f64; 2]>, String> {
    // This diagnostic and the native harness both produce the fixed 44-byte
    // IEEE float32 WAV header. Reject other layouts instead of guessing PCM.
    if bytes.len() < 44
        || &bytes[..4] != b"RIFF"
        || &bytes[8..16] != b"WAVEfmt "
        || &bytes[36..40] != b"data"
    {
        return Err("expected the diagnostic's 44-byte float WAV header".into());
    }
    let u16_at = |offset| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    let u32_at = |offset| {
        u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("checked header"),
        )
    };
    if u32_at(16) != 16
        || u16_at(20) != 3
        || u16_at(22) != 2
        || u32_at(24) != RATE as u32
        || u32_at(28) != (RATE * STRIDE) as u32
        || u16_at(32) != STRIDE as u16
        || u16_at(34) != 32
        || u32_at(4) as usize + 8 != bytes.len()
        || u32_at(40) as usize + 44 != bytes.len()
        || (bytes.len() - 44) % STRIDE != 0
    {
        return Err("WAV size/format mismatch; require stereo float32 at 48000 Hz".into());
    }
    let frames: Vec<_> = bytes[44..]
        .chunks_exact(STRIDE)
        .map(|frame| {
            [
                f64::from(f32::from_le_bytes(
                    frame[..4].try_into().expect("frame stride"),
                )),
                f64::from(f32::from_le_bytes(
                    frame[4..].try_into().expect("frame stride"),
                )),
            ]
        })
        .collect();
    if frames.iter().flatten().any(|sample| !sample.is_finite()) {
        return Err("non-finite sample in recording".into());
    }
    Ok(frames)
}

pub fn analyze(wav: &Path, kind: &str, report: &Path, seconds: u64) -> Result<(), String> {
    let frequencies = match kind {
        "a" => [440.0, 660.0],
        "b" => [997.0, 47.0],
        _ => return Err("kind must be a or b".into()),
    };
    // Same bound as the recorder's preallocated storage, plus header slack.
    let limit =
        (RATE as u64 * super::storage_seconds(seconds) * STRIDE as u64).max(40_000_000) + 1_024;
    let mut bytes = Vec::new();
    File::open(wav)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "WAV exceeds the {seconds}-second diagnostic's storage bound"
        ));
    }
    let result = decode_wav(&bytes).and_then(|frames| measure(&frames, frequencies, seconds));
    match result {
        Ok(metrics) => {
            write_json(report, &metrics)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&metrics).map_err(|e| e.to_string())?
            );
            if metrics["passed"] == true {
                Ok(())
            } else {
                Err("signal diagnostic failed; preserve WAVs and metrics".into())
            }
        }
        Err(error) => {
            write_json(report, &json!({"passed":false,"error":error}))?;
            Err(error)
        }
    }
}

/// Active-signal length must be the requested duration within ±0.2 s.
fn duration_gate(active_frames: usize, seconds: u64) -> (bool, Option<String>) {
    let expected = RATE * seconds as usize;
    let tolerance = RATE / 5;
    if active_frames + tolerance < expected {
        (
            false,
            Some(format!(
                "fewer than {:.1} seconds of active signal; incomplete {seconds}-second diagnostic",
                (expected - tolerance) as f64 / RATE as f64
            )),
        )
    } else if active_frames > expected + tolerance {
        (
            false,
            Some(format!(
                "more than {:.1} seconds of active signal; unexpected replay or timing",
                (expected + tolerance) as f64 / RATE as f64
            )),
        )
    } else {
        (true, None)
    }
}

fn measure(frames: &[[f64; 2]], frequencies: [f64; 2], seconds: u64) -> Result<Value, String> {
    if frames.iter().flatten().any(|sample| !sample.is_finite()) {
        return Err("non-finite samples".into());
    }
    // Locate the bridge's active interval without counting pre/post-lease
    // silence as noise. Only two boundary quanta (20 ms total) are excluded
    // from fitting; all raw samples and packet flags are retained for review.
    let active: Vec<_> = frames
        .chunks_exact(480)
        .enumerate()
        .filter(|(_, block)| {
            (0..2).all(|ch| block.iter().map(|f| f[ch] * f[ch]).sum::<f64>() / 480.0 > 0.0001)
        })
        .map(|(index, _)| index)
        .collect();
    let first = *active.first().ok_or("no stereo signal")?;
    let last = *active.last().ok_or("no stereo signal")?;
    let begin = (first + 1) * 480;
    let end = last * 480;
    if end <= begin {
        return Err("too little interior signal to fit after boundary exclusion".into());
    }
    // A duration failure must not hide the waveform evidence needed to explain
    // it. Keep the same acceptance bounds, but fit every available window and
    // retain both failures in the report instead of returning before fitting.
    let (duration_passed, duration_error) = duration_gate(end - begin, seconds);
    let mut channels = Vec::new();
    let mut passed = duration_passed;
    for (channel, frequency) in frequencies.into_iter().enumerate() {
        let mut worst_rms: f64 = 0.0;
        let mut worst_peak: f64 = 0.0;
        let mut worst_dc: f64 = 0.0;
        let mut min_amplitude: f64 = f64::INFINITY;
        let mut max_amplitude: f64 = 0.0;
        let mut previous_phase: Option<f64> = None;
        let mut max_phase_jump: f64 = 0.0;
        let mut windows = Vec::new();
        for (index, block) in frames[begin..end].chunks(RATE).enumerate() {
            let first_frame = begin + index * RATE;
            let fit = fit_sine(block, channel, frequency, first_frame)?;
            worst_rms = worst_rms.max(fit.rms);
            worst_peak = worst_peak.max(fit.peak);
            worst_dc = worst_dc.max(fit.dc.abs());
            min_amplitude = min_amplitude.min(fit.amplitude);
            max_amplitude = max_amplitude.max(fit.amplitude);
            let phase_jump = if let Some(previous) = previous_phase {
                let jump = (fit.phase - previous + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                max_phase_jump = max_phase_jump.max(jump.abs());
                Some(jump)
            } else {
                None
            };
            previous_phase = Some(fit.phase);
            windows.push(json!({"firstFrame":first_frame,"frames":block.len(),
                "amplitude":fit.amplitude,"phaseRadians":fit.phase,"dc":fit.dc,
                "residualRms":fit.rms,"residualPeak":fit.peak,
                "phaseJumpRadians":phase_jump}));
        }
        // Conservative triage limits, explicitly not VCAB-20 bit-exact or
        // full THD+N/latency qualification. Hiss, drop/replay and clipping fail.
        let clean = worst_rms <= 0.00001
            && worst_peak <= 0.0001
            && worst_dc <= 0.00001
            && min_amplitude >= 0.24
            && max_amplitude <= 0.26
            && max_phase_jump <= 0.001;
        passed &= clean;
        channels.push(
            json!({"frequencyHz":frequency,"residualRms":worst_rms,"residualPeak":worst_peak,
            "dc":worst_dc,"minAmplitude":min_amplitude,"maxAmplitude":max_amplitude,
            "phaseJumpRadians":max_phase_jump,"passed":clean,"windows":windows}),
        );
    }
    Ok(
        json!({"passed":passed,"qualification":false,"rate":RATE,"totalFrames":frames.len(),
        "fitStartFrame":begin,"fitEndFrame":end,"boundaryQuantaExcluded":2,
        "analyzedSeconds":(end-begin) as f64 / RATE as f64,"expectedSeconds":seconds,
        "durationPassed":duration_passed,"durationError":duration_error,"channels":channels}),
    )
}

struct Fit {
    amplitude: f64,
    phase: f64,
    dc: f64,
    rms: f64,
    peak: f64,
}

fn fit_sine(
    frames: &[[f64; 2]],
    channel: usize,
    frequency: f64,
    first: usize,
) -> Result<Fit, String> {
    let basis = |i: usize| {
        let phase = std::f64::consts::TAU * frequency * (first + i) as f64 / RATE as f64;
        [phase.sin(), phase.cos(), 1.0]
    };
    let mut system = [[0.0; 4]; 3];
    for (i, frame) in frames.iter().enumerate() {
        let b = basis(i);
        for row in 0..3 {
            for col in 0..3 {
                system[row][col] += b[row] * b[col];
            }
            system[row][3] += b[row] * frame[channel];
        }
    }
    for col in 0..3 {
        let pivot = system[col][col];
        if pivot.abs() < 1e-9 {
            return Err("singular signal fit".into());
        }
        for value in &mut system[col][col..] {
            *value /= pivot;
        }
        let pivot_row = system[col];
        for (row, values) in system.iter_mut().enumerate() {
            if row != col {
                let factor = values[col];
                for j in col..4 {
                    values[j] -= factor * pivot_row[j];
                }
            }
        }
    }
    let coefficient = [system[0][3], system[1][3], system[2][3]];
    let mut squared = 0.0;
    let mut peak: f64 = 0.0;
    for (i, frame) in frames.iter().enumerate() {
        let b = basis(i);
        let residual = frame[channel] - (0..3).map(|j| coefficient[j] * b[j]).sum::<f64>();
        squared += residual * residual;
        peak = peak.max(residual.abs());
    }
    Ok(Fit {
        amplitude: coefficient[0].hypot(coefficient[1]),
        phase: coefficient[1].atan2(coefficient[0]),
        dc: coefficient[2],
        rms: (squared / frames.len() as f64).sqrt(),
        peak,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone() -> Vec<[f64; 2]> {
        (0..RATE * 30)
            .map(|i| {
                [997.0, 47.0].map(|f| {
                    f64::from(
                        (0.25 * (std::f64::consts::TAU * f * i as f64 / RATE as f64 + 0.7).sin())
                            as f32,
                    )
                })
            })
            .collect()
    }
    fn passes(frames: &[[f64; 2]]) -> bool {
        measure(frames, [997.0, 47.0], 30).unwrap()["passed"] == true
    }
    #[test]
    fn clean_unknown_phase_float32_passes() {
        assert!(passes(&tone()));
    }
    #[test]
    fn hiss_clipping_and_channel_swap_fail() {
        let mut frames = tone();
        for (i, f) in frames.iter_mut().enumerate() {
            f[0] += if i % 2 == 0 { 0.001 } else { -0.001 };
        }
        assert!(!passes(&frames));
        let mut frames = tone();
        for f in &mut frames {
            f[0] = f[0].clamp(-0.2, 0.2);
        }
        assert!(!passes(&frames));
        let frames: Vec<_> = tone().into_iter().map(|[a, b]| [b, a]).collect();
        assert!(!passes(&frames));
    }
    #[test]
    fn missing_and_repeated_quanta_fail() {
        let mut frames = tone();
        frames.drain(RATE * 10..RATE * 10 + 480);
        assert!(!passes(&frames));
        let mut frames = tone();
        let repeated = frames[RATE * 10..RATE * 10 + 480].to_vec();
        frames.splice(RATE * 10..RATE * 10, repeated);
        assert!(!passes(&frames));
    }
    #[test]
    fn silence_short_signal_and_nan_fail() {
        assert!(measure(&vec![[0.0; 2]; RATE], [997.0, 47.0], 30).is_err());
        let mut frames = tone();
        frames[500][0] = f64::NAN;
        assert!(measure(&frames, [997.0, 47.0], 30).is_err());
        assert!(!passes(&tone()[..RATE]));
    }

    #[test]
    fn whole_second_loss_cannot_hide_in_integer_tone_periods() {
        let mut frames = tone();
        frames.drain(RATE * 10..RATE * 11);
        assert!(!passes(&frames));
        let mut frames = tone();
        let repeated = frames[RATE * 10..RATE * 11].to_vec();
        frames.splice(RATE * 10..RATE * 10, repeated);
        assert!(!passes(&frames));
    }

    #[test]
    fn duration_failure_preserves_signal_metrics_and_phase_breaks() {
        let mut frames = tone();
        frames.drain(RATE * 26..RATE * 26 + 12_912);
        let report = measure(&frames, [997.0, 47.0], 30).unwrap();
        assert_eq!(report["passed"], false);
        assert_eq!(report["durationPassed"], false);
        assert!(report["durationError"].as_str().unwrap().contains("29.8"));
        assert!(report["fitEndFrame"].as_u64().unwrap() > 0);
        assert!(report["channels"][1]["residualRms"].as_f64().unwrap() > 0.001);
        assert!(report["channels"][1]["windows"].as_array().unwrap().len() > 25);

        let mut clean_short = tone();
        clean_short.truncate(RATE * 29);
        let report = measure(&clean_short, [997.0, 47.0], 30).unwrap();
        assert_eq!(report["durationPassed"], false);
        assert_eq!(report["passed"], false);
        assert_eq!(report["channels"][0]["passed"], true);
        assert_eq!(report["channels"][1]["passed"], true);
    }

    #[test]
    fn duration_gate_scales_with_the_requested_length() {
        assert_eq!(duration_gate(RATE * 30, 30), (true, None));
        assert!(duration_gate(RATE * 30 - RATE / 5, 30).0);
        assert!(!duration_gate(RATE * 30 - RATE / 5 - 1, 30).0);
        assert!(duration_gate(RATE * 30, 30).1.is_none());
        assert!(duration_gate(RATE * 29, 30).1.unwrap().contains("29.8"));
        assert!(duration_gate(RATE * 300, 300).0);
        assert!(
            !duration_gate(RATE * 30, 300).0,
            "a 30-second signal cannot pass 300"
        );
        assert!(
            !duration_gate(RATE * 300, 30).0,
            "a 300-second signal cannot pass 30"
        );
        assert!(duration_gate(RATE * 299, 300).1.unwrap().contains("299.8"));
        assert!(duration_gate(RATE * 301, 300).1.unwrap().contains("300.2"));
        // A clean 30-second tone measured as a 300-second run fails on duration only.
        let report = measure(&tone(), [997.0, 47.0], 300).unwrap();
        assert_eq!(report["durationPassed"], false);
        assert_eq!(report["channels"][0]["passed"], true);
        assert_eq!(report["expectedSeconds"], 300);
    }

    #[test]
    fn wav_decoder_rejects_pcm_wrong_rate_and_truncation() {
        let path =
            std::env::temp_dir().join(format!("ar-direct-{}-wav-test.wav", std::process::id()));
        let _ = std::fs::remove_file(&path);
        write_wav(&path, &[0; STRIDE * 10]).unwrap();
        let original = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(decode_wav(&original).unwrap().len(), 10);
        for (offset, replacement) in [(20, 1u32), (24, 44_100u32)] {
            let mut wrong = original.clone();
            let width = if offset == 20 { 2 } else { 4 };
            wrong[offset..offset + width].copy_from_slice(&replacement.to_le_bytes()[..width]);
            assert!(decode_wav(&wrong).is_err());
        }
        assert!(decode_wav(&original[..original.len() - 1]).is_err());
    }
}
