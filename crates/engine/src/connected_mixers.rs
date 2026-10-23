//! Compile connected convergence nodes into the existing bounded branch cache.
//! This module runs only on the control thread. Runtime processing remains the
//! nonblocking, preallocated output-chain loop in the parent module.
use super::*;
use audiorouter_domain::{EntityId, NodeKind, PortDirection, Session};

pub(super) fn compile(
    session: &Session,
    generation: RuntimeGeneration,
    plugins: &HashMap<EntityId, Arc<dyn RealtimePluginProcessor>>,
    audio_media: &HashMap<String, Arc<DecodedAudio>>,
) -> Result<CompiledMixerFanoutGraph, GraphCompileError> {
    let unsupported = || GraphCompileError::UnsupportedTopology;
    let incoming = |id: &EntityId| {
        session
            .edges
            .iter()
            .filter(|edge| edge.enabled && &edge.destination_node == id)
            .collect::<Vec<_>>()
    };
    let sources = session
        .nodes
        .iter()
        .filter(|node| incoming(&node.id).is_empty())
        .collect::<Vec<_>>();
    if sources.is_empty() || sources.len() > MAX_MIXER_INPUTS {
        return Err(unsupported());
    }
    let mut graph = CompiledMixerFanoutGraph {
        generation,
        // This staging stage captures each native source once. Authored
        // convergence happens below; this scratch signal is never delivered.
        mixer: MixerStage::new(
            2,
            sources
                .iter()
                .map(|node| {
                    let channels = node
                        .ports
                        .iter()
                        .find(|port| port.direction == PortDirection::Output)
                        .map_or(0, |port| usize::from(port.channels));
                    vec![0.0; 2 * channels]
                })
                .collect(),
        )
        .map_err(|_| unsupported())?,
        input_chains: sources.iter().map(|_| None).collect(),
        input_switch: None,
        processing_graph: None,
        privacy_mute: PrivacyMute::default(),
        input_node_ids: sources.iter().map(|node| node.id.clone()).collect(),
        input_meters: sources.iter().map(|_| BlockMeter::default()).collect(),
        output_matrices: Vec::new(),
        output_node_ids: Vec::new(),
        output_sources: Vec::new(),
        output_gates: Vec::new(),
        output_switches: Vec::new(),
        output_chains: Vec::new(),
        input_output_branches: Vec::new(),
        output_meters: Vec::new(),
        mixer_node_id: None,
        mixer_meter: BlockMeter::default(),
    };
    let mut prepared = HashMap::<EntityId, OutputBranchSource>::new();
    let mut inherited_gates = HashMap::<EntityId, Vec<OutputSwitchGate>>::new();
    // Output Switch node -> (state index, fails closed), and its gated side chains.
    let mut output_switch_indices = HashMap::<EntityId, (usize, bool)>::new();
    let mut side_sources = HashMap::<(EntityId, String), OutputBranchSource>::new();
    let mut pending = session.nodes.iter().collect::<Vec<_>>();
    for (index, source) in sources.iter().enumerate() {
        if !matches!(
            source.kind,
            NodeKind::PhysicalInput
                | NodeKind::ApplicationCapture
                | NodeKind::EndpointLoopback
                | NodeKind::VirtualRenderSource
                | NodeKind::TestSignal
                | NodeKind::AudioFile
                | NodeKind::NetworkReceive
        ) {
            return Err(unsupported());
        }
        let channels = source
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Output)
            .ok_or_else(unsupported)?
            .channels;
        graph.input_output_branches.push(InputOutputBranch {
            input_index: index,
            channels: usize::from(channels),
            block: RealtimeDsp::new(
                AudioBlock::new(usize::from(channels), PROCESSING_QUANTUM_FRAMES)
                    .map_err(|_| unsupported())?,
            ),
            ready: AtomicBool::new(false),
        });
        let mut source_ref = OutputBranchSource::Input(index);
        if matches!(source.kind, NodeKind::TestSignal | NodeKind::AudioFile) {
            let chain_index = graph.output_chains.len();
            graph.output_chains.push(OutputProcessorChain {
                source: source_ref,
                entry_matrix: vec![0.0; usize::from(channels).pow(2)],
                graph: compile_processor_chain(
                    session.id.as_str(),
                    &format!("cache_{chain_index}"),
                    std::slice::from_ref(*source),
                    channels,
                    generation,
                    plugins,
                    audio_media,
                )?,
                channels: usize::from(channels),
                block: RealtimeDsp::new(
                    AudioBlock::new(usize::from(channels), PROCESSING_QUANTUM_FRAMES)
                        .map_err(|_| unsupported())?,
                ),
                ready: AtomicBool::new(false),
                convergence: None,
                gate: None,
            });
            source_ref = OutputBranchSource::Chain(chain_index);
        }
        prepared.insert(source.id.clone(), source_ref);
    }
    pending.retain(|node| !prepared.contains_key(&node.id));
    // A validated DAG always makes progress. The bound is the persisted node
    // budget, not an unbounded runtime retry; cycles cannot reach the callback.
    while !pending.is_empty() {
        let position = pending
            .iter()
            .position(|node| {
                incoming(&node.id)
                    .iter()
                    .all(|edge| prepared.contains_key(&edge.source_node))
            })
            .ok_or_else(unsupported)?;
        let node = pending.remove(position);
        let feeds = incoming(&node.id);
        // Each Output Switch side is gated where it leaves the switch (its
        // side chain), so a Mixer can sum it with other feeds. A sink also
        // re-applies the gates that every path to it passes through, which
        // silences what tools after the side add (tails, plugin noise).
        let mut feed_gates = Vec::with_capacity(feeds.len());
        for edge in &feeds {
            let mut chain = inherited_gates
                .get(&edge.source_node)
                .cloned()
                .unwrap_or_default();
            if let Some(&(index, muted)) = output_switch_indices.get(&edge.source_node) {
                chain.push(OutputSwitchGate {
                    switch: index,
                    side_b: edge.source_port == "b",
                    muted,
                });
            }
            feed_gates.push(chain);
        }
        let gates = feed_gates
            .first()
            .map(|first| {
                first
                    .iter()
                    .copied()
                    .filter(|gate| feed_gates.iter().all(|other| other.contains(gate)))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let source_of = |edge: &audiorouter_domain::Edge| {
            side_sources
                .get(&(edge.source_node.clone(), edge.source_port.clone()))
                .copied()
                .unwrap_or(prepared[&edge.source_node])
        };
        let sink = matches!(
            node.kind,
            NodeKind::PhysicalOutput
                | NodeKind::VirtualCaptureSink
                | NodeKind::Recorder
                | NodeKind::NetworkSend
        );
        if sink {
            if feeds.len() != 1 || graph.output_node_ids.len() >= MAX_FANOUT_BRANCHES {
                return Err(unsupported());
            }
            if node.enabled {
                graph.output_sources.push(source_of(feeds[0]));
                graph.output_matrices.push(scaled_matrix(
                    &feeds[0].matrix,
                    if node.bypass { 0.0 } else { 1.0 },
                ));
                graph.output_node_ids.push(node.id.clone());
                graph.output_meters.push(BlockMeter::default());
                graph.output_gates.push(gates.clone());
            }
            // Sinks are not legal parents, but recording an identity here allows
            // the compile-time progress check to diagnose malformed topology.
            let parent = source_of(feeds[0]);
            prepared.insert(node.id.clone(), parent);
            inherited_gates.insert(node.id.clone(), gates);
            continue;
        }
        let input = node
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Input)
            .ok_or_else(unsupported)?;
        let output_ports = node
            .ports
            .iter()
            .filter(|port| port.direction == PortDirection::Output)
            .collect::<Vec<_>>();
        if node.kind == NodeKind::OutputSwitch {
            if feeds.len() != 1
                || node.ports.len() != 3
                || node
                    .ports
                    .iter()
                    .filter(|port| port.direction == PortDirection::Input)
                    .count()
                    != 1
                || output_ports.len() != 2
                || !output_ports.iter().any(|port| port.name == "a")
                || !output_ports.iter().any(|port| port.name == "b")
                || output_ports
                    .iter()
                    .any(|port| port.channels != input.channels)
            {
                return Err(unsupported());
            }
            let children = session
                .edges
                .iter()
                .filter(|edge| edge.enabled && edge.source_node == node.id)
                .collect::<Vec<_>>();
            if children.is_empty()
                || children
                    .iter()
                    .any(|edge| !matches!(edge.source_port.as_str(), "a" | "b"))
            {
                return Err(unsupported());
            }
            let state_index = graph.output_switches.len();
            graph.output_switches.push(OutputSwitchState::new(
                node.id.clone(),
                node.parameters
                    .get("selected")
                    .and_then(|value| value.as_str())
                    == Some("b"),
            ));
            // A disabled or bypassed routing node fails closed on both sides.
            let muted = !node.enabled || node.bypass;
            let parent = source_of(feeds[0]);
            let mut sides = Vec::new();
            for side in ["a", "b"] {
                if !children.iter().any(|edge| edge.source_port == side) {
                    continue;
                }
                let index = graph.output_chains.len();
                graph.output_chains.push(OutputProcessorChain {
                    source: parent,
                    entry_matrix: feeds[0].matrix.clone(),
                    graph: compile_processor_chain(
                        session.id.as_str(),
                        &format!("output_switch_{}_{side}", node.id.as_str()),
                        &[],
                        input.channels,
                        generation,
                        plugins,
                        audio_media,
                    )?,
                    channels: usize::from(input.channels),
                    block: RealtimeDsp::new(
                        AudioBlock::new(usize::from(input.channels), PROCESSING_QUANTUM_FRAMES)
                            .map_err(|_| unsupported())?,
                    ),
                    ready: AtomicBool::new(false),
                    convergence: None,
                    gate: Some(OutputSwitchGate {
                        switch: state_index,
                        side_b: side == "b",
                        muted,
                    }),
                });
                sides.push((side, index));
            }
            output_switch_indices.insert(node.id.clone(), (state_index, muted));
            for (side, index) in sides {
                side_sources.insert(
                    (node.id.clone(), side.to_owned()),
                    OutputBranchSource::Chain(index),
                );
            }
            // Only the side chains are read; this entry keeps the progress
            // check able to order the switch's children.
            prepared.insert(node.id.clone(), parent);
            inherited_gates.insert(node.id.clone(), gates);
            continue;
        }
        let output = output_ports.first().copied().ok_or_else(unsupported)?;
        if input.channels != output.channels {
            return Err(unsupported());
        }
        let index = graph.output_chains.len();
        let convergence = if matches!(node.kind, NodeKind::Mixer | NodeKind::InputSwitch) {
            if feeds.is_empty() || feeds.len() > MAX_MIXER_INPUTS {
                return Err(unsupported());
            }
            let is_switch = node.kind == NodeKind::InputSwitch;
            if is_switch
                && (feeds.len() != 2
                    || !feeds.iter().any(|edge| edge.destination_port == "a")
                    || !feeds.iter().any(|edge| edge.destination_port == "b"))
            {
                return Err(unsupported());
            }
            Some(CachedConvergence {
                node_id: node.id.clone(),
                sources: feeds.iter().map(|edge| source_of(edge)).collect(),
                mixer: MixerStage::new(
                    usize::from(output.channels),
                    feeds
                        .iter()
                        .map(|edge| {
                            let gain = if !node.enabled || node.bypass {
                                0.0
                            } else if is_switch {
                                1.0
                            } else {
                                audiorouter_domain::mixer_input_volume(node, &edge.source_node)
                            };
                            scaled_matrix(&edge.matrix, gain)
                        })
                        .collect(),
                )
                .map_err(|_| unsupported())?,
                switch: is_switch.then(|| {
                    InputSwitchState::new(
                        node.parameters
                            .get("selected")
                            .and_then(|value| value.as_str())
                            == Some("b"),
                        if node.parameters.get("fade").and_then(|value| value.as_str())
                            == Some("slow")
                        {
                            2.0
                        } else {
                            0.5
                        },
                        feeds
                            .iter()
                            .map(|edge| edge.destination_port == "b")
                            .collect(),
                    )
                }),
                meter: BlockMeter::default(),
            })
        } else {
            if feeds.len() != 1 || !is_chain_processor(node.kind) {
                return Err(unsupported());
            }
            None
        };
        let runtime = compile_processor_chain(
            session.id.as_str(),
            &format!("cache_{index}"),
            if convergence.is_some() {
                &[]
            } else {
                std::slice::from_ref(node)
            },
            input.channels,
            generation,
            plugins,
            audio_media,
        )?;
        graph.output_chains.push(OutputProcessorChain {
            source: source_of(feeds[0]),
            entry_matrix: feeds[0].matrix.clone(),
            graph: runtime,
            channels: usize::from(output.channels),
            block: RealtimeDsp::new(
                AudioBlock::new(usize::from(output.channels), PROCESSING_QUANTUM_FRAMES)
                    .map_err(|_| unsupported())?,
            ),
            ready: AtomicBool::new(false),
            convergence,
            gate: None,
        });
        prepared.insert(node.id.clone(), OutputBranchSource::Chain(index));
        inherited_gates.insert(node.id.clone(), gates);
    }
    if graph.output_node_ids.is_empty() {
        return Err(unsupported());
    }
    graph.link_node_levels(session.id.as_str());
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_domain::{Edge, Node, Port};

    fn fixture() -> Session {
        let node = |id: &str, kind: NodeKind, width: u8| Node {
            id: EntityId::new(id),
            name: id.into(),
            kind,
            type_version: 1,
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: [("in", PortDirection::Input), ("out", PortDirection::Output)]
                .into_iter()
                .filter(|(_, direction)| match kind {
                    NodeKind::PhysicalInput | NodeKind::ApplicationCapture => {
                        *direction == PortDirection::Output
                    }
                    NodeKind::PhysicalOutput => *direction == PortDirection::Input,
                    _ => true,
                })
                .map(|(name, direction)| Port {
                    name: name.into(),
                    direction,
                    channels: width,
                })
                .collect(),
        };
        let mut nodes = vec![
            node("mic", NodeKind::PhysicalInput, 1),
            node("game", NodeKind::PhysicalInput, 2),
            node("discord", NodeKind::ApplicationCapture, 2),
            node("voice-eq", NodeKind::Gain, 2),
            node("voice-compressor", NodeKind::Compressor, 2),
            node("voice-gate", NodeKind::Gate, 2),
            node("voice-meter", NodeKind::Meter, 2),
            node("game-eq", NodeKind::Gain, 2),
            node("game-mix", NodeKind::Mixer, 2),
            node("monitor-mix", NodeKind::Mixer, 2),
            node("voice-output", NodeKind::PhysicalOutput, 2),
            node("game-output", NodeKind::PhysicalOutput, 2),
            node("monitor", NodeKind::PhysicalOutput, 2),
        ];
        // Dynamics stay in the route with deliberate dry bypass; signal math
        // tests do not depend on attack/release settling or detector thresholds.
        for node in &mut nodes {
            if matches!(node.kind, NodeKind::Compressor | NodeKind::Gate) {
                node.bypass = true;
            }
            if node.kind == NodeKind::ApplicationCapture {
                node.parameters = serde_json::from_value(serde_json::json!({
                    "executable": "Discord.exe", "processPolicy": "selectedInstance",
                    "processId": 1, "creationTime100ns": "1"
                }))
                .unwrap();
            }
        }
        let edges = [
            ("mic", "voice-eq"),
            ("voice-eq", "voice-compressor"),
            ("voice-compressor", "voice-gate"),
            ("voice-gate", "voice-output"),
            ("voice-gate", "voice-meter"),
            ("voice-meter", "monitor-mix"),
            ("game", "game-eq"),
            ("game-eq", "game-mix"),
            ("discord", "game-mix"),
            ("game-mix", "game-output"),
            ("game-mix", "monitor-mix"),
            ("monitor-mix", "monitor"),
        ]
        .into_iter()
        .map(|(source, destination)| Edge {
            id: EntityId::new(format!("{source}-{destination}")),
            source_node: EntityId::new(source),
            source_port: "out".into(),
            destination_node: EntityId::new(destination),
            destination_port: "in".into(),
            matrix: if source == "mic" {
                vec![1.0, 1.0]
            } else {
                vec![1.0, 0.0, 0.0, 1.0]
            },
            enabled: true,
        })
        .collect();
        Session {
            id: EntityId::new("connected-test"),
            name: "Connected Mixers".into(),
            schema_version: 1,
            revision: 214,
            nodes,
            edges,
        }
    }

    fn compile_fixture(session: &Session) -> CompiledPathSet {
        compile_native_paths_with_plugins_and_audio(
            session,
            RuntimeGeneration::new(5),
            &Default::default(),
            &Default::default(),
        )
        .unwrap()
    }

    fn input_blocks(graph: &CompiledMixerFanoutGraph) -> Vec<AudioBlock> {
        graph
            .input_node_ids
            .iter()
            .map(|id| {
                let (channels, value) = match id.as_str() {
                    "mic" => (1, 0.1),
                    "game" => (2, 0.2),
                    "discord" => (2, 0.3),
                    _ => unreachable!(),
                };
                let mut block = AudioBlock::new(channels, PROCESSING_QUANTUM_FRAMES).unwrap();
                for channel in 0..channels {
                    block.channel_mut(channel).unwrap().fill(value);
                }
                block
            })
            .collect()
    }

    fn run(graph: &CompiledMixerFanoutGraph) -> HashMap<String, f32> {
        let inputs = input_blocks(graph);
        let mut scratch = AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap();
        let mut outputs = graph
            .output_node_ids
            .iter()
            .map(|_| AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap())
            .collect::<Vec<_>>();
        graph
            .process(
                &inputs,
                &mut scratch,
                &mut outputs.iter_mut().collect::<Vec<_>>(),
            )
            .unwrap();
        graph
            .output_node_ids
            .iter()
            .zip(outputs)
            .map(|(id, block)| (id.as_str().to_owned(), block.channel(0).unwrap()[0]))
            .collect()
    }

    #[test]
    fn output_switch_routes_connected_mixer_convergence_to_one_destination() {
        let mut session = fixture();
        let switch_id = EntityId::new("output-switch");
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        session.nodes.push(Node {
            id: switch_id.clone(),
            name: "Output Switch".into(),
            kind: NodeKind::OutputSwitch,
            type_version: 1,
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(serde_json::json!({ "selected": "a" })).unwrap(),
            ports: vec![
                port("in", PortDirection::Input),
                port("a", PortDirection::Output),
                port("b", PortDirection::Output),
            ],
        });
        let mut obs = session
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "monitor")
            .unwrap()
            .clone();
        obs.id = EntityId::new("obs-monitor");
        obs.name = "OBS monitor".into();
        session.nodes.push(obs);
        let monitor_edge = session
            .edges
            .iter_mut()
            .find(|edge| edge.id.as_str() == "monitor-mix-monitor")
            .unwrap();
        monitor_edge.destination_node = switch_id.clone();
        monitor_edge.destination_port = "in".into();
        for side in ["a", "b"] {
            session.edges.push(Edge {
                id: EntityId::new(format!("output-switch-{side}")),
                source_node: switch_id.clone(),
                source_port: side.into(),
                destination_node: EntityId::new(if side == "a" {
                    "monitor"
                } else {
                    "obs-monitor"
                }),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            });
        }

        let set = compile_fixture(&session);
        assert_eq!(set.path_count(), 1);
        let output = run(&set.paths()[0]);
        assert!((output["monitor"] - 0.6).abs() < 1e-6);
        assert!(output["obs-monitor"].abs() < 1e-6);
    }

    fn output_switch_node(id: &EntityId, selected: &str) -> Node {
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        Node {
            id: id.clone(),
            name: "Output Switch".into(),
            kind: NodeKind::OutputSwitch,
            type_version: 1,
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(serde_json::json!({ "selected": selected }))
                .unwrap(),
            ports: vec![
                port("in", PortDirection::Input),
                port("a", PortDirection::Output),
                port("b", PortDirection::Output),
            ],
        }
    }

    fn stereo_edge(id: &str, source: &str, port: &str, destination: &str) -> Edge {
        Edge {
            id: EntityId::new(id),
            source_node: EntityId::new(source),
            source_port: port.into(),
            destination_node: EntityId::new(destination),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        }
    }

    #[test]
    fn output_switch_side_mixes_with_other_feeds_and_gates_tools_after_it() {
        // game-eq -> switch; a -> obs-out, b -> side-meter -> game-mix, which
        // also mixes Discord. The unselected side must be silent at the Mixer
        // and at the Meter, and Discord must keep its level in both positions.
        for selected in ["a", "b"] {
            let mut session = fixture();
            let switch_id = EntityId::new("output-switch");
            session.nodes.push(output_switch_node(&switch_id, selected));
            let feed = session
                .edges
                .iter_mut()
                .find(|edge| edge.id.as_str() == "game-eq-game-mix")
                .unwrap();
            feed.destination_node = switch_id.clone();
            let template = session
                .nodes
                .iter()
                .find(|node| node.id.as_str() == "voice-meter")
                .unwrap()
                .clone();
            session.nodes.push(Node {
                id: EntityId::new("side-meter"),
                ..template
            });
            let mut obs = session
                .nodes
                .iter()
                .find(|node| node.id.as_str() == "game-output")
                .unwrap()
                .clone();
            obs.id = EntityId::new("obs-out");
            session.nodes.push(obs);
            session.edges.extend([
                stereo_edge("switch-a", "output-switch", "a", "obs-out"),
                stereo_edge("switch-b", "output-switch", "b", "side-meter"),
                stereo_edge("side-meter-game-mix", "side-meter", "out", "game-mix"),
            ]);

            let set = compile_fixture(&session);
            assert_eq!(set.path_count(), 1);
            let graph = &set.paths()[0];
            let output = run(graph);
            let game = if selected == "b" { 0.2 } else { 0.0 };
            assert!(
                (output["obs-out"] - (0.2 - game)).abs() < 1e-6,
                "{selected}"
            );
            assert!(
                (output["game-output"] - (0.3 + game)).abs() < 1e-6,
                "{selected}"
            );
            assert!(
                (output["monitor"] - (0.4 + game)).abs() < 1e-6,
                "{selected}"
            );
            assert!((output["voice-output"] - 0.1).abs() < 1e-6, "{selected}");
            let meter = graph
                .meter_snapshot_for_node(&EntityId::new("side-meter"))
                .unwrap();
            assert!((meter.peak_abs - game).abs() < 1e-6, "{selected}");
        }
    }

    #[test]
    fn output_switch_gates_a_meter_on_the_unselected_side_of_a_single_chain() {
        // mic -> switch; a -> voice-output, b -> side-meter -> game-output.
        let mut session = fixture();
        session.nodes.retain(|node| {
            matches!(
                node.id.as_str(),
                "mic" | "voice-meter" | "voice-output" | "game-output"
            )
        });
        session.edges.clear();
        let switch_id = EntityId::new("output-switch");
        session.nodes.push(output_switch_node(&switch_id, "a"));
        let mic = session
            .nodes
            .iter_mut()
            .find(|node| node.id.as_str() == "mic")
            .unwrap();
        mic.ports[0].channels = 2;
        session.edges.extend([
            stereo_edge("mic-switch", "mic", "out", "output-switch"),
            stereo_edge("switch-a", "output-switch", "a", "voice-output"),
            stereo_edge("switch-b", "output-switch", "b", "voice-meter"),
            stereo_edge("meter-out", "voice-meter", "out", "game-output"),
        ]);
        let set = compile_fixture(&session);
        let graph = &set.paths()[0];
        let mut input = AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap();
        for channel in 0..2 {
            input.channel_mut(channel).unwrap().fill(0.25);
        }
        let mut scratch = AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap();
        let mut outputs = graph
            .output_node_ids
            .iter()
            .map(|_| AudioBlock::new(2, PROCESSING_QUANTUM_FRAMES).unwrap())
            .collect::<Vec<_>>();
        graph
            .process(
                std::slice::from_ref(&input),
                &mut scratch,
                &mut outputs.iter_mut().collect::<Vec<_>>(),
            )
            .unwrap();
        let level = |id: &str| {
            let index = graph
                .output_node_ids
                .iter()
                .position(|node| node.as_str() == id)
                .unwrap();
            outputs[index].channel(0).unwrap()[0]
        };
        assert!((level("voice-output") - 0.25).abs() < 1e-6);
        assert!(level("game-output").abs() < 1e-6);
        let meter = graph
            .meter_snapshot_for_node(&EntityId::new("voice-meter"))
            .unwrap();
        assert_eq!(meter.observed_frames, PROCESSING_QUANTUM_FRAMES as u64);
        assert!(meter.peak_abs < 1e-6, "{}", meter.peak_abs);
    }

    #[test]
    fn connected_mixers_preserve_three_source_branch_isolation_and_volumes() {
        for volume in [30.0, 100.0, 30.0] {
            let mut session = fixture();
            session
                .nodes
                .iter_mut()
                .find(|node| node.id.as_str() == "game-mix")
                .unwrap()
                .parameters
                .insert("inputVolume:game-eq".into(), serde_json::json!(volume));
            let set = compile_fixture(&session);
            assert_eq!(set.path_count(), 1);
            assert_eq!(set.input_node_ids().len(), 3);
            let graph = &set.paths[0];
            let output = run(graph);
            assert!((output["voice-output"] - 0.1).abs() < 1e-6);
            let game = 0.3 + 0.2 * volume as f32 / 100.0;
            assert!((output["game-output"] - game).abs() < 1e-6);
            assert!((output["monitor"] - (0.1 + game)).abs() < 1e-6);
            for id in [
                "game-mix",
                "monitor-mix",
                "voice-meter",
                "voice-eq",
                "voice-gate",
            ] {
                assert_eq!(
                    graph
                        .meter_snapshot_for_node(&EntityId::new(id))
                        .unwrap()
                        .observed_frames,
                    PROCESSING_QUANTUM_FRAMES as u64
                );
            }
            assert!(graph.reset_meter_for_node(&EntityId::new("voice-meter")));
        }
    }

    #[test]
    fn connected_mixers_pass_a_bypassed_duck_dry() {
        // Regression: bypassing a Duck between two Mixers was rejected as an
        // unsupported path, so the live toggle failed and Play refused the route.
        for bypass in [false, true] {
            let mut session = fixture();
            let duck = session
                .nodes
                .iter_mut()
                .find(|node| node.id.as_str() == "game-eq")
                .unwrap();
            duck.kind = NodeKind::Duck;
            duck.bypass = bypass;
            duck.parameters = serde_json::from_value(
                serde_json::json!({ "trigger": "siegeRound", "amountDb": 20.0 }),
            )
            .unwrap();
            let set = compile_fixture(&session);
            let output = run(&set.paths[0]);
            // No round signal is published here, so an active Duck is released too.
            assert!(
                (output["game-output"] - 0.5).abs() < 1e-4,
                "bypass {bypass}: {}",
                output["game-output"]
            );
        }
    }

    #[test]
    fn connected_mixers_flags_silence_only_authored_contributions() {
        for disabled in [false, true] {
            let mut session = fixture();
            let mixer = session
                .nodes
                .iter_mut()
                .find(|node| node.id.as_str() == "game-mix")
                .unwrap();
            mixer.enabled = !disabled;
            mixer.bypass = !disabled;
            let set = compile_fixture(&session);
            let output = run(&set.paths[0]);
            assert_eq!(output["game-output"], 0.0);
            assert!((output["voice-output"] - 0.1).abs() < 1e-6);
            assert!((output["monitor"] - 0.1).abs() < 1e-6);
        }
        let mut session = fixture();
        session
            .nodes
            .iter_mut()
            .find(|node| node.id.as_str() == "mic")
            .unwrap()
            .enabled = false;
        let set = compile_fixture(&session);
        assert_eq!(set.input_node_ids().len(), 2);
        let output = run(&set.paths[0]);
        assert!((output["monitor"] - 0.5).abs() < 1e-6);
        assert!(!output.contains_key("voice-output"));
    }

    #[test]
    fn connected_mixers_clear_busy_cache_without_replaying_previous_microphone_audio() {
        let set = compile_fixture(&fixture());
        let graph = &set.paths[0];
        assert!(run(graph)["voice-output"] > 0.0);
        let chain = graph
            .output_chains
            .iter()
            .find(|chain| {
                chain
                    .graph
                    .meter_snapshot_for_node(&EntityId::new("voice-gate"))
                    .is_some()
            })
            .unwrap();
        chain
            .block
            .try_with(|_| {
                let output = run(graph);
                assert_eq!(output["voice-output"], 0.0);
                assert!((output["game-output"] - 0.5).abs() < 1e-6);
                assert!((output["monitor"] - 0.5).abs() < 1e-6);
            })
            .unwrap();
    }

    #[test]
    fn connected_mixers_keep_prepared_identity_for_live_parameter_edits() {
        let mut session = fixture();
        let set = compile_fixture(&session);
        let mut runtime =
            RealtimeMixerFanout::from_paths(set, 4, &[1, 2, 2], PROCESSING_QUANTUM_FRAMES).unwrap();
        session
            .nodes
            .iter_mut()
            .find(|node| node.id.as_str() == "game-mix")
            .unwrap()
            .parameters
            .insert("inputVolume:game-eq".into(), serde_json::json!(30));
        runtime.replace_paths(compile_fixture(&session)).unwrap();
        runtime.set_privacy_muted(true);
        assert_eq!(
            run(&runtime.paths[0].graph).values().copied().sum::<f32>(),
            0.0
        );
        runtime.set_privacy_muted(false);
        assert!((run(&runtime.paths[0].graph)["monitor"] - 0.46).abs() < 1e-6);
    }

    #[test]
    fn connected_mixers_generator_transport_remains_discoverable() {
        let mut session = fixture();
        let source = session
            .nodes
            .iter_mut()
            .find(|node| node.id.as_str() == "discord")
            .unwrap();
        source.kind = NodeKind::TestSignal;
        source.parameters = Default::default();
        let set = compile_fixture(&session);
        let graph = &set.paths[0];
        let signal = graph
            .test_signal_source_for_node(&EntityId::new("discord"))
            .unwrap();
        assert!(!signal.is_playing());
        signal.play();
        assert!(signal.is_playing());
        let output = run(graph);
        assert!((output["monitor"] - output["game-output"] - 0.1).abs() < 1e-6);
    }

    #[test]
    #[ignore = "explicit private session JSON; compile only, never opens devices"]
    fn saved_connected_session_compiles_without_changing_user_data() {
        let path =
            std::env::var_os("AUDIOROUTER_COMPILE_SESSION_JSON").expect("explicit session copy");
        let session: Session = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let set = compile_fixture(&session);
        assert_eq!(set.input_node_ids().len(), 3);
        assert_eq!(set.output_node_ids().len(), 3);
        assert_eq!(set.path_count(), 1);
        eprintln!(
            "Saved revision {} compiled: 3 exact sources, 3 destinations, 1 component",
            session.revision
        );
    }
}
