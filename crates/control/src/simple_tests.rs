//! Tests for the task-shaped control methods, through JSON-RPC dispatch.

use super::*;

fn node(id: &str, kind: NodeKind, name: &str, inputs: &[&str], output: bool, parameters: Value) -> Node {
    let mut ports = inputs.iter().map(|name| Port { name: (*name).into(), direction: PortDirection::Input, channels: 2 }).collect::<Vec<_>>();
    if output {
        ports.push(Port { name: "out".into(), direction: PortDirection::Output, channels: 2 });
    }
    Node { id: EntityId::new(id), kind, type_version: 1, name: name.into(), enabled: true, bypass: false, parameters: serde_json::from_value(parameters).unwrap(), ports }
}

fn edge(id: &str, from: &str, to: &str) -> Edge {
    Edge { id: EntityId::new(id), source_node: EntityId::new(from), source_port: "out".into(), destination_node: EntityId::new(to), destination_port: "in".into(), matrix: vec![1.0, 0.0, 0.0, 1.0], enabled: true }
}

fn plane() -> ControlPlane {
    let session = Session {
        id: EntityId::new("stream"),
        name: "Stream".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            node("mic", NodeKind::PhysicalInput, "Microphone", &[], true, json!({})),
            node("voice", NodeKind::Gain, "Voice gain", &["in"], true, json!({ "gainDb": 0.0 })),
            node("headphones", NodeKind::PhysicalOutput, "Headphones", &["in"], false, json!({})),
            node("game", NodeKind::TestSignal, "Game", &[], true, json!({})),
            node("speakers", NodeKind::PhysicalOutput, "Speakers", &["in"], false, json!({})),
        ],
        edges: vec![edge("e1", "mic", "voice"), edge("e2", "voice", "headphones"), edge("e3", "game", "speakers")],
    };
    let mut plane = ControlPlane::default();
    plane.insert_session(session).unwrap();
    plane
}

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Result<Value, String> {
    let response = plane.dispatch(JsonRpcRequest { jsonrpc: "2.0".into(), id: Some(json!(1)), method: method.into(), params: Some(params) });
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
    assert!(items.iter().any(|item| item["kind"] == "compressor" && item["parameters"].as_array().unwrap().iter().any(|parameter| parameter["name"] == "thresholdDb")));
    assert!(items.iter().all(|item| item["kind"] != "plugin" && item["kind"] != "applicationCapture"));
    let switch = items.iter().find(|item| item["kind"] == "inputSwitch").unwrap();
    assert_eq!(switch["inputs"], json!(["a", "b"]));
}

#[test]
fn summary_reads_like_the_canvas() {
    let summary = call(&mut plane(), "sessions.summary", json!({})).unwrap();
    assert_eq!(summary["sessionId"], "stream");
    assert_eq!(summary["playing"], false);
    let voice = summary["nodes"].as_array().unwrap().iter().find(|node| node["name"] == "Voice gain").unwrap();
    assert_eq!(voice["settings"]["gainDb"], "0 dB");
    assert!(summary["connections"].as_array().unwrap().iter().any(|connection| connection["from"] == "Microphone" && connection["to"] == "Voice gain"));
}

#[test]
fn set_changes_a_setting_by_name_and_saves_a_revision_once_per_key() {
    let mut plane = plane();
    let result = call(&mut plane, "nodes.set", json!({ "node": "voice GAIN", "parameters": { "gainDb": 6.0 }, "idempotencyKey": "set-1" })).unwrap();
    assert_eq!(result["nodeId"], "voice");
    assert_eq!(session(&plane).revision, 1);
    assert_eq!(session(&plane).nodes[1].parameters["gainDb"], json!(6.0));
    // Same key: replayed, not applied twice.
    assert_eq!(call(&mut plane, "nodes.set", json!({ "node": "voice GAIN", "parameters": { "gainDb": 6.0 }, "idempotencyKey": "set-1" })).unwrap(), result);
    assert_eq!(session(&plane).revision, 1);
    // Flags and rename.
    call(&mut plane, "nodes.set", json!({ "node": "voice", "bypass": true, "name": "Voice", "idempotencyKey": "set-2" })).unwrap();
    assert!(session(&plane).nodes[1].bypass);
    assert_eq!(session(&plane).nodes[1].name, "Voice");
    // Invalid values and unknown names are refused with guidance.
    assert!(call(&mut plane, "nodes.set", json!({ "node": "Voice", "parameters": { "gainDb": 999.0 }, "idempotencyKey": "set-3" })).is_err());
    let unknown = call(&mut plane, "nodes.set", json!({ "node": "Guitar", "parameters": { "gainDb": 1.0 }, "idempotencyKey": "set-4" })).unwrap_err();
    assert!(unknown.contains("no node named") && unknown.contains("\"Microphone\""), "{unknown}");
    assert!(call(&mut plane, "nodes.set", json!({ "node": "Voice", "idempotencyKey": "set-5" })).unwrap_err().contains("nothing to change"));
    assert!(call(&mut plane, "nodes.set", json!({ "node": "Voice", "parameters": { "gainDb": 1.0 } })).unwrap_err().contains("idempotencyKey"));
}

