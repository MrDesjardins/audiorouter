//! Tests for `setup_check.rs`. Each finding has a regression, including the
//! 2026-11-02 cases: a 7.1 cable feeding a Surround input is correct, the
//! same cable as an output is refused, and two routes looping through cables.

use super::*;
use audiorouter_domain::{Edge, Node, Port};

/// SPEAKER_7POINT1_SURROUND (front, center, LFE, back and side pairs).
const MASK_7_1: u32 = 0x63f;

fn device(id: &str, render: bool, channels: u16, cable: Option<&str>) -> DeviceFacts {
    DeviceFacts {
        id: id.into(),
        render,
        name: id.into(),
        sample_rate_hz: 48_000,
        channels,
        is_float32: true,
        channel_mask: if channels == 8 { MASK_7_1 } else { 0x3 },
        cable_key: cable.map(Into::into),
    }
}

/// The owner's cables: CABLE, CABLE-A and CABLE-B with CABLE-B Input at 7.1.
fn devices() -> Vec<DeviceFacts> {
    vec![
        device("cable-in", true, 2, Some("base")),
        device("cable-out", false, 2, Some("base")),
        device("cable-a-in", true, 2, Some("a")),
        device("cable-a-out", false, 2, Some("a")),
        device("cable-b-in", true, 8, Some("b")),
        device("cable-b-out", false, 2, Some("b")),
        device("headphones", true, 2, None),
        device("mic", false, 1, None),
    ]
}

fn node(id: &str, kind: NodeKind, endpoint: &str) -> Node {
    let port = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(json!({ "endpointId": endpoint })).unwrap(),
        ports: match kind {
            NodeKind::PhysicalInput => vec![port("out", PortDirection::Output)],
            NodeKind::PhysicalOutput => vec![port("in", PortDirection::Input)],
            _ => unreachable!(),
        },
    }
}

/// `input` → `output` in one session; `surround` sets Surround to headphones.
fn route(id: &str, input: &str, surround: bool, output: &str) -> Session {
    let mut source = node(&format!("{id}-in"), NodeKind::PhysicalInput, input);
    if surround {
        source
            .parameters
            .insert("spatialMode".into(), json!("headphones"));
    }
    let sink = node(&format!("{id}-out"), NodeKind::PhysicalOutput, output);
    Session {
        id: EntityId::new(id),
        name: id.into(),
        schema_version: 1,
        revision: 0,
        edges: vec![Edge {
            id: EntityId::new(format!("{id}-edge")),
            source_node: source.id.clone(),
            source_port: "out".into(),
            destination_node: sink.id.clone(),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        }],
        nodes: vec![source, sink],
    }
}

fn codes(findings: &[SetupFinding]) -> Vec<&'static str> {
    findings.iter().map(|finding| finding.code).collect()
}

#[test]
fn a_7_1_cable_feeding_a_surround_input_is_correct() {
    // The owner's Siege input: CABLE-B Input at 7.1, loopback with Surround.
    let siege = route("siege", "cable-b-in", true, "headphones");
    assert!(check_setup(&[(&siege, true)], &devices()).is_empty());
}

#[test]
fn a_surround_input_on_a_stereo_device_is_refused() {
    let mut devices = devices();
    devices[4].channels = 2;
    devices[4].channel_mask = 0x3;
    let siege = route("siege", "cable-b-in", true, "headphones");
    let findings = check_setup(&[(&siege, false)], &devices);
    assert_eq!(codes(&findings), ["surroundDevice"]);
    assert_eq!(findings[0].severity, Severity::Problem);
    assert!(findings[0].fix.contains("7.1"));
    assert_eq!(findings[0].device_id.as_deref(), Some("cable-b-in"));
}

#[test]
fn an_output_to_a_7_1_device_is_refused_with_the_play_rule() {
    let test = route("test", "cable-out", false, "cable-b-in");
    let findings = check_setup(&[(&test, false)], &devices());
    assert_eq!(codes(&findings), ["outputFormat"]);
    assert!(findings[0].message.contains("8 channel(s)"));
    // The check and native preparation share the predicate.
    assert!(!output_format_supported(true, 8, 48_000));
    assert!(output_format_supported(true, 2, 44_100));
    assert!(!output_format_supported(false, 2, 48_000));
}

#[test]
fn surround_predicate_matches_the_play_refusal() {
    assert_eq!(surround_capture_refusal(true, 8, MASK_7_1, 48_000), None);
    assert_eq!(surround_capture_refusal(true, 6, 0x3f, 48_000), None);
    assert_eq!(
        surround_capture_refusal(true, 2, 0x3, 48_000),
        Some(SurroundRefusal::Layout)
    );
    assert_eq!(
        surround_capture_refusal(false, 8, MASK_7_1, 48_000),
        Some(SurroundRefusal::Format)
    );
    // A mask whose speaker count disagrees with the channels.
    assert_eq!(
        surround_capture_refusal(true, 8, 0x3f, 48_000),
        Some(SurroundRefusal::Layout)
    );
}

