//! Every source tool must reach every route layout: a single route, a fan-out,
//! a Mixer with any other source, and connected Mixers with any two others,
//! through the native multi-path compiler the desktop app uses.
//!
//! Regression origin: the Soundboard compiled alone but a Mixer route
//! containing it was refused as an "audio combination the engine cannot play"
//! (2026-10-29), because the connected-Mixer source list was written by hand.
//! `role` has no wildcard arm, so adding a `NodeKind` fails the build until
//! someone decides what it is; a new source is then covered automatically.
use std::collections::HashMap;
use std::sync::Arc;

use audiorouter_domain::{Edge, EntityId, Node, NodeKind, Port, PortDirection, Session};
use audiorouter_engine::{
    compile_native_paths_with_plugins_and_audio, AudioBlock, CompiledMixerFanoutGraph,
    CompiledPathSet, DecodedAudio, RuntimeGeneration,
};
use serde_json::json;

const FRAMES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Role {
    /// Produces audio and has no input: must be accepted by a Mixer.
    Source,
    /// Final destination of a route.
    Sink,
    /// One audio input and one audio output, covered by `tool_combinations`.
    Processor,
    /// Combines inputs; the layouts below build it explicitly.
    Mixer,
    /// Needs a worker or a second input the device-free harness cannot give.
    Special,
}

/// Exhaustive on purpose: no `_` arm.
fn role(kind: NodeKind) -> Role {
    match kind {
        NodeKind::PhysicalInput
        | NodeKind::ApplicationCapture
        | NodeKind::EndpointLoopback
        | NodeKind::VirtualRenderSource
        | NodeKind::TestSignal
        | NodeKind::AudioFile
        | NodeKind::NetworkReceive
        | NodeKind::Soundboard => Role::Source,
        NodeKind::PhysicalOutput | NodeKind::VirtualCaptureSink | NodeKind::NetworkSend => {
            Role::Sink
        }
        NodeKind::Mixer => Role::Mixer,
        NodeKind::Gain
        | NodeKind::Mute
        | NodeKind::Meter
        | NodeKind::ParametricEq
        | NodeKind::Compressor
        | NodeKind::Gate
        | NodeKind::Limiter
        | NodeKind::Delay
        | NodeKind::GraphicEq
        | NodeKind::Pitch
        | NodeKind::Volume
        | NodeKind::BassTreble
        | NodeKind::Dehum
        | NodeKind::Declick
        | NodeKind::Denoise
        | NodeKind::SpeechDenoise
        | NodeKind::FirFilter
        | NodeKind::SpectralGate
        | NodeKind::TimeShift
        | NodeKind::Duck
        | NodeKind::FunVoiceChanger => Role::Processor,
        NodeKind::Recorder | NodeKind::Plugin | NodeKind::InputSwitch | NodeKind::OutputSwitch => {
            Role::Special
        }
    }
}

fn sources() -> Vec<NodeKind> {
    NodeKind::ALL
        .into_iter()
        .filter(|kind| role(*kind) == Role::Source)
        .collect()
}

fn port(name: &str, direction: PortDirection) -> Port {
    Port {
        name: name.into(),
        direction,
        channels: 2,
    }
}

fn source_node(id: &str, kind: NodeKind) -> Node {
    let parameters = match kind {
        NodeKind::AudioFile => json!({ "mediaId": "media-1" }),
        NodeKind::Soundboard => json!({ "slot1MediaId": "media-1" }),
        NodeKind::EndpointLoopback => json!({ "defaultRole": "console" }),
        NodeKind::VirtualRenderSource => json!({ "busId": "bus-1" }),
        NodeKind::NetworkReceive => json!({ "sender": "192.168.1.20", "port": 48000 }),
        NodeKind::ApplicationCapture => {
            json!({ "executable": "game.exe", "processPolicy": "allVerifiedInstances" })
        }
        _ => json!({}),
    };
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports: vec![port("out", PortDirection::Output)],
    }
}

