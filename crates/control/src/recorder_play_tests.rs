//! Regression: a device-free Recorder route must keep playing after control
//! restart. The installed-app harness played this route, restarted the app
//! after an uninstall/reinstall, and then saw `sessions.play` report running
//! while `sessions.summary` reported not playing and `audioSources.transport`
//! refused. Reproduce that sequence through the real dispatcher.

use super::*;

fn recorder_session() -> Session {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/installer-recorder-session.json"
    ))
    .expect("installer recorder fixture parses")
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

fn play_summarize_and_transport(plane: &mut ControlPlane, key: &str) {
    let played = call(
        plane,
        "sessions.play",
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": key }),
    )
    .expect("sessions.play succeeds");
    assert_eq!(played["state"], "running", "{played}");
    let summary = call(
        plane,
        "sessions.summary",
        json!({ "sessionId": "qualification-recorder" }),
    )
    .expect("sessions.summary succeeds");
    assert_eq!(
        summary["playing"], true,
        "Play must not report running for a route the summary calls stopped: {summary}"
    );
    let tone = call(
        plane,
        "audioSources.transport",
        json!({ "sessionId": "qualification-recorder", "nodeId": "tone", "action": "play" }),
    )
    .expect("the Tone of a running route is controllable");
    assert_eq!(tone["state"], "playing", "{tone}");
    let stopped = call(
        plane,
        "sessions.stop",
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": format!("{key}-stop") }),
    )
    .expect("sessions.stop succeeds");
    assert_eq!(stopped["state"], "stopped", "{stopped}");
}

fn remove_database(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
}

#[test]
fn device_free_recorder_route_plays_across_a_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-recorder-play-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    remove_database(&path);
    let storage = Storage::open(&path).expect("first storage opens");
    let mut plane = ControlPlane::with_storage("recorder-play", storage);
    plane
        .insert_session(recorder_session())
        .expect("fixture saved");
    play_summarize_and_transport(&mut plane, "play-before-restart");
    drop(plane);

    // A relaunched control plane loads the retained session from its database.
    let storage = Storage::open(&path).expect("storage reopens");
    let mut restarted = ControlPlane::with_storage("recorder-play", storage);
    let listed = call(&mut restarted, "sessions.list", json!({})).expect("sessions list succeeds");
    assert_eq!(
        listed["items"][0]["id"], "qualification-recorder",
        "the retained session must still be loaded: {listed}"
    );
    play_summarize_and_transport(&mut restarted, "play-after-restart");
    drop(restarted);
    remove_database(&path);
}

#[test]
fn a_replayed_play_after_a_restart_does_not_report_a_stopped_route_as_running() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-recorder-replay-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    remove_database(&path);
    let play =
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": "qualification-play" });
    let storage = Storage::open(&path).expect("first storage opens");
    let mut plane = ControlPlane::with_storage("recorder-replay", storage);
    plane
        .insert_session(recorder_session())
        .expect("fixture saved");
    let first = call(&mut plane, "sessions.play", play.clone()).expect("first play succeeds");
    assert_eq!(first["state"], "running", "{first}");
    drop(plane);

    // The idempotency journal survived the restart, but the runtime did not.
    // Replaying the same request must start the route again, not answer with a
    // cached "running" for a session the summary calls stopped.
    let storage = Storage::open(&path).expect("storage reopens");
    let mut restarted = ControlPlane::with_storage("recorder-replay", storage);
    let replay = call(&mut restarted, "sessions.play", play).expect("replayed play succeeds");
    let summary = call(
        &mut restarted,
        "sessions.summary",
        json!({ "sessionId": "qualification-recorder" }),
    )
    .expect("summary succeeds");
    assert_eq!(
        summary["playing"], true,
        "a replayed Play must not report running while nothing plays (reply: {replay}, summary: {summary})"
    );
    call(
        &mut restarted,
        "audioSources.transport",
        json!({ "sessionId": "qualification-recorder", "nodeId": "tone", "action": "play" }),
    )
    .expect("the Tone of the restarted route is controllable");
    drop(restarted);
    remove_database(&path);
}
