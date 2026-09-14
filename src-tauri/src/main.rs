#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiorouter_control::{ClientGrant, ClientRole, ControlPlane};
use audiorouter_domain::{
    Edge, EntityId, Node, NodeKind, PermissionScope, Port, PortDirection, Session,
};
use audiorouter_protocol::{decode_frame, encode_frame, JsonRpcRequest, JsonRpcResponse};
use audiorouter_storage::Storage;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, State, WebviewUrl, WebviewWindowBuilder,
};

mod startup;

const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\audiorouter-control";
const DEFAULT_DATABASE_DIRECTORY: &str = "AudioRouter";
const DEFAULT_DATABASE_FILE: &str = "state.sqlite";
const DESKTOP_SESSION_ID: &str = "desktop-session";

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
    let response = forward_rpc_request(&request, &state.pipe_name);
    if request.method == "system.describe" {
        if let Some(path) = &state.probe_file {
            // The probe is diagnostic-only. Preserve the production command's
            // Result contract, but serialize transport failures as JSON-RPC
            // errors so the acceptance can distinguish a reached command from
            // a WebView that never executed the initialization script.
            let marker_response = match &response {
                Ok(response) => response.clone(),
                Err(error) => JsonRpcResponse::failure(
                    request.id.clone(),
                    -32000,
                    format!("shell transport probe failed: {error}"),
                ),
            };
            let contents = serde_json::to_vec(&marker_response)
                .map_err(|error| format!("probe response encoding failed: {error}"))?;
            write_probe_marker(path, &contents)?;
        }
    }
    response
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

/// Apply the current user's reversible sign-in registration. This command is
/// deliberately separate from audio/session control and has no elevation
/// path; callers must invoke it explicitly after the startup consent flow.
#[tauri::command]
fn startup_register(enabled: bool) -> Result<String, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("startup executable lookup failed: {error}"))?;
    let command_line = startup::command_line(&executable)?;
    startup::apply(enabled, &executable)?;
    Ok(command_line)
}

#[tauri::command]
fn startup_status() -> Result<&'static str, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("startup executable lookup failed: {error}"))?;
    Ok(if startup::is_registered(&executable)? {
        "registered"
    } else {
        "unregistered"
    })
}

fn session_initialization_script(session_id: &str, frontend_probe: bool) -> String {
    let encoded = serde_json::to_string(session_id).expect("session id is serializable");
    let probe = if frontend_probe {
        "window.__AUDIO_ROUTER_FRONTEND_PROBE__ = true;"
    } else {
        ""
    };
    format!(
        "window.__AUDIO_ROUTER_SESSION_ID__ = {encoded};window.__AUDIO_ROUTER_HOST__ = {{sessionId: {encoded}, transport: {{send: (request) => window.__TAURI_INTERNALS__.invoke('rpc_request', {{request}})}}}};{probe}"
    )
}

fn default_database_path() -> Result<std::path::PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "LOCALAPPDATA is unavailable; set AUDIOROUTER_DATABASE".to_owned())?;
    Ok(std::path::PathBuf::from(root)
        .join(DEFAULT_DATABASE_DIRECTORY)
        .join(DEFAULT_DATABASE_FILE))
}

