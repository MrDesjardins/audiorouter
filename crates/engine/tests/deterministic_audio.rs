//! Signal qualification through the production graph compiler. Oracles use
//! specified transfer laws, known frequencies and direct sample arithmetic.
//! Set AUDIOROUTER_SIGNAL_ARTIFACTS to retain generated float WAVs and metrics.
use audiorouter_domain::{EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_session_at_sample_rate, AudioBlock, RuntimeGeneration, RuntimeGraph,
};
use serde_json::{json, Value};
use std::{f64::consts::TAU, fs, io::Write, path::Path};

const QUANTUM: usize = 128;
const RATES: [u32; 3] = [44_100, 48_000, 96_000];

fn session(kind: NodeKind, parameters: Value, channels: usize, bypass: bool) -> Session {
    Session {
        id: EntityId::new("signal-session"),
        name: "Synthetic signal qualification".into(),
        schema_version: 1,
        revision: 1,
        nodes: vec![Node {
            id: EntityId::new("tool"),
            kind,
            type_version: 1,
            name: "Tool under test".into(),
            enabled: true,
            bypass,
            parameters: serde_json::from_value(parameters).unwrap(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: channels as u8,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: channels as u8,
                },
            ],
        }],
        edges: vec![],
    }
}

fn graph(kind: NodeKind, params: Value, rate: u32, channels: usize) -> RuntimeGraph {
    compile_session_at_sample_rate(
        &session(kind, params, channels, false),
        RuntimeGeneration::new(1),
        rate,
    )
    .unwrap()
}

fn render(graph: &RuntimeGraph, input: &[f32], channels: usize) -> Vec<f32> {
    assert_eq!(input.len() % (QUANTUM * channels), 0);
    let mut block = AudioBlock::new(channels, QUANTUM).unwrap();
    let mut output = vec![0.0; input.len()];
    for (source, destination) in input
        .chunks(QUANTUM * channels)
        .zip(output.chunks_mut(QUANTUM * channels))
    {
        block.copy_from_interleaved(source).unwrap();
        graph.process(&mut block);
        block.copy_to_interleaved(destination).unwrap();
    }
    assert!(
        output.iter().all(|sample| sample.is_finite()),
        "non-finite sink sample"
    );
    output
}

fn frames(rate: u32, seconds: usize) -> usize {
    (rate as usize * seconds / QUANTUM) * QUANTUM
}
fn tone(rate: u32, frequency: f64, amplitude: f32, seconds: usize) -> Vec<f32> {
    (0..frames(rate, seconds))
        .map(|frame| amplitude * (TAU * frequency * frame as f64 / rate as f64).sin() as f32)
        .collect()
}
fn noise(length: usize, seed: u32, amplitude: f32) -> Vec<f32> {
    let mut state = seed;
    (0..length)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            amplitude * ((state >> 8) as f32 / 8_388_608.0 - 1.0)
        })
        .collect()
}
fn rms(samples: &[f32]) -> f64 {
    (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt()
}
fn db(linear: f64) -> f64 {
    20.0 * linear.max(1e-15).log10()
}
fn max_error(actual: &[f32], expected: &[f32]) -> f64 {
    assert_eq!(actual.len(), expected.len());
    actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (f64::from(*a) - f64::from(*b)).abs())
        .fold(0.0, f64::max)
}
fn projection(samples: &[f32], rate: u32, frequency: f64) -> f64 {
    let (mut real, mut imag) = (0.0, 0.0);
    for (frame, sample) in samples.iter().enumerate() {
        let phase = TAU * frequency * frame as f64 / rate as f64;
        real += f64::from(*sample) * phase.cos();
        imag += f64::from(*sample) * phase.sin();
    }
    2.0 * real.hypot(imag) / samples.len() as f64
}

// Independent double-precision evaluation of the specified peaking transfer
// law, including overlap from the other harmonic cuts. Never calls DSP's
// coefficients or response API to derive the expected result.
fn peaking_db(rate: u32, center: f64, q: f64, gain: f64, frequency: f64) -> f64 {
    let w = TAU * center / rate as f64;
    let alpha = w.sin() / (2.0 * q);
    let a = 10f64.powf(gain / 40.0);
    let t = TAU * frequency / rate as f64;
    let magnitude = |b0: f64, b1: f64, b2: f64| {
        (b0 + b1 * t.cos() + b2 * (2.0 * t).cos()).hypot(b1 * t.sin() + b2 * (2.0 * t).sin())
    };
    db(magnitude(1.0 + alpha * a, -2.0 * w.cos(), 1.0 - alpha * a)
        / magnitude(1.0 + alpha / a, -2.0 * w.cos(), 1.0 - alpha / a))
}

fn wav(path: &Path, samples: &[f32], rate: u32, channels: usize) {
    let bytes = (samples.len() * 4) as u32;
    let mut file = std::io::BufWriter::new(fs::File::create(path).unwrap());
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&3u16.to_le_bytes()).unwrap();
    file.write_all(&(channels as u16).to_le_bytes()).unwrap();
    file.write_all(&rate.to_le_bytes()).unwrap();
    file.write_all(&(rate * channels as u32 * 4).to_le_bytes())
        .unwrap();
    file.write_all(&(channels as u16 * 4).to_le_bytes())
        .unwrap();
    file.write_all(&32u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&bytes.to_le_bytes()).unwrap();
    for sample in samples {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
    file.flush().unwrap();
}

