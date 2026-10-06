//! Synthetic full graph compilation/processing. No WASAPI or private audio.
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{compile_session, AudioBlock, RuntimeGeneration};
use serde_json::json;

const PROCESSORS: &[NodeKind] = &[
    NodeKind::Meter,
    NodeKind::Gain,
    NodeKind::Volume,
    NodeKind::BassTreble,
    NodeKind::Dehum,
    NodeKind::Declick,
    NodeKind::Denoise,
    NodeKind::SpeechDenoise,
    NodeKind::SpectralGate,
    NodeKind::FirFilter,
    NodeKind::TimeShift,
    NodeKind::Mute,
    NodeKind::ParametricEq,
    NodeKind::Compressor,
    NodeKind::Gate,
    NodeKind::Limiter,
    NodeKind::Delay,
    NodeKind::GraphicEq,
    NodeKind::Pitch,
];

#[test]
fn inserted_meter_preserves_every_sample_and_resets_only_statistics() {
    for channels in [1, 2] {
        let mut session = route(NodeKind::Meter);
        for node in &mut session.nodes {
            for port in &mut node.ports {
                port.channels = channels;
            }
        }
        for edge in &mut session.edges {
            edge.matrix = if channels == 1 {
                vec![1.0]
            } else {
                vec![1.0, 0.0, 0.0, 1.0]
            };
        }
        let graph = compile_session(&session, RuntimeGeneration::new(1)).unwrap();
        for quantum in 0..40 {
            let mut block = AudioBlock::new(channels as usize, 128).unwrap();
            for channel in 0..channels as usize {
                for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
                    *sample = ((quantum * 128 + frame) as f32 * 0.17 + channel as f32).sin() * 1.2;
                }
            }
            let before = (0..channels as usize)
                .map(|c| block.channel(c).unwrap().to_vec())
                .collect::<Vec<_>>();
            graph.process(&mut block);
            for (channel, expected) in before.iter().enumerate() {
                assert_eq!(block.channel(channel).unwrap(), expected);
            }
        }
        let id = EntityId::new("tool");
        let before = graph.meter_snapshot_for_node(&id).unwrap();
        assert_eq!(before.observed_frames, 40 * 128);
        assert!(before.clipped_samples > 0);
        assert!(graph.reset_meter_for_node(&id));
        let after = graph.meter_snapshot_for_node(&id).unwrap();
        assert_eq!(after.clipped_samples, 0);
        assert_eq!(after.peak_db, -120.0);
        assert_eq!(after.observed_frames, 0);
        assert!(!graph.reset_meter_for_node(&EntityId::new("missing")));
    }
}

#[test]
fn meter_after_every_builtin_preserves_the_processed_audio() {
    for &kind in PROCESSORS {
        let session = route(kind);
        let reference = compile_session(&session, RuntimeGeneration::new(1)).unwrap();
        let mut metered = session.clone();
        let mut meter = metered.nodes[1].clone();
        meter.id = EntityId::new("inserted-meter");
        meter.kind = NodeKind::Meter;
        meter.parameters.clear();
        metered.nodes.push(meter);
        metered.edges[1].destination_node = EntityId::new("inserted-meter");
        let mut output = session.edges[1].clone();
        output.id = EntityId::new("meter-output");
        output.source_node = EntityId::new("inserted-meter");
        metered.edges.push(output);
        let graph = compile_session(&metered, RuntimeGeneration::new(2)).unwrap();
        for quantum in 0..96 {
            let mut expected = AudioBlock::new(2, 128).unwrap();
            let mut actual = AudioBlock::new(2, 128).unwrap();
            signal(&mut expected, quantum * 128);
            signal(&mut actual, quantum * 128);
            reference.process(&mut expected);
            graph.process(&mut actual);
            for channel in 0..2 {
                assert_eq!(
                    actual.channel(channel),
                    expected.channel(channel),
                    "{kind:?}, quantum {quantum}"
                );
            }
        }
        assert_eq!(
            graph
                .meter_snapshot_for_node(&EntityId::new("inserted-meter"))
                .unwrap()
                .observed_frames,
            96 * 128
        );
    }
}

