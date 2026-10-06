//! Run labelled Siege reference takes through the engine's own Advanced EQ,
//! Compressor and Limiter, and measure how loud distant steps and drones are
//! relative to gunfire (docs/plans/future/siege-footstep-eq.md).
//!
//! Usage: cargo run -p audiorouter-dsp --example siege_chain --release -- <folder> [--grid]
//! Reads `NN-*.wav` (PCM 16/24-bit or float, stereo, 48 kHz). Writes nothing.

use audiorouter_dsp::{
    BiquadParams, Compressor, CompressorParams, FilterKind, LimiterParams, ParametricEq,
    PeakLimiter, PARAMETRIC_EQ_BANDS,
};
use std::path::Path;

const RATE: f32 = 48_000.0;
const BLOCK: usize = 128;
const WINDOW: usize = 2_400; // 50 ms

struct Take {
    name: String,
    samples: Vec<f32>, // interleaved stereo
}

fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let (mut offset, mut fmt, mut data) = (12usize, None, None);
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let body = offset + 8;
        if id == b"fmt " {
            let mut format = u16::from_le_bytes([bytes[body], bytes[body + 1]]);
            if format == 0xFFFE {
                format = u16::from_le_bytes([bytes[body + 24], bytes[body + 25]]);
            }
            let channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
            let rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().unwrap());
            let bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
            fmt = Some((format, channels, rate, bits));
        } else if id == b"data" {
            data = Some((body, size.min(bytes.len() - body)));
        }
        offset = body + size + (size & 1);
    }
    let (format, channels, rate, bits) = fmt.ok_or("missing fmt chunk")?;
    let (start, size) = data.ok_or("missing data chunk")?;
    if channels != 2 || rate != 48_000 {
        return Err(format!("need stereo 48 kHz, got {channels} ch {rate} Hz"));
    }
    let width = usize::from(bits / 8);
    let raw = &bytes[start..start + size - size % width];
    let samples = raw
        .chunks_exact(width)
        .map(|s| match (format, bits) {
            (1, 16) => f32::from(i16::from_le_bytes([s[0], s[1]])) / 32_768.0,
            (1, 24) => ((i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32) / 8_388_608.0,
            (3, 32) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            _ => f32::NAN,
        })
        .collect::<Vec<_>>();
    if samples.iter().any(|v| v.is_nan()) {
        return Err(format!("unsupported format {format}/{bits}-bit"));
    }
    Ok(samples)
}

/// The Siege Footstep EQ v2 bands as saved in the user's session.
fn footstep_eq() -> ParametricEq {
    let band = |kind, frequency_hz, q, gain_db| {
        Some(BiquadParams {
            kind,
            frequency_hz,
            q,
            gain_db,
            sample_rate: RATE,
        })
    };
    let mut bands: [Option<BiquadParams>; PARAMETRIC_EQ_BANDS] = [None; PARAMETRIC_EQ_BANDS];
    let v2 = [
        band(FilterKind::HighPass, 50.0, 0.707, 0.0),
        band(FilterKind::Peaking, 200.0, 1.0, 4.0),
        band(FilterKind::HighShelf, 3_000.0, 0.7, 3.0),
        band(FilterKind::Peaking, 7_000.0, 0.9, 6.0),
        band(FilterKind::Peaking, 165.0, 0.65, -3.5),
        band(FilterKind::Peaking, 770.0, 1.0, 3.0),
        band(FilterKind::Peaking, 2_100.0, 2.5, -2.0),
        band(FilterKind::Peaking, 3_200.0, 2.8, 3.5),
    ];
    bands[..v2.len()].copy_from_slice(&v2);
    ParametricEq::new(bands, 2).expect("valid EQ bands")
}

#[derive(Clone, Copy, Debug)]
struct Dynamics {
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    makeup_db: f32,
}

const CURRENT: Dynamics = Dynamics {
    threshold_db: -42.0,
    ratio: 3.0,
    attack_ms: 15.0,
    release_ms: 120.0,
    makeup_db: 6.0,
};

