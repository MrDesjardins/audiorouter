//! Tests for the task-shaped control methods, through JSON-RPC dispatch.

use super::*;

fn node(
    id: &str,
    kind: NodeKind,
    name: &str,
    inputs: &[&str],
    output: bool,
    parameters: Value,
) -> Node {
    let mut ports = inputs
        .iter()
        .map(|name| Port {
            name: (*name).into(),
            direction: PortDirection::Input,
            channels: 2,
        })
        .collect::<Vec<_>>();
    if output {
        ports.push(Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        });
    }
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: name.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports,
    }
}

fn edge(id: &str, from: &str, to: &str) -> Edge {
    Edge {
        id: EntityId::new(id),
        source_node: EntityId::new(from),
        source_port: "out".into(),
        destination_node: EntityId::new(to),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    }
}

fn plane() -> ControlPlane {
    let session = Session {
        id: EntityId::new("stream"),
        name: "Stream".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node(
                "mic",
                NodeKind::PhysicalInput,
                "Microphone",
                &[],
                true,
                json!({}),
            ),
            node(
                "voice",
                NodeKind::Gain,
                "Voice gain",
                &["in"],
                true,
                json!({ "gainDb": 0.0 }),
            ),
            node(
                "headphones",
                NodeKind::PhysicalOutput,
                "Headphones",
                &["in"],
                false,
                json!({}),
            ),
            node("game", NodeKind::TestSignal, "Game", &[], true, json!({})),
            node(
                "speakers",
                NodeKind::PhysicalOutput,
                "Speakers",
                &["in"],
                false,
                json!({}),
            ),
        ],
        edges: vec![
            edge("e1", "mic", "voice"),
            edge("e2", "voice", "headphones"),
            edge("e3", "game", "speakers"),
        ],
    };
    let mut plane = ControlPlane::default();
    plane.insert_session(session).unwrap();
    plane
}

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Result<Value, String> {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    });
    match (response.result, response.error) {
        (Some(result), None) => Ok(result),
        (_, Some(error)) => Err(error.message),
        _ => Err("no result".into()),
    }
}

fn session(plane: &ControlPlane) -> Session {
    plane.get_session(&EntityId::new("stream")).unwrap().clone()
}

