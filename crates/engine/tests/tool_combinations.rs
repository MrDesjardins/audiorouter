//! Combinations of every built-in chain tool in 4–6-tool chains, across route
//! layouts and Enabled/Bypass patterns, through the native multi-path compiler
//! the desktop app uses. No WASAPI or private audio.
//!
//! Each combination must compile, process finite audio, be accepted as a live
//! replacement of the all-active route (flags apply while playing), and, when
//! every tool is bypassed or off, deliver exactly the dry source mix.
//! Regression origin: a bypassed Duck between two Mixers made the whole route
//! unsupported (2026-10-02).
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_native_paths_with_plugins_and_audio, AudioBlock, AudioBlockRing, CompiledPathSet,
    RealtimeMixerFanout, RuntimeGeneration,
};
use serde_json::json;

const FRAMES: usize = 128;

/// Every built-in one-in/one-out tool a chain may contain.
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

/// Chain lengths and strides: every tool lands in many chains and positions.
const LENGTHS: [usize; 3] = [4, 5, 6];
const STRIDES: [usize; 1] = [3];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Layout {
    /// source → chain → output
    Single,
    /// source → chain → two outputs
    FanOut,
    /// source A → chain → Mixer ← source B; Mixer → output
    Mixer,
    /// A → first half → Mixer 1 ← B; Mixer 1 → second half → Mixer 2 ← C; → output
    ConnectedMixers,
    /// A → first half → output 1; B → second half → output 2
    TwoPaths,
}