#[test]
fn bass_treble_frequency_parameters_have_backend_bounds() {
    for (name, min, max) in [
        ("bassFrequencyHz", 80.0, 1000.0),
        ("trebleFrequencyHz", 800.0, 12000.0),
    ] {
        for value in [min, max] {
            let mut session = route(NodeKind::BassTreble);
            session.nodes[1]
                .parameters
                .insert(name.into(), json!(value));
            assert!(compile_session(&session, RuntimeGeneration::new(1)).is_ok());
        }
        for value in [min - 1.0, max + 1.0] {
            let mut session = route(NodeKind::BassTreble);
            session.nodes[1]
                .parameters
                .insert(name.into(), json!(value));
            assert!(compile_session(&session, RuntimeGeneration::new(1)).is_err());
        }
    }
}

fn route(kind: NodeKind) -> Session {
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let node = |id: &str, kind, ports| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::Map::new(),
        ports,
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
    let mut session = Session {
        id: EntityId::new("synthetic-tools"),
        name: "Synthetic tool qualification".into(),
        schema_version: 1,
        revision: 1,
        nodes: vec![
            node(
                "source",
                NodeKind::PhysicalInput,
                vec![port("out", PortDirection::Output)],
            ),
            node(
                "tool",
                kind,
                vec![
                    port("in", PortDirection::Input),
                    port("out", PortDirection::Output),
                ],
            ),
            node(
                "sink",
                NodeKind::PhysicalOutput,
                vec![port("in", PortDirection::Input)],
            ),
        ],
        edges: vec![
            edge("source-tool", "source", "tool"),
            edge("tool-sink", "tool", "sink"),
        ],
    };
    if kind == NodeKind::TimeShift {
        session.nodes[1]
            .parameters
            .insert("bufferSeconds".into(), json!(10));
    }
    if kind == NodeKind::Mute {
        session.nodes[1]
            .parameters
            .insert("muted".into(), json!(false));
    }
    session
}

fn signal(block: &mut AudioBlock, offset: usize) {
    for channel in 0..2 {
        for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
            *sample = (0.1
                * ((offset + frame) as f64
                    * std::f64::consts::TAU
                    * (440.0 + 440.0 * channel as f64)
                    / 48_000.0)
                    .sin()) as f32;
        }
    }
}

#[test]
fn every_builtin_processor_compiles_and_processes_stereo_with_finite_meters() {
    for &kind in PROCESSORS {
        let graph = compile_session(&route(kind), RuntimeGeneration::new(1))
            .unwrap_or_else(|error| panic!("{kind:?}: {error:?}"));
        let mut block = AudioBlock::new(2, 128).unwrap();
        for quantum in 0..96 {
            signal(&mut block, quantum * 128);
            graph.process(&mut block);
            for channel in 0..2 {
                assert!(
                    block
                        .channel(channel)
                        .unwrap()
                        .iter()
                        .all(|sample| sample.is_finite()),
                    "{kind:?} quantum {quantum}"
                );
            }
        }
        assert!(
            block.peak_abs() > 1e-9,
            "{kind:?}: unexplained silence after warm-up"
        );
        let meter = graph
            .meter_snapshot_for_node(&EntityId::new("sink"))
            .expect("sink boundary meter");
        assert!(
            meter.peak_db.is_finite() && meter.rms_db.is_finite(),
            "{kind:?}"
        );
        assert_eq!(meter.clipped_samples, 0, "{kind:?}");
    }
}

