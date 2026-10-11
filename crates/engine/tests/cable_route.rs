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
