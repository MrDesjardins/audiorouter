//! Hold-to-compare rules that need no audio device.

use super::*;

#[cfg(windows)]
#[test]
fn native_comparison_keeps_a_playing_generated_source_and_its_transport_handle() {
    use audiorouter_domain::{Node, Port, PortDirection};
    let receiver = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut route = crate::test_support::session();
    route.nodes[0].kind = NodeKind::TestSignal;
    route.nodes[0].parameters =
        serde_json::from_value(json!({"frequencyHz":997,"levelDb":-12,"durationMs":600000}))
            .unwrap();
    route.nodes[1].kind = NodeKind::NetworkSend;
    route.nodes[1].parameters = serde_json::from_value(
        json!({"host":"127.0.0.1","port":receiver.local_addr().unwrap().port()}),
    )
    .unwrap();
    route.nodes.push(Node {
        id: EntityId::new("gain"),
        kind: NodeKind::Gain,
        type_version: 1,
        name: "Gain".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(json!({"gainDb":-12})).unwrap(),
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
    let mut output = route.edges[0].clone();
    output.id = EntityId::new("gain-output");
    output.source_node = EntityId::new("gain");
    output.source_port = "out".into();
    route.edges[0].destination_node = EntityId::new("gain");
    route.edges[0].destination_port = "in".into();
    route.edges.push(output);
    for node in &mut route.nodes {
        for port in &mut node.ports {
            port.channels = 2;
        }
    }
    for edge in &mut route.edges {
        edge.matrix = vec![1.0, 0.0, 0.0, 1.0];
    }
    let id = route.id.clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(route).unwrap();
    let invoke = |plane: &mut ControlPlane, method: &str, params: Value| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        });
        assert!(response.error.is_none(), "{method}: {response:?}");
        response.result.unwrap()
    };
    invoke(&mut plane, "nativePaths.prepare", json!({"sessionId":id}));
    invoke(
        &mut plane,
        "session.start",
        json!({"sessionId":id,"idempotencyKey":"start"}),
    );
    invoke(
        &mut plane,
        "audioSources.transport",
        json!({"sessionId":id,"nodeId":"in","action":"play"}),
    );
    let source = plane.test_signal_sources[&(id.clone(), EntityId::new("in"))].clone();
    call(
        &mut plane,
        json!({"sessionId":id,"nodes":["gain"],"idempotencyKey":"hold"}),
    )
    .unwrap();
    let held_source = &plane.test_signal_sources[&(id.clone(), EntityId::new("in"))];
    assert!(
        Arc::ptr_eq(&source, held_source),
        "comparison replaced the live transport handle"
    );
    assert!(held_source.is_playing());
    plane.expire_comparison(Instant::now() + MAX_HOLD);
    assert!(Arc::ptr_eq(
        &source,
        &plane.test_signal_sources[&(id.clone(), EntityId::new("in"))]
    ));
    invoke(
        &mut plane,
        "audioSources.transport",
        json!({"sessionId":id,"nodeId":"in","action":"stop"}),
    );
    assert!(
        !source.is_playing(),
        "transport commands must still control the published source"
    );
    plane.session_stop(&id).unwrap();
}

#[test]
fn two_sessions_publish_holds_and_restore_only_the_released_or_expired_route() {
    let mut plane = ControlPlane::new("test");
    let a = goal_session(&mut plane);
    let b = EntityId::new("second");
    plane.duplicate_session(&a, b.clone(), None).unwrap();
    plane.session_start(&a).unwrap();
    plane.session_start(&b).unwrap();
    // Capture the publication boundary instead of opening microphones.
    plane.comparison_test_publications = Some(Vec::new());
    for (id, key) in [(&a, "hold-a"), (&b, "hold-b")] {
        call(
            &mut plane,
            json!({"sessionId": id, "nodes": ["gate"], "idempotencyKey": key}),
        )
        .unwrap();
    }
    call(
        &mut plane,
        json!({"sessionId": b, "nodes": [], "idempotencyKey": "release-b"}),
    )
    .unwrap();
    assert!(plane.comparison.contains_key(&a));
    let refused = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.delete".into(),
        params: Some(json!({"sessionId": a, "idempotencyKey": "delete-held"})),
    });
    assert!(refused.error.is_some());
    assert!(plane.comparison.contains_key(&a));
    plane.expire_comparison(Instant::now() + MAX_HOLD);
    assert!(plane.comparison.is_empty());
    let publications = plane.comparison_test_publications.as_ref().unwrap();
    assert_eq!(publications.len(), 4);
    for (published, (id, bypass)) in
        publications
            .iter()
            .zip([(&a, true), (&b, true), (&b, false), (&a, false)])
    {
        assert_eq!(&published.id, id);
        assert_eq!(
            published
                .nodes
                .iter()
                .find(|node| node.id.as_str() == "gate")
                .unwrap()
                .bypass,
            bypass
        );
    }
}

