//! The real desktop app, started like a fresh install: a new database and no
//! developer opt-in variable. Release 0.0.1 refused Play with "Permission
//! denied" on a second computer because nothing tested this path. The app
//! serves its own backend on the default pipe, exactly as installed, so no
//! other AudioRouter may be running (checked first). Opt in with
//! AUDIOROUTER_SHELL_EXE=<path to audiorouter-shell.exe>. The app window
//! opens for a few seconds; no audio device is opened.

#[cfg(windows)]
#[test]
#[ignore = "launches the desktop app; set AUDIOROUTER_SHELL_EXE"]
fn a_fresh_install_asks_once_for_device_access_then_plays() {
    use serde_json::{json, Value};
    use std::time::{Duration, Instant};

    let exe = std::env::var("AUDIOROUTER_SHELL_EXE").expect("AUDIOROUTER_SHELL_EXE");
    let pipe = r"\\.\pipe\audiorouter-control";
    let request = |method: &str, params: Value| {
        audiorouter_protocol::encode_frame(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })).unwrap()
    };
    assert!(
        audiorouter_transport::round_trip(pipe, &request("status.get", json!({}))).is_err(),
        "another AudioRouter is running; close it first"
    );
    let folder = std::env::temp_dir().join(format!("audiorouter-fresh-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    struct Kill(std::process::Child);
    impl Drop for Kill {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let app = Kill(
        std::process::Command::new(&exe)
            .env("AUDIOROUTER_DATABASE", folder.join("state.sqlite"))
            .env_remove("AUDIOROUTER_CONTROL_PIPE")
            .env_remove("AUDIOROUTER_ALLOW_DEVICE_ADMIN")
            .spawn()
            .expect("start the desktop app"),
    );
    let call = |method: &str, params: Value| -> Value {
        let response = audiorouter_transport::round_trip(pipe, &request(method, params))
            .unwrap_or_else(|error| panic!("{method}: {error:?}"));
        audiorouter_protocol::decode_frame(&response).unwrap()
    };
    // The embedded backend starts with the app.
    let deadline = Instant::now() + Duration::from_secs(30);
    while audiorouter_transport::round_trip(pipe, &request("status.get", json!({}))).is_err() {
        assert!(Instant::now() < deadline, "the app's backend did not start");
        std::thread::sleep(Duration::from_millis(200));
    }
    let denied = |response: &Value| {
        response["error"]["message"].as_str().is_some_and(|message| message.contains("permission denied"))
    };

    // First launch: not yet allowed; preparing devices for Play is refused.
    let access = call("devices.getAccess", json!({}));
    assert_eq!(access["result"]["allowed"], false, "{access}");
    let session = "desktop-session";
    let refused = call("nativePaths.prepare", json!({ "sessionId": session }));
    assert!(denied(&refused), "fresh install refuses before consent: {refused}");
    // The first launch already has the full desktop grant (Record).
    let root = folder.join("Recordings");
    let set_root = call("recordings.setRoot", json!({ "root": root, "create": true, "idempotencyKey": "fresh-root" }));
    assert!(set_root["error"].is_null(), "first launch can choose a recording folder: {set_root}");
    // The user allows once in the window; Play's device step is authorized.
    let allowed = call("devices.setAccess", json!({ "allowed": true, "idempotencyKey": "fresh-allow" }));
    assert_eq!(allowed["result"]["allowed"], true, "{allowed}");
    let authorized = call("nativePaths.prepare", json!({ "sessionId": session }));
    assert!(!denied(&authorized), "authorized after consent (any remaining error is about devices): {authorized}");
    eprintln!("after consent, prepare answered: {}", authorized["error"]["message"]);
    drop(app);
    let _ = std::fs::remove_dir_all(&folder);
}
