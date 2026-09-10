#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiorouter_protocol::{decode_frame, encode_frame, JsonRpcRequest, JsonRpcResponse};
use tauri::{State, WebviewUrl, WebviewWindowBuilder};

const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\audiorouter-control";

#[derive(Clone)]
struct ShellState {
    pipe_name: String,
    session_id: String,
    probe_file: Option<std::path::PathBuf>,
}

#[tauri::command]
fn rpc_request(
    request: JsonRpcRequest,
    state: State<'_, ShellState>,
) -> Result<JsonRpcResponse, String> {
    let response = forward_rpc_request(&request, &state.pipe_name)?;
    if request.method == "system.describe" {
        if let Some(path) = &state.probe_file {
            let contents = serde_json::to_vec(&response)
                .map_err(|error| format!("probe response encoding failed: {error}"))?;
            write_probe_marker(path, &contents)?;
        }
    }
    Ok(response)
}

fn write_probe_marker(path: &std::path::Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "probe marker has no parent directory".to_owned())?;
    let metadata = std::fs::symlink_metadata(parent)
        .map_err(|error| format!("probe marker parent inspection failed: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("probe marker parent must be a regular directory".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("probe marker create failed: {error}"))?;
    use std::io::Write;
    file.write_all(contents)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("probe marker write failed: {error}"))
}

fn forward_rpc_request(
    request: &JsonRpcRequest,
    pipe_name: &str,
) -> Result<JsonRpcResponse, String> {
    let frame =
        encode_frame(&request).map_err(|error| format!("request encoding failed: {error}"))?;

    #[cfg(windows)]
    let response_frame = audiorouter_transport::round_trip(pipe_name, &frame)
        .map_err(|error| format!("control pipe request failed: {error}"))?;
    #[cfg(not(windows))]
    let response_frame = {
        return Err("native shell transport is only available on Windows".into());
    };

    decode_frame(&response_frame).map_err(|error| format!("response decoding failed: {error}"))
}

#[tauri::command]
fn session_id(state: State<'_, ShellState>) -> String {
    state.session_id.clone()
}

fn session_initialization_script(session_id: &str, frontend_probe: bool) -> String {
    let encoded = serde_json::to_string(session_id).expect("session id is serializable");
    let probe = if frontend_probe {
        "window.__TAURI_INTERNALS__.invoke('rpc_request',{request:{jsonrpc:'2.0',id:'shell-probe',method:'system.describe'}});"
    } else {
        ""
    };
    format!(
        "window.__AUDIO_ROUTER_SESSION_ID__ = {encoded};window.__AUDIO_ROUTER_HOST__ = {{sessionId: {encoded}, transport: {{send: (request) => window.__TAURI_INTERNALS__.invoke('rpc_request', {{request}})}}}};{probe}"
    )
}

fn main() {
    let pipe_name =
        std::env::var("AUDIOROUTER_CONTROL_PIPE").unwrap_or_else(|_| DEFAULT_PIPE_NAME.to_owned());
    let state = ShellState {
        pipe_name,
        session_id: format!("tauri-shell-{}", std::process::id()),
        probe_file: std::env::var_os("AUDIOROUTER_SHELL_PROBE_FILE").map(std::path::PathBuf::from),
    };
    let session_script =
        session_initialization_script(&state.session_id, state.probe_file.is_some());
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![rpc_request, session_id])
        .setup(move |app| {
            let session_script = session_script.clone();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("AudioRouter")
                .inner_size(1280.0, 800.0)
                .resizable(true)
                .initialization_script(session_script.clone())
                .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Closing the editor must not tear down an independently owned
                // backend/audio process. A future tray Quit action will use an
                // explicit stop/finalize path before allowing application exit.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AudioRouter shell");
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_control::ControlPlane;
    use serde_json::json;

    #[test]
    fn session_initialization_script_uses_json_string_encoding() {
        let script = session_initialization_script("shell\";window.pwned=true;\\escape", false);
        assert!(script.starts_with(
            r#"window.__AUDIO_ROUTER_SESSION_ID__ = "shell\";window.pwned=true;\\escape";"#
        ));
        assert!(script.contains("window.__AUDIO_ROUTER_HOST__"));
        assert!(script.contains("window.__TAURI_INTERNALS__.invoke('rpc_request', {request})"));
    }

    #[test]
    fn optional_frontend_probe_is_not_present_by_default() {
        let script = session_initialization_script("probe", false);
        assert!(!script.contains("shell-probe"));
    }

    #[test]
    fn optional_frontend_probe_uses_the_native_command() {
        let script = session_initialization_script("probe", true);
        assert!(script.contains("id:'shell-probe'"));
        assert!(script.contains("method:'system.describe'"));
    }

    #[test]
    fn probe_marker_is_create_only() {
        let path =
            std::env::temp_dir().join(format!("audiorouter-shell-marker-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        write_probe_marker(&path, br#"{"ok":true}"#).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), br#"{"ok":true}"#);
        assert!(write_probe_marker(&path, b"replacement").is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn forwards_a_request_through_the_authenticated_control_server() {
        let pipe_name = format!(r"\\.\pipe\audiorouter-shell-forward-{}", std::process::id());
        let server = {
            let pipe_name = pipe_name.clone();
            std::thread::spawn(move || {
                audiorouter_transport::serve_control_connections_as_role(
                    &pipe_name,
                    1,
                    ControlPlane::new("shell-forward-test"),
                    audiorouter_control::ClientRole::Observer,
                )
                .unwrap();
            })
        };
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "system.describe".into(),
            params: None,
        };
        let response = forward_rpc_request(&request, &pipe_name).unwrap();
        assert!(response.error.is_none());
        assert_eq!(response.result.unwrap()["protocolVersion"]["major"], 1);
        server.join().unwrap();
    }
}
