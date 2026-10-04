//! Play and stop the saved session without the editor window: the tray's
//! Play/Stop items and autoplay when AudioRouter starts. It performs the same
//! backend calls as the window's Play for a saved route (prepare every path
//! of the session, then start it); the backend then services the audio, so no
//! page is needed. Results are short texts for the tray status line.

use serde_json::{json, Value};

/// One backend call: the method's `result`, or the error message.
pub type Call<'a> = dyn FnMut(&str, Value) -> Result<Value, String> + 'a;

/// Longest tray status text; Windows menu items are single lines.
const MAX_STATUS_CHARS: usize = 90;

fn short(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= MAX_STATUS_CHARS {
        return text.to_owned();
    }
    let mut cut = text.chars().take(MAX_STATUS_CHARS - 1).collect::<String>();
    cut.push('…');
    cut
}

fn idempotency_key(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("{prefix}-{nanos}")
}

fn active_session_ids(call: &mut Call<'_>) -> Result<Vec<String>, String> {
    let status = call("status.get", json!({}))?;
    Ok(status
        .get("activeSessionIds")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default())
}

/// Play the selected (last used) session, as the window's Play does for a
/// saved route. Returns the tray status text either way.
pub fn play_saved_session(call: &mut Call<'_>) -> Result<String, String> {
    let selected = call("sessions.active.get", json!({}))?;
    let Some(session_id) = selected.get("sessionId").and_then(Value::as_str).map(str::to_owned) else {
        return Err("No session to play. Open AudioRouter and create one.".into());
    };
    if active_session_ids(call)?.contains(&session_id) {
        return Ok("Already playing".into());
    }
    // A stopped worker left from an earlier run of this session is detached
    // first, as the window does before preparing again.
    let diagnostics = call("system.diagnostics", json!({}))?;
    if diagnostics.get("nativeSessionId").and_then(Value::as_str) == Some(session_id.as_str()) {
        call("nativeEndpoints.detach", json!({ "sessionId": session_id }))?;
    }
    call("nativePaths.prepare", json!({ "sessionId": session_id })).map_err(|error| {
        if error.contains("permission denied") {
            "Play failed: allow device access in the window first (press Play there once).".to_owned()
        } else {
            short(&format!("Play failed: {error}"))
        }
    })?;
    let started = call("session.start", json!({ "sessionId": session_id, "idempotencyKey": idempotency_key("tray-play") }))
        .map_err(|error| short(&format!("Play failed: {error}")))?;
    if started.get("runtime").and_then(Value::as_str) != Some("native") {
        // Never leave a silent simulated run "playing" (the window does the same).
        let _ = call("session.stop", json!({ "sessionId": session_id, "idempotencyKey": idempotency_key("tray-play-undo") }));
        return Err("Play failed: no device route. Open AudioRouter and choose the devices.".into());
    }
    Ok("Playing".into())
}

/// Stop every playing session. Returns the tray status text.
pub fn stop_playing(call: &mut Call<'_>) -> Result<String, String> {
    let playing = active_session_ids(call)?;
    if playing.is_empty() {
        return Ok("Stopped".into());
    }
    for session_id in playing {
        call("session.stop", json!({ "sessionId": session_id, "idempotencyKey": idempotency_key("tray-stop") }))
            .map_err(|error| short(&format!("Stop failed: {error}")))?;
    }
    Ok("Stopped".into())
}