#[test]
fn catalog_describes_addable_tools_in_plain_words() {
    let catalog = call(&mut plane(), "nodes.catalog", json!({})).unwrap();
    let items = catalog.as_array().unwrap();
    let duck = items.iter().find(|item| item["kind"] == "duck").unwrap();
    assert!(duck["description"].as_str().unwrap().contains("while"));
    assert_eq!(duck["inputs"], json!(["in"]));
    assert!(items.iter().any(|item| item["kind"] == "compressor"
        && item["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|parameter| parameter["name"] == "thresholdDb")));
    assert!(items
        .iter()
        .all(|item| item["kind"] != "plugin" && item["kind"] != "applicationCapture"));
    let switch = items
        .iter()
        .find(|item| item["kind"] == "inputSwitch")
        .unwrap();
    assert_eq!(switch["inputs"], json!(["a", "b"]));
}

/// Describing device and source settings must not change what `nodes.add`
/// saves: these kinds are added with no parameters, and a Soundboard
/// family (`slot<N>Mode`) never becomes a literal key.
#[test]
fn adding_device_and_source_nodes_saves_no_parameters() {
    let mut plane = plane();
    for (index, kind) in [
        "physicalInput",
        "physicalOutput",
        "meter",
        "audioFile",
        "networkSend",
        "networkReceive",
        "soundboard",
    ]
    .into_iter()
    .enumerate()
    {
        let added = call(
            &mut plane,
            "nodes.add",
            json!({ "kind": kind, "idempotencyKey": format!("device-{index}") }),
        )
        .unwrap();
        let id = added["nodeId"].as_str().unwrap().to_owned();
        let saved = session(&plane);
        let node = saved
            .nodes
            .iter()
            .find(|node| node.id.as_str() == id)
            .unwrap();
        assert!(node.parameters.is_empty(), "{kind}: {:?}", node.parameters);
    }
    let catalog = call(&mut plane, "nodes.catalog", json!({})).unwrap();
    let soundboard = catalog
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"] == "soundboard")
        .unwrap();
    assert!(soundboard["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .any(|parameter| parameter["namePattern"] == "slot<N>MediaId"));
    let summary = call(&mut plane, "sessions.summary", json!({})).unwrap();
    for node in summary["nodes"].as_array().unwrap() {
        if node["kind"] == "soundboard" || node["kind"] == "physicalInput" {
            assert_eq!(node["settings"], json!({}), "{node}");
        }
    }
}

/// Every kind `nodes.add` offers commits with the defaults it saves.
#[test]
fn every_addable_kind_commits_with_its_catalog_defaults() {
    let mut plane = plane();
    for (index, kind) in NodeKind::ALL.into_iter().enumerate() {
        if node_template(kind).is_none() {
            continue;
        }
        let kind = kind_name(kind);
        call(
            &mut plane,
            "nodes.add",
            json!({ "kind": kind, "idempotencyKey": format!("every-{index}") }),
        )
        .unwrap_or_else(|error| panic!("{kind}: {error}"));
    }
}

#[test]
fn summary_reads_like_the_canvas() {
    let summary = call(&mut plane(), "sessions.summary", json!({})).unwrap();
    assert_eq!(summary["sessionId"], "stream");
    assert_eq!(summary["playing"], false);
    let voice = summary["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["name"] == "Voice gain")
        .unwrap();
    assert_eq!(voice["settings"]["gainDb"], "0 dB");
    assert!(summary["connections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|connection| connection["from"] == "Microphone" && connection["to"] == "Voice gain"));
}

#[test]
fn set_changes_a_setting_by_name_and_saves_a_revision_once_per_key() {
    let mut plane = plane();
    let result = call(
        &mut plane,
        "nodes.set",
        json!({ "node": "voice GAIN", "parameters": { "gainDb": 6.0 }, "idempotencyKey": "set-1" }),
    )
    .unwrap();
    assert_eq!(result["nodeId"], "voice");
    assert_eq!(session(&plane).revision, 1);
    assert_eq!(session(&plane).nodes[1].parameters["gainDb"], json!(6.0));
    // Same key: replayed, not applied twice.
    assert_eq!(call(&mut plane, "nodes.set", json!({ "node": "voice GAIN", "parameters": { "gainDb": 6.0 }, "idempotencyKey": "set-1" })).unwrap(), result);
    assert_eq!(session(&plane).revision, 1);
    // Flags and rename.
    call(
        &mut plane,
        "nodes.set",
        json!({ "node": "voice", "bypass": true, "name": "Voice", "idempotencyKey": "set-2" }),
    )
    .unwrap();
    assert!(session(&plane).nodes[1].bypass);
    assert_eq!(session(&plane).nodes[1].name, "Voice");
    // Invalid values and unknown names are refused with guidance.
    assert!(call(
        &mut plane,
        "nodes.set",
        json!({ "node": "Voice", "parameters": { "gainDb": 999.0 }, "idempotencyKey": "set-3" })
    )
    .is_err());
    let unknown = call(
        &mut plane,
        "nodes.set",
        json!({ "node": "Guitar", "parameters": { "gainDb": 1.0 }, "idempotencyKey": "set-4" }),
    )
    .unwrap_err();
    assert!(
        unknown.contains("no node named") && unknown.contains("\"Microphone\""),
        "{unknown}"
    );
    assert!(call(
        &mut plane,
        "nodes.set",
        json!({ "node": "Voice", "idempotencyKey": "set-5" })
    )
    .unwrap_err()
    .contains("nothing to change"));
    assert!(call(
        &mut plane,
        "nodes.set",
        json!({ "node": "Voice", "parameters": { "gainDb": 1.0 } })
    )
    .unwrap_err()
    .contains("idempotencyKey"));
}

#[test]
fn ambiguous_names_list_the_ids() {
    let mut plane = plane();
    call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "meter", "name": "Monitor", "idempotencyKey": "add-1" }),
    )
    .unwrap();
    call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "meter", "name": "monitor", "idempotencyKey": "add-2" }),
    )
    .unwrap();
    let error = call(
        &mut plane,
        "nodes.toggle",
        json!({ "node": "MONITOR", "target": "enabled", "idempotencyKey": "t-1" }),
    )
    .unwrap_err();
    assert!(
        error.contains("several nodes") && error.contains("meter-1") && error.contains("meter-2"),
        "{error}"
    );
}

#[test]
fn add_inserts_tools_between_or_after_nodes_and_remove_bridges_them() {
    let mut plane = plane();
    let added = call(&mut plane, "nodes.add", json!({ "kind": "compressor", "between": { "from": "Microphone", "to": "Voice gain" }, "idempotencyKey": "a1" })).unwrap();
    assert_eq!(added["nodeId"], "compressor-1");
    assert_eq!(added["name"], "Compressor 1");
    let graph = session(&plane);
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == "mic"
            && edge.destination_node.as_str() == "compressor-1"));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == "compressor-1"
            && edge.destination_node.as_str() == "voice"));
    assert!(!graph.edges.iter().any(
        |edge| edge.source_node.as_str() == "mic" && edge.destination_node.as_str() == "voice"
    ));
    assert_eq!(graph.nodes.last().unwrap().parameters["ratio"], json!(3.0));
    // After a node with one outgoing connection: inserted before it.
    call(&mut plane, "nodes.add", json!({ "kind": "meter", "after": "Voice gain", "name": "Voice meter", "idempotencyKey": "a2" })).unwrap();
    assert!(session(&plane)
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == "meter-1"
            && edge.destination_node.as_str() == "headphones"));
    // A Duck with its trigger, on the game line.
    call(&mut plane, "nodes.add", json!({ "kind": "duck", "between": { "from": "Game", "to": "Speakers" }, "parameters": { "keyNodeId": "mic", "amountDb": 10.0 }, "idempotencyKey": "a3" })).unwrap();
    assert_eq!(
        session(&plane).nodes.last().unwrap().parameters["keyNodeId"],
        "mic"
    );
    // Removing the compressor reconnects Microphone to Voice gain.
    let removed = call(
        &mut plane,
        "nodes.remove",
        json!({ "node": "Compressor 1", "idempotencyKey": "r1" }),
    )
    .unwrap();
    assert_eq!(removed["bridged"], true);
    assert!(session(&plane).edges.iter().any(
        |edge| edge.source_node.as_str() == "mic" && edge.destination_node.as_str() == "voice"
    ));
    // Kinds that need a UI-chosen identity, and unknown kinds, are refused.
    assert!(call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "plugin", "idempotencyKey": "a4" })
    )
    .unwrap_err()
    .contains("identity"));
    assert!(call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "reverb", "idempotencyKey": "a5" })
    )
    .unwrap_err()
    .contains("nodes.catalog"));
}

