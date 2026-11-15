//! Tests for `plugins.rs`.

use super::*;
use crate::test_support::*;

fn sidechain_request(plane: &mut ControlPlane, session_id: &str, node_id: &str) -> JsonRpcResponse {
    plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.addSidechainPort".into(),
        params: Some(json!({ "sessionId": session_id, "nodeId": node_id })),
    })
}

#[test]
fn add_sidechain_port_refuses_unsupported_nodes_without_changing_the_session() {
    use audiorouter_domain::Port;
    let mut plane = ControlPlane::default();
    let mut original = session();
    original.nodes.truncate(1);
    original.edges.clear();
    let node_id = original.nodes[0].id.clone();
    let plugin_ports = || {
        vec![
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
        ]
    };
    // 1. Not a plugin node.
    plane.insert_session(original.clone()).unwrap();
    let reply = sidechain_request(&mut plane, original.id.as_str(), node_id.as_str());
    let message = reply.error.expect("a non-plugin node is refused").message;
    assert!(message.contains("only a plugin node"), "{message}");
    // 2. A VST2 plugin has no declared auxiliary bus in this path.
    let mut vst2 = original.clone();
    vst2.id = EntityId::new("vst2-session");
    vst2.nodes[0].kind = NodeKind::Plugin;
    vst2.nodes[0].ports = plugin_ports();
    vst2.nodes[0].parameters =
        serde_json::from_value(json!({ "path": "C:\\VST2\\x.dll", "format": "vst2" })).unwrap();
    plane.insert_session(vst2.clone()).unwrap();
    let reply = sidechain_request(&mut plane, vst2.id.as_str(), node_id.as_str());
    let message = reply.error.expect("VST2 is refused").message;
    assert!(message.contains("VST3"), "{message}");
    // 3. A VST3 plugin whose class was never chosen.
    let mut unclassed = vst2.clone();
    unclassed.id = EntityId::new("vst3-unclassed");
    unclassed.nodes[0].parameters = serde_json::from_value(
        json!({ "path": "C:\\VST3\\x.vst3", "format": "vst3", "classId": "default" }),
    )
    .unwrap();
    plane.insert_session(unclassed.clone()).unwrap();
    let reply = sidechain_request(&mut plane, unclassed.id.as_str(), node_id.as_str());
    let message = reply.error.expect("no class is refused").message;
    assert!(message.contains("exact VST3 class"), "{message}");
    // 4. An unknown node, and no explicit scan for a real-looking plugin.
    let reply = sidechain_request(&mut plane, original.id.as_str(), "missing");
    assert!(reply.error.is_some());
    let mut unscanned = unclassed.clone();
    unscanned.id = EntityId::new("vst3-unscanned");
    unscanned.nodes[0]
        .parameters
        .insert("classId".into(), json!("41347FD6FED64094AFBB12B7DBA1D441"));
    plane.insert_session(unscanned.clone()).unwrap();
    let reply = sidechain_request(&mut plane, unscanned.id.as_str(), node_id.as_str());
    assert!(reply.error.is_some(), "an unscanned plugin is refused");
    for before in [&original, &vst2, &unclassed, &unscanned] {
        assert_eq!(plane.get_session(&before.id).unwrap(), before);
    }
}

#[test]
fn remove_sidechain_port_refuses_a_used_port_then_removes_a_free_one() {
    use audiorouter_domain::{Edge, Port};
    let mut plane = ControlPlane::default();
    let mut original = session();
    original.nodes.truncate(2);
    original.edges.clear();
    let source_id = original.nodes[0].id.clone();
    let port = |name: &str, direction, channels| Port {
        name: name.into(),
        direction,
        channels,
    };
    original.nodes[0].ports = vec![port("out", PortDirection::Output, 1)];
    let plugin = &mut original.nodes[1];
    plugin.kind = NodeKind::Plugin;
    plugin.parameters = serde_json::from_value(json!({
        "path": "C:\\VST3\\x.vst3", "format": "vst3",
        "classId": "41347FD6FED64094AFBB12B7DBA1D441", "fingerprint": "a".repeat(64),
    }))
    .unwrap();
    plugin.ports = vec![
        port("in", PortDirection::Input, 2),
        port("out", PortDirection::Output, 2),
        port("sidechain", PortDirection::Input, 1),
    ];
    let plugin_id = plugin.id.clone();
    // A commit re-checks every plugin against a current scan result.
    plane.plugin_inventories.insert(
        "C:\\VST3".into(),
        json!({ "entries": [{
            "path": "C:\\VST3\\x.vst3",
            "identity": { "format": "vst3", "sha256": "a".repeat(64) },
        }] }),
    );
    original.edges.push(Edge {
        id: EntityId::new("key-edge"),
        source_node: source_id,
        source_port: "out".into(),
        destination_node: plugin_id.clone(),
        destination_port: "sidechain".into(),
        matrix: vec![1.0],
        enabled: true,
    });
    plane.insert_session(original.clone()).unwrap();
    let remove = |plane: &mut ControlPlane| {
        plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "plugins.removeSidechainPort".into(),
            params: Some(json!({ "sessionId": original.id, "nodeId": plugin_id })),
        })
    };
    // A used port is refused and nothing changes.
    let refused = remove(&mut plane);
    let message = refused.error.expect("a used port is refused").message;
    assert!(message.contains("disconnect it first"), "{message}");
    assert_eq!(plane.get_session(&original.id).unwrap(), &original);
    // Without the connection it is removed in one revision.
    let mut free = original.clone();
    free.id = EntityId::new("free-sidechain");
    free.edges.clear();
    plane.insert_session(free.clone()).unwrap();
    let reply = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "plugins.removeSidechainPort".into(),
        params: Some(json!({ "sessionId": free.id, "nodeId": plugin_id })),
    });
    assert!(reply.error.is_none(), "{:?}", reply.error);
    let changed = plane.get_session(&free.id).unwrap();
    assert_eq!(changed.revision, 1);
    assert!(changed.nodes[1].plugin_sidechain_port().is_none());
    // Nothing left to remove.
    let again = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "plugins.removeSidechainPort".into(),
        params: Some(json!({ "sessionId": free.id, "nodeId": plugin_id })),
    });
    assert!(again.error.unwrap().message.contains("no sidechain port"));
}

