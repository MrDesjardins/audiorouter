//! Render one deterministic tool-guide fixture through a production processor.
//!
//! This example is a local media-generation harness, not a runtime audio path.
//! It writes raw interleaved f32 input/output plus telemetry so the real
//! Properties panel can replay the same offline processing timeline.
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_session_at_sample_rate_with_plugins_and_audio, AudioBlock, DecodedAudio,
    RuntimeGeneration,
};
use serde_json::{json, Map, Value};
use std::{collections::HashMap, sync::Arc};
use std::{env, fs, path::PathBuf};

const RATE: u32 = 48_000;
const CHANNELS: usize = 2;
const QUANTUM: usize = 128;
const DURATION_SECONDS: usize = 8;
const CHANGE_FRAME: usize = RATE as usize * 16 / 5;

enum GuideRuntime {
    Linear(audiorouter_engine::RuntimeGraph),
    Native(NativeRender),
}

struct NativeRender {
    prepared: Option<audiorouter_engine::CompiledPathSet>,
    runtime: Option<audiorouter_engine::RealtimeMixerFanout>,
    outputs: Vec<audiorouter_engine::AudioBlockRing>,
}

impl GuideRuntime {
    fn spectrum(&self, id: &EntityId) -> Option<audiorouter_engine::SpectrumLevels> {
        match self {
            Self::Linear(graph) => graph.spectrum_levels_for_node(id),
            Self::Native(native) => native.runtime.as_ref()?.spectrum_levels_for_node(id),
        }
    }
    fn process(&mut self, block: &mut AudioBlock, start: usize) {
        match self {
            Self::Linear(graph) => {
                graph.process(block);
            }
            Self::Native(native) => {
                if native.runtime.is_none() {
                    let paths = native.prepared.take().unwrap();
                    let channels = vec![CHANNELS; paths.input_node_ids().len()];
                    native.runtime = Some(
                        audiorouter_engine::RealtimeMixerFanout::from_paths(
                            paths, 4, &channels, QUANTUM,
                        )
                        .unwrap(),
                    );
                }
                let runtime = native.runtime.as_mut().unwrap();
                runtime.set_privacy_muted(false);
                for index in 0..runtime.input_count() {
                    let mut source = AudioBlock::new(CHANNELS, QUANTUM).unwrap();
                    let frequency = if index == 0 { 110.0 } else { 880.0 };
                    let samples: Vec<f32> = (0..QUANTUM)
                        .flat_map(|frame| {
                            let value = 0.08
                                * ((start + frame) as f32 / RATE as f32
                                    * std::f32::consts::TAU
                                    * frequency)
                                    .sin();
                            [value, value]
                        })
                        .collect();
                    source.copy_from_interleaved(&samples).unwrap();
                    assert!(runtime
                        .try_submit_input(index, runtime.generation(), &source)
                        .unwrap());
                }
                let refs: Vec<_> = native.outputs.iter().collect();
                assert_eq!(runtime.process_once(&refs).unwrap(), native.outputs.len());
                for (index, ring) in native.outputs.iter().enumerate() {
                    let output = ring.try_receive().unwrap();
                    if index == 0 {
                        let mut samples = vec![0.0; QUANTUM * CHANNELS];
                        output.copy_to_interleaved(&mut samples).unwrap();
                        block.copy_from_interleaved(&samples).unwrap();
                    }
                    ring.try_recycle(output).unwrap();
                }
            }
        }
    }

    fn processor_telemetry_for_node(
        &self,
        id: &EntityId,
    ) -> Option<audiorouter_engine::ProcessorTelemetry> {
        match self {
            Self::Linear(graph) => graph.processor_telemetry_for_node(id),
            Self::Native(native) => native.runtime.as_ref()?.processor_telemetry_for_node(id),
        }
    }

    fn replace(&mut self, mut next: Self) {
        if let (Self::Native(current), Self::Native(candidate)) = (&mut *self, &mut next) {
            current
                .runtime
                .as_mut()
                .unwrap()
                .replace_paths(candidate.prepared.take().unwrap())
                .unwrap();
        } else {
            *self = next;
        }
    }
}