#[test]
fn removing_a_tool_composes_both_rectangular_edge_matrices() {
    let mut original = session(&plane());
    original
        .nodes
        .retain(|node| ["mic", "voice", "headphones"].contains(&node.id.as_str()));
    original
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "mic")
        .unwrap()
        .ports[0]
        .channels = 1;
    original
        .nodes
        .iter_mut()
        .find(|node| node.id.as_str() == "headphones")
        .unwrap()
        .ports[0]
        .channels = 1;
    original.edges.retain(|edge| edge.id.as_str() != "e3");
    original.edges[0].matrix = vec![0.2, 0.3];
    original.edges[1].matrix = vec![0.25, 0.5];
    let mut plane = ControlPlane::default();
    plane.insert_session(original).unwrap();
    let result = call(
        &mut plane,
        "nodes.remove",
        json!({"node":"voice", "bridge":true, "idempotencyKey":"compose"}),
    )
    .unwrap();
    assert_eq!(result["bridged"], true);
    let matrix = &session(&plane).edges[0].matrix;
    assert_eq!(matrix.len(), 1);
    assert!((matrix[0] - 0.2).abs() < 1e-6);
}

#[test]
fn inserting_a_stereo_tool_in_a_mono_connection_preserves_its_gain() {
    let mut original = session(&plane());
    original
        .nodes
        .retain(|node| ["mic", "voice"].contains(&node.id.as_str()));
    for node in &mut original.nodes {
        for port in &mut node.ports {
            port.channels = 1;
        }
    }
    original.edges.retain(|edge| edge.id.as_str() == "e1");
    original.edges[0].matrix = vec![0.4];
    let mut plane = ControlPlane::default();
    plane.insert_session(original).unwrap();
    call(
        &mut plane,
        "nodes.add",
        json!({"kind":"meter", "after":"mic", "idempotencyKey":"mono-insert"}),
    )
    .unwrap();
    let inserted = session(&plane);
    assert_eq!(
        inserted
            .edges
            .iter()
            .find(|edge| edge.destination_node.as_str() == "voice")
            .unwrap()
            .matrix,
        vec![0.2, 0.2]
    );
    call(
        &mut plane,
        "nodes.remove",
        json!({"node":"meter-1", "idempotencyKey":"mono-remove"}),
    )
    .unwrap();
    assert_eq!(session(&plane).edges[0].matrix, vec![0.4]);
}