#[cfg(windows)]
#[test]
#[ignore = "requires SDK AGain and a rebuilt native worker override"]
fn add_sidechain_port_adds_the_real_again_auxiliary_input_once_and_undoes() {
    use audiorouter_domain::Port;
    let path = std::path::PathBuf::from(std::env::var("AUDIOROUTER_VST3_FIXTURE").unwrap());
    let root = path.parent().unwrap().to_path_buf();
    let identity =
        audiorouter_plugin_host::inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
    let mut plane = ControlPlane::default();
    let scan = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.scan".into(),
        params: Some(json!({ "directory": root })),
    });
    assert!(scan.error.is_none(), "{:?}", scan.error);
    let mut original = session();
    original.nodes.truncate(1);
    original.edges.clear();
    let node = &mut original.nodes[0];
    node.kind = NodeKind::Plugin;
    node.parameters = serde_json::from_value(json!({
        "path": path, "format": "vst3",
        "classId": "41347FD6FED64094AFBB12B7DBA1D441", "fingerprint": identity.sha256,
    }))
    .unwrap();
    node.ports = vec![
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
    ];
    let node_id = node.id.clone();
    plane.insert_session(original.clone()).unwrap();
    let reply = sidechain_request(&mut plane, original.id.as_str(), node_id.as_str());
    assert!(reply.error.is_none(), "{:?}", reply.error);
    let result = reply.result.unwrap();
    assert_eq!(result["port"], "sidechain");
    assert_eq!(result["channels"], 2);
    let changed = plane.get_session(&original.id).unwrap().clone();
    assert_eq!(changed.revision, 1);
    assert_eq!(changed.edges.len(), 0, "no connection is created");
    assert_eq!(changed.nodes.len(), 1, "no Mixer is created");
    assert!(changed.nodes[0].plugin_sidechain_port().is_some());
    // A second call is refused or leaves the same single port.
    let again = sidechain_request(&mut plane, original.id.as_str(), node_id.as_str());
    let after = plane.get_session(&original.id).unwrap();
    assert_eq!(
        after.nodes[0]
            .ports
            .iter()
            .filter(|port| port.name == "sidechain")
            .count(),
        1,
        "{again:?}"
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires SDK AGain and a rebuilt native worker override"]
fn plugins_buses_reports_the_real_again_auxiliary_input() {
    let path = std::path::PathBuf::from(std::env::var("AUDIOROUTER_VST3_FIXTURE").unwrap());
    let root = path.parent().unwrap().to_path_buf();
    let mut plane = ControlPlane::default();
    let scan = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.scan".into(),
        params: Some(json!({ "directory": root })),
    });
    assert!(scan.error.is_none(), "{:?}", scan.error);
    let buses = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "plugins.buses".into(),
        params: Some(json!({
            "path": path,
            "classId": "41347FD6FED64094AFBB12B7DBA1D441",
        })),
    });
    assert!(buses.error.is_none(), "{:?}", buses.error);
    let buses = buses.result.unwrap();
    assert_eq!(buses["format"], "vst3");
    assert_eq!(buses["buses"]["inputs"].as_array().unwrap().len(), 2);
    assert_eq!(buses["buses"]["inputs"][0]["role"], "main");
    assert_eq!(buses["buses"]["inputs"][1]["role"], "auxiliary");
    assert_eq!(buses["buses"]["inputs"][1]["nativeIndex"], 1);
    assert_eq!(buses["buses"]["outputs"][0]["role"], "main");
}

