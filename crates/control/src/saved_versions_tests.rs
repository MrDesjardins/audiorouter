//! Saved versions through JSON-RPC dispatch.

use super::*;

fn call(plane: &mut ControlPlane, method: &str, params: Value) -> Result<Value, String> {
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    });
    match response.error {
        Some(error) => Err(error.message),
        None => Ok(response.result.expect("a result")),
    }
}

fn save_renamed(plane: &mut ControlPlane, id: &str, name: &str) -> u64 {
    let current = call(plane, "sessions.get", json!({ "sessionId": id })).unwrap();
    let session = current.get("session").unwrap_or(&current).clone();
    let base = session["revision"].as_u64().unwrap();
    let mut candidate = session;
    candidate["name"] = json!(name);
    let plan = call(
        plane,
        "graph.plan",
        json!({ "sessionId": id, "baseRevision": base, "candidate": candidate }),
    )
    .unwrap();
    let committed = call(
        plane,
        "graph.commit",
        json!({ "planId": plan["planId"], "baseRevision": base, "idempotencyKey": format!("save-{name}") }),
    )
    .unwrap();
    committed["revision"].as_u64().unwrap()
}

fn session_with_goal(plane: &mut ControlPlane) -> String {
    let created = call(
        plane,
        "sessions.createFromGoal",
        json!({ "goal": "clean-mic", "idempotencyKey": "goal" }),
    )
    .unwrap();
    created["session"]["id"].as_str().unwrap().to_owned()
}

#[test]
fn versions_list_what_was_saved_with_when_and_never_invent_a_saver() {
    let mut plane = ControlPlane::new("test");
    let id = session_with_goal(&mut plane);
    let revision = save_renamed(&mut plane, &id, "One");
    let versions = call(&mut plane, "graph.versions", json!({ "sessionId": id })).unwrap();
    let items = versions["items"].as_array().unwrap();
    let saved = items
        .iter()
        .find(|item| item["revision"] == revision)
        .unwrap();
    assert_eq!(saved["name"], "One");
    assert_eq!(saved["toolCount"], 5);
    assert!(saved["savedAtMs"].as_u64().unwrap() > 0);
    // Unauthenticated dispatch has no client identity to report.
    assert!(saved["savedBy"].is_null());
}

#[test]
fn going_back_is_a_new_save_and_keeps_later_versions() {
    let mut plane = ControlPlane::new("test");
    let id = session_with_goal(&mut plane);
    let first = save_renamed(&mut plane, &id, "First");
    let second = save_renamed(&mut plane, &id, "Second");
    let plan = call(
        &mut plane,
        "graph.restorePlan",
        json!({ "sessionId": id, "baseRevision": second, "revision": first }),
    )
    .unwrap();
    let committed = call(
        &mut plane,
        "graph.commit",
        json!({ "planId": plan["planId"], "baseRevision": second, "idempotencyKey": "back" }),
    )
    .unwrap();
    let current = call(&mut plane, "sessions.get", json!({ "sessionId": id })).unwrap();
    let current = current.get("session").unwrap_or(&current);
    assert_eq!(current["name"], "First");
    assert!(committed["revision"].as_u64().unwrap() > second);
    let versions = call(&mut plane, "graph.versions", json!({ "sessionId": id })).unwrap();
    assert!(versions["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["revision"] == second));
}

#[test]
fn refused_delete_preserves_saved_version_notes() {
    let mut plane = ControlPlane::with_storage("test", Storage::open_memory().unwrap());
    let id = session_with_goal(&mut plane);
    save_renamed(&mut plane, &id, "Keep my notes");
    let before = call(&mut plane, "graph.versions", json!({ "sessionId": id })).unwrap();
    plane.session_start(&EntityId::new(&id)).unwrap();
    let error = call(
        &mut plane,
        "sessions.delete",
        json!({ "sessionId": id, "idempotencyKey": "refused-delete" }),
    )
    .unwrap_err();
    assert!(error.contains("stop the session"), "{error}");
    let after = call(&mut plane, "graph.versions", json!({ "sessionId": id })).unwrap();
    assert_eq!(after, before);
    // Reopening must retain the durable note too, not just its cached copy.
    let mut reopened = ControlPlane::with_storage("test", plane.storage.take().unwrap());
    assert_eq!(
        call(&mut reopened, "graph.versions", json!({ "sessionId": id })).unwrap(),
        before
    );
}

#[test]
fn an_unknown_or_current_version_is_refused_in_plain_words() {
    let mut plane = ControlPlane::new("test");
    let id = session_with_goal(&mut plane);
    let revision = save_renamed(&mut plane, &id, "One");
    let message = call(
        &mut plane,
        "graph.restorePlan",
        json!({ "sessionId": id, "baseRevision": revision, "revision": revision }),
    )
    .unwrap_err();
    assert!(message.contains("already at that version"), "{message}");
    let message = call(
        &mut plane,
        "graph.restorePlan",
        json!({ "sessionId": id, "baseRevision": revision, "revision": 9_000 }),
    )
    .unwrap_err();
    assert!(message.contains("no longer kept"), "{message}");
}