#[test]
fn inserting_and_bridging_preserve_ports_matrices_flags_and_mixer_volume() {
    for (kind, inputs, port_name) in [
        (NodeKind::Mixer, vec!["in"], "in"),
        (NodeKind::InputSwitch, vec!["a", "b"], "b"),
        (NodeKind::Duck, vec!["in", "key"], "key"),
    ] {
        for enabled in [true, false] {
            for after in [true, false] {
                let mut original = session(&plane());
                original.nodes = vec![
                    node("mic", NodeKind::PhysicalInput, "Mic", &[], true, json!({})),
                    node(
                        "target",
                        kind,
                        "Target",
                        &inputs,
                        true,
                        if kind == NodeKind::Mixer {
                            json!({"inputVolume:mic":40.0})
                        } else {
                            json!({})
                        },
                    ),
                ];
                let mut connection = edge("original", "mic", "target");
                connection.destination_port = port_name.into();
                original.nodes[0].ports[0].name = "capture".into();
                connection.source_port = "capture".into();
                connection.matrix = vec![0.2, 0.3, 0.4, 0.5];
                connection.enabled = enabled;
                original.edges = vec![connection.clone()];
                let mut plane = ControlPlane::default();
                plane.insert_session(original).unwrap();
                let mut request = json!({"kind":"meter", "idempotencyKey":"insert"});
                if after {
                    request["after"] = json!("mic");
                } else {
                    request["between"] = json!({"from":"mic", "to":"target"});
                }
                call(&mut plane, "nodes.add", request).unwrap();
                let inserted = session(&plane);
                let downstream = inserted
                    .edges
                    .iter()
                    .find(|edge| edge.destination_node.as_str() == "target")
                    .unwrap();
                assert_eq!(downstream.destination_port, port_name);
                assert_eq!(downstream.matrix, connection.matrix);
                assert_eq!(downstream.enabled, enabled);
                if kind == NodeKind::Mixer {
                    let mixer = inserted
                        .nodes
                        .iter()
                        .find(|node| node.id.as_str() == "target")
                        .unwrap();
                    assert_eq!(
                        audiorouter_domain::mixer_input_volume(mixer, &EntityId::new("meter-1")),
                        0.4
                    );
                    assert!(!mixer.parameters.contains_key("inputVolume:mic"));
                }
                let removed = call(
                    &mut plane,
                    "nodes.remove",
                    json!({"node":"meter-1", "bridge":true,"idempotencyKey":"remove"}),
                )
                .unwrap();
                assert_eq!(removed["bridged"], true);
                let restored = session(&plane);
                let bridge = &restored.edges[0];
                assert_eq!(bridge.source_node, connection.source_node);
                assert_eq!(bridge.source_port, connection.source_port);
                assert_eq!(bridge.destination_port, port_name);
                assert_eq!(bridge.matrix, connection.matrix);
                assert_eq!(bridge.enabled, enabled);
                if kind == NodeKind::Mixer {
                    let mixer = restored
                        .nodes
                        .iter()
                        .find(|node| node.id.as_str() == "target")
                        .unwrap();
                    assert_eq!(
                        audiorouter_domain::mixer_input_volume(mixer, &EntityId::new("mic")),
                        0.4
                    );
                    assert!(!mixer.parameters.contains_key("inputVolume:meter-1"));
                }
            }
        }
    }
}