fn default_desktop_session() -> Session {
    Session {
        id: EntityId::new(DESKTOP_SESSION_ID),
        name: "AudioRouter desktop".into(),
        schema_version: 1,
        revision: 0,
        nodes: vec![
            Node {
                id: EntityId::new("desktop-input"),
                kind: NodeKind::PhysicalInput,
                type_version: 1,
                name: "Physical input".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Output,
                    channels: 2,
                }],
            },
            Node {
                id: EntityId::new("desktop-gain"),
                kind: NodeKind::Gain,
                type_version: 1,
                name: "Neutral gain".into(),
                enabled: true,
                bypass: false,
                parameters: serde_json::Map::from_iter([("gainDb".into(), serde_json::json!(0.0))]),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 2,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 2,
                    },
                ],
            },
            Node {
                id: EntityId::new("desktop-output"),
                kind: NodeKind::PhysicalOutput,
                type_version: 1,
                name: "Physical output".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Input,
                    channels: 2,
                }],
            },
        ],
        edges: vec![
            Edge {
                id: EntityId::new("desktop-input-gain"),
                source_node: EntityId::new("desktop-input"),
                source_port: "main".into(),
                destination_node: EntityId::new("desktop-gain"),
                destination_port: "in".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("desktop-gain-output"),
                source_node: EntityId::new("desktop-gain"),
                source_port: "out".into(),
                destination_node: EntityId::new("desktop-output"),
                destination_port: "main".into(),
                matrix: vec![1.0, 0.0, 0.0, 1.0],
                enabled: true,
            },
        ],
    }
}

#[cfg(windows)]
fn prepare_configured_native_worker(
    plane: &mut ControlPlane,
    session_id: EntityId,
) -> Result<bool, String> {
    let Some(capture_id) = std::env::var_os("AUDIOROUTER_CAPTURE_ENDPOINT_ID") else {
        return Ok(false);
    };
    let Some(render_id) = std::env::var_os("AUDIOROUTER_RENDER_ENDPOINT_ID") else {
        return Err(
            "AUDIOROUTER_CAPTURE_ENDPOINT_ID requires AUDIOROUTER_RENDER_ENDPOINT_ID".into(),
        );
    };
    let capture_id = capture_id
        .to_str()
        .ok_or_else(|| "capture endpoint ID is not valid UTF-8".to_owned())?;
    let render_id = render_id
        .to_str()
        .ok_or_else(|| "render endpoint ID is not valid UTF-8".to_owned())?;
    let endpoints = audiorouter_windows_audio::enumerate_active_endpoints()
        .map_err(|error| format!("endpoint inventory for native worker failed: {error:?}"))?;
    let capture = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.id == capture_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
        })
        .ok_or_else(|| "configured capture endpoint is not an active exact match".to_owned())?;
    let render = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.id == render_id
                && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
        })
        .ok_or_else(|| "configured render endpoint is not an active exact match".to_owned())?;
    plane
        .prepare_native_endpoint_worker(session_id, capture, render, 0, 3, 100)
        .map_err(|error| format!("native endpoint worker preparation failed: {error:?}"))?;
    Ok(true)
}

