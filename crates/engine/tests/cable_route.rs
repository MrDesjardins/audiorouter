//! A cable-only route (`VirtualRenderSource → VirtualCaptureSink`) through the
//! engine compiler and `RealtimeScheduler` must be bit-exact (VCAB-20): no
//! gain, dither or clamping between the two cables. The single deviation is
//! pinned: negative zero arrives as +0.0. Portable; no driver or WASAPI.
use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    RealtimeScheduler, RuntimeGeneration, MAX_CHANNELS, PROCESSING_QUANTUM_FRAMES,
};
use serde_json::json;

// The engine quantum; the cable worker re-blocks bridge blocks to it.
const FRAMES: usize = PROCESSING_QUANTUM_FRAMES;

fn node(id: &str, kind: NodeKind, bus: &str, port: Port) -> Node {
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: json!({ "busId": bus }).as_object().unwrap().clone(),
        ports: vec![port],
    }
}

/// The session the cable route tool builds: Cable A's render source feeds
/// Cable B's capture sink with an identity matrix.
pub fn cable_route_session(channels: u8) -> Session {
    let identity = (0..usize::from(channels))
        .flat_map(|row| (0..usize::from(channels)).map(move |col| f32::from(u8::from(row == col))))
        .collect();
    Session {
        id: EntityId::new("cable-route"),
        name: "Cable route".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "source",
                NodeKind::VirtualRenderSource,
                "cable-a",
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels,
                },
            ),
            node(
                "sink",
                NodeKind::VirtualCaptureSink,
                "cable-b",
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels,
                },
            ),
        ],
        edges: vec![Edge {
            id: EntityId::new("source-sink"),
            source_node: EntityId::new("source"),
            source_port: "out".into(),
            destination_node: EntityId::new("sink"),
            destination_port: "in".into(),
            matrix: identity,
            enabled: true,
        }],
    }
}

/// Deterministic noise on the 2^-24 grid with |x| <= 0.5, the probe's
/// `cable-bitexact` signal, plus exact zeros, negative zeros and the extremes.
fn signal(channels: usize, block: u64) -> Vec<f32> {
    let mut state = 0x9E37_79B9_7F4A_7C15u64 ^ block.wrapping_mul(0xD1B5_4A32_D192_ED03);
    (0..FRAMES * channels)
        .map(|index| match index % 97 {
            0 => 0.0,
            1 => -0.0,
            2 => 0.5,
            3 => -0.5,
            _ => {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let step = (state >> 40) as i32 % (1 << 23);
                step as f32 / (1u32 << 24) as f32
            }
        })
        .collect()
}

fn run_route(channels: u8) {
    let channels_usize = usize::from(channels);
    let scheduler = RealtimeScheduler::new(4, channels_usize, FRAMES).unwrap();
    scheduler
        .activate_session_at_sample_rate(
            &cable_route_session(channels),
            RuntimeGeneration::new(1),
            48_000,
        )
        .expect("a cable-only route compiles");
    let mut output = vec![0.0f32; FRAMES * channels_usize];
    for block in 0..64u64 {
        let input = signal(channels_usize, block);
        let mut buffer = scheduler.acquire_input().unwrap();
        buffer.copy_from_interleaved(&input).unwrap();
        assert!(scheduler.submit_input(buffer).is_ok());
        let generation = scheduler.process_once().unwrap().expect("processed");
        let processed = scheduler
            .receive_output_for_generation(generation)
            .expect("output");
        processed.copy_to_interleaved(&mut output).unwrap();
        let _ = scheduler.output().try_recycle(processed);
        for (index, (sent, received)) in input.iter().zip(&output).enumerate() {
            // The channel matrix sums from +0.0 by design (`mapped_mono`), so
            // -0.0 arrives as +0.0: numerically equal, one bit different.
            let expected = if sent.to_bits() == (-0.0f32).to_bits() {
                0.0
            } else {
                *sent
            };
            assert_eq!(
                expected.to_bits(),
                received.to_bits(),
                "channels {channels}, block {block}, sample {index}: {sent:e} became {received:e}"
            );
        }
    }
}

#[test]
fn cable_only_route_is_bit_exact_in_stereo() {
    run_route(2);
}

#[test]
fn cable_only_route_is_bit_exact_in_mono() {
    run_route(1);
}

/// Eight-channel cables (VCAB-20) exceed the engine's processing width;
/// pin the limit so a change to it revisits the cable worker.
#[test]
fn engine_processing_width_is_still_stereo() {
    assert_eq!(MAX_CHANNELS, 2);
    assert!(RealtimeScheduler::new(4, 8, FRAMES).is_err());
}

