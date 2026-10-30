//! Render short synthetic Gate guide fixtures through the production graph.
//!
//! This example is an isolated content spike, not a shipping audio path. It
//! writes raw interleaved f32 audio and per-quantum processor telemetry to the
//! requested directory so a browser capture can replay the same timeline.
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{compile_session_at_sample_rate, AudioBlock, RuntimeGeneration};
use serde_json::{json, Value};
use std::{env, fs, path::PathBuf};

const RATE: u32 = 48_000;
const CHANNELS: usize = 2;
const QUANTUM: usize = 128;
const DURATION_SECONDS: usize = 8;

fn fixture() -> Vec<f32> {
    let mut samples = Vec::with_capacity(RATE as usize * DURATION_SECONDS * CHANNELS);
    let mut noise_state = 0x6d2b_79f5_u32;
    for frame in 0..RATE as usize * DURATION_SECONDS {
        noise_state ^= noise_state << 13;
        noise_state ^= noise_state >> 17;
        noise_state ^= noise_state << 5;
        let noise = (noise_state as i32 as f32 / i32::MAX as f32) * 0.00045;
        let time = frame as f32 / RATE as f32;
        let speaking = [
            (0.65, 1.15),
            (1.7, 2.2),
            (2.75, 3.15),
            (4.3, 4.8),
            (5.4, 6.0),
            (6.6, 7.1),
        ]
        .iter()
        .any(|(start, end)| time >= *start && time < *end);
        let voice = if speaking {
            let phase = time * std::f32::consts::TAU;
            let syllable = (1.0 - ((time * 7.0).fract() - 0.45).abs() * 1.35).clamp(0.15, 1.0);
            syllable
                * (0.045 * (phase * 137.0).sin()
                    + 0.018 * (phase * 274.0).sin()
                    + 0.009 * (phase * 411.0).sin())
        } else {
            0.0
        };
        let sample = noise + voice;
        samples.extend([sample, sample]);
    }
    samples
}

fn parameters(mode: &str, changed: bool) -> Value {
    let (threshold_db, range_db, hold_ms) = match mode {
        "threshold" => (if changed { -52.0 } else { -80.0 }, 60.0, 100.0),
        "hold" => (-45.0, 60.0, if changed { 600.0 } else { 100.0 }),
        "range" => (-45.0, if changed { 60.0 } else { 12.0 }, 100.0),
        _ => panic!("mode must be threshold, hold, or range"),
    };
    json!({
        "thresholdDb": threshold_db,
        "hysteresisDb": 0.0,
        "ratio": 4.0,
        "rangeDb": range_db,
        "attackMs": 5.0,
        "holdMs": hold_ms,
        "releaseMs": 150.0
    })
}

fn change_frame(mode: &str) -> usize {
    let tenths = if mode == "hold" { 30 } else { 32 };
    (RATE as usize * tenths / 10 / QUANTUM) * QUANTUM
}

fn compile(mode: &str, changed: bool) -> audiorouter_engine::RuntimeGraph {
    let endpoint = |id: &str, kind: NodeKind, direction: PortDirection| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![Port {
            name: if direction == PortDirection::Output {
                "out"
            } else {
                "in"
            }
            .into(),
            direction,
            channels: CHANNELS as u8,
        }],
    };
    let node = Node {
        id: EntityId::new("voice"),
        kind: NodeKind::Gate,
        type_version: 1,
        name: "Voice Gate".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters(mode, changed)).unwrap(),
        ports: vec![
            Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 2,
            },
            Port {
                name: "out".into(),
                direction: PortDirection::Output,
                channels: 2,
            },
        ],
    };
    let session = Session {
        id: EntityId::new("guide-gate"),
        name: "Synthetic Gate guide fixture".into(),
        schema_version: 1,
        revision: if changed { 2 } else { 1 },
        nodes: vec![
            endpoint("input", NodeKind::PhysicalInput, PortDirection::Output),
            node,
            endpoint("output", NodeKind::PhysicalOutput, PortDirection::Input),
        ],
        edges: vec![
            Edge {
                id: EntityId::new("input-to-gate"),
                source_node: EntityId::new("input"),
                source_port: "out".into(),
                destination_node: EntityId::new("voice"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("gate-to-output"),
                source_node: EntityId::new("voice"),
                source_port: "out".into(),
                destination_node: EntityId::new("output"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
        ],
    };
    compile_session_at_sample_rate(&session, RuntimeGeneration::new(1), RATE).unwrap()
}

fn write_f32(path: PathBuf, values: &[f32]) {
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    fs::write(path, bytes).unwrap();
}

fn main() {
    let mut args = env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .expect("usage: tool_guide_gate_render <out-dir> <mode>"),
    );
    let mode = args.next().expect("mode must be threshold, hold, or range");
    fs::create_dir_all(&out).unwrap();

    let input = fixture();
    write_f32(out.join("input.f32le"), &input);
    let mut output = vec![0.0_f32; input.len()];
    let mut telemetry = Vec::new();
    let mut block = AudioBlock::new(CHANNELS, QUANTUM).unwrap();
    let frames = RATE as usize * DURATION_SECONDS;
    let change_frame = change_frame(&mode);
    let mut next_telemetry_frame = 0;
    let mut graph = compile(&mode, false);
    let mut changed = false;

    for start in (0..frames).step_by(QUANTUM) {
        if !changed && start >= change_frame {
            graph = compile(&mode, true);
            changed = true;
        }
        let input_start = start * CHANNELS;
        let input_end = input_start + QUANTUM * CHANNELS;
        block
            .copy_from_interleaved(&input[input_start..input_end])
            .unwrap();
        graph.process(&mut block);
        block
            .copy_to_interleaved(&mut output[input_start..input_end])
            .unwrap();

        let end_frame = start + QUANTUM;
        if end_frame >= next_telemetry_frame {
            if let Some(value) = graph.processor_telemetry_for_node(&EntityId::new("voice")) {
                telemetry.push(json!({
                    "atSeconds": end_frame as f64 / RATE as f64,
                    "gainReductionDb": value.gain_reduction_db,
                    "gateOpen": value.gate_open,
                    "inputLevelDb": value.input_level_db,
                    "outputLevelDb": value.output_level_db
                }));
            }
            next_telemetry_frame += RATE as usize / 20;
        }
    }
    write_f32(out.join(format!("{mode}.f32le")), &output);
    fs::write(
        out.join(format!("{mode}.telemetry.json")),
        serde_json::to_vec_pretty(&json!({
            "sampleRate": RATE,
            "channels": CHANNELS,
            "durationSeconds": DURATION_SECONDS,
            "changeSeconds": change_frame as f64 / RATE as f64,
            "mode": mode,
            "parametersBefore": parameters(&mode, false),
            "parametersAfter": parameters(&mode, true),
            "samples": telemetry
        }))
        .unwrap(),
    )
    .unwrap();
}