#[cfg(windows)]
#[test]
#[ignore = "requires SDK AGain and rebuilt native worker overrides"]
fn backend_prepares_a_verified_sidechain_for_the_real_compiled_route() {
    use audiorouter_domain::{Edge, Node, Port};
    let path = std::path::PathBuf::from(std::env::var("AUDIOROUTER_VST3_FIXTURE").unwrap());
    let root = path.parent().unwrap().to_path_buf();
    let identity =
        audiorouter_plugin_host::inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
    let make = |id: &str, kind, widths: &[(&str, PortDirection, u8)]| Node {
        id: EntityId::new(id),
        name: id.into(),
        kind,
        type_version: 1,
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: widths
            .iter()
            .map(|(name, direction, channels)| Port {
                name: (*name).into(),
                direction: *direction,
                channels: *channels,
            })
            .collect(),
    };
    let mut plugin = make(
        "effect",
        NodeKind::Plugin,
        &[
            ("sidechain", PortDirection::Input, 1),
            ("in", PortDirection::Input, 2),
            ("out", PortDirection::Output, 2),
        ],
    );
    plugin.parameters = serde_json::from_value(json!({ "path": path, "format": "vst3", "classId": "41347FD6FED64094AFBB12B7DBA1D441", "fingerprint": identity.sha256 })).unwrap();
    let edge = |id: &str, from: &str, to: &str, port: &str, matrix: Vec<f32>| Edge {
        id: EntityId::new(id),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: port.into(),
        matrix,
        enabled: true,
    };
    let mut session = Session {
        id: EntityId::new("prepared-sidechain"),
        name: "prepared sidechain".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            make(
                "main",
                NodeKind::PhysicalInput,
                &[("out", PortDirection::Output, 2)],
            ),
            make(
                "key",
                NodeKind::PhysicalInput,
                &[("out", PortDirection::Output, 1)],
            ),
            plugin,
            make(
                "sink",
                NodeKind::PhysicalOutput,
                &[("in", PortDirection::Input, 2)],
            ),
        ],
        edges: vec![
            edge("main-in", "main", "effect", "in", vec![1.0, 0.0, 0.0, 1.0]),
            edge("key-in", "key", "effect", "sidechain", vec![1.0]),
            edge("output", "effect", "sink", "in", vec![1.0, 0.0, 0.0, 1.0]),
        ],
    };
    let request = || PluginStageRequest {
        node_id: EntityId::new("effect"),
        scanned_path: path.clone(),
        configured_root: root.clone(),
        fingerprint: identity.sha256.clone(),
        channels: 2,
        state: None,
        parameters: vec![audiorouter_plugin_host::ParameterEvent::new(0, 0.75, 0).unwrap()],
    };
    // Same binary, wrong class: discovery must refuse the absent aux input.
    session.nodes[2]
        .parameters
        .insert("classId".into(), json!("84E8DE5F92554F5396FAE4133C935A18"));
    assert!(start_plugin_bridges(&session, vec![request()], 48_000).is_err());
    session.nodes[2]
        .parameters
        .insert("classId".into(), json!("41347FD6FED64094AFBB12B7DBA1D441"));
    let prepared = start_plugin_bridges(&session, vec![request()], 48_000).unwrap();
    let processors = prepared
        .into_iter()
        .map(|(id, bridge)| (id, bridge.processor()))
        .collect();
    let set = audiorouter_engine::compile_native_paths_with_plugins_and_audio(
        &session,
        RuntimeGeneration::new(2),
        &processors,
        &HashMap::new(),
    )
    .unwrap();
    let graph = &set.paths()[0];
    let inputs = graph
        .input_node_ids()
        .iter()
        .map(|id| {
            let mut block = AudioBlock::new(
                if id.as_str() == "main" { 2 } else { 1 },
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
            .unwrap();
            for channel in 0..block.channels() {
                block
                    .channel_mut(channel)
                    .unwrap()
                    .fill(if id.as_str() == "main" { 0.1 } else { 0.2 });
            }
            block
        })
        .collect::<Vec<_>>();
    let mut scratch = AudioBlock::new(2, audiorouter_engine::PROCESSING_QUANTUM_FRAMES).unwrap();
    let mut output = AudioBlock::new(2, audiorouter_engine::PROCESSING_QUANTUM_FRAMES).unwrap();
    let mut heard = false;
    for _ in 0..40 {
        graph
            .process(&inputs, &mut scratch, &mut [&mut output])
            .unwrap();
        heard |= output
            .channel(0)
            .unwrap()
            .iter()
            .any(|sample| sample.abs() > 0.0);
        assert!(output
            .channel(0)
            .unwrap()
            .iter()
            .all(|sample| sample.is_finite()));
        std::thread::sleep(Duration::from_millis(3));
    }
    assert!(heard, "compiled route must receive prepared worker output");
}

#[test]
fn plugin_update_preparation_preserves_state_and_wiring_through_dispatch() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-plugin-update-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let binary = root.join("effect.vst3");
    // Metadata-only PE fixture; this regression never loads third-party code.
    let mut pe = vec![0; 256];
    pe[..2].copy_from_slice(b"MZ");
    pe[0x3c..0x40].copy_from_slice(&128_u32.to_le_bytes());
    pe[128..132].copy_from_slice(b"PE\0\0");
    pe[132..134].copy_from_slice(&0x8664_u16.to_le_bytes());
    std::fs::write(&binary, pe).unwrap();
    let storage = Storage::open(root.join("state.sqlite")).unwrap();
    let state_root = storage.plugin_state_directory().unwrap();
    std::fs::create_dir_all(&state_root).unwrap();
    let old_hash = "a".repeat(64);
    let asset = audiorouter_plugin_host::PluginStateAsset::new(1, vec![7; 64]).unwrap();
    let old_path =
        audiorouter_plugin_host::write_state_asset(&state_root, "before-update", &asset).unwrap();
    storage
        .save_plugin_state(&audiorouter_storage::PluginStateRecord {
            id: "before-update".into(),
            plugin_id: old_hash.clone(),
            plugin_sha256: old_hash.clone(),
            version: 1,
            path: old_path.to_string_lossy().into_owned(),
            state_sha256: asset.sha256.clone(),
            size_bytes: 64,
        })
        .unwrap();
    let mut saved = session();
    saved.nodes.push(Node { id: EntityId::new("effect"), name:"My updated effect".into(), kind:NodeKind::Plugin, type_version:1, enabled:false, bypass:false,
        parameters:serde_json::from_value(json!({"path":binary,"format":"vst3","classId":"default","fingerprint":old_hash,"stateId":"before-update","pluginParameter:0":0.3})).unwrap(),
        ports:vec![Port {name:"in".into(),direction:PortDirection::Input,channels:1},Port{name:"out".into(),direction:PortDirection::Output,channels:1}] });
    let mut plane = ControlPlane::with_storage("update-test", storage);
    plane.insert_session(saved.clone()).unwrap();
    plane.active_session_id = Some(saved.id.clone());
    let request = |method: &str, params| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    };
    let scan = plane.dispatch(request("plugins.scan", json!({"directory":root})));
    assert!(scan.error.is_none(), "{:?}", scan.error);
    let new_hash = scan.result.unwrap()["entries"][0]["identity"]["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let update_request = request(
        "plugins.prepareUpdate",
        json!({"nodeId":"My updated effect"}),
    );
    let denied = plane.dispatch_authorized(update_request.clone(), &ClientGrant::read_only());
    assert_eq!(
        denied.error.unwrap().data.unwrap()["code"],
        "permissionDenied"
    );
    let response = plane.dispatch(update_request);
    assert!(response.error.is_none(), "{:?}", response.error);
    let result = response.result.unwrap();
    assert_eq!(result["parameters"]["fingerprint"], new_hash);
    assert_eq!(result["statePreserved"], true);
    let new_id = result["parameters"]["stateId"].as_str().unwrap();
    assert_ne!(new_id, "before-update");
    assert_eq!(
        ControlPlane::load_plugin_state_asset(plane.storage.as_ref().unwrap(), new_id, &new_hash)
            .unwrap(),
        asset
    );
    assert!(old_path.is_file());
    assert_eq!(plane.get_session(&saved.id).unwrap(), &saved);
    // Start from defaults is one-shot: it drops the saved-state reference and
    // forgets the latest capture, without a marker that would also discard
    // every capture made afterwards. The old state file stays on disk.
    plane
        .storage
        .as_ref()
        .unwrap()
        .set_plugin_node_state(saved.id.as_str(), "effect", "before-update")
        .unwrap();
    let reset = plane
        .dispatch(request(
            "plugins.prepareUpdate",
            json!({"nodeId":"effect","resetState":true}),
        ))
        .result
        .unwrap();
    assert!(reset["parameters"].get("stateId").is_none());
    assert!(reset["parameters"].get("pluginParameter:0").is_none());
    assert_eq!(reset["statePreserved"], false);
    let storage = plane.storage.as_ref().unwrap();
    assert_eq!(
        storage
            .plugin_node_state(saved.id.as_str(), "effect")
            .unwrap(),
        None
    );
    assert!(old_path.is_file());
    storage
        .set_plugin_node_state(saved.id.as_str(), "effect", "after-reset")
        .unwrap();
    assert_eq!(
        storage
            .plugin_node_state(saved.id.as_str(), "effect")
            .unwrap()
            .as_deref(),
        Some("after-reset")
    );
    drop(plane);
    std::fs::remove_dir_all(root).unwrap();
}