fn plain(id: &str, kind: NodeKind, ports: Vec<Port>) -> Node {
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

fn mixer(id: &str) -> Node {
    plain(
        id,
        NodeKind::Mixer,
        vec![
            port("in", PortDirection::Input),
            port("out", PortDirection::Output),
        ],
    )
}

fn output(id: &str) -> Node {
    plain(
        id,
        NodeKind::PhysicalOutput,
        vec![port("in", PortDirection::Input)],
    )
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

fn session(label: &str, nodes: Vec<Node>, edges: Vec<Edge>) -> Session {
    Session {
        id: EntityId::new(label),
        name: label.into(),
        schema_version: 1,
        revision: 1,
        nodes,
        edges,
    }
}

fn compile(session: &Session) -> Result<CompiledPathSet, String> {
    let media = HashMap::from([(
        "media-1".to_owned(),
        Arc::new(DecodedAudio {
            channels: 2,
            sample_rate_hz: 48_000,
            samples: vec![0.1; 2 * 4_800].into(),
        }),
    )]);
    compile_native_paths_with_plugins_and_audio(
        session,
        RuntimeGeneration::new(1),
        &Default::default(),
        &media,
    )
    .map_err(|error| format!("{error:?}"))
}

/// Whether a source renders its own sound in the engine (so the device-free
/// harness can hear it) rather than reading a native input. Exhaustive on
/// purpose: a new source must decide here.
fn renders_own_audio(kind: NodeKind) -> bool {
    match kind {
        NodeKind::TestSignal | NodeKind::AudioFile | NodeKind::Soundboard => true,
        NodeKind::PhysicalInput
        | NodeKind::ApplicationCapture
        | NodeKind::EndpointLoopback
        | NodeKind::VirtualRenderSource
        | NodeKind::NetworkReceive => false,
        NodeKind::PhysicalOutput
        | NodeKind::VirtualCaptureSink
        | NodeKind::NetworkSend
        | NodeKind::Mixer
        | NodeKind::Gain
        | NodeKind::Mute
        | NodeKind::Meter
        | NodeKind::ParametricEq
        | NodeKind::Compressor
        | NodeKind::Gate
        | NodeKind::Limiter
        | NodeKind::Delay
        | NodeKind::GraphicEq
        | NodeKind::Pitch
        | NodeKind::Volume
        | NodeKind::BassTreble
        | NodeKind::Dehum
        | NodeKind::Declick
        | NodeKind::Denoise
        | NodeKind::SpeechDenoise
        | NodeKind::FirFilter
        | NodeKind::SpectralGate
        | NodeKind::TimeShift
        | NodeKind::Duck
        | NodeKind::FunVoiceChanger
        | NodeKind::Recorder
        | NodeKind::Plugin
        | NodeKind::InputSwitch
        | NodeKind::OutputSwitch => false,
    }
}

/// Press Play (or trigger slot 1) on every generator the path holds, through
/// the same transport handles the control plane uses.
fn start_generators(set: &CompiledPathSet, session: &Session, label: &str) {
    for path in set.paths() {
        for node in &session.nodes {
            match node.kind {
                NodeKind::TestSignal => {
                    if let Some(source) = path.test_signal_source_for_node(&node.id) {
                        source.play();
                    }
                }
                NodeKind::AudioFile => {
                    if let Some(source) = path.audio_file_source_for_node(&node.id) {
                        source.play();
                    }
                }
                NodeKind::Soundboard => {
                    if let Some(board) = path.soundboard_for_node(&node.id) {
                        assert!(board.trigger(0), "{label}: Soundboard trigger rejected");
                    }
                }
                // Guarded by the exhaustive classifier: a new generator kind
                // fails here until it gets a start arm above.
                _ => assert!(!renders_own_audio(node.kind), "{label}: {:?}", node.kind),
            }
        }
    }
}

/// Process one silent-input quantum through `path`.
fn process_quantum(path: &CompiledMixerFanoutGraph, label: &str) -> Vec<AudioBlock> {
    let inputs = path
        .input_node_ids()
        .iter()
        .map(|_| AudioBlock::new(2, FRAMES).unwrap())
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
    .unwrap_or_else(|error| panic!("{label}: {error:?}"));
    outputs
}

fn peak(outputs: &[AudioBlock]) -> f32 {
    outputs
        .iter()
        .flat_map(|block| block.channel(0).unwrap().iter())
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

/// Compile and process a few quanta; every output must stay finite, and a
/// started generator must reach an output and its own source meter.
fn compile_and_run(session: &Session, label: &str) {
    let set = compile(session).unwrap_or_else(|error| panic!("{label}: {error}"));
    start_generators(&set, session, label);
    for quantum in 0..4 {
        for path in set.paths() {
            let outputs = process_quantum(path, &format!("{label}: quantum {quantum}"));
            for block in &outputs {
                for channel in 0..2 {
                    assert!(
                        block
                            .channel(channel)
                            .unwrap()
                            .iter()
                            .all(|sample| sample.is_finite()),
                        "{label}: non-finite output"
                    );
                }
            }
            for node in session
                .nodes
                .iter()
                .filter(|node| renders_own_audio(node.kind))
            {
                if !path.input_node_ids().contains(&node.id) {
                    continue;
                }
                let kind = node.kind;
                let meter = path
                    .meter_snapshot_for_node(&node.id)
                    .unwrap_or_else(|| panic!("{label}: no {kind:?} source meter"));
                assert!(
                    meter.peak_abs > 0.01,
                    "{label}: {kind:?} meter measured silence at quantum {quantum}"
                );
                assert_eq!(
                    meter.observed_frames,
                    ((quantum + 1) * FRAMES) as u64,
                    "{label}: {kind:?} meter must count rendered frames once"
                );
                assert!(
                    peak(&outputs) > 0.01,
                    "{label}: started {kind:?} did not reach an output"
                );
            }
        }
    }
}

/// Run `quanta` quanta of a session whose generator `a` feeds a full Mute;
/// the source meter must show the generator, the outputs must stay silent.
fn assert_meter_is_taken_before_the_tool(session: &Session, label: &str) {
    let set = compile(session).unwrap_or_else(|error| panic!("{label}: {error}"));
    start_generators(&set, session, label);
    let id = EntityId::new("a");
    let path = set
        .paths()
        .iter()
        .find(|path| path.input_node_ids().contains(&id))
        .unwrap_or_else(|| panic!("{label}: no path for the generator"));
    for quantum in 0..4 {
        let outputs = process_quantum(path, label);
        assert!(
            peak(&outputs) < 1e-6,
            "{label}: the Mute after the generator must silence the output"
        );
        let meter = path.meter_snapshot_for_node(&id).unwrap();
        assert!(
            meter.peak_abs > 0.05,
            "{label}: the source meter showed the Mute's output ({}), not the source",
            meter.peak_abs
        );
        assert_eq!(
            meter.observed_frames,
            ((quantum + 1) * FRAMES) as u64,
            "{label}: meter must count rendered frames once"
        );
    }
}

#[test]
fn the_source_list_matches_the_node_kinds() {
    let sources = sources();
    for kind in NodeKind::ALL {
        let has_no_input_port = matches!(role(kind), Role::Source);
        assert_eq!(sources.contains(&kind), has_no_input_port);
    }
    assert!(
        sources.contains(&NodeKind::Soundboard) && sources.len() >= 8,
        "{sources:?}"
    );
}

#[test]
fn every_source_reaches_a_single_route_and_a_fan_out() {
    for kind in sources() {
        let label = format!("single {kind:?}");
        compile_and_run(
            &session(
                &label,
                vec![source_node("a", kind), output("out-1")],
                vec![edge("a", "out-1")],
            ),
            &label,
        );
        let label = format!("fan-out {kind:?}");
        compile_and_run(
            &session(
                &label,
                vec![source_node("a", kind), output("out-1"), output("out-2")],
                vec![edge("a", "out-1"), edge("a", "out-2")],
            ),
            &label,
        );
    }
}

#[test]
fn every_pair_of_sources_meets_in_a_mixer() {
    for first in sources() {
        for second in sources() {
            let label = format!("mixer {first:?} + {second:?}");
            compile_and_run(
                &session(
                    &label,
                    vec![
                        source_node("a", first),
                        source_node("b", second),
                        mixer("mix-1"),
                        output("out-1"),
                    ],
                    vec![
                        edge("a", "mix-1"),
                        edge("b", "mix-1"),
                        edge("mix-1", "out-1"),
                    ],
                ),
                &label,
            );
        }
    }
}

#[test]
fn every_triple_of_sources_meets_in_connected_mixers() {
    for first in sources() {
        for second in sources() {
            for third in sources() {
                let label = format!("connected mixers {first:?} + {second:?} + {third:?}");
                compile_and_run(
                    &session(
                        &label,
                        vec![
                            source_node("a", first),
                            source_node("b", second),
                            source_node("c", third),
                            mixer("mix-1"),
                            mixer("mix-2"),
                            output("out-1"),
                        ],
                        vec![
                            edge("a", "mix-1"),
                            edge("b", "mix-1"),
                            edge("mix-1", "mix-2"),
                            edge("c", "mix-2"),
                            edge("mix-2", "out-1"),
                        ],
                    ),
                    &label,
                );
            }
        }
    }
}

/// A generator's source meter (canvas animation, Duck key) is its own sound,
/// read before the tools after it: Generator -> Mute must still show level
/// on the generator while the route is silent (2026-10-31 review).
#[test]
fn every_generator_meters_its_own_sound_before_the_next_tool() {
    let mute = || {
        plain(
            "mute-1",
            NodeKind::Mute,
            vec![
                port("in", PortDirection::Input),
                port("out", PortDirection::Output),
            ],
        )
    };
    for kind in sources()
        .into_iter()
        .filter(|kind| renders_own_audio(*kind))
    {
        let label = format!("single {kind:?} -> Mute");
        assert_meter_is_taken_before_the_tool(
            &session(
                &label,
                vec![source_node("a", kind), mute(), output("out-1")],
                vec![edge("a", "mute-1"), edge("mute-1", "out-1")],
            ),
            &label,
        );
        let label = format!("mixer {kind:?} -> Mute + PhysicalInput");
        assert_meter_is_taken_before_the_tool(
            &session(
                &label,
                vec![
                    source_node("a", kind),
                    source_node("b", NodeKind::PhysicalInput),
                    mute(),
                    mixer("mix-1"),
                    output("out-1"),
                ],
                vec![
                    edge("a", "mute-1"),
                    edge("mute-1", "mix-1"),
                    edge("b", "mix-1"),
                    edge("mix-1", "out-1"),
                ],
            ),
            &label,
        );
        let label = format!("connected mixers {kind:?} -> Mute");
        assert_meter_is_taken_before_the_tool(
            &session(
                &label,
                vec![
                    source_node("a", kind),
                    source_node("b", NodeKind::PhysicalInput),
                    source_node("c", NodeKind::PhysicalInput),
                    mute(),
                    mixer("mix-1"),
                    mixer("mix-2"),
                    output("out-1"),
                ],
                vec![
                    edge("a", "mute-1"),
                    edge("mute-1", "mix-1"),
                    edge("b", "mix-1"),
                    edge("mix-1", "mix-2"),
                    edge("c", "mix-2"),
                    edge("mix-2", "out-1"),
                ],
            ),
            &label,
        );
    }
}