fn start_owned_backend(pipe_name: &str) -> Result<Option<std::thread::JoinHandle<()>>, String> {
    if std::env::var_os("AUDIOROUTER_CONTROL_PIPE").is_some() {
        return Ok(None);
    }
    #[cfg(not(windows))]
    {
        let _ = pipe_name;
        return Ok(None);
    }
    #[cfg(windows)]
    {
        let database = std::env::var_os("AUDIOROUTER_DATABASE")
            .map(std::path::PathBuf::from)
            .map(Ok)
            .unwrap_or_else(default_database_path)?;
        if !database.is_absolute() {
            return Err("AUDIOROUTER_DATABASE must be an absolute path".into());
        }
        if let Some(parent) = database.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("backend database directory creation failed: {error}"))?;
        }
        let pipe_name = pipe_name.to_owned();
        let handle = std::thread::Builder::new()
            .name("audiorouter-control".into())
            .spawn(move || {
                let result = (|| -> Result<(), String> {
                    // ControlPlane contains COM-backed endpoint state and is
                    // deliberately constructed on the serving thread.
                    let storage = Storage::open(&database)
                        .map_err(|error| format!("backend database open failed: {error:?}"))?;
                    let sid = audiorouter_transport::current_user_sid().map_err(|error| {
                        format!("current user identity lookup failed: {error:?}")
                    })?;
                    let enrollment = storage
                        .load_client_enrollment(&sid)
                        .map_err(|error| format!("backend enrollment lookup failed: {error:?}"))?;
                    if enrollment.as_ref().is_some_and(|(_, revoked)| *revoked) {
                        return Err("the current user enrollment is revoked".into());
                    }
                    let mut plane = ControlPlane::with_storage("desktop-shell", storage);
                    if enrollment.is_none() {
                        // The shell is the user's authenticated local editor:
                        // grant graph/session control on first launch. Device
                        // administration remains outside this built-in role.
                        plane
                            .enroll_client(&sid, ClientRole::Operator)
                            .map_err(|error| {
                                format!("initial operator enrollment failed: {error:?}")
                        })?;
                    }
                    // Rehydrate only the previously persisted, user-approved
                    // startup policy. A disabled policy is intentionally a
                    // no-op so the shell never deletes a value it did not
                    // create; explicit disable goes through the consent flow.
                    let startup = plane
                        .dispatch(JsonRpcRequest {
                            jsonrpc: "2.0".into(),
                            id: Some(serde_json::json!("startup-rehydrate")),
                            method: "startup.get".into(),
                            params: None,
                        })
                        .result
                        .and_then(|result| result.get("enabled").and_then(serde_json::Value::as_bool))
                        .unwrap_or(false);
                    if startup {
                        match std::env::current_exe()
                            .map_err(|error| format!("startup executable lookup failed: {error}"))
                            .and_then(|executable| startup::apply(true, &executable))
                        {
                            Ok(()) => eprintln!("AudioRouter sign-in startup registration rehydrated"),
                            Err(error) => eprintln!("AudioRouter startup registration unavailable: {error}"),
                        }
                    }
                    let session_id = EntityId::new(DESKTOP_SESSION_ID);
                    if plane.get_session(&session_id).is_err() {
                        plane
                            .insert_session(default_desktop_session())
                            .map_err(|error| {
                                format!("default desktop session creation failed: {error:?}")
                            })?;
                    }
                    if std::env::var_os("AUDIOROUTER_CAPTURE_ENDPOINT_ID").is_some() {
                        match prepare_configured_native_worker(&mut plane, session_id.clone()) {
                            Ok(true) => eprintln!("AudioRouter native endpoint worker prepared; session start remains explicit"),
                            Ok(false) => unreachable!("configured native worker returned false"),
                            Err(error) => eprintln!("AudioRouter native endpoint worker unavailable: {error}"),
                        }
                    }
                    let grant = if std::env::var_os("AUDIOROUTER_ALLOW_DEVICE_ADMIN")
                        .is_some_and(|value| value == "1")
                    {
                        if !enrollment
                            .as_ref()
                            .is_some_and(|(role, revoked)| role == "operator" && !revoked)
                        {
                            return Err(
                                "device-administration opt-in requires a non-revoked operator enrollment"
                                    .into(),
                            );
                        }
                        eprintln!("AudioRouter device-administration process grant enabled by explicit opt-in");
                        ClientGrant::with_scopes([
                            PermissionScope::Read,
                            PermissionScope::GraphWrite,
                            PermissionScope::SessionControl,
                            // Startup registration is a separate explicit
                            // capability. The desktop shell exposes it only
                            // to the current user's local control surface;
                            // device administration and capture remain opt-in.
                            PermissionScope::StartupWrite,
                            PermissionScope::DeviceAdministration,
                        ])
                    } else if enrollment
                        .as_ref()
                        .is_some_and(|(role, revoked)| role == "operator" && !revoked)
                    {
                        // The shell is the enrolled operator's local UI. It
                        // may request startup registration explicitly, but
                        // still receives no capture or device-administration
                        // authority through this path.
                        ClientGrant::for_desktop_shell()
                    } else {
                        plane
                            .grant_for_client(&sid)
                            .map_err(|error| format!("current user enrollment lookup failed: {error:?}"))?
                            .ok_or_else(|| "current user is not enrolled".to_owned())?
                    };
                    audiorouter_transport::serve_control_connections_forever_with_grant(
                        &pipe_name, plane, grant,
                    )
                    .map_err(|error| format!("control backend stopped: {error:?}"))
                })();
                if let Err(error) = result {
                    eprintln!("AudioRouter control backend stopped: {error:?}");
                }
            })
            .map_err(|error| format!("backend thread creation failed: {error}"))?;
        Ok(Some(handle))
    }
}