/// Offline qualification of a user-installed effect, including the public
/// dispatcher and the real isolated worker. Never opens an audio endpoint.
#[cfg(windows)]
#[test]
#[ignore = "requires AUDIOROUTER_TEST_PLUGIN and both packaged plugin workers"]
fn installed_effect_describes_processes_and_restores_state() {
    use audiorouter_plugin_host::{
        inspect_binary, worker_clock_tick, SupervisedWorkerProcess, WorkerFrame,
    };
    let path = std::path::PathBuf::from(std::env::var_os("AUDIOROUTER_TEST_PLUGIN").unwrap());
    let root = path.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
    let mut plane = ControlPlane::default();
    for (method, params) in [
        ("plugins.scan", json!({"directory": root})),
        ("plugins.parameters", json!({"path": path})),
    ] {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        if method == "plugins.parameters" {
            assert!(!response.result.unwrap()["parameters"]
                .as_array()
                .unwrap()
                .is_empty());
        }
    }
    let kind = PluginWorkerKind::for_identity(identity.format, identity.architecture);
    let executable = require_plugin_worker(kind).unwrap();
    for channels in [1, 2] {
        let spawn = || match kind {
            PluginWorkerKind::Vst2 | PluginWorkerKind::Vst2X86 => {
                SupervisedWorkerProcess::spawn_verified_with_sample_rate(
                    &executable,
                    &identity,
                    std::slice::from_ref(&root),
                    channels,
                    48_000,
                    Instant::now(),
                )
            }
            PluginWorkerKind::Vst3 | PluginWorkerKind::Vst3X86 => {
                SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
                    &executable,
                    &identity,
                    std::slice::from_ref(&root),
                    channels,
                    48_000,
                    Instant::now(),
                )
            }
        };
        let mut worker = spawn().unwrap();
        let saved = worker
            .save_state(Instant::now())
            .expect("save installed effect state");
        assert!(!saved.bytes.is_empty());
        let mut peak = 0.0_f32;
        let mut difference = 0.0_f32;
        for block in 0..200_u64 {
            let samples: Vec<f32> = (0..128)
                .flat_map(|frame| {
                    let value = 0.25
                        * (std::f32::consts::TAU * 220.0 * (block * 128 + frame) as f32 / 48_000.0)
                            .sin();
                    std::iter::repeat_n(value, usize::from(channels))
                })
                .collect();
            let frame = WorkerFrame::new(
                block + 1,
                worker_clock_tick().saturating_add(10_000),
                channels,
                samples.clone(),
            )
            .unwrap();
            let output = worker.process(frame, vec![], Instant::now()).unwrap();
            for (sample, input) in output.samples.into_iter().zip(samples) {
                assert!(sample.is_finite());
                peak = peak.max(sample.abs());
                difference = difference.max((sample - input).abs());
            }
        }
        assert!(peak > 0.001, "installed effect returned silence");
        assert!(
            difference > 0.001,
            "installed effect only passed input through"
        );
        worker.restore_state(saved.clone(), Instant::now()).unwrap();
        assert!(worker.shutdown().unwrap().success());
        let mut fresh = spawn().unwrap();
        fresh
            .restore_state(saved, Instant::now())
            .expect("restore into a new worker");
        assert!(fresh.shutdown().unwrap().success());
        eprintln!("{}: {channels} channel(s), dispatcher, 200 audio blocks, state save/restore passed; peak {peak}", path.display());
    }
}