/// Wait for the backend to answer after launch, then play (autoplay).
pub fn autoplay(call: &mut Call<'_>, attempts: u32, delay: std::time::Duration) -> Result<String, String> {
    for attempt in 0..attempts {
        if call("status.get", json!({})).is_ok() {
            return play_saved_session(call);
        }
        if attempt + 1 < attempts {
            std::thread::sleep(delay);
        }
    }
    Err("Autoplay failed: the audio engine did not start".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scripted backend: answers by method, recording each call.
    struct Backend {
        calls: Vec<String>,
        selected: Value,
        playing: Vec<&'static str>,
        native_session: Option<&'static str>,
        prepare: Result<Value, String>,
        runtime: &'static str,
    }

    impl Backend {
        fn new() -> Self {
            Self { calls: Vec::new(), selected: json!("mine"), playing: Vec::new(), native_session: None, prepare: Ok(json!({})), runtime: "native" }
        }
        fn call(&mut self, method: &str, _params: Value) -> Result<Value, String> {
            self.calls.push(method.to_owned());
            match method {
                "sessions.active.get" => Ok(json!({ "sessionId": self.selected })),
                "status.get" => Ok(json!({ "activeSessionIds": self.playing })),
                "system.diagnostics" => Ok(json!({ "nativeSessionId": self.native_session })),
                "nativeEndpoints.detach" | "session.stop" => Ok(json!({})),
                "nativePaths.prepare" => self.prepare.clone(),
                "session.start" => Ok(json!({ "runtime": self.runtime })),
                other => Err(format!("unexpected {other}")),
            }
        }
    }

    fn play(backend: &mut Backend) -> Result<String, String> {
        play_saved_session(&mut |method: &str, params: Value| backend.call(method, params))
    }

    #[test]
    fn plays_the_selected_session_like_the_window() {
        let mut backend = Backend::new();
        assert_eq!(play(&mut backend), Ok("Playing".into()));
        assert_eq!(backend.calls, ["sessions.active.get", "status.get", "system.diagnostics", "nativePaths.prepare", "session.start"]);
    }

    #[test]
    fn detaches_a_stopped_worker_of_the_same_session_first() {
        let mut backend = Backend::new();
        backend.native_session = Some("mine");
        assert_eq!(play(&mut backend), Ok("Playing".into()));
        assert!(backend.calls.contains(&"nativeEndpoints.detach".to_owned()));
    }

    #[test]
    fn reports_already_playing_and_no_session_without_starting() {
        let mut backend = Backend::new();
        backend.playing = vec!["mine"];
        assert_eq!(play(&mut backend), Ok("Already playing".into()));
        assert!(!backend.calls.contains(&"session.start".to_owned()));
        let mut empty = Backend::new();
        empty.selected = Value::Null;
        assert!(play(&mut empty).unwrap_err().starts_with("No session to play"));
        assert_eq!(empty.calls, ["sessions.active.get"]);
    }

    #[test]
    fn explains_missing_device_consent_and_never_leaves_a_silent_run() {
        let mut denied = Backend::new();
        denied.prepare = Err("permission denied: device administration".into());
        assert!(play(&mut denied).unwrap_err().contains("allow device access"));
        assert!(!denied.calls.contains(&"session.start".to_owned()));
        let mut simulated = Backend::new();
        simulated.runtime = "simulation";
        assert!(play(&mut simulated).unwrap_err().contains("no device route"));
        assert_eq!(simulated.calls.last().map(String::as_str), Some("session.stop"));
        let mut failing = Backend::new();
        failing.prepare = Err("x".repeat(500));
        assert!(play(&mut failing).unwrap_err().chars().count() <= MAX_STATUS_CHARS);
    }

    #[test]
    fn stops_every_playing_session() {
        let mut backend = Backend::new();
        backend.playing = vec!["mine", "other"];
        assert_eq!(stop_playing(&mut |method: &str, params: Value| backend.call(method, params)), Ok("Stopped".into()));
        assert_eq!(backend.calls.iter().filter(|call| *call == "session.stop").count(), 2);
    }

    #[test]
    fn autoplay_waits_for_the_backend_then_gives_up() {
        let mut backend = Backend::new();
        let mut down = 2;
        let result = autoplay(&mut |method: &str, params: Value| {
            if method == "status.get" && down > 0 { down -= 1; return Err("pipe not ready".into()); }
            backend.call(method, params)
        }, 5, std::time::Duration::ZERO);
        assert_eq!(result, Ok("Playing".into()));
        let never = autoplay(&mut |_: &str, _: Value| Err("pipe not ready".into()), 3, std::time::Duration::ZERO);
        assert!(never.unwrap_err().starts_with("Autoplay failed"));
    }
}
