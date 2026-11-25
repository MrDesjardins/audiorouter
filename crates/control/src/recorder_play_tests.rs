//! API-07 idempotency rules for runtime actions, through the real dispatcher.
//!
//! A same-key retry reveals the outcome that was recorded, and must never
//! perform the side effect again. That holds inside one backend lifetime even
//! when the live route has since been stopped or started under another key.
//! Across a restart the journal survives but the runtime does not, so a
//! recorded "running" outcome cannot be replayed or re-executed: the request is
//! refused as an idempotency conflict and the caller sends a new request.

use super::*;

fn tone_recorder_session() -> Session {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/installer-recorder-session.json"
    ))
    .expect("installer recorder fixture parses")
}

fn dispatch(plane: &mut ControlPlane, method: &str, params: Value) -> JsonRpcResponse {
    plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    })
}

fn ok(plane: &mut ControlPlane, method: &str, params: Value) -> Value {
    let response = dispatch(plane, method, params);
    assert!(
        response.error.is_none(),
        "{method} failed: {:?}",
        response.error
    );
    response.result.expect("a successful response has a result")
}

fn error_code(plane: &mut ControlPlane, method: &str, params: Value) -> String {
    let response = dispatch(plane, method, params);
    let error = response
        .error
        .expect("the request must be refused, not executed");
    assert!(response.result.is_none(), "a refusal has no result");
    error
        .data
        .as_ref()
        .and_then(|data| data["code"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn playing(plane: &mut ControlPlane) -> bool {
    ok(
        plane,
        "sessions.summary",
        json!({ "sessionId": "qualification-recorder" }),
    )["playing"]
        .as_bool()
        .expect("summary reports playing")
}

fn start(plane: &mut ControlPlane, key: &str) -> Value {
    ok(
        plane,
        "sessions.start",
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": key }),
    )
}

fn play(plane: &mut ControlPlane, key: &str) -> Value {
    ok(
        plane,
        "sessions.play",
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": key }),
    )
}

fn toggle(plane: &mut ControlPlane, key: &str) -> Value {
    ok(
        plane,
        "sessions.togglePlay",
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": key }),
    )
}

fn stop(plane: &mut ControlPlane, key: &str) -> Value {
    ok(
        plane,
        "sessions.stop",
        json!({ "sessionId": "qualification-recorder", "idempotencyKey": key }),
    )
}

fn database(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-{name}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    remove_database(&path);
    path
}

fn remove_database(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("sqlite-wal"));
    let _ = std::fs::remove_file(path.with_extension("sqlite-shm"));
}

fn open(path: &std::path::Path) -> ControlPlane {
    ControlPlane::with_storage(
        "recorder-rules",
        Storage::open(path).expect("storage opens"),
    )
}

#[test]
fn a_same_lifetime_replay_after_stop_reveals_the_outcome_without_restarting_audio() {
    let mut plane = ControlPlane::default();
    plane
        .insert_session(tone_recorder_session())
        .expect("fixture saved");
    let first = play(&mut plane, "play-k");
    assert_eq!(first["state"], "running", "{first}");
    stop(&mut plane, "stop-k");
    assert!(!playing(&mut plane), "Stop must stop the route");
    let replay = play(&mut plane, "play-k");
    assert_eq!(
        replay, first,
        "a same-key retry must replay the recorded result"
    );
    assert!(
        !playing(&mut plane),
        "a same-key retry must not start the route again after Stop"
    );
}

#[test]
fn a_replayed_toggle_does_not_stop_a_route_started_under_another_key() {
    let mut plane = ControlPlane::default();
    plane
        .insert_session(tone_recorder_session())
        .expect("fixture saved");
    // Start without preparation so the toggle below only stops a route; a
    // generated route's prepared worker cannot be re-prepared after a stop.
    start(&mut plane, "start-k");
    let toggled = toggle(&mut plane, "toggle-k");
    assert_eq!(toggled["state"], "stopped", "{toggled}");
    let started = play(&mut plane, "play-again");
    assert_eq!(started["state"], "running", "{started}");
    assert!(playing(&mut plane));
    let replay = toggle(&mut plane, "toggle-k");
    assert_eq!(
        replay, toggled,
        "a same-key retry must replay the recorded result"
    );
    assert!(
        playing(&mut plane),
        "a replayed toggle must not stop a route started under another key"
    );
}

#[test]
fn a_replayed_toggle_does_not_start_a_route_stopped_under_another_key() {
    let mut plane = ControlPlane::default();
    plane
        .insert_session(tone_recorder_session())
        .expect("fixture saved");
    let toggled = toggle(&mut plane, "toggle-k");
    assert_eq!(toggled["state"], "running", "{toggled}");
    stop(&mut plane, "stop-k");
    assert!(!playing(&mut plane));
    let replay = toggle(&mut plane, "toggle-k");
    assert_eq!(
        replay, toggled,
        "a same-key retry must replay the recorded result"
    );
    assert!(
        !playing(&mut plane),
        "a replayed toggle must not start a route stopped under another key"
    );
}

#[test]
fn a_replayed_start_keeps_its_recorded_generation() {
    let mut plane = ControlPlane::default();
    plane
        .insert_session(tone_recorder_session())
        .expect("fixture saved");
    let first = start(&mut plane, "start-a");
    let first_generation = first["generation"].as_u64().expect("first generation");
    stop(&mut plane, "stop-a");
    let second = start(&mut plane, "start-b");
    assert_ne!(second["generation"].as_u64(), Some(first_generation));
    assert!(playing(&mut plane));
    let replay = start(&mut plane, "start-a");
    assert_eq!(
        replay, first,
        "a same-key retry must replay the recorded generation, not the live one"
    );
    assert!(playing(&mut plane), "the live route keeps playing");
}

#[test]
fn a_replayed_play_from_an_earlier_lifetime_is_refused_then_a_new_key_plays() {
    let path = database("replay-restart");
    {
        let mut plane = open(&path);
        plane
            .insert_session(tone_recorder_session())
            .expect("fixture saved");
        let first = play(&mut plane, "play-k");
        assert_eq!(first["state"], "running", "{first}");
        stop(&mut plane, "stop-k");
    }
    let mut restarted = open(&path);
    assert!(!playing(&mut restarted), "a restart loses the runtime");
    assert_eq!(
        error_code(
            &mut restarted,
            "sessions.play",
            json!({ "sessionId": "qualification-recorder", "idempotencyKey": "play-k" })
        ),
        "idempotencyConflict",
        "an earlier-lifetime replay must be refused"
    );
    assert!(
        !playing(&mut restarted),
        "the refused request starts nothing"
    );
    let fresh = play(&mut restarted, "play-fresh");
    assert_eq!(fresh["state"], "running", "{fresh}");
    assert!(playing(&mut restarted));
    drop(restarted);
    remove_database(&path);
}

#[test]
fn a_replayed_play_across_a_restart_after_stop_is_refused() {
    let path = database("replay-restart-after-stop");
    {
        let mut plane = open(&path);
        plane
            .insert_session(tone_recorder_session())
            .expect("fixture saved");
        play(&mut plane, "play-k");
    }
    let mut restarted = open(&path);
    assert_eq!(
        error_code(
            &mut restarted,
            "sessions.play",
            json!({ "sessionId": "qualification-recorder", "idempotencyKey": "play-k" })
        ),
        "idempotencyConflict"
    );
    assert!(!playing(&mut restarted));
    drop(restarted);
    remove_database(&path);
}

#[test]
fn a_replayed_play_from_an_earlier_lifetime_is_refused_even_when_another_key_is_running() {
    let path = database("replay-restart-running");
    {
        let mut plane = open(&path);
        plane
            .insert_session(tone_recorder_session())
            .expect("fixture saved");
        play(&mut plane, "play-k");
    }
    let mut restarted = open(&path);
    let fresh = play(&mut restarted, "play-fresh");
    assert_eq!(fresh["state"], "running", "{fresh}");
    assert!(playing(&mut restarted));
    assert_eq!(
        error_code(
            &mut restarted,
            "sessions.play",
            json!({ "sessionId": "qualification-recorder", "idempotencyKey": "play-k" })
        ),
        "idempotencyConflict",
        "a stale outcome is refused even while a different request plays the route"
    );
    assert!(playing(&mut restarted));
    drop(restarted);
    remove_database(&path);
}

#[test]
fn a_replayed_recorder_start_from_an_earlier_lifetime_is_refused() {
    let path = database("replay-recorder-start");
    let root = std::env::temp_dir().join(format!(
        "audiorouter-replay-recordings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    {
        let mut plane = open(&path);
        plane
            .insert_session(tone_recorder_session())
            .expect("fixture saved");
        ok(
            &mut plane,
            "recordings.setRoot",
            json!({
                "root": root.to_string_lossy(),
                "create": true,
                "idempotencyKey": "record-root"
            }),
        );
        play(&mut plane, "play-k");
        let started = ok(
            &mut plane,
            "recorders.startRecording",
            json!({
                "sessionId": "qualification-recorder",
                "nodeId": "recorder",
                "idempotencyKey": "record-k"
            }),
        );
        assert!(
            matches!(started["state"].as_str(), Some("recording" | "armed")),
            "{started}"
        );
    }
    let mut restarted = open(&path);
    assert_eq!(
        error_code(
            &mut restarted,
            "recorders.startRecording",
            json!({
                "sessionId": "qualification-recorder",
                "nodeId": "recorder",
                "idempotencyKey": "record-k"
            })
        ),
        "idempotencyConflict",
        "an earlier-lifetime recording cannot be reported as running"
    );
    drop(restarted);
    remove_database(&path);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_replayed_stop_from_an_earlier_lifetime_is_refused_when_the_route_plays_again() {
    let path = database("replay-stop-running");
    {
        let mut plane = open(&path);
        plane
            .insert_session(tone_recorder_session())
            .expect("fixture saved");
        play(&mut plane, "play-k");
        stop(&mut plane, "stop-k");
    }
    let mut restarted = open(&path);
    play(&mut restarted, "play-fresh");
    assert!(playing(&mut restarted));
    assert_eq!(
        error_code(
            &mut restarted,
            "sessions.stop",
            json!({ "sessionId": "qualification-recorder", "idempotencyKey": "stop-k" })
        ),
        "idempotencyConflict",
        "a stale stopped outcome must not be returned while the route plays"
    );
    assert!(playing(&mut restarted));
    drop(restarted);
    remove_database(&path);
}

#[test]
fn a_replayed_stop_from_an_earlier_lifetime_replays_while_nothing_plays() {
    let path = database("replay-stop-idle");
    {
        let mut plane = open(&path);
        plane
            .insert_session(tone_recorder_session())
            .expect("fixture saved");
        play(&mut plane, "play-k");
        stop(&mut plane, "stop-k");
    }
    let mut restarted = open(&path);
    assert_eq!(
        ok(
            &mut restarted,
            "sessions.stop",
            json!({ "sessionId": "qualification-recorder", "idempotencyKey": "stop-k" })
        )["state"],
        "stopped"
    );
    assert!(!playing(&mut restarted));
    drop(restarted);
    remove_database(&path);
}