#[test]
fn another_sessions_release_or_failed_hold_preserves_the_first_hold() {
    let mut plane = ControlPlane::new("test");
    let a = goal_session(&mut plane);
    let b = EntityId::new("second");
    plane.duplicate_session(&a, b.clone(), None).unwrap();
    plane.session_start(&a).unwrap();
    plane.session_start(&b).unwrap();
    plane.comparison.insert(
        a.clone(),
        Comparison {
            nodes: vec![EntityId::new("gate")],
            expires_at: Instant::now() + MAX_HOLD,
        },
    );
    call(
        &mut plane,
        json!({"sessionId": b, "nodes": [], "idempotencyKey": "release-b"}),
    )
    .unwrap();
    assert!(plane
        .with_comparison(plane.get_session(&a).unwrap().clone())
        .nodes
        .iter()
        .any(|node| node.id.as_str() == "gate" && node.bypass));
    // A fake runtime has no native worker: a refused hold on B must also leave A alone.
    assert!(call(
        &mut plane,
        json!({"sessionId": b, "nodes": ["gate"], "idempotencyKey": "hold-b"})
    )
    .is_err());
    assert!(plane
        .with_comparison(plane.get_session(&a).unwrap().clone())
        .nodes
        .iter()
        .any(|node| node.id.as_str() == "gate" && node.bypass));
    let refused = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.delete".into(),
        params: Some(json!({"sessionId": a, "idempotencyKey": "delete-a"})),
    });
    assert!(refused.error.is_some());
    assert!(plane
        .with_comparison(plane.get_session(&a).unwrap().clone())
        .nodes
        .iter()
        .any(|node| node.id.as_str() == "gate" && node.bypass));
    plane.expire_comparison(Instant::now() + MAX_HOLD);
    assert!(plane
        .with_comparison(plane.get_session(&a).unwrap().clone())
        .nodes
        .iter()
        .all(|node| !node.bypass));
}

#[test]
fn expiration_removes_only_expired_sessions() {
    let mut plane = ControlPlane::new("test");
    let a = goal_session(&mut plane);
    let b = EntityId::new("second");
    plane.duplicate_session(&a, b.clone(), None).unwrap();
    let now = Instant::now();
    for (id, expires_at) in [(&a, now), (&b, now + MAX_HOLD)] {
        plane.comparison.insert(
            id.clone(),
            Comparison {
                nodes: vec![EntityId::new("gate")],
                expires_at,
            },
        );
    }
    plane.expire_comparison(now);
    assert!(!plane.comparison.contains_key(&a));
    assert!(plane.comparison.contains_key(&b));
    assert!(plane
        .with_comparison(plane.get_session(&b).unwrap().clone())
        .nodes
        .iter()
        .any(|node| node.bypass));
    plane.expire_comparison(now + MAX_HOLD);
    assert!(plane.comparison.is_empty());
}

fn call(plane: &mut ControlPlane, params: Value) -> Result<Value, String> {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "nodes.compare".into(),
        params: Some(params),
    });
    match response.error {
        Some(error) => Err(error.message),
        None => Ok(response.result.expect("a result")),
    }
}

fn goal_session(plane: &mut ControlPlane) -> EntityId {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.createFromGoal".into(),
        params: Some(json!({ "goal": "clean-mic", "idempotencyKey": "goal" })),
    });
    EntityId::new(response.result.unwrap()["session"]["id"].as_str().unwrap())
}

#[test]
fn a_stopped_route_cannot_be_compared_and_release_is_always_safe() {
    let mut plane = ControlPlane::new("test");
    let id = goal_session(&mut plane);
    let message = call(
        &mut plane,
        json!({ "sessionId": id, "nodes": ["gate"], "idempotencyKey": "a" }),
    )
    .unwrap_err();
    assert!(message.contains("press Play first"), "{message}");
    assert!(plane.comparison.is_empty());
    let released = call(
        &mut plane,
        json!({ "sessionId": id, "nodes": [], "idempotencyKey": "b" }),
    )
    .unwrap();
    assert_eq!(released["comparing"], json!([]));
}

#[test]
fn sources_and_outputs_cannot_be_left_out() {
    let mut plane = ControlPlane::new("test");
    let id = goal_session(&mut plane);
    for node in ["microphone", "voice-out"] {
        let message = call(
            &mut plane,
            json!({ "sessionId": id, "nodes": [node], "idempotencyKey": node }),
        )
        .unwrap_err();
        assert!(message.contains("cannot be left out"), "{message}");
    }
}

#[test]
fn a_comparison_marks_only_the_chosen_tools_and_is_never_saved() {
    let mut plane = ControlPlane::new("test");
    let id = goal_session(&mut plane);
    plane.comparison.insert(
        id.clone(),
        Comparison {
            nodes: vec![EntityId::new("gate")],
            expires_at: Instant::now() + MAX_HOLD,
        },
    );
    let saved = plane.get_session(&id).unwrap().clone();
    let live = plane.with_comparison(saved.clone());
    let bypassed = |session: &Session| {
        session
            .nodes
            .iter()
            .filter(|node| node.bypass)
            .map(|node| node.id.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(bypassed(&live), ["gate"]);
    assert!(bypassed(plane.get_session(&id).unwrap()).is_empty());
}

#[test]
fn a_hold_that_is_never_released_ends_by_itself() {
    let mut plane = ControlPlane::new("test");
    let id = goal_session(&mut plane);
    let now = Instant::now();
    plane.comparison.insert(
        id.clone(),
        Comparison {
            nodes: vec![EntityId::new("gate")],
            expires_at: now,
        },
    );
    // An expired hold no longer changes what would be published, even before the service tick.
    let saved = plane.get_session(&id).unwrap().clone();
    assert!(plane
        .with_comparison(saved)
        .nodes
        .iter()
        .all(|node| !node.bypass));
    plane.expire_comparison(now);
    assert!(plane.comparison.is_empty());
}

#[test]
fn stopping_a_route_ends_its_comparison() {
    let mut plane = ControlPlane::new("test");
    let id = goal_session(&mut plane);
    plane.comparison.insert(
        id.clone(),
        Comparison {
            nodes: vec![EntityId::new("gate")],
            expires_at: Instant::now() + MAX_HOLD,
        },
    );
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "session.stop".into(),
        params: Some(json!({ "sessionId": id.as_str(), "idempotencyKey": "stop" })),
    });
    // A route that was not running may refuse the stop; the hold must still be gone.
    let _ = response;
    assert!(plane.comparison.is_empty());
}