#[test]
fn plugin_worker_resolves_from_the_packaged_resource_directory() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-worker-resource-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let resources = root.join("resources");
    std::fs::create_dir_all(&resources).unwrap();
    let shell = root.join(if cfg!(windows) {
        "audiorouter-shell.exe"
    } else {
        "audiorouter-shell"
    });
    let worker_name = if cfg!(windows) {
        "audiorouter-plugin-worker.exe"
    } else {
        "audiorouter-plugin-worker"
    };
    let worker = resources.join(worker_name);
    std::fs::write(&worker, b"test executable placeholder").unwrap();
    assert_eq!(
        packaged_plugin_worker_path(&shell, PluginWorkerKind::Vst2),
        Some(worker.clone())
    );
    let adjacent = root.join(worker_name);
    std::fs::write(&adjacent, b"adjacent executable placeholder").unwrap();
    assert_eq!(
        packaged_plugin_worker_path(&shell, PluginWorkerKind::Vst2),
        Some(adjacent)
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn missing_plugin_worker_error_names_search_paths_and_repair_action() {
    let root = std::env::temp_dir().join("audiorouter-worker-error");
    let shell = root.join(if cfg!(windows) {
        "audiorouter-shell.exe"
    } else {
        "audiorouter-shell"
    });
    let message = plugin_worker_unavailable_message(Some(&shell), PluginWorkerKind::Vst2);
    let worker = if cfg!(windows) {
        "audiorouter-plugin-worker.exe"
    } else {
        "audiorouter-plugin-worker"
    };
    assert!(message.contains(&root.join(worker).display().to_string()));
    assert!(message.contains(&root.join("resources").join(worker).display().to_string()));
    assert!(message.contains("rebuild the complete AudioRouter package"));
    assert!(message.contains("AUDIOROUTER_PLUGIN_WORKER_PATH"));
}

/// A 32-bit plugin cannot load into a 64-bit process, so it gets the 32-bit
/// build of its format's worker (2026-11-13, full VST compatibility plan).
#[test]
fn x86_plugins_resolve_the_32_bit_workers() {
    use audiorouter_plugin_host::{PeArchitecture, PluginFormat};
    for (format, architecture, kind) in [
        (
            PluginFormat::Vst2,
            PeArchitecture::X64,
            PluginWorkerKind::Vst2,
        ),
        (
            PluginFormat::Vst3,
            PeArchitecture::X64,
            PluginWorkerKind::Vst3,
        ),
        (
            PluginFormat::Vst2,
            PeArchitecture::X86,
            PluginWorkerKind::Vst2X86,
        ),
        (
            PluginFormat::Vst3,
            PeArchitecture::X86,
            PluginWorkerKind::Vst3X86,
        ),
    ] {
        assert_eq!(PluginWorkerKind::for_identity(format, architecture), kind);
    }
    assert_ne!(
        PluginWorkerKind::Vst2.file_name(),
        PluginWorkerKind::Vst2X86.file_name()
    );
    assert!(PluginWorkerKind::Vst3X86.file_name().contains("x86"));
    assert_ne!(
        PluginWorkerKind::Vst2.override_variable(),
        PluginWorkerKind::Vst2X86.override_variable()
    );
}

/// VST3 effects run in their own native worker, never the VST2-only one
/// (which reported `VST2 load failed: MissingEntryPoint` for every VST3).
#[test]
fn vst3_plugins_resolve_the_native_vst3_worker() {
    assert_eq!(
        PluginWorkerKind::for_format(audiorouter_plugin_host::PluginFormat::Vst3),
        PluginWorkerKind::Vst3
    );
    assert_eq!(
        PluginWorkerKind::for_format(audiorouter_plugin_host::PluginFormat::Vst2),
        PluginWorkerKind::Vst2
    );
    let root = std::env::temp_dir().join(format!(
        "audiorouter-vst3-worker-resource-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let resources = root.join("resources");
    std::fs::create_dir_all(&resources).unwrap();
    let shell = root.join("audiorouter-shell.exe");
    std::fs::write(
        resources.join(PluginWorkerKind::Vst2.file_name()),
        b"vst2 worker placeholder",
    )
    .unwrap();
    assert_eq!(
        packaged_plugin_worker_path(&shell, PluginWorkerKind::Vst3),
        None
    );
    let vst3 = resources.join(PluginWorkerKind::Vst3.file_name());
    std::fs::write(&vst3, b"vst3 worker placeholder").unwrap();
    assert_eq!(
        packaged_plugin_worker_path(&shell, PluginWorkerKind::Vst3),
        Some(vst3)
    );
    let message = plugin_worker_unavailable_message(Some(&shell), PluginWorkerKind::Vst3);
    assert!(message.contains(PluginWorkerKind::Vst3.file_name()));
    assert!(message.contains("AUDIOROUTER_VST3_WORKER_PATH"));
    std::fs::remove_dir_all(&root).unwrap();
}

/// Installed VST3 effects start in the native VST3 worker through the
/// backend's own RPCs: scan a folder, then describe each matching plugin's
/// parameters (which launches its worker). Instruments are refused with a
/// plain reason. Set `AUDIOROUTER_VST3_WORKER_PATH` to the built
/// `audiorouter-vst3-worker.exe`; `AUDIOROUTER_LIVE_VST3_MATCH` selects
/// plugins by path text (default `uaudio_`, the UADx titles).
#[cfg(windows)]
#[test]
#[ignore = "installed VST3 plugins and the native VST3 worker"]
fn installed_vst3_plugins_start_in_the_native_worker() {
    if std::env::var_os("AUDIOROUTER_VST3_WORKER_PATH").is_none() {
        return;
    }
    let directory = std::env::var("AUDIOROUTER_LIVE_VST3_DIRECTORY")
        .unwrap_or_else(|_| r"C:\Program Files\Common Files\VST3".into());
    let pattern = std::env::var("AUDIOROUTER_LIVE_VST3_MATCH").unwrap_or_else(|_| "uaudio_".into());
    let mut plane = ControlPlane::default();
    let grant = ClientGrant::for_desktop_shell();
    let mut call = |method: &str, params: Value| {
        plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            },
            &grant,
        )
    };
    let scan = call("plugins.scan", json!({ "directory": directory }));
    assert!(scan.error.is_none(), "{:?}", scan.error);
    let entries = scan.result.unwrap()["entries"].as_array().unwrap().clone();
    let mut started = 0;
    for entry in entries.iter().filter(|entry| {
        entry["path"]
            .as_str()
            .is_some_and(|path| path.contains(&pattern))
            && entry["identity"]["compatibility"] == "supportedVst3X64"
    }) {
        let path = entry["path"].as_str().unwrap();
        let response = call("plugins.parameters", json!({ "path": path }));
        match (response.result, response.error) {
            (Some(result), None) => {
                let parameters = result["parameters"].as_array().unwrap();
                eprintln!("{path}: {} parameters", parameters.len());
                assert!(!parameters.is_empty(), "{path} exposed no parameters");
                started += 1;
            }
            (_, Some(error)) => {
                let text = format!("{error:?}");
                eprintln!("{path}: refused: {text}");
                assert!(
                    text.contains("instrument"),
                    "{path} failed to start: {text}"
                );
            }
            (None, None) => unreachable!(),
        }
    }
    assert!(started > 0, "no VST3 plugin matching {pattern} started");
}

/// A plugin setting changed while playing (as in its editor) must come
/// back after Stop and Play. Same environment as the test above; the
/// session must contain a plugin node `AUDIOROUTER_LIVE_PLUGIN_NODE`
/// (default `reaeq`). Privacy mute keeps it silent.
#[cfg(windows)]
#[test]
#[ignore = "live Windows audio devices and plugins"]
fn live_plugin_settings_survive_stop_and_play() {
    let Some(database) = std::env::var_os("AUDIOROUTER_LIVE_PATHS_DATABASE") else {
        return;
    };
    let session_id = std::env::var("AUDIOROUTER_LIVE_PATHS_SESSION")
        .unwrap_or_else(|_| "patrick-main-session".into());
    let node_id = EntityId::new(
        std::env::var("AUDIOROUTER_LIVE_PLUGIN_NODE").unwrap_or_else(|_| "reaeq".into()),
    );
    let storage = audiorouter_storage::Storage::open(std::path::Path::new(&database)).unwrap();
    let mut plane = ControlPlane::with_storage("live-plugin-state", storage);
    let run_id = format!(
        "live-plugin-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: (!params.is_null()).then_some(params),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    let play = |plane: &mut ControlPlane, run: &str| {
        call(
            plane,
            "nativePaths.prepare",
            json!({ "sessionId": session_id }),
        );
        let started = call(
            plane,
            "session.start",
            json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-{run}-start") }),
        );
        let generation = started["generation"].as_u64().unwrap();
        let until = Instant::now() + Duration::from_millis(800);
        while Instant::now() < until {
            call(
                plane,
                "nativeMultiInputs.pump",
                json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        generation
    };
    call(
        &mut plane,
        "safety.setPrivacyMute",
        json!({ "muted": true, "idempotencyKey": format!("{run_id}-mute") }),
    );
    let generation = play(&mut plane, "first");
    let session = EntityId::new(&session_id);
    let bridge = plane.plugin_bridge(&session, &node_id).unwrap();
    let before = bridge.save_state().unwrap();
    // Change settings the way an editor does: on the running instance only.
    // The values differ on every run, so a restored earlier run's values
    // never turn the change into a no-op.
    let seed = 0.1
        + std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_millis() as f32
            / 1000.0
            * 0.6;
    bridge
        .set_parameters(
            (0..4)
                .map(|parameter_id| audiorouter_plugin_host::ParameterEvent {
                    parameter_id,
                    normalized_value: (seed + parameter_id as f32 * 0.05).min(0.95),
                    sample_offset: 0,
                })
                .collect(),
        )
        .unwrap();
    let until = Instant::now() + Duration::from_millis(500);
    while Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
        call(
            &mut plane,
            "nativeMultiInputs.pump",
            json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
        );
    }
    let changed = bridge.save_state().unwrap();
    assert_ne!(
        changed.bytes, before.bytes,
        "the test must actually change the plugin's state"
    );
    drop(bridge);
    call(
        &mut plane,
        "session.stop",
        json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-first-stop") }),
    );
    call(
        &mut plane,
        "nativeEndpoints.detach",
        json!({ "sessionId": session_id }),
    );
    play(&mut plane, "second");
    let restored = plane
        .plugin_bridge(&session, &node_id)
        .unwrap()
        .save_state()
        .unwrap();
    call(
        &mut plane,
        "session.stop",
        json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-second-stop") }),
    );
    // Some plugins re-round a stored double when reloading (ReaEQ moves
    // one frequency by its last bit), so allow a byte or two of drift but
    // require the restore to match the change, not the original state.
    let differing = |left: &[u8], right: &[u8]| {
        left.iter().zip(right).filter(|(a, b)| a != b).count() + left.len().abs_diff(right.len())
    };
    assert!(
        differing(&restored.bytes, &changed.bytes) <= 2
            && differing(&restored.bytes, &before.bytes)
                > differing(&restored.bytes, &changed.bytes),
        "Stop then Play must restore the plugin's settings: restored {:?}, changed {:?}",
        restored.bytes,
        changed.bytes
    );
    eprintln!(
        "verified {} settings survive Stop and Play ({} state bytes)",
        node_id.as_str(),
        restored.bytes.len()
    );
}

#[test]
fn plugin_scan_is_read_only_and_keeps_invalid_candidates_visible() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-plugin-scan-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("candidate.dll"), b"not a PE binary").unwrap();
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.scan".into(),
        params: Some(json!({ "directory": root.to_string_lossy() })),
    };
    let denied = plane.dispatch_authorized(request.clone(), &ClientGrant::read_only());
    assert_eq!(
        denied.error.unwrap().data.unwrap()["code"],
        "permissionDenied"
    );
    let response = plane.dispatch_authorized(
        request,
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(result["directory"], root.to_string_lossy().to_string());
    assert_eq!(result["entries"].as_array().unwrap().len(), 1);
    assert!(result["entries"][0]["identity"].is_null());
    assert!(result["entries"][0]["error"].is_string());
    assert_eq!(result["entries"][0]["errorCode"], "notPe");
    let listed = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "plugins.list".into(),
            params: Some(json!({ "directory": root.to_string_lossy() })),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(listed.result.unwrap(), result);
    let retry_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "plugins.retry".into(),
        params: Some(json!({
            "directory": root.to_string_lossy(),
            "idempotencyKey": "plugin-retry"
        })),
    };
    let retried = plane.dispatch_authorized(
        retry_request.clone(),
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(
        retried.result.as_ref().unwrap()["entries"],
        result["entries"]
    );
    let replayed = plane.dispatch_authorized(
        retry_request,
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(replayed.result.unwrap(), retried.result.unwrap());
    let inspected = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "plugins.inspect".into(),
            params: Some(json!({ "path": root.join("candidate.dll").to_string_lossy() })),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert!(inspected.error.is_none());
    let inspected_result = inspected.result.unwrap();
    assert!(inspected_result["identity"].is_null());
    assert_eq!(inspected_result["errorCode"], "notPe");
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(windows)]
#[test]
fn desktop_dispatch_does_not_apply_the_http_network_path_policy() {
    let mut plane = ControlPlane::default();
    // The local machine's missing share needs no remote host or credentials.
    // Scan errors are allowed; an HTTP path-policy rejection is not.
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.scan".into(),
        params: Some(json!({"directory": r"\\localhost\AudioRouterMissingScanFixture"})),
    });
    if let Some(error) = response.error {
        assert!(
            matches!(
                error.data.as_ref().and_then(|data| data["code"].as_str()),
                Some("invalidRoot" | "io")
            ),
            "The scanner must receive the desktop path: {error:?}"
        );
    }
}