/// Process a take; returns output samples and the compressor's gain reduction per window.
fn process(take: &Take, eq: bool, dynamics: Option<Dynamics>) -> (Vec<f32>, Vec<f32>) {
    let mut out = take.samples.clone();
    let mut equalizer = footstep_eq();
    let mut compressor = dynamics.map(|d| {
        Compressor::new(
            CompressorParams {
                threshold_db: d.threshold_db,
                ratio: d.ratio,
                attack_ms: d.attack_ms,
                release_ms: d.release_ms,
                knee_db: 6.0,
                makeup_db: d.makeup_db,
                sample_rate: RATE,
            },
            2,
        )
        .expect("valid compressor")
    });
    let mut limiter = dynamics.map(|_| {
        PeakLimiter::new_at_sample_rate(
            LimiterParams {
                ceiling_db: -1.0,
                lookahead_ms: 5.0,
                release_ms: 100.0,
            },
            RATE,
            2,
        )
        .expect("valid limiter")
    });
    let mut reduction = Vec::new();
    let (mut window_sum, mut window_frames) = (0.0f32, 0usize);
    for block in out.chunks_mut(BLOCK * 2) {
        if eq {
            equalizer.process_interleaved(block);
        }
        if let Some(c) = compressor.as_mut() {
            c.process_interleaved(block);
            window_sum += c.gain_reduction_db() * (block.len() / 2) as f32;
            window_frames += block.len() / 2;
            if window_frames >= WINDOW {
                reduction.push(window_sum / window_frames as f32);
                (window_sum, window_frames) = (0.0, 0);
            }
        }
        if let Some(l) = limiter.as_mut() {
            l.process_interleaved(block);
        }
    }
    (out, reduction)
}

fn window_levels(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks_exact(WINDOW * 2)
        .map(|w| {
            10.0 * (w.iter().map(|v| v * v).sum::<f32>() / w.len() as f32)
                .max(1e-12)
                .log10()
        })
        .collect()
}

fn percentile(values: &[f32], fraction: f32) -> f32 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f32::total_cmp);
    sorted[((sorted.len() as f32 * fraction) as usize).min(sorted.len() - 1)]
}

fn power_mean(values: &[f32]) -> f32 {
    10.0 * (values.iter().map(|db| 10f32.powf(db / 10.0)).sum::<f32>() / values.len().max(1) as f32)
        .log10()
}

/// Distant events: windows at least 6 dB above the take's quiet floor, quieter half.
/// Gunfire and ambience: the loud (95th) and typical (median) window.
struct Measure {
    far_db: f32,
    loud_db: f32,
    event_windows: Vec<usize>,
}

fn measure(levels: &[f32]) -> Measure {
    let floor_db = percentile(levels, 0.2);
    let mut events: Vec<(usize, f32)> = levels
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, l)| *l >= floor_db + 6.0)
        .collect();
    events.sort_by(|a, b| a.1.total_cmp(&b.1));
    let far = &events[..(events.len() / 2).max(events.len().min(1))];
    Measure {
        far_db: power_mean(&far.iter().map(|(_, l)| *l).collect::<Vec<_>>()),
        loud_db: percentile(levels, 0.95),
        event_windows: events.iter().map(|(i, _)| *i).collect(),
    }
}

struct Scenario {
    steps_vs_gun: f32,
    drones_vs_gun: f32,
    step_squeeze_db: f32,
    gun_reduction_db: f32,
    ambience_lift_db: f32,
    output_peak_db: f32,
}

fn evaluate(
    takes: &[Take],
    eq: bool,
    dynamics: Option<Dynamics>,
) -> (Scenario, Vec<(String, f32)>) {
    let results: Vec<_> = takes
        .iter()
        .map(|t| (t, process(t, eq, dynamics)))
        .collect();
    let find = |prefix: &str| {
        results
            .iter()
            .find(|(t, _)| t.name.starts_with(prefix))
            .expect("labelled take present")
    };
    let gun = measure(&window_levels(&find("12-").1 .0));
    let ambience_raw = percentile(&window_levels(&find("13-").0.samples), 0.5);
    let ambience = percentile(&window_levels(&find("13-").1 .0), 0.5);
    let (mut steps, mut drones, mut squeeze, mut per_take) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut peak = f32::MIN;
    for (take, (out, reduction)) in &results {
        peak = peak.max(out.iter().fold(0.0f32, |m, v| m.max(v.abs())));
        let number: u32 = take
            .name
            .split('-')
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        if !(1..=11).contains(&number) && !(15..=16).contains(&number) {
            continue;
        }
        let m = measure(&window_levels(out));
        let relative = m.far_db - gun.loud_db;
        per_take.push((take.name.clone(), relative));
        if number <= 11 {
            steps.push(relative);
            squeeze.extend(
                m.event_windows
                    .iter()
                    .filter_map(|i| reduction.get(*i))
                    .copied(),
            );
        } else {
            drones.push(relative);
        }
    }
    let gun_reduction = {
        let (_, reduction) = &find("12-").1;
        if reduction.is_empty() {
            0.0
        } else {
            percentile(reduction, 0.95)
        }
    };
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    (
        Scenario {
            steps_vs_gun: mean(&steps),
            drones_vs_gun: mean(&drones),
            step_squeeze_db: if squeeze.is_empty() {
                0.0
            } else {
                percentile(&squeeze, 0.5)
            },
            gun_reduction_db: gun_reduction,
            ambience_lift_db: ambience - ambience_raw,
            output_peak_db: 20.0 * peak.max(1e-9).log10(),
        },
        per_take,
    )
}