/// Built-in one-in/one-out tools, as in `tool_combinations.rs`. Duck uses
/// its key-less round trigger: a cable route has one source.
const TOOLS: &[NodeKind] = &[
    NodeKind::Meter,
    NodeKind::Gain,
    NodeKind::Volume,
    NodeKind::BassTreble,
    NodeKind::Dehum,
    NodeKind::Declick,
    NodeKind::Denoise,
    NodeKind::SpeechDenoise,
    NodeKind::SpectralGate,
    NodeKind::FunVoiceChanger,
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
    NodeKind::Duck,
];

/// Cable A → `chain` → Cable B, every tool active or every tool bypassed.
fn tool_route(chain: &[NodeKind], bypass: bool, serial: usize) -> Session {
    let mut session = cable_route_session(2);
    session.id = EntityId::new(format!("cable-tools-{serial}"));
    session.edges.clear();
    let mut previous = "source".to_owned();
    for (index, &kind) in chain.iter().enumerate() {
        let id = format!("t{index}");
        let parameters = match kind {
            NodeKind::TimeShift => json!({ "bufferSeconds": 10 }),
            NodeKind::Mute => json!({ "muted": false }),
            NodeKind::Duck => json!({ "trigger": "siegeRound", "amountDb": 20.0 }),
            NodeKind::Gain => json!({ "gainDb": -3.0 }),
            _ => json!({}),
        };
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        session.nodes.push(Node {
            id: EntityId::new(&id),
            kind,
            type_version: 1,
            name: id.clone(),
            enabled: true,
            bypass,
            parameters: parameters.as_object().unwrap().clone(),
            ports: vec![
                port("in", PortDirection::Input),
                port("out", PortDirection::Output),
            ],
        });
        session.edges.push(link(&previous, &id));
        previous = id;
    }
    session.edges.push(link(&previous, "sink"));
    session
}

fn link(source: &str, destination: &str) -> Edge {
    Edge {
        id: EntityId::new(format!("{source}->{destination}")),
        source_node: EntityId::new(source),
        source_port: "out".into(),
        destination_node: EntityId::new(destination),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    }
}

/// Every built-in tool, in 4–6-tool chains, compiles between two cables
/// (spec 17 §7.3, validated lesson 2026-10-02) and processes finite audio;
/// with every tool bypassed the route stays bit-exact.
#[test]
fn cable_route_accepts_every_tool_chain() {
    let mut serial = 0;
    for length in [4, 5, 6] {
        for start in 0..TOOLS.len() {
            let chain: Vec<NodeKind> = (0..length)
                .map(|step| TOOLS[(start + step * 3) % TOOLS.len()])
                .collect();
            for bypass in [false, true] {
                serial += 1;
                let scheduler = RealtimeScheduler::new(4, 2, FRAMES).unwrap();
                scheduler
                    .activate_session_at_sample_rate(
                        &tool_route(&chain, bypass, serial),
                        RuntimeGeneration::new(1),
                        48_000,
                    )
                    .unwrap_or_else(|error| panic!("{chain:?} bypass={bypass}: {error:?}"));
                let mut output = vec![0.0f32; FRAMES * 2];
                for block in 0..24u64 {
                    let input = signal(2, block);
                    let mut buffer = scheduler.acquire_input().unwrap();
                    buffer.copy_from_interleaved(&input).unwrap();
                    assert!(scheduler.submit_input(buffer).is_ok());
                    let generation = scheduler.process_once().unwrap().expect("processed");
                    let processed = scheduler.receive_output_for_generation(generation).unwrap();
                    processed.copy_to_interleaved(&mut output).unwrap();
                    let _ = scheduler.output().try_recycle(processed);
                    assert!(output.iter().all(|sample| sample.is_finite()), "{chain:?}");
                    if bypass {
                        for (sent, received) in input.iter().zip(&output) {
                            let expected = if *sent == 0.0 { 0.0 } else { *sent };
                            assert_eq!(
                                expected.to_bits(),
                                received.to_bits(),
                                "{chain:?} bypassed"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// A multichannel cable route bypasses the engine (WP-09 slice 5): its
/// compilers must refuse more than the processing width with an error,
/// never a panic, so a mistaken route fails cleanly.
#[test]
fn engine_refuses_multichannel_cable_graphs_cleanly() {
    let mut session = cable_route_session(2);
    for node in &mut session.nodes {
        node.ports[0].channels = 8;
    }
    session.edges[0].matrix = (0..64).map(|i| f32::from(u8::from(i % 9 == 0))).collect();
    assert!(audiorouter_domain::validate_session(&session).is_ok());
    let scheduler = RealtimeScheduler::new(4, 2, FRAMES).unwrap();
    assert!(scheduler
        .activate_session_at_sample_rate(&session, RuntimeGeneration::new(1), 48_000)
        .is_err());
    assert!(audiorouter_engine::compile_session_at_sample_rate(
        &session,
        RuntimeGeneration::new(1),
        48_000
    )
    .is_err());
}