#[test]
fn plugin_placeholder_plan_requires_current_scan_evidence() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.nodes.push(Node {
        id: EntityId::new("plugin"),
        kind: NodeKind::Plugin,
        type_version: 1,
        name: "Unbound plugin".into(),
        enabled: false,
        bypass: false,
        parameters: [
            ("path".into(), json!("C:\\Plugins\\effect.dll")),
            ("format".into(), json!("vst2")),
            (
                "fingerprint".into(),
                json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
            ),
            ("classId".into(), json!("effect-class")),
        ]
        .into_iter()
        .collect(),
        ports: vec![
            Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 1,
            },
            Port {
                name: "out".into(),
                direction: PortDirection::Output,
                channels: 1,
            },
        ],
    });
    let error = plane
        .plan_graph(&original.id, original.revision, candidate)
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message.contains("current explicit scan")
    ));
}

#[test]
fn plugin_inventory_cache_evicts_oldest_scan_roots() {
    let mut plane = ControlPlane::default();
    for index in 0..=MAX_PLUGIN_INVENTORY_ROOTS {
        plane.remember_plugin_inventory(
            format!("C:\\plugin-root-{index}"),
            json!({ "directory": format!("C:\\plugin-root-{index}"), "entries": [] }),
        );
    }

    assert_eq!(plane.plugin_inventories.len(), MAX_PLUGIN_INVENTORY_ROOTS);
    assert!(!plane.plugin_inventories.contains_key("C:\\plugin-root-0"));
    assert!(plane
        .plugin_inventories
        .contains_key(&format!("C:\\plugin-root-{MAX_PLUGIN_INVENTORY_ROOTS}")));
    assert_eq!(
        plane.plugin_inventory_order.len(),
        MAX_PLUGIN_INVENTORY_ROOTS
    );
}