#[test]
fn every_builtin_processor_off_and_bypass_preserve_the_dry_stereo_signal() {
    for &kind in PROCESSORS {
        for disabled in [true, false] {
            let mut session = route(kind);
            session.nodes[1].enabled = !disabled;
            session.nodes[1].bypass = !disabled;
            let graph = compile_session(&session, RuntimeGeneration::new(2)).unwrap();
            let mut block = AudioBlock::new(2, 128).unwrap();
            let mut expected = AudioBlock::new(2, 128).unwrap();
            for quantum in 0..8 {
                signal(&mut block, quantum * 128);
                signal(&mut expected, quantum * 128);
                graph.process(&mut block);
                for channel in 0..2 {
                    for (actual, expected) in block
                        .channel(channel)
                        .unwrap()
                        .iter()
                        .zip(expected.channel(channel).unwrap())
                    {
                        assert!(
                            (actual - expected).abs() <= 1e-6,
                            "{kind:?}, disabled={disabled}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn disabled_source_and_sink_are_silent_for_every_builtin_processor_route() {
    for &kind in PROCESSORS {
        for index in [0, 2] {
            let mut session = route(kind);
            session.nodes[index].enabled = false;
            let graph = compile_session(&session, RuntimeGeneration::new(3)).unwrap();
            let mut block = AudioBlock::new(2, 128).unwrap();
            for quantum in 0..16 {
                signal(&mut block, quantum * 128);
                graph.process(&mut block);
                assert_eq!(block.peak_abs(), 0.0, "{kind:?}, disabled node {index}");
            }
        }
    }
}

#[test]
fn gain_and_volume_parameters_change_real_samples_and_keep_channels_isolated() {
    for (kind, name, value, expected) in [
        (NodeKind::Gain, "gainDb", json!(-6.020599913), 0.1),
        (NodeKind::Volume, "percent", json!(50), 0.1),
    ] {
        let mut session = route(kind);
        session.nodes[1].parameters.insert(name.into(), value);
        let graph = compile_session(&session, RuntimeGeneration::new(4)).unwrap();
        let mut block = AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.2);
        graph.process(&mut block);
        assert!(
            (block.channel(0).unwrap()[127] - expected).abs() <= 1e-6,
            "{kind:?}"
        );
        assert_eq!(block.channel(1).unwrap(), &[0.0; 128]);
    }
}

#[test]
fn advanced_eq_reports_the_incoming_spectrum_without_changing_audio() {
    let session = route(NodeKind::ParametricEq);
    let graph = compile_session(&session, RuntimeGeneration::new(5)).unwrap();
    let id = EntityId::new("tool");
    assert!(
        graph.spectrum_levels_for_node(&id).is_none(),
        "no spectrum before audio"
    );
    let reference = compile_session(&route(NodeKind::Gain), RuntimeGeneration::new(6)).unwrap();
    for quantum in 0..120 {
        let mut block = AudioBlock::new(2, 128).unwrap();
        for channel in 0..2 {
            for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
                *sample = 0.3
                    * (std::f64::consts::TAU * 1_000.0 * (quantum * 128 + frame) as f64 / 48_000.0)
                        .sin() as f32;
            }
        }
        let mut flat = AudioBlock::new(2, 128).unwrap();
        for channel in 0..2 {
            flat.channel_mut(channel)
                .unwrap()
                .copy_from_slice(block.channel(channel).unwrap());
        }
        graph.process(&mut block);
        reference.process(&mut flat);
        // A default (flat) Advanced EQ passes the tone like a 0 dB Gain.
        for channel in 0..2 {
            for (eq, gain) in block
                .channel(channel)
                .unwrap()
                .iter()
                .zip(flat.channel(channel).unwrap())
            {
                assert!(
                    (eq - gain).abs() < 1e-4,
                    "analysis must not change the audio"
                );
            }
        }
    }
    let levels = graph
        .spectrum_levels_for_node(&id)
        .expect("spectrum while playing");
    let centers = audiorouter_engine::spectrum_band_frequencies_hz();
    let loudest = (0..levels.len())
        .max_by(|a, b| levels[*a].total_cmp(&levels[*b]))
        .unwrap();
    assert!(
        (800.0..1_250.0).contains(&centers[loudest]),
        "peak at {} Hz",
        centers[loudest]
    );
    assert!(
        compile_session(&route(NodeKind::BassTreble), RuntimeGeneration::new(7))
            .unwrap()
            .spectrum_levels_for_node(&id)
            .is_none()
    );
}