fn row(label: &str, s: &Scenario) {
    println!(
        "{label:<44} steps-gun {:>6.1}  drones-gun {:>6.1}  step squeeze {:>4.1}  gun GR {:>4.1}  ambience {:>+5.1}  peak {:>5.1}",
        s.steps_vs_gun, s.drones_vs_gun, s.step_squeeze_db, s.gun_reduction_db, s.ambience_lift_db, s.output_peak_db
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let folder = args
        .next()
        .expect("usage: siege_chain <folder> [--grid] [--try thr,ratio,attack,release,makeup ...]");
    let rest: Vec<String> = args.collect();
    let grid = rest.iter().any(|a| a == "--grid");
    let tries: Vec<Dynamics> = rest
        .iter()
        .skip_while(|a| *a != "--try")
        .skip(1)
        .map(|spec| {
            let v: Vec<f32> = spec
                .split(',')
                .map(|n| n.parse().expect("number"))
                .collect();
            assert_eq!(v.len(), 5, "--try needs thr,ratio,attack,release,makeup");
            Dynamics {
                threshold_db: v[0],
                ratio: v[1],
                attack_ms: v[2],
                release_ms: v[3],
                makeup_db: v[4],
            }
        })
        .collect();
    let mut entries: Vec<_> = std::fs::read_dir(&folder)
        .expect("readable folder")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.ends_with(".wav")
                && name
                    .split('-')
                    .next()
                    .is_some_and(|n| n.parse::<u32>().is_ok())
        })
        .collect();
    entries.sort();
    let takes: Vec<Take> = entries
        .iter()
        .map(|p| Take {
            name: p.file_name().unwrap().to_string_lossy().into_owned(),
            samples: read_wav(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
        })
        .collect();
    println!("{} takes. Levels are 50 ms RMS dBFS; 'steps-gun' = distant steps minus loud gunfire (higher is better).", takes.len());
    let (raw, raw_takes) = evaluate(&takes, false, None);
    row("raw (no processing)", &raw);
    let (eq_only, _) = evaluate(&takes, true, None);
    row("EQ v2 only", &eq_only);
    let (current, current_takes) = evaluate(&takes, true, Some(CURRENT));
    row("EQ v2 + current compressor + limiter", &current);
    println!("\nPer take, distant level minus gunfire (raw -> current chain):");
    for ((name, before), (_, after)) in raw_takes.iter().zip(&current_takes) {
        println!(
            "  {name:<44} {before:>6.1} -> {after:>6.1}  ({:+.1})",
            after - before
        );
    }
    if !tries.is_empty() {
        println!("\nRequested settings (EQ v2 + compressor + limiter):");
        for d in &tries {
            let (s, _) = evaluate(&takes, true, Some(*d));
            row(
                &format!(
                    "thr {:.0} {:.0}:1 att {:.0} rel {:.0} makeup {:.0}",
                    d.threshold_db, d.ratio, d.attack_ms, d.release_ms, d.makeup_db
                ),
                &s,
            );
        }
    }
    if grid {
        println!("\nGrid (EQ v2 + compressor + limiter), best first by steps-gun with step squeeze <= 2 dB and peak <= -1 dBFS:");
        let mut candidates = Vec::new();
        for threshold_db in [-50.0, -46.0, -42.0, -38.0, -34.0] {
            for ratio in [2.0, 3.0, 4.0, 6.0] {
                for attack_ms in [5.0, 15.0, 30.0] {
                    for release_ms in [80.0, 120.0, 200.0] {
                        for makeup_db in [4.0, 6.0, 8.0, 10.0] {
                            let d = Dynamics {
                                threshold_db,
                                ratio,
                                attack_ms,
                                release_ms,
                                makeup_db,
                            };
                            let (s, _) = evaluate(&takes, true, Some(d));
                            if s.step_squeeze_db <= 2.0 && s.output_peak_db <= -0.9 {
                                candidates.push((d, s));
                            }
                        }
                    }
                }
            }
        }
        candidates.sort_by(|a, b| b.1.steps_vs_gun.total_cmp(&a.1.steps_vs_gun));
        for (d, s) in candidates.iter().take(8) {
            row(
                &format!(
                    "thr {:.0} {:.0}:1 att {:.0} rel {:.0} makeup {:.0}",
                    d.threshold_db, d.ratio, d.attack_ms, d.release_ms, d.makeup_db
                ),
                s,
            );
        }
        println!("{} settings met the limits.", candidates.len());
    }
}