fn fixture(name: &str) -> Vec<f32> {
    let mut samples = Vec::with_capacity(RATE as usize * DURATION_SECONDS * CHANNELS);
    let mut noise_state = 0x9e37_79b9_u32;
    let mut pink_state = 0.0_f32;
    for frame in 0..RATE as usize * DURATION_SECONDS {
        noise_state ^= noise_state << 13;
        noise_state ^= noise_state >> 17;
        noise_state ^= noise_state << 5;
        let white = noise_state as i32 as f32 / i32::MAX as f32;
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
        let sample = match name {
            "speech-bursts" | "noisy-speech" => {
                let noise = white
                    * if name == "noisy-speech" {
                        0.008
                    } else {
                        0.00045
                    };
                if speaking {
                    let phase = time * std::f32::consts::TAU;
                    let syllable =
                        (1.0 - ((time * 7.0).fract() - 0.45).abs() * 1.35).clamp(0.15, 1.0);
                    noise
                        + syllable
                            * (0.045 * (phase * 137.0).sin()
                                + 0.018 * (phase * 274.0).sin()
                                + 0.009 * (phase * 411.0).sin())
                } else {
                    noise
                }
            }
            "pink-noise" => {
                pink_state = 0.985 * pink_state + 0.015 * white;
                (white * 0.55 + pink_state * 2.0) * 0.045
            }
            "drum-loop" => {
                let beat = (time * 2.0).fract();
                let kick = if beat < 0.035 {
                    (-(beat * 115.0)).exp() * (time * 180.0).sin() * 0.45
                } else {
                    0.0
                };
                let snare = if (beat - 0.5).abs() < 0.025 {
                    white * 0.18
                } else {
                    0.0
                };
                let hat = if (time * 8.0).fract() < 0.02 {
                    white * 0.08
                } else {
                    0.0
                };
                kick + snare + hat
            }
            "tone-steps" => {
                let frequency = [220.0, 440.0, 880.0, 1760.0][(time as usize / 2).min(3)];
                0.12 * (time * std::f32::consts::TAU * frequency).sin()
            }
            "mains-hum" => {
                let voice = if speaking {
                    (time * std::f32::consts::TAU * 193.0).sin() * 0.06
                } else {
                    0.0
                };
                voice + 0.035 * (time * std::f32::consts::TAU * 50.0).sin() + white * 0.0002
            }
            "click-train" => {
                let click = (time * 10.0).fract() < 0.002;
                white * 0.003 + if click { white * 0.7 } else { 0.0 }
            }
            "two-tones" => {
                0.055 * (time * std::f32::consts::TAU * 110.0).sin()
                    + 0.04 * (time * std::f32::consts::TAU * 880.0).sin()
            }
            _ => {
                let envelope = if speaking { 0.18 } else { 0.025 };
                envelope * (time * std::f32::consts::TAU * 220.0).sin() + white * 0.0002
            }
        };
        samples.extend([sample, sample]);
    }
    samples
}

fn route_node(id: &str, kind: NodeKind, direction: PortDirection) -> Node {
    Node {
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
    }
}