#[test]
fn ambiguous_names_list_the_ids() {
    let mut plane = plane();
    call(&mut plane, "nodes.add", json!({ "kind": "meter", "name": "Monitor", "idempotencyKey": "add-1" })).unwrap();
    call(&mut plane, "nodes.add", json!({ "kind": "meter", "name": "monitor", "idempotencyKey": "add-2" })).unwrap();
    let error = call(&mut plane, "nodes.toggle", json!({ "node": "MONITOR", "target": "enabled", "idempotencyKey": "t-1" })).unwrap_err();
    assert!(error.contains("several nodes") && error.contains("meter-1") && error.contains("meter-2"), "{error}");
}

#[test]
fn add_inserts_tools_between_or_after_nodes_and_remove_bridges_them() {
    let mut plane = plane();
    let added = call(&mut plane, "nodes.add", json!({ "kind": "compressor", "between": { "from": "Microphone", "to": "Voice gain" }, "idempotencyKey": "a1" })).unwrap();
    assert_eq!(added["nodeId"], "compressor-1");
    assert_eq!(added["name"], "Compressor 1");
    let graph = session(&plane);
    assert!(graph.edges.iter().any(|edge| edge.source_node.as_str() == "mic" && edge.destination_node.as_str() == "compressor-1"));
    assert!(graph.edges.iter().any(|edge| edge.source_node.as_str() == "compressor-1" && edge.destination_node.as_str() == "voice"));
    assert!(!graph.edges.iter().any(|edge| edge.source_node.as_str() == "mic" && edge.destination_node.as_str() == "voice"));
    assert_eq!(graph.nodes.last().unwrap().parameters["ratio"], json!(3.0));
    // After a node with one outgoing connection: inserted before it.
    call(&mut plane, "nodes.add", json!({ "kind": "meter", "after": "Voice gain", "name": "Voice meter", "idempotencyKey": "a2" })).unwrap();
    assert!(session(&plane).edges.iter().any(|edge| edge.source_node.as_str() == "meter-1" && edge.destination_node.as_str() == "headphones"));
    // A Duck with its trigger, on the game line.
    call(&mut plane, "nodes.add", json!({ "kind": "duck", "between": { "from": "Game", "to": "Speakers" }, "parameters": { "keyNodeId": "mic", "amountDb": 10.0 }, "idempotencyKey": "a3" })).unwrap();
    assert_eq!(session(&plane).nodes.last().unwrap().parameters["keyNodeId"], "mic");
    // Removing the compressor reconnects Microphone to Voice gain.
    let removed = call(&mut plane, "nodes.remove", json!({ "node": "Compressor 1", "idempotencyKey": "r1" })).unwrap();
    assert_eq!(removed["bridged"], true);
    assert!(session(&plane).edges.iter().any(|edge| edge.source_node.as_str() == "mic" && edge.destination_node.as_str() == "voice"));
    // Kinds that need a UI-chosen identity, and unknown kinds, are refused.
    assert!(call(&mut plane, "nodes.add", json!({ "kind": "plugin", "idempotencyKey": "a4" })).unwrap_err().contains("identity"));
    assert!(call(&mut plane, "nodes.add", json!({ "kind": "reverb", "idempotencyKey": "a5" })).unwrap_err().contains("nodes.catalog"));
}