#[test]
fn a_device_that_is_gone_is_reported_once_per_node() {
    let route = route("voice", "old-mic-id", false, "cable-a-in");
    let findings = check_setup(&[(&route, false)], &devices());
    assert_eq!(codes(&findings), ["deviceMissing"]);
    assert_eq!(findings[0].node_id.as_deref(), Some("voice-in"));
    assert!(findings[0].fix.contains("driver update"));
}

#[test]
fn a_playback_device_is_not_a_recording_device_without_surround() {
    // A plain input naming a playback device cannot record it.
    let route = route("voice", "cable-b-in", false, "headphones");
    assert_eq!(
        codes(&check_setup(&[(&route, false)], &devices())),
        ["deviceMissing"]
    );
}

#[test]
fn disabled_or_unconnected_nodes_are_not_judged() {
    let mut route = route("test", "gone", false, "cable-b-in");
    route.edges[0].enabled = false;
    assert!(check_setup(&[(&route, false)], &devices()).is_empty());
    route.edges[0].enabled = true;
    route.nodes[0].enabled = false;
    route.nodes[1].enabled = false;
    assert!(check_setup(&[(&route, false)], &devices()).is_empty());
}

#[test]
fn bypassed_devices_do_not_produce_format_or_missing_device_findings() {
    let mut route = route("bypassed", "gone", false, "cable-b-in");
    for node in &mut route.nodes {
        node.bypass = true;
    }
    assert!(check_setup(&[(&route, false)], &devices()).is_empty());
}

#[test]
fn three_sessions_can_form_a_loop_without_any_pair_looping() {
    let a = route("a", "cable-out", false, "cable-a-in");
    let b = route("b", "cable-a-out", false, "headphones");
    let mut c = route("c", "cable-b-out", false, "cable-in");
    let mut b = b;
    b.nodes[1]
        .parameters
        .insert("endpointId".into(), json!("cable-b-in"));
    let mut inventory = devices();
    inventory[4].channels = 2;
    let findings = check_setup(&[(&a, true), (&b, true), (&c, true)], &inventory);
    assert!(findings
        .iter()
        .any(|finding| finding.code == "crossSessionLoop"));
    assert!(
        !check_setup(&[(&a, false), (&b, false), (&c, false)], &inventory)
            .iter()
            .any(|finding| finding.code == "crossSessionLoop")
    );
    c.edges[0].enabled = false;
    assert!(
        !check_setup(&[(&a, true), (&b, true), (&c, true)], &inventory)
            .iter()
            .any(|finding| finding.code == "crossSessionLoop")
    );
}

#[test]
fn a_loop_inside_one_session_is_the_play_refusal() {
    // CABLE Output → CABLE Input: what it plays comes straight back.
    let route = route("loop", "cable-out", false, "cable-in");
    let findings = check_setup(&[(&route, false)], &devices());
    assert_eq!(codes(&findings), ["feedbackLoop"]);
    assert_eq!(findings[0].severity, Severity::Problem);
    assert!(findings[0].message.contains("feedback loop"));
}

#[test]
fn two_routes_looping_through_cables_are_reported_once() {
    // 2026-11-02: the test route played CABLE Output into CABLE-B Input; the
    // owner's route loopback-recorded CABLE-B Input and played to CABLE Input.
    let mut devices = devices();
    devices[4].channels = 2;
    devices[4].channel_mask = 0x3;
    let test = route("test", "cable-out", false, "cable-b-in");
    let mut siege = route("siege", "cable-b-in", true, "cable-in");
    // Surround needs 7.1; judge only the loop here.
    siege.nodes[0].parameters.remove("spatialMode");
    siege.nodes[0]
        .parameters
        .insert("endpointId".into(), json!("cable-b-out"));
    let findings = check_setup(&[(&test, false), (&siege, true)], &devices);
    assert_eq!(codes(&findings), ["crossSessionLoop"]);
    assert_eq!(findings[0].severity, Severity::Warning);
    assert!(findings[0]
        .message
        .starts_with("Playing \"test\" while \"siege\""));
    // Neither playing: nothing loops yet.
    assert!(check_setup(&[(&test, false), (&siege, false)], &devices).is_empty());
    // Both playing: one finding, not one per session.
    let both = check_setup(&[(&test, true), (&siege, true)], &devices);
    assert_eq!(codes(&both), ["crossSessionLoop"]);
    assert!(both[0].message.contains("both playing"));
}

