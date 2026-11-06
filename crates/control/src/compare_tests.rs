//! Hold-to-compare rules that need no audio device.

use super::*;

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
    assert!(plane.comparison.is_none());
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
    plane.comparison = Some(Comparison {
        session: id.clone(),
        nodes: vec![EntityId::new("gate")],
        expires_at: Instant::now() + MAX_HOLD,
    });
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
    plane.comparison = Some(Comparison {
        session: id.clone(),
        nodes: vec![EntityId::new("gate")],
        expires_at: now,
    });
    // An expired hold no longer changes what would be published, even before the service tick.
    let saved = plane.get_session(&id).unwrap().clone();
    assert!(plane
        .with_comparison(saved)
        .nodes
        .iter()
        .all(|node| !node.bypass));
    plane.expire_comparison(now);
    assert!(plane.comparison.is_none());
}

#[test]
fn stopping_a_route_ends_its_comparison() {
    let mut plane = ControlPlane::new("test");
    let id = goal_session(&mut plane);
    plane.comparison = Some(Comparison {
        session: id.clone(),
        nodes: vec![EntityId::new("gate")],
        expires_at: Instant::now() + MAX_HOLD,
    });
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "session.stop".into(),
        params: Some(json!({ "sessionId": id.as_str(), "idempotencyKey": "stop" })),
    });
    // A route that was not running may refuse the stop; the hold must still be gone.
    let _ = response;
    assert!(plane.comparison.is_none());
}