#[test]
fn toggles_flip_flags_on_off_settings_and_two_way_choices() {
    let mut plane = plane();
    call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "inputSwitch", "name": "Switch", "idempotencyKey": "s" }),
    )
    .unwrap();
    assert_eq!(
        call(
            &mut plane,
            "nodes.toggle",
            json!({ "node": "Switch", "target": "selected", "idempotencyKey": "t1" })
        )
        .unwrap()["value"],
        "b"
    );
    assert_eq!(
        call(
            &mut plane,
            "nodes.toggle",
            json!({ "node": "Switch", "target": "selected", "idempotencyKey": "t2" })
        )
        .unwrap()["value"],
        "a"
    );
    call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "outputSwitch", "name": "Output Switch", "idempotencyKey": "o" }),
    )
    .unwrap();
    assert_eq!(
        call(
            &mut plane,
            "nodes.toggle",
            json!({ "node": "Output Switch", "target": "selected", "idempotencyKey": "o1" })
        )
        .unwrap()["value"],
        "b"
    );
    assert_eq!(
        call(
            &mut plane,
            "nodes.toggle",
            json!({ "node": "Voice gain", "target": "enabled", "idempotencyKey": "t3" })
        )
        .unwrap()["value"],
        false
    );
    call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "mute", "name": "Mic mute", "idempotencyKey": "m" }),
    )
    .unwrap();
    assert_eq!(
        call(
            &mut plane,
            "nodes.toggle",
            json!({ "node": "Mic mute", "target": "muted", "idempotencyKey": "t4" })
        )
        .unwrap()["value"],
        true
    );
    // An unset on/off setting with no default (Audio file loop) is off, so
    // the first toggle turns it on. Before, it "cannot be toggled".
    call(
        &mut plane,
        "nodes.add",
        json!({ "kind": "audioFile", "name": "Intro", "idempotencyKey": "f" }),
    )
    .unwrap();
    assert_eq!(
        call(
            &mut plane,
            "nodes.toggle",
            json!({ "node": "Intro", "target": "loop", "idempotencyKey": "t6" })
        )
        .unwrap()["value"],
        true
    );
    assert!(call(
        &mut plane,
        "nodes.toggle",
        json!({ "node": "Voice gain", "target": "gainDb", "idempotencyKey": "t5" })
    )
    .unwrap_err()
    .contains("cannot be toggled"));
}

#[test]
fn connections_can_select_output_switch_ports_and_remove_only_one() {
    let mut plane = plane();
    let added = call(
        &mut plane,
        "nodes.add",
        json!({"kind": "outputSwitch", "name": "Destinations", "idempotencyKey": "switch"}),
    )
    .unwrap();
    let switch = added["nodeId"].as_str().unwrap();
    for (port, destination) in [("a", "Headphones"), ("b", "Speakers")] {
        let source = if port == "a" { "Voice gain" } else { "Game" };
        call(
            &mut plane,
            "connections.remove",
            json!({"from": source, "to": destination, "idempotencyKey": format!("unwire-{port}")}),
        )
        .unwrap();
        call(&mut plane, "connections.add", json!({"from": switch, "fromOutput": port, "to": destination, "toInput": "in", "idempotencyKey": format!("wire-{port}")})).unwrap();
    }
    let summary = call(&mut plane, "sessions.summary", json!({})).unwrap();
    for port in ["a", "b"] {
        assert!(summary["connections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["fromOutput"] == port));
    }
    call(&mut plane, "connections.remove", json!({"from": switch, "fromOutput": "b", "to": "Speakers", "toInput": "in", "idempotencyKey": "remove-b"})).unwrap();
    assert!(session(&plane)
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == switch && edge.source_port == "a"));
    assert!(!session(&plane)
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == switch && edge.source_port == "b"));
    let error = call(&mut plane, "connections.add", json!({"from": switch, "fromOutput": "missing", "to": "Speakers", "idempotencyKey": "bad-port"})).unwrap_err();
    assert!(error.contains("a") && error.contains("b"), "{error}");
    let before = session(&plane);
    let error = call(&mut plane, "connections.add", json!({"from": switch, "fromOutput": 7, "to": "Speakers", "idempotencyKey": "wrong-port-type"})).unwrap_err();
    assert!(error.contains("fromOutput"), "{error}");
    assert_eq!(session(&plane), before);
}