#[test]
fn a_loop_through_a_7_1_loopback_input_is_found() {
    // The owner's real topology: Siege loopback-records the 7.1 CABLE-B
    // Input with Surround; another route plays CABLE Output to CABLE-B.
    let siege = route("siege", "cable-b-in", true, "cable-in");
    let mut test = route("test", "cable-out", false, "headphones");
    test.nodes[1]
        .parameters
        .insert("endpointId".into(), json!("cable-b-in"));
    let findings = check_setup(&[(&test, false), (&siege, true)], &devices());
    // The 7.1 output is refused too; the loop is still named.
    assert_eq!(codes(&findings), ["outputFormat", "crossSessionLoop"]);
}

#[test]
fn unrelated_paths_that_share_cables_are_not_a_loop() {
    // A: mic → CABLE-A, and CABLE Output → headphones. B: CABLE-A Output →
    // CABLE Input. Audio goes mic → B → headphones, never back to the mic.
    let mut a = route("a", "mic", false, "cable-a-in");
    let monitor = route("monitor", "cable-out", false, "headphones");
    a.nodes.extend(monitor.nodes);
    a.edges.extend(monitor.edges);
    let b = route("b", "cable-a-out", false, "cable-in");
    assert!(check_setup(&[(&a, true), (&b, true)], &devices()).is_empty());
}

#[test]
fn a_cable_not_at_48_khz_gets_one_tip() {
    let mut devices = devices();
    devices[2].sample_rate_hz = 44_100;
    let mut route = route("voice", "mic", false, "cable-a-in");
    let second = node("voice-out-2", NodeKind::PhysicalOutput, "cable-a-in");
    route.edges.push(Edge {
        id: EntityId::new("e2"),
        source_node: EntityId::new("voice-in"),
        source_port: "out".into(),
        destination_node: second.id.clone(),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    });
    route.nodes.push(second);
    let findings = check_setup(&[(&route, false)], &devices);
    assert_eq!(codes(&findings), ["cableRate"]);
    assert_eq!(findings[0].severity, Severity::Tip);
    assert!(findings[0].message.contains("44.1 kHz"));
    // A device that is not a cable is not tipped.
    devices[2].cable_key = None;
    assert!(check_setup(&[(&route, false)], &devices).is_empty());
}

#[test]
fn every_device_kind_is_classified() {
    for kind in NodeKind::ALL {
        let binds = device_use(kind).is_some();
        assert_eq!(
            binds,
            matches!(
                kind,
                NodeKind::PhysicalInput | NodeKind::PhysicalOutput | NodeKind::EndpointLoopback
            ),
            "{kind:?}"
        );
    }
}

#[test]
fn problems_precede_tips_before_the_response_limit() {
    let mut inventory = devices();
    inventory[2].sample_rate_hz = 44_100;
    let mut routes = (0..101)
        .map(|index| route(&format!("tip-{index}"), "mic", false, "cable-a-in"))
        .collect::<Vec<_>>();
    routes.push(route("problem", "gone", false, "headphones"));
    let sessions = routes
        .iter()
        .map(|session| (session, false))
        .collect::<Vec<_>>();
    let findings = check_setup(&sessions, &inventory);
    assert!(findings.len() > MAX_SETUP_FINDINGS);
    assert_eq!(findings[0].code, "deviceMissing");
}

#[test]
fn dispatch_reports_through_the_control_plane() {
    let mut plane = ControlPlane::default();
    plane
        .insert_session(route("saved", "no-such-device", false, "headphones-gone"))
        .unwrap();
    plane
        .insert_session(route("other", "also-gone", false, "gone-too"))
        .unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "system.checkSetup".into(),
        params: Some(json!({ "sessionId": "saved" })),
    });
    let result = response.result.expect("checkSetup succeeds");
    assert!(result["notChecked"][0]
        .as_str()
        .unwrap()
        .contains("other apps"));
    assert_eq!(result["sessionsChecked"], 1);
    let findings = result["findings"].as_array().unwrap();
    if result["notChecked"].as_array().unwrap().len() == 1 {
        // The device list was read: both saved devices are missing, and only
        // the requested session is reported.
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings
            .iter()
            .all(|finding| finding["code"] == "deviceMissing"
                && finding["sessionId"] == "saved"
                && finding["severity"] == "problem"));
    } else {
        assert!(findings.is_empty());
    }
    let missing = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "system.checkSetup".into(),
        params: Some(json!({ "sessionId": "no-such-session" })),
    });
    assert!(missing.error.is_some());
    let unknown = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "system.checkSetup".into(),
        params: Some(json!({ "other": true })),
    });
    assert!(unknown.error.is_some());
    let oversized = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "system.checkSetup".into(),
        params: Some(
            json!({ "sessionId": "a".repeat(audiorouter_domain::MAX_ENTITY_ID_BYTES + 1) }),
        ),
    });
    assert!(oversized.error.unwrap().message.contains("ID limit"));
}