fn compile(kind: NodeKind, parameters: Map<String, Value>, revision: u64) -> GuideRuntime {
    let mut processor = Node {
        id: EntityId::new("tool"),
        kind,
        type_version: 1,
        name: kind.type_name().into(),
        enabled: true,
        bypass: false,
        parameters,
        ports: vec![
            Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: CHANNELS as u8,
            },
            Port {
                name: "out".into(),
                direction: PortDirection::Output,
                channels: CHANNELS as u8,
            },
        ],
    };
    let edge = |id: &str, source: &str, destination: &str| Edge {
        id: EntityId::new(id),
        source_node: EntityId::new(source),
        source_port: "out".into(),
        destination_node: EntityId::new(destination),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    };
    if kind == NodeKind::OutputSwitch {
        processor.ports[1].name = "a".into();
        processor.ports.push(Port {
            name: "b".into(),
            direction: PortDirection::Output,
            channels: CHANNELS as u8,
        });
    }
    let mut output_edge = edge("tool-to-output", "tool", "output");
    if kind == NodeKind::OutputSwitch {
        output_edge.source_port = "a".into();
    }
    let mut session = Session {
        id: EntityId::new("tool-guide-render"),
        name: format!("Synthetic {} guide fixture", kind.type_name()),
        schema_version: 1,
        revision,
        nodes: vec![
            route_node("input", NodeKind::PhysicalInput, PortDirection::Output),
            processor,
            route_node("output", NodeKind::PhysicalOutput, PortDirection::Input),
        ],
        edges: vec![edge("input-to-tool", "input", "tool"), output_edge],
    };
    let mut media = HashMap::new();
    if kind == NodeKind::FirFilter {
        media.insert(
            "lib:guide-impulse".to_string(),
            Arc::new(DecodedAudio {
                channels: 1,
                sample_rate_hz: RATE,
                samples: vec![1.0 / 32.0; 32].into(),
            }),
        );
    }
    if kind == NodeKind::InputSwitch {
        session.nodes[1].ports[0].name = "a".into();
        session.nodes[1].ports.push(Port {
            name: "b".into(),
            direction: PortDirection::Input,
            channels: CHANNELS as u8,
        });
        session.edges[0].destination_port = "a".into();
        session.nodes.push(route_node(
            "input-b",
            NodeKind::PhysicalInput,
            PortDirection::Output,
        ));
        let mut second = edge("input-b-to-tool", "input-b", "tool");
        second.destination_port = "b".into();
        session.edges.push(second);
    }
    if kind == NodeKind::OutputSwitch {
        session.nodes.push(route_node(
            "output-b",
            NodeKind::PhysicalOutput,
            PortDirection::Input,
        ));
        let mut second = edge("tool-to-output-b", "tool", "output-b");
        second.source_port = "b".into();
        session.edges.push(second);
    }
    if matches!(kind, NodeKind::InputSwitch | NodeKind::OutputSwitch) {
        let paths = audiorouter_engine::compile_native_paths_with_plugins_and_audio(
            &session,
            RuntimeGeneration::new(1),
            &HashMap::new(),
            &media,
        )
        .unwrap();
        let outputs = paths
            .output_node_ids()
            .iter()
            .map(|_| audiorouter_engine::AudioBlockRing::new(4, CHANNELS, QUANTUM).unwrap())
            .collect();
        return GuideRuntime::Native(NativeRender {
            prepared: Some(paths),
            runtime: None,
            outputs,
        });
    }
    if matches!(kind, NodeKind::AudioFile | NodeKind::Soundboard) {
        session.nodes.remove(0);
        session.edges.remove(0);
        session.nodes[0]
            .ports
            .retain(|p| p.direction == PortDirection::Output);
        media.insert(
            "lib:guide-fixture".to_string(),
            Arc::new(DecodedAudio {
                channels: CHANNELS,
                sample_rate_hz: RATE,
                samples: fixture("level-bursts").into(),
            }),
        );
    }
    let graph = compile_session_at_sample_rate_with_plugins_and_audio(
        &session,
        RuntimeGeneration::new(revision),
        RATE,
        &HashMap::new(),
        &media,
    )
    .unwrap();
    if let Some(source) = graph.audio_file_source_for_node(&EntityId::new("tool")) {
        source.play();
    }
    if let Some(source) = graph.test_signal_source_for_node(&EntityId::new("tool")) {
        source.play();
    }
    if let Some(source) = graph.soundboard_for_node(&EntityId::new("tool")) {
        source.trigger(0);
    }
    GuideRuntime::Linear(graph)
}

fn parse_parameters(text: &str) -> Value {
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .expect("parameter JSON must be an object");
    assert!(value.is_object(), "parameter JSON must be an object");
    value
}