fn evidence(
    name: &str,
    rate: u32,
    channels: usize,
    params: &Value,
    input: &[f32],
    output: &[f32],
    expectation: Value,
) {
    let report = json!({"case":name,"sampleRate":rate,"channels":channels,"quantum":QUANTUM,
        "fixtureSeeds":[7,11],"parameters":params,"inputFrames":input.len()/channels,"outputFrames":output.len()/channels,
        "inputRms":rms(input),"outputRms":rms(output),"outputPeak":output.iter().map(|v|v.abs()).fold(0.0_f32,f32::max),
        "expectation":expectation});
    if let Some(root) = std::env::var_os("AUDIOROUTER_SIGNAL_ARTIFACTS") {
        let root = Path::new(&root).join(name);
        fs::create_dir_all(&root).unwrap();
        wav(&root.join("input.wav"), input, rate, channels);
        wav(&root.join("output.wav"), output, rate, channels);
        fs::write(
            root.join("metrics.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    println!("{report}");
}

#[test]
fn gain_volume_mute_apply_exact_level_to_both_channels() {
    for rate in RATES {
        let input = noise(QUANTUM * 2 * 32, 7, 0.2);
        let cases = [-60.0, -12.0, 0.0, 6.0, 24.0]
            .into_iter()
            .map(|gain| {
                (
                    NodeKind::Gain,
                    json!({"gainDb":gain}),
                    10f32.powf(gain / 20.0),
                )
            })
            .chain([0.0, 1.0, 50.0, 100.0, 200.0].into_iter().map(|percent| {
                (
                    NodeKind::Volume,
                    json!({"percent":percent}),
                    percent / 100.0,
                )
            }))
            .chain([false, true].into_iter().map(|muted| {
                (
                    NodeKind::Mute,
                    json!({"muted":muted}),
                    if muted { 0.0 } else { 1.0 },
                )
            }));
        for (index, (kind, params, gain)) in cases.enumerate() {
            let output = render(&graph(kind, params.clone(), rate, 2), &input, 2);
            let expected: Vec<_> = input.iter().map(|v| v * gain).collect();
            let error = max_error(&output, &expected);
            evidence(
                &format!("level-{rate}-{index}"),
                rate,
                2,
                &params,
                &input,
                &output,
                json!({"linearGain":gain,"maxSampleError":error,"tolerance":1e-6}),
            );
            assert!(error <= 1e-6, "{kind:?} {params}: {error}");
        }
    }
}

#[test]
fn delay_moves_markers_by_the_requested_samples_without_crosstalk() {
    for rate in RATES {
        for delay in [0.0, 0.5, 5.0, 100.0, 1_000.0] {
            let input = noise(frames(rate, 2) * 2, 7, 0.1);
            let params = json!({"delayMs":delay});
            let output = render(&graph(NodeKind::Delay, params.clone(), rate, 2), &input, 2);
            let latency = (delay * rate as f64 / 1_000.0).round() as usize;
            let mut expected = vec![0.0; input.len()];
            expected[latency * 2..].copy_from_slice(&input[..input.len() - latency * 2]);
            let error = max_error(&output, &expected);
            evidence(
                &format!("delay-{rate}-{delay}"),
                rate,
                2,
                &params,
                &input,
                &output,
                json!({"latencySamples":latency,"maxSampleError":error,"tolerance":1e-6}),
            );
            assert!(error <= 1e-6, "delay {rate}/{delay}: {error}");
        }
    }
}

#[test]
fn advanced_eq_meets_center_transfer_laws_for_every_filter_shape() {
    for rate in RATES {
        for q in [0.5, 1.0, 4.0] {
            for (shape, gain, expected_db) in [
                ("peaking", -24.0, -24.0),
                ("peaking", 0.0, 0.0),
                ("peaking", 24.0, 24.0),
                ("lowShelf", 12.0, 6.0),
                ("highShelf", -12.0, -6.0),
                ("lowPass", 0.0, db(q)),
                ("highPass", 0.0, db(q)),
                ("bandPass", 0.0, 0.0),
                ("allPass", 0.0, 0.0),
                ("notch", 0.0, -30.0),
            ] {
                let params = json!({"band0Enabled":true,"band0Type":shape,"band0FrequencyHz":1_000.0,"band0Q":q,"band0GainDb":gain});
                let input = tone(rate, 1_000.0, 0.02, 2);
                let output = render(
                    &graph(NodeKind::ParametricEq, params.clone(), rate, 1),
                    &input,
                    1,
                );
                let start = rate as usize;
                let measured = db(projection(&output[start..], rate, 1_000.0)
                    / projection(&input[start..], rate, 1_000.0));
                evidence(
                    &format!("eq-{rate}-{q}-{shape}-{gain}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"expectedDb":expected_db,"measuredDb":measured,"toleranceDb":0.5}),
                );
                if shape == "notch" {
                    assert!(measured <= -30.0, "notch: {measured}");
                } else {
                    assert!(
                        (measured - expected_db).abs() <= 0.5,
                        "{rate}/{q}/{shape}: {measured} expected {expected_db}"
                    );
                }
                if shape == "allPass" {
                    assert!(
                        max_error(&output[start..], &input[start..]) > 0.01,
                        "all-pass must change phase"
                    );
                }
            }
        }
    }
}

#[test]
fn graphic_eq_each_control_shapes_its_own_center_frequency() {
    let frequencies = [
        31.5, 63.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
    ];
    for rate in RATES {
        for (band, frequency) in frequencies.iter().enumerate() {
            for gain in [-18.0, 0.0, 18.0] {
                let params = json!({format!("band{band}Db"):gain});
                let input = tone(rate, *frequency, 0.02, 3);
                let output = render(
                    &graph(NodeKind::GraphicEq, params.clone(), rate, 1),
                    &input,
                    1,
                );
                let start = rate as usize * 2;
                let measured = db(projection(&output[start..], rate, *frequency)
                    / projection(&input[start..], rate, *frequency));
                evidence(
                    &format!("graphic-{rate}-{band}-{gain}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"expectedDb":gain,"measuredDb":measured,"toleranceDb":0.5}),
                );
                assert!(
                    (measured - gain).abs() <= 0.5,
                    "graphic band {band} {rate}/{gain}: {measured}"
                );
            }
        }
    }
}

#[test]
fn bass_treble_shelves_and_dehum_harmonics_follow_controls() {
    for rate in RATES {
        for (field, frequency, cutoff) in [
            ("bassDb", 50.0, 500.0),
            ("trebleDb", rate as f64 * 0.4, 1_500.0),
        ] {
            for gain in [-12.0, 0.0, 12.0] {
                let params = if field == "bassDb" {
                    json!({"bassDb":gain,"trebleDb":0.0,"bassFrequencyHz":cutoff})
                } else {
                    json!({"bassDb":0.0,"trebleDb":gain,"trebleFrequencyHz":cutoff})
                };
                let input = tone(rate, frequency, 0.05, 2);
                let output = render(
                    &graph(NodeKind::BassTreble, params.clone(), rate, 1),
                    &input,
                    1,
                );
                let start = rate as usize;
                let measured = db(projection(&output[start..], rate, frequency)
                    / projection(&input[start..], rate, frequency));
                evidence(
                    &format!("tone-{rate}-{field}-{gain}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"expectedAsymptoticDb":gain,"measuredDb":measured,"toleranceDb":0.5}),
                );
                assert!((measured - gain).abs() <= 0.5, "{field}/{gain}: {measured}");
            }
        }
        for fundamental in [50.0, 60.0] {
            for harmonics in [1, 4, 8] {
                for amount in [0.0, 50.0, 100.0] {
                    let params = json!({"frequencyHz":fundamental,"harmonics":harmonics,"amountPercent":amount});
                    for frequency in [fundamental, fundamental * harmonics as f64, 1_003.0] {
                        // Q20 at a deep low-frequency cut settles slowly.
                        // Measure steady state after eight seconds, not the
                        // residual startup transient of a two-second warmup.
                        let input = tone(rate, frequency, 0.1, 9);
                        let output =
                            render(&graph(NodeKind::Dehum, params.clone(), rate, 1), &input, 1);
                        let start = rate as usize * 8;
                        let measured = db(projection(&output[start..], rate, frequency)
                            / projection(&input[start..], rate, frequency));
                        let expected = (1..=harmonics)
                            .map(|harmonic| {
                                peaking_db(
                                    rate,
                                    fundamental * harmonic as f64,
                                    20.0,
                                    -0.36 * amount,
                                    frequency,
                                )
                            })
                            .sum::<f64>();
                        evidence(
                            &format!("dehum-{rate}-{fundamental}-{harmonics}-{amount}-{frequency}"),
                            rate,
                            1,
                            &params,
                            &input,
                            &output,
                            json!({"expectedDb":expected,"measuredDb":measured,"toleranceDb":0.5}),
                        );
                        assert!(
                            (measured - expected).abs() <= 0.5,
                            "hum {params} {frequency}: {measured} expected {expected}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "Known DSP-12 acceptance failure: eight Q20 peaking cuts attenuate 1003 Hz more than 5%; do not waive or weaken the specification"]
fn dehum_eight_harmonics_preserves_wanted_band_within_five_percent() {
    let rate = 48_000;
    let params = json!({"frequencyHz":60.0,"harmonics":8,"amountPercent":100.0});
    let input = tone(rate, 1_003.0, 0.1, 3);
    let output = render(&graph(NodeKind::Dehum, params.clone(), rate, 1), &input, 1);
    let ratio =
        projection(&output[96_000..], rate, 1_003.0) / projection(&input[96_000..], rate, 1_003.0);
    evidence(
        "known-dehum-wanted-band",
        rate,
        1,
        &params,
        &input,
        &output,
        json!({"wantedBandAmplitudeRatio":ratio,"maximumDeviation":0.05,"knownFailure":true}),
    );
    assert!(
        (ratio - 1.0).abs() <= 0.05,
        "DSP-12 wanted-band amplitude ratio {ratio}"
    );
}

#[test]
fn compressor_matches_static_soft_knee_transfer_and_makeup() {
    let rate = 48_000;
    for threshold in [-30.0_f64, -18.0] {
        for ratio in [1.0_f64, 2.0, 4.0, 20.0] {
            for knee in [0.0_f64, 6.0, 24.0] {
                for level in [-50.0_f64, -24.0, -18.0, -12.0, -3.0] {
                    for makeup in [0.0, 6.0] {
                        let params = json!({"thresholdDb":threshold,"ratio":ratio,"kneeDb":knee,"attackMs":0.1,"releaseMs":10.0,"makeupDb":makeup});
                        let input = vec![10f32.powf(level as f32 / 20.0); frames(rate, 1)];
                        let output = render(
                            &graph(NodeKind::Compressor, params.clone(), rate, 1),
                            &input,
                            1,
                        );
                        let distance = level - threshold;
                        let reduction = if knee > 0.0 && distance.abs() < knee / 2.0 {
                            (1.0 - 1.0 / ratio) * (distance + knee / 2.0).powi(2) / (2.0 * knee)
                        } else {
                            (1.0 - 1.0 / ratio) * distance.max(0.0)
                        };
                        let expected = level - reduction + makeup;
                        let measured = db(rms(&output[24_000..]));
                        evidence(
                            &format!("compressor-{threshold}-{ratio}-{knee}-{level}-{makeup}"),
                            rate,
                            1,
                            &params,
                            &input,
                            &output,
                            json!({"expectedOutputDb":expected,"measuredOutputDb":measured,"toleranceDb":0.5}),
                        );
                        assert!(
                            (measured - expected).abs() <= 0.5,
                            "compressor {params}, {level}: {measured} expected {expected}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn gate_expansion_and_range_match_static_amplitude_law() {
    let rate = 48_000;
    for ratio in [1.0_f64, 4.0, 20.0] {
        for range in [0.0_f64, 20.0, 60.0, 80.0] {
            for level in [-75.0_f64, -50.0, -45.0, -30.0] {
                let params = json!({"thresholdDb":-45.0,"hysteresisDb":3.0,"ratio":ratio,"rangeDb":range,"attackMs":0.1,"holdMs":0.0,"releaseMs":10.0});
                let input = vec![10f32.powf(level as f32 / 20.0); frames(rate, 1)];
                let output = render(&graph(NodeKind::Gate, params.clone(), rate, 1), &input, 1);
                let attenuation: f64 = (((-45.0 - level).max(0.0)) * (ratio - 1.0)).min(range);
                let measured = db(rms(&output[24_000..])) - level;
                evidence(
                    &format!("gate-{ratio}-{range}-{level}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"expectedGainDb":-attenuation,"measuredGainDb":measured,"toleranceDb":1.0}),
                );
                assert!(
                    (measured + attenuation).abs() <= 1.0,
                    "gate {params}, {level}: {measured} expected {}",
                    -attenuation
                );
            }
        }
    }
}

#[test]
fn limiter_respects_ceiling_and_lookahead_for_impulses_and_overload() {
    for rate in RATES {
        for ceiling in [-12.0, -6.0, -1.0, 0.0] {
            for lookahead in [0.0, 5.0, 10.0] {
                let params = json!({"ceilingDb":ceiling,"lookaheadMs":lookahead,"releaseMs":100.0});
                let mut input = tone(rate, 997.0, 2.0, 1);
                input[0] = 4.0;
                let output = render(
                    &graph(NodeKind::Limiter, params.clone(), rate, 1),
                    &input,
                    1,
                );
                let peak = output.iter().map(|v| v.abs() as f64).fold(0.0, f64::max);
                let latency = (lookahead * rate as f64 / 1_000.0).round() as usize;
                evidence(
                    &format!("limiter-{rate}-{ceiling}-{lookahead}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"ceilingDb":ceiling,"measuredPeakDb":db(peak),"latencySamples":latency,"toleranceDb":0.1}),
                );
                assert!(db(peak) <= ceiling + 0.1, "limiter ceiling: {}", db(peak));
                assert!(
                    db(peak) >= ceiling - 0.1,
                    "limiter cannot pass by muting everything"
                );
                assert!(output[..latency].iter().all(|v| *v == 0.0));
                assert!(output[latency].abs() > 0.0);
            }
        }
    }
}

#[test]
fn effect_bypass_nulls_against_input_including_high_latency_tools() {
    let kinds = [
        NodeKind::Gain,
        NodeKind::Volume,
        NodeKind::BassTreble,
        NodeKind::Dehum,
        NodeKind::Declick,
        NodeKind::Denoise,
        NodeKind::SpeechDenoise,
        NodeKind::FirFilter,
        NodeKind::SpectralGate,
        NodeKind::TimeShift,
        NodeKind::ParametricEq,
        NodeKind::GraphicEq,
        NodeKind::Compressor,
        NodeKind::Gate,
        NodeKind::Limiter,
        NodeKind::Delay,
        NodeKind::Pitch,
    ];
    let input = noise(QUANTUM * 2 * 64, 7, 0.1);
    for kind in kinds {
        let tool = session(kind, json!({}), 2, true);
        let graph =
            compile_session_at_sample_rate(&tool, RuntimeGeneration::new(1), 48_000).unwrap();
        let output = render(&graph, &input, 2);
        let error = max_error(&output, &input);
        evidence(
            &format!("bypass-{kind:?}"),
            48_000,
            2,
            &json!({"bypass":true}),
            &input,
            &output,
            json!({"maxSampleError":error,"tolerance":1e-6}),
        );
        assert!(error <= 1e-6, "bypass {kind:?}: {error}");
    }
}

#[test]
fn pitch_changes_frequency_by_semitones_and_cents_without_changing_frame_count() {
    for rate in RATES {
        for semitones in [-12.0, -7.0, 0.0, 7.0, 12.0] {
            for cents in [-100.0, 0.0, 100.0] {
                let params = json!({"semitones":semitones,"cents":cents});
                let input = tone(rate, 440.0, 0.3, 4);
                let output = render(&graph(NodeKind::Pitch, params.clone(), rate, 1), &input, 1);
                let tail = &output[rate as usize * 2..];
                let crossings: Vec<f64> = tail
                    .windows(2)
                    .enumerate()
                    .filter(|(_, pair)| pair[0] <= 0.0 && pair[1] > 0.0)
                    .map(|(frame, pair)| frame as f64 + f64::from(-pair[0] / (pair[1] - pair[0])))
                    .collect();
                assert!(crossings.len() > 10, "pitch cannot pass by muting output");
                let measured = (crossings.len() - 1) as f64 * rate as f64
                    / (crossings.last().unwrap() - crossings[0]);
                let expected = 440.0 * 2f64.powf((semitones + cents / 100.0) / 12.0);
                let error = 1_200.0 * (measured / expected).log2();
                evidence(
                    &format!("pitch-{rate}-{semitones}-{cents}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"expectedHz":expected,"measuredHz":measured,"errorCents":error,"toleranceCents":10.0}),
                );
                assert!(
                    error.abs() <= 10.0,
                    "pitch {rate}/{params}: {measured}, error {error} cents"
                );
                assert!(rms(tail) > 0.05, "pitch must retain wanted signal");
                assert_eq!(input.len(), output.len());
            }
        }
    }
}

#[test]
fn declick_repairs_spikes_and_keeps_clean_audio_after_lookahead() {
    let rate = 48_000;
    let clean = tone(rate, 440.0, 0.2, 1);
    let latency = audiorouter_dsp::restoration::DECLICK_LOOKAHEAD;
    for threshold in [0.0, 50.0, 100.0] {
        let params = json!({"thresholdPercent":threshold});
        let output = render(
            &graph(NodeKind::Declick, params.clone(), rate, 1),
            &clean,
            1,
        );
        let error = max_error(&output[latency..], &clean[..clean.len() - latency]);
        assert!(error <= 1e-6, "clean Declick {threshold}: {error}");
        let mut clicked = clean.clone();
        for frame in [6_000, 12_000, 24_000, 36_000] {
            clicked[frame] = 1.0;
        }
        let output = render(
            &graph(NodeKind::Declick, params.clone(), rate, 1),
            &clicked,
            1,
        );
        let error = max_error(
            &output[latency + 1_000..],
            &clean[1_000..clean.len() - latency],
        );
        evidence(
            &format!("declick-{threshold}"),
            rate,
            1,
            &params,
            &clicked,
            &output,
            json!({"latencySamples":latency,"maxErrorAgainstClean":error,"tolerance":0.01}),
        );
        assert!(error < 0.01, "click repair {threshold}: {error}");
    }
}

#[test]
fn spectral_tools_learn_profiles_reduce_noise_and_retain_wanted_tone() {
    let rate = 48_000;
    let input = noise(frames(rate, 2), 7, 0.05);
    let latency = audiorouter_dsp::spectral::SPECTRAL_LATENCY;
    for kind in [NodeKind::Denoise, NodeKind::SpectralGate] {
        let params = json!({"learning":true});
        let learner = graph(kind, params.clone(), rate, 1);
        let output = render(&learner, &input, 1);
        let error = max_error(
            &output[latency + 2_048..],
            &input[2_048..input.len() - latency],
        );
        evidence(
            &format!("learn-{kind:?}"),
            rate,
            1,
            &params,
            &input,
            &output,
            json!({"latencySamples":latency,"maxSampleError":error,"tolerance":0.001}),
        );
        assert!(
            error < 1e-3,
            "learning must be transparent after warmup: {error}"
        );
        let profile = learner
            .noise_profile_for_node(&EntityId::new("tool"))
            .expect("profile from real graph");
        if kind == NodeKind::Denoise {
            for floor in [50.0, 100.0] {
                let params = json!({"noiseProfile":profile,"learning":false,"reductionPercent":100.0,"floorPercent":floor});
                let hiss = noise(frames(rate, 2), 11, 0.05);
                let output = render(&graph(kind, params.clone(), rate, 1), &hiss, 1);
                let input_tail = &hiss[72_000 - latency..hiss.len() - latency];
                let measured = db(rms(&output[72_000..]) / rms(input_tail));
                assert!(
                    measured >= db(floor / 100.0) - 0.2 && measured <= 0.2,
                    "floor {floor}: {measured}"
                );
                if floor == 100.0 {
                    assert!(max_error(&output[72_000..], input_tail) < 1e-3);
                }
                evidence(
                    &format!("denoise-floor-{floor}"),
                    rate,
                    1,
                    &params,
                    &hiss,
                    &output,
                    json!({"minimumGainDb":db(floor/100.0),"measuredGainDb":measured,"toleranceDb":0.2,"fullFloorMaxSampleError":if floor==100.0 {Some(max_error(&output[72_000..],input_tail))}else{None}}),
                );
            }
        }
        for setting in [0.0, 50.0, 100.0] {
            let params = if kind == NodeKind::Denoise {
                json!({"noiseProfile":profile,"learning":false,"reductionPercent":setting,"floorPercent":0.0})
            } else {
                json!({"noiseProfile":profile,"learning":false,"thresholdDb":3.0,"reductionDb":setting*0.6})
            };
            let hiss = noise(frames(rate, 2), 11, 0.05);
            let output = render(&graph(kind, params.clone(), rate, 1), &hiss, 1);
            let attenuation =
                db(rms(&output[72_000..]) / rms(&hiss[72_000 - latency..hiss.len() - latency]));
            evidence(
                &format!("noise-{kind:?}-{setting}"),
                rate,
                1,
                &params,
                &hiss,
                &output,
                json!({"measuredNoiseGainDb":attenuation,"requiredMaximumDb":if setting==0.0 {0.5}else{-6.0}}),
            );
            if setting == 0.0 {
                assert!(attenuation.abs() < 0.5);
            } else {
                assert!(attenuation < -6.0, "{kind:?}/{setting}: {attenuation}");
            }
            let wanted = tone(rate, 1_000.0, 0.3, 2);
            let mixed: Vec<_> = wanted.iter().zip(&hiss).map(|(a, b)| a + b).collect();
            let output = render(&graph(kind, params.clone(), rate, 1), &mixed, 1);
            let retained = projection(&output[72_000..], rate, 1_000.0)
                / projection(&wanted[72_000..], rate, 1_000.0);
            evidence(
                &format!("wanted-{kind:?}-{setting}"),
                rate,
                1,
                &params,
                &mixed,
                &output,
                json!({"retainedToneRatio":retained,"minimum":0.8}),
            );
            assert!(
                retained > 0.8,
                "{kind:?}/{setting} removed wanted tone: {retained}"
            );
        }
    }
}

#[test]
fn speech_denoise_suppresses_noise_but_retains_syllabic_signal() {
    let rate = 48_000;
    let latency = audiorouter_dsp::spectral::SPECTRAL_LATENCY;
    let hiss = noise(frames(rate, 2), 7, 0.05);
    let wanted = tone(rate, 500.0, 0.3, 2);
    let mixed: Vec<_> = wanted
        .iter()
        .zip(&hiss)
        .enumerate()
        .map(|(frame, (voice, noise))| {
            noise
                + if (frame / 6_000) % 2 == 0 {
                    *voice
                } else {
                    0.0
                }
        })
        .collect();
    for strength in [0.0, 50.0, 100.0] {
        let params = json!({"strengthPercent":strength});
        let output = render(
            &graph(NodeKind::SpeechDenoise, params.clone(), rate, 1),
            &hiss,
            1,
        );
        let attenuation =
            db(rms(&output[72_000..]) / rms(&hiss[72_000 - latency..hiss.len() - latency]));
        evidence(
            &format!("speech-noise-{strength}"),
            rate,
            1,
            &params,
            &hiss,
            &output,
            json!({"measuredNoiseGainDb":attenuation,"maximumDb":if strength==0.0 {0.5}else{-6.0}}),
        );
        if strength == 0.0 {
            assert!(attenuation.abs() < 0.5);
        } else {
            assert!(attenuation < -6.0, "speech noise {strength}: {attenuation}");
        }
        let output = render(
            &graph(NodeKind::SpeechDenoise, params.clone(), rate, 1),
            &mixed,
            1,
        );
        let voiced: Vec<_> = (72_000..mixed.len() - latency)
            .filter(|frame| {
                (frame / 6_000) % 2 == 0 && frame % 6_000 > 1_500 && frame % 6_000 < 4_500
            })
            .map(|frame| output[frame + latency])
            .collect();
        let retained = rms(&voiced) / (0.3 / 2f64.sqrt());
        evidence(
            &format!("speech-wanted-{strength}"),
            rate,
            1,
            &params,
            &mixed,
            &output,
            json!({"retainedVoicedRmsRatio":retained,"minimum":0.7}),
        );
        assert!(retained > 0.7, "wanted speech-like signal: {retained}");
    }
}

#[test]
fn fir_graph_matches_independent_direct_convolution_with_wet_gain_and_stereo() {
    use audiorouter_engine::{compile_session_at_sample_rate_with_plugins_and_audio, DecodedAudio};
    use std::{collections::HashMap, sync::Arc};
    let rate = 48_000;
    let input = noise(QUANTUM * 2 * 64, 7, 0.1);
    // Sparse stereo IR crosses a convolution partition boundary; each channel
    // has a different tap sequence and is normalized independently.
    let mut ir = vec![0.0_f32; 700 * 2];
    ir[0] = 0.5;
    ir[17 * 2] = 0.25;
    ir[600 * 2] = -0.125;
    ir[3 * 2 + 1] = 0.4;
    ir[513 * 2 + 1] = 0.3;
    let media = HashMap::from([(
        "test-ir".into(),
        Arc::new(DecodedAudio {
            channels: 2,
            sample_rate_hz: rate,
            samples: ir.clone().into(),
        }),
    )]);
    for wet in [0.0, 25.0, 100.0] {
        for gain in [-24.0, 0.0, 12.0] {
            let params = json!({"mediaId":"test-ir","wetPercent":wet,"gainDb":gain});
            let fixture = session(NodeKind::FirFilter, params.clone(), 2, false);
            let graph = compile_session_at_sample_rate_with_plugins_and_audio(
                &fixture,
                RuntimeGeneration::new(1),
                rate,
                &HashMap::new(),
                &media,
            )
            .unwrap();
            let output = render(&graph, &input, 2);
            let mut expected = vec![0.0; input.len()];
            let latency = audiorouter_dsp::spectral::CONVOLUTION_BLOCK;
            for channel in 0..2 {
                let energy = (0..700)
                    .map(|tap| f64::from(ir[tap * 2 + channel]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                let taps: Vec<_> = (0..700)
                    .filter(|tap| ir[tap * 2 + channel] != 0.0)
                    .collect();
                for frame in 0..input.len() / 2 - latency {
                    let convolution = taps
                        .iter()
                        .filter(|tap| **tap <= frame)
                        .map(|tap| {
                            f64::from(input[(frame - tap) * 2 + channel])
                                * f64::from(ir[tap * 2 + channel])
                                / energy
                        })
                        .sum::<f64>()
                        * 10f64.powf(gain / 20.0);
                    expected[(frame + latency) * 2 + channel] = ((wet / 100.0) * convolution
                        + (1.0 - wet / 100.0) * f64::from(input[frame * 2 + channel]))
                        as f32;
                }
            }
            let error = max_error(&output, &expected);
            evidence(
                &format!("fir-{wet}-{gain}"),
                rate,
                2,
                &params,
                &input,
                &output,
                json!({"latencySamples":latency,"maxSampleError":error,"tolerance":0.001,"oracle":"normalized sparse direct convolution"}),
            );
            assert!(error < 1e-3, "FIR {params}: {error}");
        }
    }
}

#[test]
fn fresh_preparation_and_reset_reproduce_output_bit_for_bit() {
    let input = noise(QUANTUM * 64, 7, 0.1);
    for (kind, params) in [
        (
            NodeKind::ParametricEq,
            json!({"gainDb":6.0,"frequencyHz":1_000.0}),
        ),
        (NodeKind::Compressor, json!({})),
        (NodeKind::Gate, json!({})),
        (NodeKind::Limiter, json!({})),
        (NodeKind::Delay, json!({"delayMs":5.0})),
        (NodeKind::Pitch, json!({"semitones":7.0})),
        (NodeKind::Declick, json!({})),
        (NodeKind::Denoise, json!({"reductionPercent":0.0})),
        (NodeKind::SpeechDenoise, json!({"strengthPercent":50.0})),
        (NodeKind::SpectralGate, json!({})),
    ] {
        let first = graph(kind, params.clone(), 48_000, 1);
        let output = render(&first, &input, 1);
        let repeat = render(&graph(kind, params.clone(), 48_000, 1), &input, 1);
        assert_eq!(max_error(&output, &repeat), 0.0, "fresh {kind:?}");
        assert!(first.reset_processing_state());
        let reset = render(&first, &input, 1);
        assert_eq!(max_error(&output, &reset), 0.0, "reset {kind:?}");
    }
}

#[test]
fn gate_hold_expires_and_release_follows_the_requested_time_constant() {
    let rate = 48_000;
    let loud = 0.1_f32;
    let quiet = 0.001_f32;
    let input: Vec<_> = (0..frames(rate, 5))
        .map(|frame| {
            if frame < rate as usize {
                loud
            } else if frame < 2 * rate as usize {
                10f32.powf(-33.0 / 20.0)
            } else {
                quiet
            }
        })
        .collect();
    for hold in [0.0, 50.0, 1_000.0] {
        for release in [10.0, 150.0, 2_000.0] {
            let params = json!({"thresholdDb":-30.0,"hysteresisDb":6.0,"ratio":20.0,"rangeDb":60.0,"attackMs":0.1,"holdMs":hold,"releaseMs":release});
            let output = render(&graph(NodeKind::Gate, params.clone(), rate, 1), &input, 1);
            // A level inside hysteresis stays open. Below the closing threshold,
            // one hold expires, then gain approaches -60 dB exponentially.
            assert!((db(f64::from(output[95_999] / input[95_999]))).abs() < 0.01);
            let hold_frames = (hold * 0.001 * rate as f64) as usize;
            let mut maximum_error = 0.0_f64;
            for frame in (2 * rate as usize..input.len()).step_by(128) {
                let elapsed = (frame - 2 * rate as usize + 1).saturating_sub(hold_frames);
                let expected =
                    -60.0 * (1.0 - (-(elapsed as f64) / (release * 0.001 * rate as f64)).exp());
                let measured = db(f64::from(output[frame] / quiet));
                maximum_error = maximum_error.max((measured - expected).abs());
            }
            evidence(
                &format!("gate-hold-{hold}-release-{release}"),
                rate,
                1,
                &params,
                &input,
                &output,
                json!({"maximumEnvelopeErrorDb":maximum_error,"toleranceDb":0.1,"oracle":"finite hold followed by exponential release"}),
            );
            assert!(
                maximum_error < 0.1,
                "gate {params}: envelope error {maximum_error} dB"
            );
        }
    }
}

#[test]
fn compressor_attack_and_release_follow_the_requested_time_constants() {
    let rate = 48_000;
    let input: Vec<_> = (0..frames(rate, 4))
        .map(|frame| {
            10f32.powf(if frame < rate as usize || frame >= 2 * rate as usize {
                -40.0 / 20.0
            } else {
                -6.0 / 20.0
            })
        })
        .collect();
    for attack in [0.1, 10.0, 200.0] {
        for release in [10.0, 150.0, 2_000.0] {
            let params = json!({"thresholdDb":-18.0,"ratio":4.0,"kneeDb":0.0,"makeupDb":0.0,"attackMs":attack,"releaseMs":release});
            let output = render(
                &graph(NodeKind::Compressor, params.clone(), rate, 1),
                &input,
                1,
            );
            let mut envelope = -120.0_f64;
            let mut maximum_error = 0.0_f64;
            for (frame, (actual, input)) in output.iter().zip(&input).enumerate() {
                let level = if frame < rate as usize || frame >= 2 * rate as usize {
                    -40.0
                } else {
                    -6.0
                };
                let time = if level > envelope { attack } else { release };
                let coefficient = (-1.0 / (time * 0.001 * rate as f64)).exp();
                envelope = coefficient * envelope + (1.0 - coefficient) * level;
                let expected = -(envelope + 18.0).max(0.0) * 0.75;
                maximum_error = maximum_error.max((db(f64::from(actual / input)) - expected).abs());
            }
            evidence(
                &format!("compressor-attack-{attack}-release-{release}"),
                rate,
                1,
                &params,
                &input,
                &output,
                json!({"maximumEnvelopeErrorDb":maximum_error,"toleranceDb":0.1,"oracle":"exponential detector with specified threshold and ratio"}),
            );
            assert!(
                maximum_error < 0.1,
                "compressor {params}: {maximum_error} dB"
            );
        }
    }
}

#[test]
fn sixteen_eq_bands_compose_the_expected_response_and_respect_enable_flags() {
    for rate in RATES {
        for enabled_mode in 0..3 {
            let mut params = json!({});
            let mut bands = Vec::new();
            for band in 0..16 {
                let frequency = 50.0 * 320f64.powf(band as f64 / 15.0);
                let gain = if band % 2 == 0 { 3.0 } else { -3.0 };
                let enabled = enabled_mode == 0 || (enabled_mode == 1 && band % 2 == 0);
                params[format!("band{band}Enabled")] = json!(enabled);
                params[format!("band{band}Type")] = json!("peaking");
                params[format!("band{band}FrequencyHz")] = json!(frequency);
                params[format!("band{band}Q")] = json!(1.0);
                params[format!("band{band}GainDb")] = json!(gain);
                if enabled {
                    bands.push((frequency, gain));
                }
            }
            for frequency in [100.0, 1_000.0, 10_000.0] {
                let input = tone(rate, frequency, 0.01, 3);
                let output = render(
                    &graph(NodeKind::ParametricEq, params.clone(), rate, 1),
                    &input,
                    1,
                );
                let start = 2 * rate as usize;
                let measured = db(projection(&output[start..], rate, frequency)
                    / projection(&input[start..], rate, frequency));
                let expected = bands
                    .iter()
                    .map(|(center, gain)| peaking_db(rate, *center, 1.0, *gain, frequency))
                    .sum::<f64>();
                evidence(
                    &format!("eq-sixteen-{rate}-{enabled_mode}-{frequency}"),
                    rate,
                    1,
                    &params,
                    &input,
                    &output,
                    json!({"expectedDb":expected,"measuredDb":measured,"toleranceDb":0.5}),
                );
                assert!(
                    (measured - expected).abs() < 0.5,
                    "16-band EQ {rate}/{enabled_mode}/{frequency}: {measured} != {expected}"
                );
            }
        }
    }
}

#[test]
fn active_effects_do_not_leak_a_signal_into_the_silent_stereo_channel() {
    let rate = 48_000;
    let mono = tone(rate, 997.0, 0.2, 2);
    let input: Vec<_> = mono.iter().flat_map(|sample| [*sample, 0.0]).collect();
    for (kind, params) in [
        (NodeKind::Gain, json!({"gainDb":6.0})),
        (NodeKind::Volume, json!({"percent":150.0})),
        (NodeKind::BassTreble, json!({"bassDb":6.0})),
        (NodeKind::ParametricEq, json!({"gainDb":6.0})),
        (NodeKind::GraphicEq, json!({"band5Db":6.0})),
        (NodeKind::Dehum, json!({"harmonics":8})),
        (NodeKind::Declick, json!({})),
        (NodeKind::Denoise, json!({})),
        (NodeKind::SpeechDenoise, json!({})),
        (NodeKind::SpectralGate, json!({})),
        (NodeKind::Pitch, json!({"semitones":7.0})),
        (NodeKind::Delay, json!({"delayMs":100.0})),
        (NodeKind::Limiter, json!({"ceilingDb":-12.0})),
        (NodeKind::Compressor, json!({"thresholdDb":-30.0})),
        (NodeKind::Gate, json!({"thresholdDb":-60.0})),
    ] {
        let output = render(&graph(kind, params.clone(), rate, 2), &input, 2);
        assert!(
            output
                .iter()
                .skip(1)
                .step_by(2)
                .all(|sample| *sample == 0.0),
            "{kind:?} leaked into silent right channel"
        );
        assert!(
            output.iter().step_by(2).any(|sample| sample.abs() > 1e-5),
            "{kind:?} silently removed the signal"
        );
        evidence(
            &format!("stereo-isolation-{kind:?}"),
            rate,
            2,
            &params,
            &input,
            &output,
            json!({"rightChannelPeak":0.0,"oracle":"independent channel state; linked dynamics may share gain but never audio"}),
        );
    }
}

#[test]
fn time_shift_transport_returns_exact_recorded_audio_after_jump_fades() {
    use audiorouter_dsp::timeshift::TimeShiftCommand::*;
    let rate = 48_000;
    let input = noise(frames(rate, 17), 7, 0.1);
    let params = json!({"bufferSeconds":30});
    let mut fixture = session(NodeKind::TimeShift, params.clone(), 1, false);
    fixture.id = EntityId::new("deterministic-time-shift");
    let graph = compile_session_at_sample_rate(&fixture, RuntimeGeneration::new(1), rate).unwrap();
    let state = audiorouter_engine::time_shift_state("deterministic-time-shift", "tool").unwrap();
    let initial = 12 * rate as usize;
    let mut output = render(&graph, &input[..initial], 1);
    assert_eq!(max_error(&output, &input[..initial]), 0.0);
    for (index, (command, read_start, delay)) in [
        (Pause, None, 1.0),
        (Resume, Some(initial), 1.0),
        (Back, Some(3 * rate as usize), 11.0),
        (Forward, Some(14 * rate as usize), 1.0),
        (Live, Some(16 * rate as usize), 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        state.post(command);
        let start = initial + index * rate as usize;
        let segment = render(&graph, &input[start..start + rate as usize], 1);
        if let Some(read_start) = read_start {
            assert_eq!(
                max_error(
                    &segment[256..],
                    &input[read_start + 256..read_start + rate as usize]
                ),
                0.0,
                "{command:?}"
            );
        } else {
            assert!(segment.iter().all(|sample| *sample == 0.0));
        }
        assert!(
            (state.status().delay_seconds - delay).abs() < 0.01,
            "{command:?}: {:?}",
            state.status()
        );
        output.extend(segment);
    }
    evidence(
        "time-shift-transport",
        rate,
        1,
        &params,
        &input,
        &output,
        json!({"oracle":"exact history at live, pause, resume, back 10 s, forward 10 s and live; skip only 256-frame jump fades"}),
    );
}

#[test]
fn mixer_and_input_switch_transform_independent_source_signals() {
    use audiorouter_domain::Edge;
    use audiorouter_engine::compile_mixer_fanout_session;
    let rate = 48_000;
    let a = noise(QUANTUM * 64, 7, 0.1);
    let b = noise(a.len(), 11, 0.2);
    let configurations = [0.0, 50.0, 100.0]
        .into_iter()
        .map(|volume| (NodeKind::Mixer, volume, "a", "normal"))
        .chain(["a", "b"].into_iter().flat_map(|selected| {
            ["normal", "slow"]
                .into_iter()
                .map(move |fade| (NodeKind::InputSwitch, 100.0, selected, fade))
        }));
    for (kind, volume, selected, fade) in configurations {
        let params = if kind == NodeKind::Mixer {
            json!({"inputVolume:a":volume,"inputVolume:b":100.0})
        } else {
            json!({"selected":selected,"fade":fade})
        };
        let mut fixture = session(kind, params.clone(), 1, false);
        let mut tool = fixture.nodes.remove(0);
        if kind == NodeKind::InputSwitch {
            tool.ports[0].name = "a".into();
            tool.ports.push(Port {
                name: "b".into(),
                direction: PortDirection::Input,
                channels: 1,
            });
        }
        let source = |id: &str| {
            let mut node = session(NodeKind::PhysicalInput, json!({}), 1, false)
                .nodes
                .remove(0);
            node.id = EntityId::new(id);
            node.ports.retain(|p| p.direction == PortDirection::Output);
            node
        };
        let mut sink = session(NodeKind::PhysicalOutput, json!({}), 1, false)
            .nodes
            .remove(0);
        sink.id = EntityId::new("sink");
        sink.ports.retain(|p| p.direction == PortDirection::Input);
        fixture.nodes = vec![source("a"), source("b"), tool, sink];
        let edge = |id: &str, source: &str, dest: &str, port: &str| Edge {
            id: EntityId::new(id),
            source_node: EntityId::new(source),
            source_port: "out".into(),
            destination_node: EntityId::new(dest),
            destination_port: port.into(),
            matrix: vec![1.0],
            enabled: true,
        };
        fixture.edges = vec![
            edge(
                "a-tool",
                "a",
                "tool",
                if kind == NodeKind::Mixer { "in" } else { "a" },
            ),
            edge(
                "b-tool",
                "b",
                "tool",
                if kind == NodeKind::Mixer { "in" } else { "b" },
            ),
            edge("tool-sink", "tool", "sink", "in"),
        ];
        let compiled = compile_mixer_fanout_session(&fixture, RuntimeGeneration::new(1)).unwrap();
        let mut output = vec![0.0; a.len()];
        let mut scratch = AudioBlock::new(1, QUANTUM).unwrap();
        let mut destination = AudioBlock::new(1, QUANTUM).unwrap();
        for ((a, b), out) in a
            .chunks(QUANTUM)
            .zip(b.chunks(QUANTUM))
            .zip(output.chunks_mut(QUANTUM))
        {
            let mut left = AudioBlock::new(1, QUANTUM).unwrap();
            let mut right = AudioBlock::new(1, QUANTUM).unwrap();
            left.copy_from_interleaved(a).unwrap();
            right.copy_from_interleaved(b).unwrap();
            compiled
                .process(&[left, right], &mut scratch, &mut [&mut destination])
                .unwrap();
            destination.copy_to_interleaved(out).unwrap();
        }
        let expected: Vec<_> = a
            .iter()
            .zip(&b)
            .map(|(a, b)| {
                if kind == NodeKind::Mixer {
                    *a * (volume as f32 / 100.0) + *b
                } else if selected == "a" {
                    *a
                } else {
                    *b
                }
            })
            .collect();
        let error = max_error(&output, &expected);
        assert!(error < 1e-6, "{kind:?}/{params}: {error}");
        let name = format!("multi-{kind:?}-{volume}-{selected}-{fade}");
        evidence(
            &name,
            rate,
            1,
            &params,
            &a,
            &output,
            json!({"secondSourceSeed":11,"maxSampleError":error,"tolerance":1e-6,"oracle":"weighted source sum or selected source"}),
        );
        if let Some(root) = std::env::var_os("AUDIOROUTER_SIGNAL_ARTIFACTS") {
            wav(
                &Path::new(&root).join(name).join("input-b.wav"),
                &b,
                rate,
                1,
            );
        }
    }
}
