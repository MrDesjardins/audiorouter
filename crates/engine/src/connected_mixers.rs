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
        output_chains: Vec::new(),
        input_output_branches: Vec::new(),
        output_meters: Vec::new(),
        mixer_node_id: None,
        mixer_meter: BlockMeter::default(),
    };
    let mut prepared = HashMap::<EntityId, OutputBranchSource>::new();
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
                graph.output_sources.push(prepared[&feeds[0].source_node]);
                graph.output_matrices.push(scaled_matrix(
                    &feeds[0].matrix,
                    if node.bypass { 0.0 } else { 1.0 },
                ));
                graph.output_node_ids.push(node.id.clone());
                graph.output_meters.push(BlockMeter::default());
            }
            // Sinks are not legal parents, but recording an identity here allows
            // the compile-time progress check to diagnose malformed topology.
            prepared.insert(node.id.clone(), prepared[&feeds[0].source_node]);
            continue;
        }
        let input = node
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Input)
            .ok_or_else(unsupported)?;
        let output = node
            .ports
            .iter()
            .find(|port| port.direction == PortDirection::Output)
            .ok_or_else(unsupported)?;
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
                sources: feeds
                    .iter()
                    .map(|edge| prepared[&edge.source_node])
                    .collect(),
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
            source: prepared[&feeds[0].source_node],
            entry_matrix: feeds[0].matrix.clone(),
            graph: runtime,
            channels: usize::from(output.channels),
            block: RealtimeDsp::new(
                AudioBlock::new(usize::from(output.channels), PROCESSING_QUANTUM_FRAMES)
                    .map_err(|_| unsupported())?,
            ),
            ready: AtomicBool::new(false),
            convergence,
        });
        prepared.insert(node.id.clone(), OutputBranchSource::Chain(index));
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