#[test]
fn plugin_scan_directory_failures_have_stable_application_codes() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-plugin-scan-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let response = ControlPlane::default().dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "plugins.scan".into(),
            params: Some(json!({ "directory": root.to_string_lossy() })),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(response.error.unwrap().data.unwrap()["code"], "invalidRoot");
}

#[test]
fn plugin_inspect_discovery_has_typed_identity_schema() {
    let method = ControlPlane::default().describe()["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "plugins.inspect")
        .unwrap()
        .clone();
    assert_eq!(method["permission"], "pluginScan");
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["type"][1],
        "null"
    );
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["required"]
            .as_array()
            .unwrap()
            .len(),
        11
    );
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["properties"]["fileBytes"]["maximum"],
        audiorouter_plugin_host::MAX_PLUGIN_BYTES
    );
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["properties"]["classIds"]["maxItems"],
        256
    );
    assert!(method["outputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "errorCode"));
    assert_eq!(
        method["outputSchema"]["properties"]["errorCode"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
}

#[test]
fn unprepared_plugin_start_explains_http_preparation_for_both_aliases() {
    for method in ["session.start", "sessions.start"] {
        let mut plane = ControlPlane::default();
        let mut saved = session();
        saved.nodes[0].kind = NodeKind::Plugin;
        saved.nodes[0].enabled = true;
        let id = saved.id.clone();
        plane.insert_session(saved.clone()).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(json!({ "sessionId": id, "idempotencyKey": "unprepared-start" })),
        });
        let error = response.error.expect("unprepared plugins must not start");
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("POST /api/v1/nativePaths/prepare"));
        assert!(error.message.contains("DeviceAdministration"));
        assert_eq!(plane.get_session(&id).unwrap(), &saved);
        assert!(!plane
            .runtimes
            .get(&id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running));
    }
}