#[test]
fn toggles_flip_flags_on_off_settings_and_two_way_choices() {
    let mut plane = plane();
    call(&mut plane, "nodes.add", json!({ "kind": "inputSwitch", "name": "Switch", "idempotencyKey": "s" })).unwrap();
    assert_eq!(call(&mut plane, "nodes.toggle", json!({ "node": "Switch", "target": "selected", "idempotencyKey": "t1" })).unwrap()["value"], "b");
    assert_eq!(call(&mut plane, "nodes.toggle", json!({ "node": "Switch", "target": "selected", "idempotencyKey": "t2" })).unwrap()["value"], "a");
    assert_eq!(call(&mut plane, "nodes.toggle", json!({ "node": "Voice gain", "target": "enabled", "idempotencyKey": "t3" })).unwrap()["value"], false);
    call(&mut plane, "nodes.add", json!({ "kind": "mute", "name": "Mic mute", "idempotencyKey": "m" })).unwrap();
    assert_eq!(call(&mut plane, "nodes.toggle", json!({ "node": "Mic mute", "target": "muted", "idempotencyKey": "t4" })).unwrap()["value"], true);
    assert!(call(&mut plane, "nodes.toggle", json!({ "node": "Voice gain", "target": "gainDb", "idempotencyKey": "t5" })).unwrap_err().contains("cannot be toggled"));
}

#[test]
fn connections_respect_single_inputs_and_mixers() {
    let mut plane = plane();
    let occupied = call(&mut plane, "connections.add", json!({ "from": "Game", "to": "Headphones", "idempotencyKey": "c1" })).unwrap_err();
    assert!(occupied.contains("already has an input") && occupied.contains("Mixer"), "{occupied}");
    call(&mut plane, "connections.remove", json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "c2" })).unwrap();
    assert!(!session(&plane).edges.iter().any(|edge| edge.source_node.as_str() == "game"));
    assert!(call(&mut plane, "connections.remove", json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "c3" })).unwrap_err().contains("not connected"));
    call(&mut plane, "connections.add", json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "c4" })).unwrap();
    assert!(session(&plane).edges.iter().any(|edge| edge.source_node.as_str() == "game" && edge.destination_node.as_str() == "speakers"));
}

#[test]
fn mute_toggle_levels_and_play_guidance() {
    let mut plane = plane();
    let before = plane.privacy_muted;
    assert_eq!(call(&mut plane, "safety.togglePrivacyMute", json!({ "idempotencyKey": "p1" })).unwrap()["muted"], !before);
    assert_eq!(call(&mut plane, "safety.togglePrivacyMute", json!({ "idempotencyKey": "p2" })).unwrap()["muted"], before);
    let levels = call(&mut plane, "meters.levels", json!({})).unwrap();
    assert_eq!(levels["playing"], false);
    assert_eq!(levels["levels"], json!([]));
    // Devices are not chosen on this test route: Play explains what to do
    // (or that native audio is unavailable on this platform).
    let error = call(&mut plane, "sessions.play", json!({ "idempotencyKey": "play-1" })).unwrap_err();
    assert!(error.contains("device") || error.contains("Windows"), "{error}");
    assert!(call(&mut plane, "sessions.summary", json!({ "sessionId": "missing" })).is_err());
}

#[test]
fn simple_methods_keep_the_existing_permission_boundaries() {
    let mut plane = plane();
    let request = |method: &str, params: Value| JsonRpcRequest { jsonrpc: "2.0".into(), id: Some(json!(1)), method: method.into(), params: Some(params) };
    let observer = ClientGrant::read_only();
    assert!(plane.dispatch_authorized(request("sessions.summary", json!({})), &observer).error.is_none());
    assert!(plane.dispatch_authorized(request("meters.levels", json!({})), &observer).error.is_none());
    assert!(plane.dispatch_authorized(request("nodes.catalog", json!({})), &observer).error.is_none());
    for (method, params) in [
        ("nodes.set", json!({ "node": "Voice gain", "parameters": { "gainDb": 3.0 }, "idempotencyKey": "x1" })),
        ("nodes.add", json!({ "kind": "meter", "idempotencyKey": "x2" })),
        ("connections.remove", json!({ "from": "Game", "to": "Speakers", "idempotencyKey": "x3" })),
        ("safety.togglePrivacyMute", json!({ "idempotencyKey": "x4" })),
    ] {
        assert!(plane.dispatch_authorized(request(method, params), &observer).error.is_some(), "{method} must need more than read");
    }
    // Play opens devices: session control alone is not enough.
    let session_control = ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::SessionControl]);
    let denied = plane.dispatch_authorized(request("sessions.play", json!({ "idempotencyKey": "x5" })), &session_control);
    assert!(denied.error.is_some_and(|error| error.message.to_lowercase().contains("permission") || error.code == -32001));
    let editor = ClientGrant::for_role(ClientRole::Editor);
    assert!(plane.dispatch_authorized(request("nodes.set", json!({ "node": "Voice gain", "parameters": { "gainDb": 3.0 }, "idempotencyKey": "x6" })), &editor).error.is_none());
    assert_eq!(session(&plane).revision, 1, "only the authorized edit was applied");
}
