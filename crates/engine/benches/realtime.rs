//! Realtime cost of the built-in tools, per 128-frame quantum at 48 kHz.
//!
//! Run with `cargo bench -p audiorouter-engine --bench realtime`. Each line
//! shows the median time per quantum and its share of the 2.67 ms quantum
//! budget. A tool that uses more than a few percent alone, or a route that
//! nears the budget, will crackle on slower machines. Optional filter:
//! `cargo bench -p audiorouter-engine --bench realtime -- Denoise`.
use std::hint::black_box;
use std::time::{Duration, Instant};

use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_native_paths_with_plugins_and_audio, AudioBlock, CompiledPathSet, RuntimeGeneration,
};

const FRAMES: usize = 128;
const SAMPLE_RATE: f64 = 48_000.0;
const WARM_UP_QUANTA: usize = 200;
const MEASURED_QUANTA: usize = 2_000;

/// Heavy and common tools first; every one-in/one-out built-in tool.
const TOOLS: &[NodeKind] = &[
    NodeKind::Denoise,
    NodeKind::SpeechDenoise,
    NodeKind::SpectralGate,
    NodeKind::Pitch,
    NodeKind::ParametricEq,
    NodeKind::GraphicEq,
    NodeKind::FirFilter,
    NodeKind::Compressor,
    NodeKind::Gate,
    NodeKind::Limiter,
    NodeKind::Dehum,
    NodeKind::Declick,
    NodeKind::BassTreble,
    NodeKind::Delay,
    NodeKind::Meter,
    NodeKind::Gain,
];

fn port(name: &str, direction: PortDirection) -> Port {
    Port {
        name: name.into(),
        direction,
        channels: 2,
    }
}

fn node(id: &str, kind: NodeKind) -> Node {
    let ports = match kind {
        NodeKind::PhysicalInput => vec![port("out", PortDirection::Output)],
        NodeKind::PhysicalOutput => vec![port("in", PortDirection::Input)],
        _ => vec![
            port("in", PortDirection::Input),
            port("out", PortDirection::Output),
        ],
    };
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    }
}

fn edge(source: &str, destination: &str) -> Edge {
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

/// Input → `chain` → output, all tools active with default settings.
fn chain_route(name: &str, chain: &[NodeKind]) -> CompiledPathSet {
    let mut nodes = vec![
        node("in", NodeKind::PhysicalInput),
        node("out", NodeKind::PhysicalOutput),
    ];
    let mut edges = Vec::new();
    let mut previous = "in".to_owned();
    for (index, &kind) in chain.iter().enumerate() {
        let id = format!("t{index}");
        nodes.push(node(&id, kind));
        edges.push(edge(&previous, &id));
        previous = id;
    }
    edges.push(edge(&previous, "out"));
    let session = Session {
        id: EntityId::new(format!("bench-{name}")),
        name: name.into(),
        schema_version: 1,
        revision: 1,
        nodes,
        edges,
    };
    compile_native_paths_with_plugins_and_audio(
        &session,
        RuntimeGeneration::new(1),
        &Default::default(),
        &Default::default(),
    )
    .unwrap_or_else(|error| panic!("{name}: {error:?}"))
}

/// A voice-like signal: a 140 Hz tone with harmonics plus a little noise.
fn voice_block(quantum: usize) -> AudioBlock {
    let mut block = AudioBlock::new(2, FRAMES).unwrap();
    let mut seed = 0x2545_f491_u32.wrapping_add(quantum as u32);
    for channel in 0..2 {
        for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
            let t = (quantum * FRAMES + frame) as f64 / SAMPLE_RATE;
            let tone = (1..=5)
                .map(|harmonic| {
                    (std::f64::consts::TAU * 140.0 * harmonic as f64 * t).sin() / harmonic as f64
                })
                .sum::<f64>();
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let noise = (seed as f64 / u32::MAX as f64 - 0.5) * 0.02;
            *sample = (0.15 * tone + noise) as f32;
        }
    }
    block
}

/// Median time to process one quantum through the route.
fn measure(set: &CompiledPathSet) -> Duration {
    let path = &set.paths()[0];
    let blocks = (0..64)
        .map(|quantum| vec![voice_block(quantum)])
        .collect::<Vec<_>>();
    let mut scratch = AudioBlock::new(path.mixer_channels(), FRAMES).unwrap();
    let mut output = AudioBlock::new(2, FRAMES).unwrap();
    for quantum in 0..WARM_UP_QUANTA {
        path.process(
            &blocks[quantum % blocks.len()],
            &mut scratch,
            &mut [&mut output],
        )
        .unwrap();
    }
    let mut samples = Vec::with_capacity(MEASURED_QUANTA);
    for quantum in 0..MEASURED_QUANTA {
        let inputs = &blocks[quantum % blocks.len()];
        let start = Instant::now();
        path.process(black_box(inputs), &mut scratch, &mut [&mut output])
            .unwrap();
        samples.push(start.elapsed());
        black_box(&output);
    }
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn report(name: &str, median: Duration) {
    let budget = Duration::from_secs_f64(FRAMES as f64 / SAMPLE_RATE);
    println!(
        "{name:<32} {:>9.1} µs/quantum {:>6.2}% of budget",
        median.as_secs_f64() * 1e6,
        100.0 * median.as_secs_f64() / budget.as_secs_f64()
    );
}

fn main() {
    // `cargo bench` passes `--bench`; any other argument filters by name.
    let filter = std::env::args()
        .skip(1)
        .find(|argument| !argument.starts_with("--"));
    let wanted = |name: &str| filter.as_deref().is_none_or(|filter| name.contains(filter));
    if cfg!(debug_assertions) {
        println!("note: unoptimized build; use `cargo bench` for real numbers");
    }
    for &kind in TOOLS {
        let name = format!("{kind:?}");
        if wanted(&name) {
            report(&name, measure(&chain_route(&name, &[kind])));
        }
    }
    // 32 tools in one route: every tool twice, cycling.
    let long_chain = (0..32)
        .map(|index| TOOLS[index % TOOLS.len()])
        .collect::<Vec<_>>();
    if wanted("route-32-tools") {
        report(
            "route-32-tools",
            measure(&chain_route("route-32-tools", &long_chain)),
        );
    }
}