#[test]
fn play_applies_only_sliders_moved_since_the_state_was_captured() {
    let parameters = json!({
        "path": "effect.vst3",
        "pluginParameter:1": 0.25,
        "pluginParameter:2": 0.5,
        "pluginParameter:3": 0.75,
    });
    let parameters = parameters.as_object().unwrap();
    // No snapshot (an old capture, or none): every slider applies.
    let all = plugin_sliders_to_apply(parameters, None);
    assert_eq!(all.len(), 3);
    // The capture saw 1 and 2 at these values; 2 has moved since and 3 is new.
    let captured = json!({ "pluginParameter:1": 0.25, "pluginParameter:2": 0.1 });
    let moved = plugin_sliders_to_apply(parameters, captured.as_object());
    let names = moved
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["pluginParameter:2", "pluginParameter:3"]);
}

/// An Instrument node starts its own worker through the same path a route
/// uses (`start_plugin_bridges`), and its bridge plays MIDI pushed to the
/// queue (phase P8b). Needs the built VST2 worker and the test instrument
/// (`tools/vst2-test-instrument`): set `AUDIOROUTER_PLUGIN_WORKER_PATH` and
/// `AUDIOROUTER_TEST_VST2_INSTRUMENT`. A VST3 instrument works too: set
/// `AUDIOROUTER_VST3_WORKER_PATH` and `AUDIOROUTER_TEST_INSTRUMENT` to its bundle.
#[cfg(windows)]
#[test]
#[ignore = "needs the built VST2 worker and the test instrument DLL"]
fn an_instrument_node_starts_a_worker_and_plays_midi() {
    use audiorouter_domain::{Node, Port};
    use audiorouter_plugin_host::inspect_binary;
    let dll = std::path::PathBuf::from(
        std::env::var_os("AUDIOROUTER_TEST_INSTRUMENT")
            .or_else(|| std::env::var_os("AUDIOROUTER_TEST_VST2_INSTRUMENT"))
            .expect("test instrument path"),
    );
    let root = dll.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&dll, std::slice::from_ref(&root)).unwrap();
    let node = |id: &str, kind: NodeKind| Node {
        id: EntityId::new(id),
        name: id.into(),
        kind,
        type_version: 1,
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    };
    let session = Session {
        id: EntityId::new("instrument-session"),
        name: "instrument".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![node("piano", NodeKind::Instrument)],
        edges: vec![],
    };
    let request = |node_id: &str| PluginStageRequest {
        node_id: EntityId::new(node_id),
        scanned_path: dll.clone(),
        configured_root: root.clone(),
        fingerprint: identity.sha256.clone(),
        channels: 2,
        state: None,
        parameters: Vec::new(),
    };
    let started = start_plugin_bridges(&session, vec![request("piano")], 48_000).unwrap();
    assert_eq!(started.len(), 1);
    let (id, bridge) = &started[0];
    let bridge = bridge
        .standard()
        .expect("instrument uses its MIDI-capable standard bridge");
    assert_eq!(id.as_str(), "piano");
    let peak = |quanta: usize| {
        let mut peak = 0.0_f32;
        let mut samples = vec![0.0_f32; audiorouter_engine::PROCESSING_QUANTUM_FRAMES * 2];
        for _ in 0..quanta {
            let mut block = audiorouter_engine::AudioBlock::new(
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
            .unwrap();
            audiorouter_engine::RealtimePluginProcessor::process(bridge.as_ref(), &mut block);
            block.copy_to_interleaved(&mut samples).unwrap();
            peak = samples
                .iter()
                .fold(peak, |peak, sample| peak.max(sample.abs()));
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        peak
    };
    assert!(peak(40) < 1e-6, "silent until a note arrives");
    // Play it the way a client does: through the dispatcher, the node named
    // rather than identified, on a session whose runtime is running.
    let mut plane = ControlPlane::default();
    plane.insert_session(session.clone()).unwrap();
    let mut runtime = audiorouter_domain::FakeRuntime::default();
    runtime.prepare(&session).unwrap();
    runtime.start().unwrap();
    plane.runtimes.insert(session.id.clone(), runtime);
    plane
        .plugin_bridges
        .lock()
        .unwrap()
        .insert((session.id.clone(), id.clone()), Arc::downgrade(bridge));
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "midi.sendEvents".into(),
        params: Some(json!({ "nodeId": "PIANO", "events": [
            { "type": "noteOn", "note": 69, "velocity": 120 }
        ] })),
    });
    let result = response.result.expect("midi.sendEvents plays");
    assert_eq!(result["accepted"], json!(1));
    assert_eq!(result["heldNotes"], json!(1));
    let sounded = peak(300);
    assert!(sounded > 0.01, "the held note sounds (peak {sounded})");
    // The panic button releases it and the output falls silent again.
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "midi.sendEvents".into(),
        params: Some(json!({ "nodeId": "piano", "events": [{ "type": "allNotesOff" }] })),
    });
    assert_eq!(response.result.expect("panic")["heldNotes"], json!(0));
    // A real synth rings out its release tail (Surge XT takes about a second);
    // the test instrument stops at once. Either way the output must reach
    // silence, and a note that never released would not.
    let mut tail = f32::MAX;
    for _ in 0..40 {
        tail = peak(60);
        if tail < 1e-6 {
            break;
        }
    }
    assert!(tail < 1e-6, "silent after all notes off (peak {tail})");
    // With no request for it, the instrument node starts nothing.
    assert!(start_plugin_bridges(&session, vec![], 48_000)
        .unwrap()
        .is_empty());
}
