#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiorouter_protocol::{decode_frame, encode_frame, JsonRpcRequest, JsonRpcResponse};
use tauri::{Manager, State};

const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\audiorouter-control";

#[derive(Clone)]
struct ShellState {
    pipe_name: String,
    session_id: String,
}

#[tauri::command]
fn rpc_request(
    request: JsonRpcRequest,
    state: State<'_, ShellState>,
) -> Result<JsonRpcResponse, String> {
    let frame =
        encode_frame(&request).map_err(|error| format!("request encoding failed: {error}"))?;

    #[cfg(windows)]
    let response_frame = audiorouter_transport::round_trip(&state.pipe_name, &frame)
        .map_err(|error| format!("control pipe request failed: {error}"))?;
    #[cfg(not(windows))]
    let response_frame = {
        let _ = state;
        return Err("native shell transport is only available on Windows".into());
    };

    decode_frame(&response_frame).map_err(|error| format!("response decoding failed: {error}"))
}

#[tauri::command]
fn session_id(state: State<'_, ShellState>) -> String {
    state.session_id.clone()
}

fn main() {
    let pipe_name =
        std::env::var("AUDIOROUTER_CONTROL_PIPE").unwrap_or_else(|_| DEFAULT_PIPE_NAME.to_owned());
    let state = ShellState {
        pipe_name,
        session_id: format!("tauri-shell-{}", std::process::id()),
    };
    let session_script =
        serde_json::to_string(&state.session_id).expect("session id is serializable");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![rpc_request, session_id])
        .setup(move |app| {
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| "main shell window was not created".to_owned())?;
            window.eval(&format!(
                "window.__AUDIO_ROUTER_SESSION_ID__ = {session_script};"
            ))?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AudioRouter shell");
}