fn tray_status_text(response: &JsonRpcResponse) -> String {
    let Some(result) = response.result.as_ref() else {
        return "Status unavailable".to_owned();
    };
    let active = result
        .get("activeSessionCount")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let muted = result
        .get("privacyMute")
        .and_then(|value| value.get("muted"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    format!(
        "Sessions: {active} active · Mic: {}",
        if muted { "muted" } else { "unmuted" }
    )
}

fn tray_recording_text(response: &JsonRpcResponse) -> String {
    let Some(result) = response.result.as_ref() else {
        return "Live recorders: unavailable".to_owned();
    };
    let Some(items) = result
        .as_array()
        .or_else(|| result.get("items").and_then(serde_json::Value::as_array))
    else {
        return "Live recorders: unavailable".to_owned();
    };
    let count = |state: &str| {
        items
            .iter()
            .filter(|item| item.get("state").and_then(serde_json::Value::as_str) == Some(state))
            .count()
    };
    format!(
        "Live recorders: {} active · {} paused · {} armed · {} failed",
        count("recording"),
        count("paused"),
        count("armed"),
        count("failed")
    )
}

fn tray_stop_succeeded(response: &JsonRpcResponse) -> bool {
    response
        .result
        .as_ref()
        .and_then(|result| result.get("state"))
        .and_then(serde_json::Value::as_str)
        == Some("stopped")
}

fn tray_privacy_muted(response: &JsonRpcResponse) -> Option<bool> {
    response
        .result
        .as_ref()
        .and_then(|result| result.get("privacyMute"))
        .and_then(|mute| mute.get("muted"))
        .and_then(serde_json::Value::as_bool)
}

fn main() {
    let pipe_name =
        std::env::var("AUDIOROUTER_CONTROL_PIPE").unwrap_or_else(|_| DEFAULT_PIPE_NAME.to_owned());
    let state = ShellState {
        pipe_name,
        session_id: DESKTOP_SESSION_ID.to_owned(),
        probe_file: std::env::var_os("AUDIOROUTER_SHELL_PROBE_FILE").map(std::path::PathBuf::from),
    };
    let _backend = start_owned_backend(&state.pipe_name).unwrap_or_else(|error| {
        eprintln!("AudioRouter backend unavailable: {error}");
        None
    });
    let session_script =
        session_initialization_script(&state.session_id, state.probe_file.is_some());
    let tray_pipe_name = state.pipe_name.clone();
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            rpc_request,
            session_id,
            startup_register,
            startup_status
        ])
        .setup(move |app| {
            let session_script = session_script.clone();
            let open = MenuItem::with_id(app, "open", "Open AudioRouter", true, None::<&str>)?;
            let close = MenuItem::with_id(app, "close", "Close window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit and stop audio", true, None::<&str>)?;
            let privacy =
                MenuItem::with_id(app, "privacy", "Toggle privacy mute", true, None::<&str>)?;
            let refresh_status =
                MenuItem::with_id(app, "refresh-status", "Refresh status", true, None::<&str>)?;
            let status =
                MenuItem::with_id(app, "status", "Status unavailable", false, None::<&str>)?;
            let recordings = MenuItem::with_id(
                app,
                "recordings",
                "Live recorders: unavailable",
                false,
                None::<&str>,
            )?;
            let pipe_name = tray_pipe_name.clone();
            let status_for_handler = status.clone();
            let recordings_for_handler = recordings.clone();
            let menu = Menu::with_items(
                app,
                &[
                    &open,
                    &close,
                    &quit,
                    &privacy,
                    &refresh_status,
                    &status,
                    &recordings,
                ],
            )?;
            TrayIconBuilder::with_id("audiorouter")
                .menu(&menu)
                .tooltip("AudioRouter")
                .on_menu_event(move |app, event| {
                    let Some(window) = app.get_webview_window("main") else {
                        return;
                    };
                    match event.id().as_ref() {
                        "open" => {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                        "close" => {
                            let _ = window.hide();
                        }
                        "quit" => {
                            let request = JsonRpcRequest {
                                jsonrpc: "2.0".into(),
                                id: Some(serde_json::json!("tray-quit-stop")),
                                method: "session.stop".into(),
                                params: Some(serde_json::json!({
                                    "sessionId": DESKTOP_SESSION_ID
                                })),
                            };
                            match forward_rpc_request(&request, &pipe_name) {
                                Ok(response) if tray_stop_succeeded(&response) => app.exit(0),
                                _ => {
                                    let _ = status_for_handler
                                        .set_text("Quit refused: session stop did not complete");
                                }
                            }
                        }
                        "privacy" => {
                            let status_request = JsonRpcRequest {
                                jsonrpc: "2.0".into(),
                                id: Some(serde_json::json!("tray-privacy-status")),
                                method: "status.get".into(),
                                params: None,
                            };
                            let Some(currently_muted) =
                                forward_rpc_request(&status_request, &pipe_name)
                                    .ok()
                                    .and_then(|response| tray_privacy_muted(&response))
                            else {
                                let _ = status_for_handler.set_text("Privacy mute unavailable");
                                return;
                            };
                            let request = JsonRpcRequest {
                                jsonrpc: "2.0".into(),
                                id: Some(serde_json::json!("tray-privacy-toggle")),
                                method: "safety.setPrivacyMute".into(),
                                params: Some(serde_json::json!({
                                    "muted": !currently_muted,
                                    "idempotencyKey": format!(
                                        "tray-privacy-{}",
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|duration| duration.as_nanos())
                                            .unwrap_or_default()
                                    )
                                })),
                            };
                            match forward_rpc_request(&request, &pipe_name)
                                .ok()
                                .and_then(|response| response.result)
                                .and_then(|result| {
                                    result.get("muted").and_then(serde_json::Value::as_bool)
                                }) {
                                Some(muted) => {
                                    let _ = status_for_handler.set_text(if muted {
                                        "Privacy mute enabled"
                                    } else {
                                        "Privacy mute disabled"
                                    });
                                }
                                None => {
                                    let _ =
                                        status_for_handler.set_text("Privacy mute change refused");
                                }
                            }
                        }
                        "refresh-status" => {
                            let request = JsonRpcRequest {
                                jsonrpc: "2.0".into(),
                                id: Some(serde_json::json!("tray-status")),
                                method: "status.get".into(),
                                params: None,
                            };
                            let text = forward_rpc_request(&request, &pipe_name)
                                .map(|response| tray_status_text(&response))
                                .unwrap_or_else(|_| "Status unavailable".to_owned());
                            let _ = status_for_handler.set_text(text);
                            let recordings_request = JsonRpcRequest {
                                jsonrpc: "2.0".into(),
                                id: Some(serde_json::json!("tray-recorders")),
                                method: "recorders.list".into(),
                                params: None,
                            };
                            let text = forward_rpc_request(&recordings_request, &pipe_name)
                                .map(|response| tray_recording_text(&response))
                                .unwrap_or_else(|_| "Live recorders: unavailable".to_owned());
                            let _ = recordings_for_handler.set_text(text);
                        }
                        _ => {}
                    }
                })
                .build(app)?;
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
    use audiorouter_domain::validate_session;
    use serde_json::json;

    #[test]
    fn session_initialization_script_uses_json_string_encoding() {
        let script = session_initialization_script("shell\";window.pwned=true;\\escape", false);
        assert!(script.starts_with(
            r#"window.__AUDIO_ROUTER_SESSION_ID__ = "shell\";window.pwned=true;\\escape";"#
        ));
        assert!(script.contains("window.__AUDIO_ROUTER_HOST__"));
    }

    #[test]
    fn default_desktop_session_is_a_valid_stopped_stereo_graph() {
        let session = default_desktop_session();
        assert_eq!(session.id.as_str(), DESKTOP_SESSION_ID);
        assert_eq!(session.revision, 0);
        assert_eq!(session.nodes.len(), 3);
        assert_eq!(session.edges.len(), 2);
        assert_eq!(session.nodes[1].kind, NodeKind::Gain);
        assert_eq!(
            session.nodes[1].parameters["gainDb"],
            serde_json::json!(0.0)
        );
        assert!(validate_session(&session).is_ok());
    }

    #[test]
    fn default_desktop_session_runs_through_the_neutral_gain_stage() {
        let session = default_desktop_session();
        let graph = audiorouter_engine::compile_session_at_sample_rate(
            &session,
            audiorouter_engine::RuntimeGeneration::new(1),
            48_000,
        )
        .expect("fresh desktop graph should compile");
        let mut block = audiorouter_engine::AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        block.channel_mut(1).unwrap().fill(-0.5);
        graph.process(&mut block);
        assert!(block.all_finite());
        assert!(block
            .channel(0)
            .unwrap()
            .iter()
            .all(|sample| (*sample - 0.25).abs() < 1.0e-6));
        assert!(block
            .channel(1)
            .unwrap()
            .iter()
            .all(|sample| (*sample + 0.5).abs() < 1.0e-6));
    }

    #[test]
    fn optional_frontend_probe_is_not_present_by_default() {
        let script = session_initialization_script("probe", false);
        assert!(!script.contains("shell-probe"));
    }

    #[test]
    fn optional_frontend_probe_uses_the_native_command() {
        let script = session_initialization_script("probe", true);
        assert!(script.contains("window.__AUDIO_ROUTER_FRONTEND_PROBE__ = true"));
    }

    #[test]
    fn tray_status_uses_authoritative_session_and_privacy_fields() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(json!("tray-status")),
            result: Some(json!({ "activeSessionCount": 2, "privacyMute": { "muted": true } })),
            error: None,
        };
        assert_eq!(
            tray_status_text(&response),
            "Sessions: 2 active · Mic: muted"
        );
        let unavailable = JsonRpcResponse {
            result: None,
            ..response
        };
        assert_eq!(tray_status_text(&unavailable), "Status unavailable");
    }

    #[test]
    fn tray_recording_status_counts_each_authoritative_state() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(json!("tray-recorders")),
            result: Some(json!([
                { "state": "recording" }, { "state": "recording" },
                { "state": "paused" }, { "state": "armed" }, { "state": "failed" }
            ])),
            error: None,
        };
        assert_eq!(
            tray_recording_text(&response),
            "Live recorders: 2 active · 1 paused · 1 armed · 1 failed"
        );
    }

    #[test]
    fn tray_quit_requires_an_authoritative_stopped_response() {
        let stopped = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(json!("tray-quit-stop")),
            result: Some(json!({ "state": "stopped" })),
            error: None,
        };
        assert!(tray_stop_succeeded(&stopped));
        assert!(!tray_stop_succeeded(&JsonRpcResponse {
            result: Some(json!({ "state": "failed" })),
            ..stopped.clone()
        }));
        assert!(!tray_stop_succeeded(&JsonRpcResponse {
            result: None,
            ..stopped
        }));
    }

    #[test]
    fn tray_privacy_state_requires_authoritative_status_shape() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(json!("tray-privacy-status")),
            result: Some(json!({ "privacyMute": { "muted": true } })),
            error: None,
        };
        assert_eq!(tray_privacy_muted(&response), Some(true));
        assert_eq!(
            tray_privacy_muted(&JsonRpcResponse {
                result: Some(json!({ "privacyMute": {} })),
                ..response.clone()
            }),
            None
        );
        assert_eq!(
            tray_privacy_muted(&JsonRpcResponse {
                result: None,
                ..response
            }),
            None
        );
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