fn parameter_map(value: &Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn meter_sample(samples: &[f32], start: usize) -> Value {
    let mut peak = [0.0_f32; CHANNELS];
    let mut squares = [0.0_f64; CHANNELS];
    for frame in 0..QUANTUM {
        for channel in 0..CHANNELS {
            let sample = samples[frame * CHANNELS + channel];
            peak[channel] = peak[channel].max(sample.abs());
            squares[channel] += f64::from(sample) * f64::from(sample);
        }
    }
    let db = |value: f32| {
        if value <= 1.0e-8 {
            -160.0
        } else {
            20.0 * value.log10()
        }
    };
    let rms: Vec<f32> = squares
        .iter()
        .map(|sum| (sum / QUANTUM as f64).sqrt() as f32)
        .collect();
    json!({
        "atSeconds": (start + QUANTUM) as f64 / RATE as f64,
        "currentPeakDb": peak.iter().copied().map(db).collect::<Vec<_>>(),
        "peakDb": peak.iter().copied().map(db).collect::<Vec<_>>(),
        "rmsDb": rms.iter().copied().map(db).collect::<Vec<_>>()
    })
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
    let out = PathBuf::from(args.next().expect(
        "usage: tool_guide_property_render <out-dir> <kind> <fixture> <before.json> <after.json>",
    ));
    let kind_name = args.next().expect("kind is required");
    let fixture_name = args.next().expect("fixture is required");
    let before_path = PathBuf::from(args.next().expect("before parameters file is required"));
    let after_path = PathBuf::from(args.next().expect("after parameters file is required"));
    let mut before_value = parse_parameters(&fs::read_to_string(before_path).unwrap());
    let mut after_value = parse_parameters(&fs::read_to_string(after_path).unwrap());
    let kind: NodeKind = serde_json::from_str(&format!("\"{kind_name}\""))
        .expect("kind must be a NodeKind serde name");
    fs::create_dir_all(&out).unwrap();
    let input = fixture(&fixture_name);
    if matches!(kind, NodeKind::Denoise | NodeKind::SpectralGate) {
        let mut learning = parameter_map(&before_value);
        learning.insert("learning".into(), json!(true));
        let mut learner = compile(kind, learning, 99);
        let mut block = AudioBlock::new(CHANNELS, QUANTUM).unwrap();
        for start in (0..RATE as usize / 2 - QUANTUM).step_by(QUANTUM) {
            block
                .copy_from_interleaved(&input[start * CHANNELS..(start + QUANTUM) * CHANNELS])
                .unwrap();
            learner.process(&mut block, start);
        }
        if let GuideRuntime::Linear(graph) = &learner {
            let profile = graph
                .noise_profile_for_node(&EntityId::new("tool"))
                .expect("noise-only warmup must produce a profile");
            for value in [&mut before_value, &mut after_value] {
                value["noiseProfile"] = json!(profile);
                value["learning"] = json!(false);
            }
        }
    }
    write_f32(out.join("input.f32le"), &input);
    let mut output = vec![0.0_f32; input.len()];
    let mut telemetry = Vec::new();
    let mut spectra = Vec::new();
    let mut meter_samples = Vec::new();
    let mut block = AudioBlock::new(CHANNELS, QUANTUM).unwrap();
    let frames = RATE as usize * DURATION_SECONDS;
    let mut graph = compile(kind, parameter_map(&before_value), 1);
    let mut changed = false;
    let mut next_telemetry_frame = 0;
    for start in (0..frames).step_by(QUANTUM) {
        if !changed && start >= CHANGE_FRAME {
            graph.replace(compile(kind, parameter_map(&after_value), 2));
            changed = true;
        }
        let input_start = start * CHANNELS;
        let input_end = input_start + QUANTUM * CHANNELS;
        block
            .copy_from_interleaved(&input[input_start..input_end])
            .unwrap();
        graph.process(&mut block, start);
        block
            .copy_to_interleaved(&mut output[input_start..input_end])
            .unwrap();
        meter_samples.push(meter_sample(&output[input_start..input_end], start));
        let end_frame = start + QUANTUM;
        if end_frame >= next_telemetry_frame {
            if let Some(levels) = graph.spectrum(&EntityId::new("tool")) {
                spectra.push(json!({ "atSeconds": end_frame as f64 / RATE as f64, "levelsDb": levels.to_vec(), "bandFrequenciesHz": audiorouter_engine::spectrum_band_frequencies_hz().to_vec() }));
            }
            if let Some(value) = graph.processor_telemetry_for_node(&EntityId::new("tool")) {
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
    assert!(
        output.iter().all(|sample| sample.is_finite()),
        "production output contains a non-finite sample"
    );
    write_f32(out.join("output.f32le"), &output);
    fs::write(
        out.join("telemetry.json"),
        serde_json::to_vec_pretty(&json!({
            "sampleRate": RATE,
            "channels": CHANNELS,
            "durationSeconds": DURATION_SECONDS,
            "changeSeconds": CHANGE_FRAME as f64 / RATE as f64,
            "kind": kind_name,
            "fixture": fixture_name,
            "parametersBefore": before_value,
            "parametersAfter": after_value,
            "samples": telemetry,
            "spectrumSamples": spectra,
            "meterSamples": meter_samples
        }))
        .unwrap(),
    )
    .unwrap();
}