#[test]
fn connections_respect_single_inputs_and_mixers() {
    let mut plane = plane();
    let occupied = call(
        &mut plane,
        "connections.add",
        json!({ "from": "Game", "to": "Headphones", "idempotencyKey": "c1" }),
    )
    .unwrap_err();
    assert!(
        occupied.contains("already has an input") && occupied.contains("Mixer"),
        "{occupied}"
    );
    call(
        &mut plane,
        "connections.remove",
        json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "c2" }),
    )
    .unwrap();
    assert!(!session(&plane)
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == "game"));
    assert!(call(
        &mut plane,
        "connections.remove",
        json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "c3" })
    )
    .unwrap_err()
    .contains("not connected"));
    call(
        &mut plane,
        "connections.add",
        json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "c4" }),
    )
    .unwrap();
    assert!(session(&plane)
        .edges
        .iter()
        .any(|edge| edge.source_node.as_str() == "game"
            && edge.destination_node.as_str() == "speakers"));
}

#[test]
fn mute_toggle_levels_and_play_guidance() {
    let mut plane = plane();
    let before = plane.privacy_muted;
    assert_eq!(
        call(
            &mut plane,
            "safety.togglePrivacyMute",
            json!({ "idempotencyKey": "p1" })
        )
        .unwrap()["muted"],
        !before
    );
    assert_eq!(
        call(
            &mut plane,
            "safety.togglePrivacyMute",
            json!({ "idempotencyKey": "p2" })
        )
        .unwrap()["muted"],
        before
    );
    let levels = call(&mut plane, "meters.levels", json!({})).unwrap();
    assert_eq!(levels["playing"], false);
    assert_eq!(levels["levels"], json!([]));
    // Devices are not chosen on this test route: Play explains what to do
    // (or that native audio is unavailable on this platform).
    let error = call(
        &mut plane,
        "sessions.play",
        json!({ "idempotencyKey": "play-1" }),
    )
    .unwrap_err();
    assert!(
        error.contains("device") || error.contains("Windows"),
        "{error}"
    );
    assert!(call(
        &mut plane,
        "sessions.summary",
        json!({ "sessionId": "missing" })
    )
    .is_err());
}

#[test]
fn simple_methods_keep_the_existing_permission_boundaries() {
    let mut plane = plane();
    let request = |method: &str, params: Value| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    };
    let observer = ClientGrant::read_only();
    assert!(plane
        .dispatch_authorized(request("sessions.summary", json!({})), &observer)
        .error
        .is_none());
    assert!(plane
        .dispatch_authorized(request("meters.levels", json!({})), &observer)
        .error
        .is_none());
    assert!(plane
        .dispatch_authorized(request("nodes.catalog", json!({})), &observer)
        .error
        .is_none());
    for (method, params) in [
        (
            "nodes.set",
            json!({ "node": "Voice gain", "parameters": { "gainDb": 3.0 }, "idempotencyKey": "x1" }),
        ),
        (
            "nodes.add",
            json!({ "kind": "meter", "idempotencyKey": "x2" }),
        ),
        (
            "connections.remove",
            json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "x3" }),
        ),
        (
            "safety.togglePrivacyMute",
            json!({ "idempotencyKey": "x4" }),
        ),
    ] {
        assert!(
            plane
                .dispatch_authorized(request(method, params), &observer)
                .error
                .is_some(),
            "{method} must need more than read"
        );
    }
    // Play opens devices: session control alone is not enough.
    let session_control =
        ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::SessionControl]);
    let denied = plane.dispatch_authorized(
        request("sessions.play", json!({ "idempotencyKey": "x5" })),
        &session_control,
    );
    assert!(denied.error.is_some_and(
        |error| error.message.to_lowercase().contains("permission") || error.code == -32001
    ));
    let editor = ClientGrant::for_role(ClientRole::Editor);
    assert!(plane.dispatch_authorized(request("nodes.set", json!({ "node": "Voice gain", "parameters": { "gainDb": 3.0 }, "idempotencyKey": "x6" })), &editor).error.is_none());
    assert_eq!(
        session(&plane).revision,
        1,
        "only the authorized edit was applied"
    );
}

#[test]
fn device_tools_added_by_a_script_get_the_names_the_app_shows() {
    let name = |kind| node_template(kind).expect("device template").0;
    assert_eq!(name(NodeKind::PhysicalInput), "Input device");
    assert_eq!(name(NodeKind::PhysicalOutput), "Output device");
}