const LAYOUTS: [Layout; 5] = [
    Layout::Single,
    Layout::FanOut,
    Layout::Mixer,
    Layout::ConnectedMixers,
    Layout::TwoPaths,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Flags {
    AllActive,
    AllBypassed,
    AllOff,
    AlternateBypass,
    AlternateOff,
    /// One tool bypassed, another off, the rest active.
    Mixed,
}

const FLAGS: [Flags; 6] = [
    Flags::AllActive,
    Flags::AllBypassed,
    Flags::AllOff,
    Flags::AlternateBypass,
    Flags::AlternateOff,
    Flags::Mixed,
];

impl Flags {
    /// (enabled, bypass) for the tool at `index` in a chain.
    fn at(self, index: usize) -> (bool, bool) {
        match self {
            Self::AllActive => (true, false),
            Self::AllBypassed => (true, true),
            Self::AllOff => (false, false),
            Self::AlternateBypass => (true, index % 2 == 1),
            Self::AlternateOff => (index % 2 == 0, false),
            Self::Mixed => match index {
                1 => (true, true),
                2 => (false, false),
                _ => (true, false),
            },
        }
    }

    fn dry(self) -> bool {
        matches!(self, Self::AllBypassed | Self::AllOff)
    }
}

/// Deterministic chains: for each length, a rotation through all tools.
fn chains() -> Vec<Vec<NodeKind>> {
    let mut chains = Vec::new();
    for &length in &LENGTHS {
        for &stride in &STRIDES {
            for start in 0..TOOLS.len() {
                chains.push(
                    (0..length)
                        .map(|step| TOOLS[(start + step * stride) % TOOLS.len()])
                        .collect(),
                );
            }
        }
    }
    chains
}

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

fn tool(id: String, kind: NodeKind, (enabled, bypass): (bool, bool), chain_index: usize) -> Node {
    let mut tool = node(&id, kind);
    tool.enabled = enabled;
    tool.bypass = bypass;
    let parameters = match kind {
        NodeKind::TimeShift => json!({ "bufferSeconds": 10 }),
        NodeKind::Mute => json!({ "muted": false }),
        // Both trigger modes; nothing publishes a level or round, so it stays released.
        NodeKind::Duck if chain_index % 2 == 0 => {
            json!({ "trigger": "siegeRound", "amountDb": 20.0 })
        }
        NodeKind::Duck => json!({ "trigger": "level", "keyNodeId": "src-b", "amountDb": 12.0 }),
        NodeKind::Gain => json!({ "gainDb": -3.0 }),
        _ => json!({}),
    };
    tool.parameters = serde_json::from_value(parameters).unwrap();
    tool
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

static SESSION_SERIAL: AtomicUsize = AtomicUsize::new(0);

/// Build the route. Session IDs are unique so per-node shared state (Time
/// Shift buffers, Duck carry) never leaks between compiles.
fn route(chain: &[NodeKind], layout: Layout, flags: Flags, chain_index: usize) -> Session {
    let tools = chain
        .iter()
        .enumerate()
        .map(|(index, &kind)| tool(format!("t{index}"), kind, flags.at(index), chain_index))
        .collect::<Vec<_>>();
    let ids = tools
        .iter()
        .map(|tool| tool.id.as_str().to_owned())
        .collect::<Vec<_>>();
    let half = ids.len() / 2;
    let mut nodes = vec![node("src-a", NodeKind::PhysicalInput)];
    let mut edges = Vec::new();
    let link = |edges: &mut Vec<Edge>, from: &str, path: &[String], to: &str| {
        let mut previous = from.to_owned();
        for id in path {
            edges.push(edge(&previous, id));
            previous = id.clone();
        }
        edges.push(edge(&previous, to));
    };
    match layout {
        Layout::Single => {
            nodes.push(node("out-1", NodeKind::PhysicalOutput));
            link(&mut edges, "src-a", &ids, "out-1");
        }
        Layout::FanOut => {
            nodes.extend([
                node("out-1", NodeKind::PhysicalOutput),
                node("out-2", NodeKind::PhysicalOutput),
            ]);
            link(&mut edges, "src-a", &ids, "out-1");
            edges.push(edge(ids.last().unwrap(), "out-2"));
        }
        Layout::Mixer => {
            nodes.extend([
                node("src-b", NodeKind::PhysicalInput),
                node("mix-1", NodeKind::Mixer),
                node("out-1", NodeKind::PhysicalOutput),
            ]);
            link(&mut edges, "src-a", &ids, "mix-1");
            edges.extend([edge("src-b", "mix-1"), edge("mix-1", "out-1")]);
        }
        Layout::ConnectedMixers => {
            nodes.extend([
                node("src-b", NodeKind::PhysicalInput),
                node("src-c", NodeKind::PhysicalInput),
                node("mix-1", NodeKind::Mixer),
                node("mix-2", NodeKind::Mixer),
                node("out-1", NodeKind::PhysicalOutput),
            ]);
            link(&mut edges, "src-a", &ids[..half], "mix-1");
            edges.push(edge("src-b", "mix-1"));
            link(&mut edges, "mix-1", &ids[half..], "mix-2");
            edges.extend([edge("src-c", "mix-2"), edge("mix-2", "out-1")]);
        }
        Layout::TwoPaths => {
            nodes.extend([
                node("src-b", NodeKind::PhysicalInput),
                node("out-1", NodeKind::PhysicalOutput),
                node("out-2", NodeKind::PhysicalOutput),
            ]);
            link(&mut edges, "src-a", &ids[..half], "out-1");
            link(&mut edges, "src-b", &ids[half..], "out-2");
        }
    }
    nodes.extend(tools);
    Session {
        id: EntityId::new(format!(
            "combo-{}",
            SESSION_SERIAL.fetch_add(1, Ordering::Relaxed)
        )),
        name: "Tool combination".into(),
        schema_version: 1,
        revision: 1,
        nodes,
        edges,
    }
}

/// A different tone per source and channel, offset by the quantum.
fn source_block(source: &str, quantum: usize) -> AudioBlock {
    let frequency = match source {
        "src-a" => 220.0,
        "src-b" => 330.0,
        _ => 495.0,
    };
    let mut block = AudioBlock::new(2, FRAMES).unwrap();
    for channel in 0..2 {
        for (frame, sample) in block.channel_mut(channel).unwrap().iter_mut().enumerate() {
            let t = (quantum * FRAMES + frame) as f64 / 48_000.0;
            *sample = (0.2
                * (std::f64::consts::TAU * frequency * (1.0 + 0.5 * channel as f64) * t).sin())
                as f32;
        }
    }
    block
}

/// Process `quanta` blocks through every path; returns the last block per output node.
fn run(set: &CompiledPathSet, quanta: usize) -> HashMap<String, AudioBlock> {
    let mut last = HashMap::new();
    for quantum in 0..quanta {
        for path in set.paths() {
            let inputs = path
                .input_node_ids()
                .iter()
                .map(|id| source_block(id.as_str(), quantum))
                .collect::<Vec<_>>();
            let mut scratch = AudioBlock::new(path.mixer_channels(), FRAMES).unwrap();
            let mut outputs = path
                .output_node_ids()
                .iter()
                .map(|_| AudioBlock::new(2, FRAMES).unwrap())
                .collect::<Vec<_>>();
            path.process(
                &inputs,
                &mut scratch,
                &mut outputs.iter_mut().collect::<Vec<_>>(),
            )
            .unwrap();
            for (id, block) in path.output_node_ids().iter().zip(outputs) {
                for channel in 0..2 {
                    assert!(
                        block
                            .channel(channel)
                            .unwrap()
                            .iter()
                            .all(|sample| sample.is_finite()),
                        "non-finite output at {id:?}"
                    );
                }
                last.insert(id.as_str().to_owned(), block);
            }
        }
    }
    last
}

/// The dry mix each output should carry when every tool passes audio unchanged.
fn dry_sources(layout: Layout, output: &str) -> &'static [&'static str] {
    match (layout, output) {
        (Layout::Mixer, _) => &["src-a", "src-b"],
        (Layout::ConnectedMixers, _) => &["src-a", "src-b", "src-c"],
        (Layout::TwoPaths, "out-2") => &["src-b"],
        _ => &["src-a"],
    }
}

fn compile(session: &Session) -> CompiledPathSet {
    compile_native_paths_with_plugins_and_audio(
        session,
        RuntimeGeneration::new(1),
        &Default::default(),
        &Default::default(),
    )
    .unwrap_or_else(|error| {
        let tools = session
            .nodes
            .iter()
            .filter(|node| TOOLS.contains(&node.kind))
            .map(|node| {
                format!(
                    "{:?}(on={},bypass={})",
                    node.kind, node.enabled, node.bypass
                )
            })
            .collect::<Vec<_>>();
        panic!("rejected {error:?}: {tools:?}")
    })
}

#[test]
fn every_tool_appears_in_many_chains_and_positions() {
    let chains = chains();
    for &kind in TOOLS {
        let chains_with = chains.iter().filter(|chain| chain.contains(&kind)).count();
        let positions = chains
            .iter()
            .flat_map(|chain| {
                chain
                    .iter()
                    .enumerate()
                    .filter(move |(_, tool)| **tool == kind)
                    .map(|(position, _)| position)
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(chains_with >= 6, "{kind:?} in only {chains_with} chains");
        assert!(
            positions.contains(&0) && positions.len() >= 4,
            "{kind:?} positions {positions:?}"
        );
    }
}

#[test]
fn tool_chains_compile_process_and_apply_flags_live_in_every_layout() {
    let chains = chains();
    let mut combinations = 0;
    for (chain_index, chain) in chains.iter().enumerate() {
        for layout in LAYOUTS {
            let base = compile(&route(chain, layout, Flags::AllActive, chain_index));
            let channels = base.input_node_ids().iter().map(|_| 2).collect::<Vec<_>>();
            let mut live = RealtimeMixerFanout::from_paths(
                compile(&route(chain, layout, Flags::AllActive, chain_index)),
                4,
                &channels,
                FRAMES,
            )
            .unwrap();
            for flags in FLAGS {
                let session = route(chain, layout, flags, chain_index);
                let set = compile(&session);
                // Same sources and outputs as the active route: the toggle applies while playing.
                assert_eq!(
                    set.input_node_ids(),
                    base.input_node_ids(),
                    "{chain:?} {layout:?} {flags:?}"
                );
                assert_eq!(
                    set.output_node_ids(),
                    base.output_node_ids(),
                    "{chain:?} {layout:?} {flags:?}"
                );
                live.replace_paths(set).unwrap_or_else(|error| {
                    panic!("live replace refused {error:?}: {chain:?} {layout:?} {flags:?}")
                });
                let outputs = run(&compile(&session), if flags.dry() { 6 } else { 12 });
                if flags.dry() {
                    for (output, block) in &outputs {
                        let mut expected = AudioBlock::new(2, FRAMES).unwrap();
                        for source in dry_sources(layout, output) {
                            let dry = source_block(source, 5);
                            for channel in 0..2 {
                                for (sum, sample) in expected
                                    .channel_mut(channel)
                                    .unwrap()
                                    .iter_mut()
                                    .zip(dry.channel(channel).unwrap())
                                {
                                    *sum += sample;
                                }
                            }
                        }
                        for channel in 0..2 {
                            for (frame, (actual, wanted)) in block
                                .channel(channel)
                                .unwrap()
                                .iter()
                                .zip(expected.channel(channel).unwrap())
                                .enumerate()
                            {
                                assert!((actual - wanted).abs() <= 1e-5, "{chain:?} {layout:?} {flags:?} {output} ch{channel} frame {frame}: {actual} vs {wanted}");
                            }
                        }
                    }
                }
                combinations += 1;
            }
        }
    }
    eprintln!("{combinations} tool-chain combinations qualified");
    assert_eq!(combinations, chains.len() * LAYOUTS.len() * FLAGS.len());
}

#[test]
fn recompiling_the_same_combination_gives_identical_audio() {
    for (chain_index, chain) in chains().iter().enumerate().step_by(7) {
        for layout in [Layout::Mixer, Layout::ConnectedMixers] {
            for flags in [Flags::AllActive, Flags::Mixed] {
                let first = run(&compile(&route(chain, layout, flags, chain_index)), 10);
                let second = run(&compile(&route(chain, layout, flags, chain_index)), 10);
                for (output, block) in &first {
                    for channel in 0..2 {
                        assert_eq!(
                            block.channel(channel),
                            second[output].channel(channel),
                            "{chain:?} {layout:?} {flags:?} {output}"
                        );
                    }
                }
            }
        }
    }
}

/// Counts heap allocations made on the current thread while `TRACKING` is
/// set. Other test threads keep allocating freely.
struct CountingAllocator;

thread_local! {
    static TRACKING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALLOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn note_allocation() {
    // `try_with` keeps thread teardown (TLS already destroyed) from panicking.
    let _ = TRACKING.try_with(|tracking| {
        if tracking.get() {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
    });
}

// SAFETY: every call forwards unchanged to the system allocator, which upholds
// the `GlobalAlloc` contract; the counter only touches const-initialized
// thread-local `Cell`s, which never allocate.
unsafe impl std::alloc::GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        note_allocation();
        // SAFETY: forwards this method's own caller contract (a valid, non-zero-size
        // `layout`) unchanged to the system allocator.
        unsafe { std::alloc::System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        note_allocation();
        // SAFETY: forwards this method's own caller contract unchanged.
        unsafe { std::alloc::System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        note_allocation();
        // SAFETY: the caller guarantees `ptr` was allocated by this allocator with
        // `layout`; every block came from `System`, so the contract carries over.
        unsafe { std::alloc::System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        // SAFETY: as for `realloc`: `ptr` and `layout` describe a block that this
        // allocator obtained from `System`.
        unsafe { std::alloc::System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Heap allocations `work` makes on this thread.
fn allocations_during(work: impl FnOnce()) -> usize {
    ALLOCATIONS.with(|count| count.set(0));
    TRACKING.with(|tracking| tracking.set(true));
    work();
    TRACKING.with(|tracking| tracking.set(false));
    ALLOCATIONS.with(|count| count.get())
}

/// Realtime rule (AGENTS.md "Architecture rules"): processing a quantum never
/// allocates. Each tool runs on its own, from the very first quantum, through
/// the caller-owned-block path; then whole chains run through the ring path
/// the native outputs use, in every layout. Add new tools to `TOOLS`.
#[test]
fn processing_never_allocates_for_any_tool_or_layout() {
    // The counter itself must see a deliberate allocation.
    assert!(allocations_during(|| drop(std::hint::black_box(vec![0u8; 64]))) >= 1);
    let quanta = 24;
    let mut failures = Vec::new();
    for (index, &kind) in TOOLS.iter().enumerate() {
        let set = compile(&route(&[kind], Layout::Single, Flags::AllActive, index));
        let path = &set.paths()[0];
        let blocks = (0..quanta)
            .map(|quantum| vec![source_block("src-a", quantum)])
            .collect::<Vec<_>>();
        let mut scratch = AudioBlock::new(path.mixer_channels(), FRAMES).unwrap();
        let mut output = AudioBlock::new(2, FRAMES).unwrap();
        let count = allocations_during(|| {
            for inputs in &blocks {
                path.process(inputs, &mut scratch, &mut [&mut output])
                    .unwrap();
            }
        });
        if count != 0 {
            failures.push(format!("{kind:?} alone: {count} allocations"));
        }
    }
    for (chain_index, chain) in chains().iter().enumerate().step_by(3) {
        for layout in LAYOUTS {
            let set = compile(&route(chain, layout, Flags::AllActive, chain_index));
            let channels = set.input_node_ids().iter().map(|_| 2).collect::<Vec<_>>();
            let inputs = set
                .input_node_ids()
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect::<Vec<_>>();
            let mut fanout = RealtimeMixerFanout::from_paths(set, 4, &channels, FRAMES).unwrap();
            let rings = (0..fanout.branch_count())
                .map(|_| AudioBlockRing::new(4, 2, FRAMES).unwrap())
                .collect::<Vec<_>>();
            let destinations = rings.iter().collect::<Vec<_>>();
            let blocks = (0..quanta)
                .map(|quantum| {
                    inputs
                        .iter()
                        .map(|id| source_block(id, quantum))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let generation = fanout.generation();
            let mut delivered = 0;
            let count = allocations_during(|| {
                for quantum in &blocks {
                    for (input, block) in quantum.iter().enumerate() {
                        assert!(fanout.try_submit_input(input, generation, block).unwrap());
                    }
                    delivered += fanout.process_once(&destinations).unwrap();
                    for ring in &rings {
                        while let Some(block) = ring.try_receive() {
                            let _ = ring.try_recycle(block);
                        }
                    }
                }
            });
            assert_eq!(delivered, quanta * rings.len(), "{chain:?} {layout:?}");
            if count != 0 {
                failures.push(format!("{chain:?} {layout:?}: {count} allocations"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "processing allocated:\n{}",
        failures.join("\n")
    );
}
