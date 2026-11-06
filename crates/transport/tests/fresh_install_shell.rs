//! The real desktop app, started like a fresh install: a new database and no
//! developer opt-in variable. Raw pipe clients must not impersonate its window
//! to approve recording folders or audio-device access (SEC-01/02/04). The app
//! serves its own backend on the default pipe, exactly as installed, so no
//! other AudioRouter may be running (checked first). Opt in with
//! AUDIOROUTER_SHELL_EXE=<path to audiorouter-shell.exe>. The app window
//! opens for a few seconds; no audio device is opened.
//!
//! It also prints how long the embedded backend took to answer its first
//! `status.get` after launch (NFR-14 startup, backend part only; the time
//! until the window's UI is ready is not measured). With
//! AUDIOROUTER_STARTUP_METRICS=<file> it writes `backend_ready_ms=<n>` there.

#[cfg(windows)]
#[test]
#[ignore = "launches the desktop app; set AUDIOROUTER_SHELL_EXE"]
fn a_fresh_install_keeps_device_and_folder_consent_exclusive_to_its_window() {
    use serde_json::{json, Value};
    use std::time::{Duration, Instant};

    let exe = std::env::var("AUDIOROUTER_SHELL_EXE").expect("AUDIOROUTER_SHELL_EXE");
    let pipe = r"\\.\pipe\audiorouter-control";
    let request = |method: &str, params: Value| {
        audiorouter_protocol::encode_frame(
            &json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }),
        )
        .unwrap()
    };
    assert!(
        audiorouter_transport::round_trip(pipe, &request("status.get", json!({}))).is_err(),
        "another AudioRouter is running; close it first"
    );
    let folder =
        std::env::temp_dir().join(format!("audiorouter-fresh-install-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    struct Kill(std::process::Child);
    impl Drop for Kill {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let launched = Instant::now();
    let app = Kill(
        std::process::Command::new(&exe)
            .env("AUDIOROUTER_DATABASE", folder.join("state.sqlite"))
            // A fresh profile too: without this the window reads and rewrites the
            // real install's remembered route and first-run state.
            .env("WEBVIEW2_USER_DATA_FOLDER", folder.join("webview"))
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
        std::thread::sleep(Duration::from_millis(25));
    }
    let backend_ready = launched.elapsed();
    eprintln!(
        "startup: backend answered status.get {} ms after launch",
        backend_ready.as_millis()
    );
    if let Ok(path) = std::env::var("AUDIOROUTER_STARTUP_METRICS") {
        std::fs::write(
            &path,
            format!("backend_ready_ms={}\n", backend_ready.as_millis()),
        )
        .unwrap_or_else(|error| panic!("{path}: {error}"));
    }
    let denied = |response: &Value| {
        response["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("permission denied"))
    };

    // First launch: not yet allowed; preparing devices for Play is refused.
    let access = call("devices.getAccess", json!({}));
    assert_eq!(access["result"]["allowed"], false, "{access}");
    let session = "desktop-session";
    let refused = call("nativePaths.prepare", json!({ "sessionId": session }));
    assert!(
        denied(&refused),
        "fresh install refuses before consent: {refused}"
    );
    // A raw external process must not receive the window's Record scope.
    let root = folder.join("Recordings");
    let set_root = call(
        "recordings.setRoot",
        json!({ "root": root, "create": true, "idempotencyKey": "fresh-root" }),
    );
    assert!(
        denied(&set_root),
        "raw caller cannot choose a recording folder: {set_root}"
    );
    // The real window owns consent; a same-user raw pipe client does not.
    let allowed = call(
        "devices.setAccess",
        json!({ "allowed": true, "idempotencyKey": "fresh-allow" }),
    );
    assert!(
        denied(&allowed),
        "raw client must not give window consent: {allowed}"
    );
    let authorized = call("nativePaths.prepare", json!({ "sessionId": session }));
    assert!(
        denied(&authorized),
        "raw client remains unauthorized: {authorized}"
    );
    eprintln!(
        "raw caller preparation answered: {}",
        authorized["error"]["message"]
    );
    drop(app);
    let _ = std::fs::remove_dir_all(&folder);
}
