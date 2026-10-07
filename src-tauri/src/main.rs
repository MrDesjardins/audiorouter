#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use audiorouter_control::{ClientGrant, ClientRole, ControlPlane};
use audiorouter_domain::{
    Edge, EntityId, Node, NodeKind, PermissionScope, Port, PortDirection, Session,
};
use audiorouter_protocol::{decode_frame, encode_frame, JsonRpcRequest, JsonRpcResponse};
use audiorouter_storage::Storage;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder,
};

#[cfg(windows)]
mod api_token;
mod backend_supervisor;
mod http_api;
#[cfg(windows)]
mod instance_windows;
mod lan_addresses;
#[cfg(windows)]
mod os_transition_windows;
#[cfg(windows)]
mod plugin_editor_windows;
#[cfg(windows)]
mod session_file_dialog;
mod shell_settings;
mod startup;
mod tray_playback;

use backend_supervisor::{BackendRestartDecision, BackendSupervisor};

const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\audiorouter-control";
const DEFAULT_DATABASE_DIRECTORY: &str = "AudioRouter";
const DEFAULT_DATABASE_FILE: &str = "state.sqlite";
const DESKTOP_SESSION_ID: &str = "desktop-session";
const DIAGNOSTIC_LOG_LIMIT_BYTES: u64 = 5 * 1024 * 1024;
/// WebView2 arguments: wry's defaults (setting any arguments replaces them)
/// plus a 256 MB V8 old-space cap. While audio plays the UI makes short-lived
/// garbage continuously; uncapped, V8 deferred full collection on a PC with
/// free RAM and the page grew to ~770 MB around a 13 MB live heap. Capped,
/// it settled near 420 MB with garbage collection at about 1% of the main
/// thread (2026-10-04 measurement, active plan item 7).
const WEBVIEW_BROWSER_ARGS: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --js-flags=--max-old-space-size=256";

/// Append bounded, privacy-conscious control-flow diagnostics for failures
/// that cannot be inspected through the WebView console in an attended shell.
/// Request parameters and audio/media data are never written to this log.
fn log_shell_rpc(request: &JsonRpcRequest, response: &Result<JsonRpcResponse, String>) {
    log_shell_rpc_with(request, response, false);
}

/// `from_panic_hook` makes the in-process lock a single try: a panic raised
/// while this thread already holds it would otherwise deadlock.
fn log_shell_rpc_with(
    request: &JsonRpcRequest,
    response: &Result<JsonRpcResponse, String>,
    from_panic_hook: bool,
) {
    if matches!(
        request.method.as_str(),
        "nativeBridges.heartbeat"
            | "nativeEndpoints.pump"
            | "nativeDuplex.pump"
            | "nativeRenderSources.pump"
            | "nativeMultiInputs.pump"
    ) {
        return;
    }
    static LOG_LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let Some(root) = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(std::path::PathBuf::from))
    else {
        return;
    };
    let directory = root.join(DEFAULT_DATABASE_DIRECTORY).join("logs");
    let lock = LOG_LOCK.get_or_init(|| std::sync::Mutex::new(()));
    let _guard = if from_panic_hook {
        match lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => return,
        }
    } else {
        let Ok(guard) = lock.lock() else { return };
        guard
    };
    let Some(_cross_process_guard) =
        audiorouter_transport::acquire_diagnostic_mutex("AudioRouter.ShellDiagnostics")
    else {
        return;
    };
    write_shell_rpc_log(&directory, request, response);
}

fn write_shell_rpc_log(
    directory: &std::path::Path,
    request: &JsonRpcRequest,
    response: &Result<JsonRpcResponse, String>,
) {
    use std::io::Write;
    // Timer-driven reads succeed many times a second; logging their
    // successes would rotate useful events out of the 5 MB file in minutes.
    if audiorouter_transport::is_routine_read_rpc(&request.method)
        && matches!(response, Ok(value) if value.error.is_none())
    {
        return;
    }
    let path = directory.join("shell.jsonl");
    if std::fs::create_dir_all(directory).is_err() {
        return;
    }
    if std::fs::metadata(&path)
        .map(|metadata| metadata.len() >= DIAGNOSTIC_LOG_LIMIT_BYTES)
        .unwrap_or(false)
    {
        let previous = directory.join("shell.previous.jsonl");
        let _ = std::fs::remove_file(&previous);
        let _ = std::fs::rename(&path, previous);
    }
    let (outcome, detail) = match response {
        Ok(response) => {
            if let Some(error) = response.error.as_ref() {
                ("error", {
                    let mut detail = audiorouter_transport::rpc_failure_log_summary(
                        &serde_json::to_value(error).unwrap_or_default(),
                    );
                    detail["reason"] = serde_json::json!(if matches!(
                        request.method.as_str(),
                        "session.start" | "sessions.start" | "sessions.play"
                    ) && error
                        .message
                        .contains("native graph rejected: UnsupportedTopology")
                    {
                        Some("UnsupportedTopology")
                    } else {
                        None
                    });
                    detail
                })
            } else {
                let result = response.result.as_ref();
                let summary = match request.method.as_str() {
                    "graph.commit" => result.map(audiorouter_transport::graph_activation_log_summary),
                    "devices.list" => result.map(audiorouter_transport::device_inventory_log_summary),
                    "shell.panic" => result.cloned(),
                    "sessions.get" => result.map(|value| serde_json::json!({
                        "revision": value.get("revision"),
                        "nodes": value.get("nodes").and_then(serde_json::Value::as_array).map(Vec::len),
                        "edges": value.get("edges").and_then(serde_json::Value::as_array).map(Vec::len),
                    })),
                    "sessions.list" => result.map(|value| {
                        let items = value.get("items").and_then(serde_json::Value::as_array);
                        serde_json::json!({
                            "sessionCount": items.map(Vec::len),
                            "graphs": items.map(|items| items.iter().map(|item| serde_json::json!({
                                "revision": item.get("revision"),
                                "nodes": item.get("nodes").and_then(serde_json::Value::as_array).map(Vec::len),
                                "edges": item.get("edges").and_then(serde_json::Value::as_array).map(Vec::len),
                            })).collect::<Vec<_>>()),
                        })
                    }),
                    _ => result.map(|value| serde_json::json!({
                        "state": value.get("state"),
                        "generation": value.get("generation"),
                    })),
                };
                ("ok", summary.unwrap_or(serde_json::Value::Null))
            }
        }
        Err(_) => (
            "transportError",
            serde_json::json!({"kind": "transportError"}),
        ),
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let entry = serde_json::json!({
        "timeUnixMs": now_ms,
        "processId": std::process::id(),
        "version": env!("CARGO_PKG_VERSION"),
        "buildId": option_env!("AUDIOROUTER_BUILD_ID").unwrap_or("development"),
        "method": request.method.chars().take(96).collect::<String>(),
        "sessionId": request.params.as_ref().and_then(|params| params.get("sessionId")).and_then(serde_json::Value::as_str).filter(|id| id.len() <= 128 && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))),
        "outcome": outcome,
        "detail": detail,
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = serde_json::to_writer(&mut file, &entry);
        let _ = file.write_all(b"\n");
        let _ = file.flush();
    }
}

fn audio_router_tray_icon(muted: bool) -> Image<'static> {
    // Keep the tray asset local and deterministic: an audio waveform on a
    // dark rounded tile remains legible at the small Windows notification-area
    // sizes without depending on an external file or installer path.
    // Privacy mute is a safety-relevant latch (see safety.setPrivacyMute);
    // toggling it previously had no glanceable feedback beyond a menu-item
    // label the user had to reopen the tray to read. The bar color switches
    // cyan/live -> red/muted so the taskbar icon itself reflects the current
    // state without opening the menu.
    const SIZE: u32 = 32;
    let bar_color: [u8; 4] = if muted {
        [233, 107, 107, 255]
    } else {
        [68, 204, 235, 255]
    };
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let bars = [5_u32, 9, 14, 20, 25, 29];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let edge = x.min(y).min(SIZE - 1 - x).min(SIZE - 1 - y);
            let rounded = edge >= 3
                || ((x.abs_diff(3) + y.abs_diff(3)) <= 3)
                || ((x.abs_diff(28) + y.abs_diff(3)) <= 3)
                || ((x.abs_diff(3) + y.abs_diff(28)) <= 3)
                || ((x.abs_diff(28) + y.abs_diff(28)) <= 3);
            let mut pixel = if rounded {
                [20, 30, 42, 255]
            } else {
                [0, 0, 0, 0]
            };
            if rounded
                && bars
                    .iter()
                    .any(|bar| x.abs_diff(*bar) <= 1 && y.abs_diff(16) <= (x.abs_diff(16) / 3 + 3))
            {
                pixel = bar_color;
            }
            rgba.extend_from_slice(&pixel);
        }
    }
    Image::new_owned(rgba, SIZE, SIZE)
}

fn transition_operation_key(sequence: usize) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "shell-os-transition-{}-{timestamp}-{sequence}",
        std::process::id()
    )
}

/// The grant the desktop shell's backend serves with, from the current
/// user's enrollment (after first-launch enrollment) and the developer
/// opt-in. An operator gets the desktop grant, which can open audio devices
/// once the user consents in the app (`devices.setAccess`). The
/// `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` opt-in grants device administration
/// up front. `None` means another role: use its enrolled grant.
fn select_shell_grant(
    device_admin_opt_in: bool,
    enrollment: Option<&(String, bool)>,
) -> Result<Option<ClientGrant>, String> {
    let operator = enrollment.is_some_and(|(role, revoked)| role == "operator" && !revoked);
    if device_admin_opt_in {
        if !operator {
            return Err(
                "device-administration opt-in requires a non-revoked operator enrollment".into(),
            );
        }
        return Ok(Some(ClientGrant::with_scopes([
            PermissionScope::Read,
            PermissionScope::GraphWrite,
            PermissionScope::SessionControl,
            // Explicitly requested recording in approved roots; device
            // capture stays separate.
            PermissionScope::Record,
            // Startup registration, local to this user's control surface.
            PermissionScope::StartupWrite,
            // Metadata scans of chosen plugin folders (authorized 2026-09-25).
            PermissionScope::PluginScan,
            PermissionScope::DeviceAdministration,
        ])));
    }
    Ok(operator.then(ClientGrant::for_desktop_shell))
}

#[derive(Clone)]
struct ShellState {
    pipe_name: String,
    session_id: String,
    probe_file: Option<std::path::PathBuf>,
    database_path: std::path::PathBuf,
}

#[derive(Default)]
struct HttpApiState(std::sync::Mutex<Option<http_api::HttpApi>>);

/// The project release page for one tag (UI-18), opened in the default
/// browser. Only `vMAJOR.MINOR.PATCH` tags of this repository are accepted.
fn release_page_url(tag: &str) -> Option<String> {
    let mut parts = tag.strip_prefix('v')?.split('.');
    let valid = (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty() && part.len() <= 6 && part.bytes().all(|byte| byte.is_ascii_digit())
        })
    }) && parts.next().is_none();
    valid.then(|| format!("https://github.com/MrDesjardins/audiorouter/releases/tag/{tag}"))
}

#[tauri::command]
fn open_release_page(tag: String) -> Result<(), String> {
    let url = release_page_url(&tag).ok_or("Not an AudioRouter release tag")?;
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let wide = url.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        // SAFETY: only this repository's release URL for a validated numeric tag is
        // opened. Both strings are NUL-terminated and stay alive throughout
        // ShellExecuteW; no user-controlled command, path or argument is passed.
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(wide.as_ptr()),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(format!("Cannot open the browser. Visit {url}"));
        }
    }
    #[cfg(not(windows))]
    let _ = url;
    Ok(())
}

/// This PC's private IPv4 addresses the API may also listen on (HTTP-09).
#[tauri::command]
fn http_api_addresses() -> Result<Vec<lan_addresses::LanAddress>, String> {
    lan_addresses::list()
}

/// `network` is one of this PC's private addresses for local-network access,
/// or absent for this PC only. It must be listed by `http_api_addresses`.
fn http_api_network(network: Option<&str>) -> Result<Option<std::net::Ipv4Addr>, String> {
    let Some(network) = network.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let address = network
        .parse::<std::net::Ipv4Addr>()
        .map_err(|_| "Choose a network address from the list")?;
    if !lan_addresses::list()?
        .iter()
        .any(|entry| entry.address == address)
    {
        return Err(format!(
            "This PC no longer has the address {address}. Choose another network."
        ));
    }
    Ok(Some(address))
}

fn start_http_api(
    port: u16,
    lan: Option<std::net::Ipv4Addr>,
    pipe: &str,
) -> Result<http_api::HttpApi, String> {
    let token = api_token::load_or_create(&api_token::default_path()?)?;
    let pipe = pipe.to_owned();
    http_api::HttpApi::start_on(
        port,
        token,
        lan,
        std::sync::Arc::new(move |request| forward_rpc_request(request, &pipe)),
    )
}

/// Remember the port and network the API started with, for auto-start.
fn remember_http_api(
    database_path: &std::path::Path,
    api: shell_settings::ApiListener,
) -> Result<(), String> {
    let path = shell_settings::path_beside(database_path);
    let mut settings = shell_settings::load(&path);
    settings.api = Some(api);
    shell_settings::save(&path, &settings)
}

/// API auto-start (Advanced): controllers such as the Stream Deck plugin
/// otherwise go offline after every restart until the user presses Start in
/// the API panel. Uses the last port and network; local-network access
/// resumes only on an address this PC still has, otherwise this PC only.
fn auto_start_http_api(
    database_path: &std::path::Path,
    pipe: &str,
) -> Result<Option<http_api::HttpApi>, String> {
    let settings = shell_settings::load(&shell_settings::path_beside(database_path));
    if !settings.api_auto_start {
        return Ok(None);
    }
    let api = settings.api.unwrap_or(shell_settings::ApiListener {
        port: 17891,
        network: None,
    });
    let lan = http_api_network(api.network.as_deref()).unwrap_or(None);
    start_http_api(api.port, lan, pipe).map(Some)
}

/// Whether the local API starts when AudioRouter starts (Advanced).
#[tauri::command]
fn api_autostart_get(state: State<'_, ShellState>) -> bool {
    shell_settings::load(&shell_settings::path_beside(&state.database_path)).api_auto_start
}

#[tauri::command]
fn api_autostart_set(enabled: bool, state: State<'_, ShellState>) -> Result<bool, String> {
    let path = shell_settings::path_beside(&state.database_path);
    let mut settings = shell_settings::load(&path);
    settings.api_auto_start = enabled;
    shell_settings::save(&path, &settings)?;
    Ok(enabled)
}

#[tauri::command]
fn http_api_control(
    action: String,
    port: Option<u16>,
    network: Option<String>,
    state: State<'_, ShellState>,
    api: State<'_, HttpApiState>,
) -> Result<serde_json::Value, String> {
    let mut listener = api.0.lock().map_err(|_| "API state unavailable")?;
    match action.as_str() {
        "start" => {
            if listener.is_none() {
                let lan = http_api_network(network.as_deref())?;
                let started = start_http_api(port.unwrap_or(17891), lan, &state.pipe_name)?;
                remember_http_api(
                    &state.database_path,
                    shell_settings::ApiListener {
                        port: started.port,
                        network: started.lan.map(|address| address.to_string()),
                    },
                )?;
                *listener = Some(started);
            }
        }
        "stop" => {
            *listener = None;
        }
        "regenerate" => {
            let token = api_token::generate()?;
            api_token::save(&api_token::default_path()?, &token)?;
            if let Some(previous) = listener.take() {
                let (active_port, lan) = (previous.port, previous.lan);
                drop(previous);
                let pipe = state.pipe_name.clone();
                *listener = Some(http_api::HttpApi::start_on(
                    active_port,
                    token,
                    lan,
                    std::sync::Arc::new(move |request| forward_rpc_request(request, &pipe)),
                )?);
            }
        }
        "openDocs" => {
            let listener = listener
                .as_ref()
                .ok_or("Start the API before opening documentation")?;
            #[cfg(windows)]
            {
                use windows::core::{w, PCWSTR};
                use windows::Win32::UI::Shell::ShellExecuteW;
                use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
                let url = format!("http://127.0.0.1:{}/docs", listener.port)
                    .encode_utf16()
                    .chain(Some(0))
                    .collect::<Vec<_>>();
                // SAFETY: only our active loopback URL is opened. Both strings are
                // NUL-terminated and remain owned/alive throughout ShellExecuteW;
                // no user-controlled command, path or argument is passed.
                let result = unsafe {
                    ShellExecuteW(
                        None,
                        w!("open"),
                        PCWSTR(url.as_ptr()),
                        None,
                        None,
                        SW_SHOWNORMAL,
                    )
                };
                if result.0 as isize <= 32 {
                    return Err(
                        "Cannot open the browser. Copy the API URL and append /docs.".into(),
                    );
                }
            }
        }
        "status" | "reveal" => {}
        _ => return Err("Unknown API action".into()),
    }
    let revealed_token = if action == "reveal" || action == "regenerate" {
        Some(match listener.as_ref() {
            Some(listener) => listener.token.clone(),
            None => api_token::load_or_create(&api_token::default_path()?)?,
        })
    } else {
        None
    };
    Ok(match listener.as_ref() {
        Some(listener) => {
            serde_json::json!({ "running": true, "port": listener.port, "url": format!("http://127.0.0.1:{}", listener.port), "network": listener.lan.map(|address| address.to_string()), "networkUrl": listener.lan.map(|address| format!("http://{address}:{}", listener.port)), "token": revealed_token })
        }
        None => {
            serde_json::json!({ "running": false, "port": port.unwrap_or(17891), "url": null, "network": null, "networkUrl": null, "token": revealed_token })
        }
    })
}

#[tauri::command]
fn rpc_request(
    request: JsonRpcRequest,
    state: State<'_, ShellState>,
) -> Result<JsonRpcResponse, String> {
    let response = forward_rpc_request(&request, &state.pipe_name);
    log_shell_rpc(&request, &response);
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

/// The window's Quit button: the same backend finalization as the tray's
/// "Quit and stop audio" (stop audio, finish recordings), then exit. A refused
/// quit leaves everything running and returns the reason to the window.
#[tauri::command]
fn quit_app(app: tauri::AppHandle, state: State<'_, ShellState>) -> Result<(), String> {
    // A fresh key per click: a refused quit must not replay on retry.
    let key = format!(
        "window-quit-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    );
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(serde_json::json!("window-quit")),
        method: "system.quit".into(),
        params: Some(serde_json::json!({ "idempotencyKey": key })),
    };
    let response = forward_rpc_request(&request, &state.pipe_name);
    log_shell_rpc(&request, &response);
    let response = response?;
    if response
        .result
        .as_ref()
        .and_then(|result| result.get("state"))
        .and_then(serde_json::Value::as_str)
        == Some("stopped")
    {
        app.exit(0);
        return Ok(());
    }
    Err(response
        .error
        .map(|error| error.message)
        .unwrap_or_else(|| "backend finalization failed".into()))
}

/// Whether the editor page holds route edits that are not saved yet. The
/// page reports every change; a fresh page starts clean.
#[derive(Default)]
struct UiUnsaved(std::sync::atomic::AtomicBool);

#[tauri::command]
fn set_ui_unsaved(unsaved: bool, flag: State<'_, UiUnsaved>) {
    flag.0.store(unsaved, std::sync::atomic::Ordering::Relaxed);
}

/// The page's initialization script, kept to recreate the editor window.
struct MainWindowScript(String);

/// One backend call through the shell's own pipe connection, for the tray
/// and autoplay (no window needed): the `result`, or the error message.
fn backend_call(
    pipe_name: &str,
) -> impl FnMut(&str, serde_json::Value) -> Result<serde_json::Value, String> + '_ {
    move |method, params| {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(format!("tray-{method}"))),
            method: method.into(),
            params: Some(params),
        };
        let response = forward_rpc_request(&request, pipe_name);
        log_shell_rpc(&request, &response);
        let response = response?;
        match (response.result, response.error) {
            (_, Some(error)) => Err(error.message),
            (Some(result), None) => Ok(result),
            (None, None) => Err("empty backend response".into()),
        }
    }
}

/// Whether the selected session plays when AudioRouter starts (Advanced).
#[tauri::command]
fn autoplay_get(state: State<'_, ShellState>) -> bool {
    shell_settings::load(&shell_settings::path_beside(&state.database_path)).auto_play
}

#[tauri::command]
fn autoplay_set(enabled: bool, state: State<'_, ShellState>) -> Result<bool, String> {
    let path = shell_settings::path_beside(&state.database_path);
    let mut settings = shell_settings::load(&path);
    settings.auto_play = enabled;
    shell_settings::save(&path, &settings)?;
    Ok(enabled)
}

/// `--tray`: started at sign-in; stay in the tray without building a window.
fn starts_in_tray(arguments: impl IntoIterator<Item = String>) -> bool {
    arguments
        .into_iter()
        .skip(1)
        .any(|argument| argument == startup::TRAY_ARGUMENT)
}

#[derive(Debug, PartialEq, Eq)]
enum CloseAction {
    Hide,
    Release,
}

/// Closing the editor to the tray frees the WebView (its page, GPU and
/// browser processes, several hundred MB) unless the page holds unsaved route
/// edits, which only a hidden page keeps. Audio is owned by this process's
/// backend, so it keeps playing either way; tray Open builds a fresh page.
fn close_action(unsaved: bool) -> CloseAction {
    if unsaved {
        CloseAction::Hide
    } else {
        CloseAction::Release
    }
}

/// Only an explicit exit (Quit, which passes a code after finalizing) ends
/// the app; the last window closing leaves the backend running in the tray.
fn allow_exit(code: Option<i32>) -> bool {
    code.is_some()
}

/// Show the editor window, building a new one when it was released.
fn open_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return Ok(());
    }
    let script = app.state::<MainWindowScript>().0.clone();
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title(format!("AudioRouter {}", env!("CARGO_PKG_VERSION")))
        .inner_size(1280.0, 800.0)
        .resizable(true)
        // Keep the editor visible on first launch. The tray's Open
        // action can still hide/show this same window; relying on a
        // framework default here made packaged-shell diagnostics
        // ambiguous when no static window entry existed in the
        // configuration.
        .visible(true)
        // Tauri's native drag-drop handler swallows HTML5 drag events
        // in WebView2, so dragging a tool onto the canvas showed no
        // preview and dropped nothing. The UI has no OS file drops.
        .disable_drag_drop_handler()
        .additional_browser_args(WEBVIEW_BROWSER_ARGS)
        .initialization_script(script)
        .build()?;
    Ok(())
}

/// Open a playing plugin node's own editor in a native window owned by this
/// shell. The backend authorizes the window for this process and asks the
/// isolated worker to create the editor inside it; closing the window closes
/// the editor and applies its edits to the audio instance.
#[tauri::command]
fn open_plugin_editor(
    session_id: String,
    node_id: String,
    title: String,
    state: State<'_, ShellState>,
) -> Result<JsonRpcResponse, String> {
    #[cfg(windows)]
    {
        let request = |method: &str, extra: serde_json::Value| {
            let mut params = serde_json::json!({ "sessionId": session_id, "nodeId": node_id });
            if let (Some(params), Some(extra)) = (params.as_object_mut(), extra.as_object()) {
                params.extend(extra.clone());
            }
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(format!("plugin-editor-{method}"))),
                method: method.into(),
                params: Some(params),
            }
        };
        let opened = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let close_request = request("plugins.closeEditor", serde_json::json!({}));
        let close_pipe = state.pipe_name.clone();
        let close_opened = std::sync::Arc::clone(&opened);
        let title = if title.trim().is_empty() {
            "Plugin editor".to_owned()
        } else {
            title.chars().take(120).collect()
        };
        let window = plugin_editor_windows::open_host_window(
            &format!("{title} — AudioRouter"),
            800,
            600,
            Box::new(move || {
                if close_opened.load(std::sync::atomic::Ordering::Acquire) {
                    let response = forward_rpc_request(&close_request, &close_pipe);
                    log_shell_rpc(&close_request, &response);
                }
            }),
        )?;
        let open_request = request(
            "plugins.openEditor",
            serde_json::json!({ "parentWindow": window as u64, "ownerProcessId": std::process::id() }),
        );
        let response = forward_rpc_request(&open_request, &state.pipe_name);
        log_shell_rpc(&open_request, &response);
        match &response {
            Ok(result) if result.error.is_none() => {
                opened.store(true, std::sync::atomic::Ordering::Release)
            }
            _ => plugin_editor_windows::close_host_window(window),
        }
        response
    }
    #[cfg(not(windows))]
    {
        let _ = (session_id, node_id, title, state);
        Err("plugin editors are only available on Windows".into())
    }
}

/// Show the native Save (`mode == "save"`) or Open dialog for a
/// `.audiorouter` session file and return the chosen path, or `None` when
/// cancelled. The backend reads and writes the file itself.
#[tauri::command]
async fn choose_session_file(
    window: tauri::WebviewWindow,
    mode: String,
    suggested_name: Option<String>,
) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        let owner = window.hwnd().map(|hwnd| hwnd.0 as isize).unwrap_or(0);
        let suggested: String = suggested_name
            .unwrap_or_default()
            .chars()
            .filter(|c| {
                !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
                    && !c.is_control()
            })
            .take(120)
            .collect();
        let suggested = if suggested.trim().is_empty() {
            "AudioRouter session".to_owned()
        } else {
            suggested
        };
        session_file_dialog::choose(mode == "save", &format!("{suggested}.audiorouter"), owner)
            .map(|path| path.map(|path| path.to_string_lossy().into_owned()))
    }
    #[cfg(not(windows))]
    {
        let _ = (window, mode, suggested_name);
        Err("session file dialogs are only available on Windows".into())
    }
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

#[tauri::command]
fn mcp_activity_list() -> Result<Vec<serde_json::Value>, String> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(std::path::PathBuf::from))
        .ok_or_else(|| "local application-data directory is unavailable".to_owned())?;
    let directory = root.join(DEFAULT_DATABASE_DIRECTORY).join("logs");
    let mut records = Vec::new();
    for name in ["mcp-activity.previous.jsonl", "mcp-activity.jsonl"] {
        let path = directory.join(name);
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 256 * 1024 {
            continue;
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("MCP activity log read failed: {error}"))?;
        for line in contents.lines() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if value.get("tool").is_some() && value.get("outcome").is_some() {
                    records.push(value);
                }
            }
        }
    }
    Ok(records.into_iter().rev().take(100).collect())
}

#[tauri::command]
fn backend_diagnostics_list() -> Result<Vec<serde_json::Value>, String> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(std::path::PathBuf::from))
        .ok_or_else(|| "local application-data directory is unavailable".to_owned())?;
    let directory = root.join(DEFAULT_DATABASE_DIRECTORY).join("logs");
    let mut records = Vec::new();
    for name in ["backend.previous.jsonl", "backend.jsonl"] {
        let path = directory.join(name);
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > DIAGNOSTIC_LOG_LIMIT_BYTES
        {
            continue;
        }
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("backend diagnostics read failed: {error}"))?;
        for line in contents.lines() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                records.push(value);
            }
        }
    }
    Ok(records.into_iter().rev().take(100).collect())
}

fn log_directory() -> Result<std::path::PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("TEMP"))
        .map(|root| {
            std::path::PathBuf::from(root)
                .join(DEFAULT_DATABASE_DIRECTORY)
                .join("logs")
        })
        .filter(|path| path.is_absolute())
        .ok_or_else(|| "The logs folder is unavailable on this PC.".into())
}

#[tauri::command]
fn log_folder_path() -> Result<String, String> {
    Ok(log_directory()?.to_string_lossy().into_owned())
}

#[tauri::command]
fn open_logs_folder() -> Result<(), String> {
    let directory = log_directory()?;
    std::fs::create_dir_all(&directory)
        .map_err(|_| "Could not create the logs folder. Check access to your local app data.")?;
    // Fixed executable and one fixed app-owned directory; no shell expansion or
    // caller-supplied path. Explorer is deliberately visible at the user's click.
    let explorer = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .filter(|root| root.is_absolute())
        .ok_or("Windows Explorer is unavailable.")?
        .join("explorer.exe");
    std::process::Command::new(explorer)
        .arg(directory)
        .spawn()
        .map_err(|_| {
            "Could not open the logs folder. Use Copy folder path and paste it into File Explorer."
        })?;
    Ok(())
}

/// File name of the Stream Deck plugin the release installer bundles as a
/// resource (`tauri.release.conf.json`).
const STREAMDECK_PLUGIN_FILE: &str = "com.mrdesjardins.audiorouter.streamDeckPlugin";

/// Hand the bundled Stream Deck plugin to the Stream Deck app, which registers
/// the `.streamDeckPlugin` extension and asks the user to confirm the install.
#[tauri::command]
fn install_streamdeck_plugin(app: tauri::AppHandle) -> Result<(), String> {
    let plugin = app
        .path()
        .resource_dir()
        .map(|directory| directory.join(STREAMDECK_PLUGIN_FILE))
        .ok()
        .filter(|path| path.is_file())
        .ok_or("This copy of AudioRouter does not include the Stream Deck plugin. Download it from the release page.")?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let wide = plugin
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // SAFETY: the path is the app's own fixed resource file, not caller input.
        // Both strings are NUL-terminated and stay alive throughout
        // ShellExecuteW; the default handler (Stream Deck) opens it.
        let result = unsafe {
            ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(wide.as_ptr()),
                None,
                None,
                SW_SHOWNORMAL,
            )
        };
        // SE_ERR_NOASSOC (31): nothing handles .streamDeckPlugin files.
        if result.0 as isize == 31 {
            return Err(
                "Install the Stream Deck app (version 7.1 or newer) first, then try again.".into(),
            );
        }
        if result.0 as isize <= 32 {
            return Err(
                "Could not open the Stream Deck plugin. Is the Stream Deck app installed?".into(),
            );
        }
    }
    #[cfg(not(windows))]
    let _ = plugin;
    Ok(())
}

#[tauri::command]
fn mcp_setup_info(
    app: tauri::AppHandle,
    state: State<'_, ShellState>,
) -> Result<serde_json::Value, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("shell executable lookup failed: {error}"))?;
    let sibling_cli = executable.with_file_name("audiorouter-cli.exe");
    let resource_cli = app
        .path()
        .resource_dir()
        .ok()
        .map(|directory| directory.join("audiorouter-cli.exe"));
    let cli = resource_cli
        .filter(|path| path.is_file())
        .unwrap_or(sibling_cli);
    Ok(serde_json::json!({
        "cliPath": cli.to_string_lossy(),
        "cliAvailable": cli.is_file(),
        "databasePath": state.database_path.to_string_lossy(),
        "pipeName": state.pipe_name,
        "transport": "local named pipe",
    }))
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

#[cfg(windows)]
fn record_backend_failure(database: &std::path::Path) -> Option<usize> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let storage = Storage::open(database).ok()?;
    storage.record_recovery_crash(timestamp).ok()
}

#[cfg(windows)]
fn recovery_safe_mode(database: &std::path::Path) -> Option<bool> {
    Storage::open(database).ok()?.recovery_safe_mode().ok()
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
                let mut supervisor = BackendSupervisor::default();
                let mut safe_mode = recovery_safe_mode(&database).unwrap_or(false);
                if safe_mode {
                    eprintln!("AudioRouter control backend starting in durable safe mode; routes remain stopped");
                }
                loop {
                    // A panic is a backend failure like any other: count it and
                    // restart, never leave the tray running with no backend.
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
                    // ControlPlane contains COM-backed endpoint state and is
                    // deliberately constructed on the serving thread.
                    let storage = Storage::open(&database)
                        .map_err(|error| format!("backend database open failed: {error:?}"))?;
                    let sid = audiorouter_transport::current_user_sid().map_err(|error| {
                        format!("current user identity lookup failed: {error:?}")
                    })?;
                    let mut enrollment = storage
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
                        // The first launch of a fresh install is now an
                        // operator: it must get the same desktop grant as
                        // every later launch, not a narrower fallback.
                        enrollment = Some(("operator".into(), false));
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
                    if !safe_mode && std::env::var_os("AUDIOROUTER_CAPTURE_ENDPOINT_ID").is_some() {
                        match prepare_configured_native_worker(&mut plane, session_id.clone()) {
                            Ok(true) => eprintln!("AudioRouter native endpoint worker prepared; session start remains explicit"),
                            Ok(false) => unreachable!("configured native worker returned false"),
                            Err(error) => eprintln!("AudioRouter native endpoint worker unavailable: {error}"),
                        }
                    }
                    let device_admin_opt_in = std::env::var_os("AUDIOROUTER_ALLOW_DEVICE_ADMIN")
                        .is_some_and(|value| value == "1");
                    if device_admin_opt_in {
                        eprintln!("AudioRouter device-administration process grant enabled by explicit opt-in");
                    }
                    let grant = match select_shell_grant(device_admin_opt_in, enrollment.as_ref())? {
                        Some(grant) => grant,
                        None => plane
                            .grant_for_client(&sid)
                            .map_err(|error| format!("current user enrollment lookup failed: {error:?}"))?
                            .ok_or_else(|| "current user is not enrolled".to_owned())?,
                    };
                        audiorouter_transport::serve_control_connections_forever_with_grant(
                            &pipe_name, plane, grant,
                        )
                        .map_err(|error| format!("control backend stopped: {error:?}"))
                    }))
                    .unwrap_or_else(|_| Err("control backend panicked".into()));
                    match result {
                        Ok(()) => break,
                        Err(error) => {
                            if safe_mode {
                                if recovery_safe_mode(&database) != Some(false) {
                                    eprintln!(
                                        "AudioRouter safe-mode control backend stopped: {error}; backend thread exiting"
                                    );
                                    break;
                                }
                                safe_mode = false;
                                supervisor = BackendSupervisor::default();
                                eprintln!("AudioRouter safe mode was explicitly cleared; bounded backend supervision resumed");
                            }
                            let now = std::time::Instant::now();
                            let durable_failures = record_backend_failure(&database);
                            let decision = supervisor.record_failure(now);
                            if durable_failures.is_some_and(|count| count >= 3) {
                                eprintln!(
                                    "AudioRouter control backend stopped: {error}; entering durable safe mode with routes stopped"
                                );
                                safe_mode = true;
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                continue;
                            }
                            match decision {
                                BackendRestartDecision::Restart { delay } => {
                                    eprintln!(
                                        "AudioRouter control backend stopped: {error}; restarting after {} ms ({} recent failures)",
                                        delay.as_millis(),
                                        supervisor.failure_count(now),
                                    );
                                    std::thread::sleep(delay);
                                }
                                BackendRestartDecision::StopSafeMode => {
                                    eprintln!(
                                        "AudioRouter control backend stopped: {error}; entering safe mode with routes stopped"
                                    );
                                    safe_mode = true;
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                }
                            }
                        }
                    }
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

#[cfg(test)]
fn tray_stop_succeeded(response: &JsonRpcResponse) -> bool {
    response
        .result
        .as_ref()
        .and_then(|result| result.get("state"))
        .and_then(serde_json::Value::as_str)
        == Some("stopped")
}

/// Extract the bounded set of currently running sessions from the
/// authoritative status response. A malformed or over-capacity response is
/// rejected so tray quit cannot claim that every session was stopped.
#[cfg(test)]
fn tray_active_session_ids(response: &JsonRpcResponse) -> Option<Vec<String>> {
    let result = response.result.as_ref()?;
    let items = result.get("activeSessionIds")?.as_array()?;
    let count = result.get("activeSessionCount")?.as_u64()?;
    if count > 2 || items.len() != count as usize {
        return None;
    }
    let ids: Vec<String> = items
        .iter()
        .map(|value| {
            let id = value.as_str()?.to_owned();
            (!id.is_empty()).then_some(id)
        })
        .collect::<Option<_>>()?;
    if ids
        .iter()
        .any(|id| ids.iter().filter(|other| *other == id).count() != 1)
    {
        return None;
    }
    Some(ids)
}

#[cfg(test)]
fn tray_recorders_to_finalize_all(
    response: &JsonRpcResponse,
) -> Option<Vec<(String, Option<String>, u64)>> {
    let items = response.result.as_ref()?.as_array()?;
    if items.len() > 8 {
        return None;
    }
    let mut to_finalize = Vec::new();
    for item in items {
        let object = item.as_object()?;
        let session_id = object.get("sessionId")?.as_str()?.to_owned();
        if session_id.is_empty() {
            return None;
        }
        let state = object.get("state")?.as_str()?;
        let last_frame = match object.get("lastFrame")? {
            value if value.is_null() => None,
            value => Some(value.as_u64()?),
        };
        let node_id = match object.get("nodeId") {
            None => None,
            Some(value) if value.is_null() => None,
            Some(value) => Some(value.as_str()?.to_owned()),
        };
        if !matches!(
            state,
            "idle" | "armed" | "recording" | "paused" | "stopping" | "completed" | "failed"
        ) {
            return None;
        }
        if matches!(state, "armed" | "recording" | "paused" | "stopping") {
            to_finalize.push((session_id, node_id, last_frame?));
        }
    }
    Some(to_finalize)
}

fn tray_privacy_muted(response: &JsonRpcResponse) -> Option<bool> {
    response
        .result
        .as_ref()
        .and_then(|result| result.get("privacyMute"))
        .and_then(|mute| mute.get("muted"))
        .and_then(serde_json::Value::as_bool)
}

fn refresh_tray_status<R: Runtime>(
    status: &MenuItem<R>,
    recordings: &MenuItem<R>,
    pipe_name: &str,
) {
    let status_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(serde_json::json!("tray-status")),
        method: "status.get".into(),
        params: None,
    };
    let text = forward_rpc_request(&status_request, pipe_name)
        .map(|response| tray_status_text(&response))
        .unwrap_or_else(|_| "Status unavailable".to_owned());
    let _ = status.set_text(text);

    let recordings_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(serde_json::json!("tray-recorders")),
        method: "recorders.list".into(),
        params: None,
    };
    let text = forward_rpc_request(&recordings_request, pipe_name)
        .map(|response| tray_recording_text(&response))
        .unwrap_or_else(|_| "Live recorders: unavailable".to_owned());
    let _ = recordings.set_text(text);
}

#[cfg(windows)]
fn log_instance_check(state: &'static str) {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(serde_json::json!("instance-check")),
        method: "shell.instanceCheck".into(),
        params: None,
    };
    log_shell_rpc(
        &request,
        &Ok(JsonRpcResponse::success(
            request.id.clone(),
            serde_json::json!({"state": state}),
        )),
    );
}

/// Bounded record of where a panic happened. The panic message is omitted
/// (it may carry runtime text); thread name and source location suffice to
/// find the defect. Release builds have no console, so without this a
/// backend panic left no trace at all.
fn panic_log_detail(
    thread: Option<&str>,
    location: Option<&std::panic::Location<'_>>,
) -> serde_json::Value {
    serde_json::json!({
        "thread": thread.map(|name| name.chars().take(64).collect::<String>()),
        "file": location.map(|location| location.file().chars().take(160).collect::<String>()),
        "line": location.map(std::panic::Location::line),
    })
}

fn install_panic_log() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "shell.panic".into(),
            params: None,
        };
        let detail = panic_log_detail(std::thread::current().name(), info.location());
        log_shell_rpc_with(&request, &Ok(JsonRpcResponse::success(None, detail)), true);
        default_hook(info);
    }));
}

fn main() {
    install_panic_log();
    // Claim the desktop before opening storage, enrolling, forwarding RPCs or
    // starting recovery supervision. Test/external-pipe clients own no backend.
    #[cfg(windows)]
    let _instance_guard = if std::env::var_os("AUDIOROUTER_CONTROL_PIPE").is_none() {
        // Already running (often in the tray): bring that window forward.
        if instance_windows::show_running_instance() {
            log_instance_check("shown");
            return;
        }
        match instance_windows::claim_or_recover() {
            Ok(Some(guard)) => {
                log_instance_check("ready");
                Some(guard)
            }
            Ok(None) => {
                log_instance_check("cancelled");
                return;
            }
            Err(message) => {
                log_instance_check("unavailable");
                instance_windows::show_error(&message);
                return;
            }
        }
    } else {
        None
    };
    let pipe_name =
        std::env::var("AUDIOROUTER_CONTROL_PIPE").unwrap_or_else(|_| DEFAULT_PIPE_NAME.to_owned());
    let database_path = std::env::var_os("AUDIOROUTER_DATABASE")
        .map(std::path::PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_database_path)
        .unwrap_or_default();
    let state = ShellState {
        pipe_name,
        session_id: DESKTOP_SESSION_ID.to_owned(),
        probe_file: std::env::var_os("AUDIOROUTER_SHELL_PROBE_FILE").map(std::path::PathBuf::from),
        database_path,
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
        .manage(HttpApiState::default())
        .manage(UiUnsaved::default())
        .invoke_handler(tauri::generate_handler![
            rpc_request,
            quit_app,
            set_ui_unsaved,
            autoplay_get,
            autoplay_set,
            api_autostart_get,
            api_autostart_set,
            http_api_control,
            http_api_addresses,
            open_release_page,
            open_plugin_editor,
            choose_session_file,
            session_id,
            mcp_activity_list,
            backend_diagnostics_list,
            log_folder_path,
            open_logs_folder,
            install_streamdeck_plugin,
            mcp_setup_info,
            startup_register,
            startup_status
        ])
        .setup(move |app| {
            let session_script = session_script.clone();
            #[cfg(windows)]
            {
                let (transition_sender, transition_receiver) =
                    std::sync::mpsc::sync_channel(16);
                match os_transition_windows::OsTransitionListener::start(transition_sender) {
                    Ok(listener) => {
                        app.manage(listener);
                        let transition_pipe = tray_pipe_name.clone();
                        std::thread::Builder::new()
                            .name("audiorouter-os-transition-rpc".into())
                            .spawn(move || {
                                for (sequence, transition) in
                                    transition_receiver.into_iter().enumerate()
                                {
                                    let operation_key = transition_operation_key(sequence);
                                    let transition = match transition {
                                        audiorouter_control::os_transition::OsTransition::Lock => "lock",
                                        audiorouter_control::os_transition::OsTransition::SignOut => "signOut",
                                        audiorouter_control::os_transition::OsTransition::Sleep => "sleep",
                                        audiorouter_control::os_transition::OsTransition::Resume => "resume",
                                    };
                                    let _ = forward_rpc_request(
                                        &JsonRpcRequest {
                                            jsonrpc: "2.0".into(),
                                            id: Some(serde_json::json!(format!(
                                                "os-transition-{sequence}"
                                            ))),
                                            method: "system.osTransition".into(),
                                            params: Some(serde_json::json!({
                                                "transition": transition,
                                                "idempotencyKey": operation_key
                                            })),
                                        },
                                        &transition_pipe,
                                    );
                                }
                            })
                            .map_err(|error| format!("OS transition RPC thread failed: {error}"))?;
                    }
                    Err(error) => {
                        eprintln!("AudioRouter OS transition listener unavailable: {error}");
                    }
                }
            }
            let open = MenuItem::with_id(app, "open", "Open AudioRouter", true, None::<&str>)?;
            let close = MenuItem::with_id(app, "close", "Close window", true, None::<&str>)?;
            let play = MenuItem::with_id(app, "play", "Play", true, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop audio", true, None::<&str>)?;
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
            let backend_endpoint = MenuItem::with_id(
                app,
                "backend-endpoint",
                format!(
                    "Backend: local Windows pipe ({}) · browser access unavailable",
                    tray_pipe_name
                ),
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
                    &play,
                    &stop,
                    &quit,
                    &privacy,
                    &refresh_status,
                    &status,
                    &recordings,
                    &backend_endpoint,
                ],
            )?;
            // Best-effort: reflect the backend's actual current privacy-mute
            // state in the initial icon rather than always assuming unmuted.
            // A prior fix that always started the icon at "unmuted" produced
            // no visible color change on the FIRST toggle whenever the
            // backend's real prior state was already muted (e.g. a restarted
            // shell reattaching to a persistent backend) — the icon and the
            // true state silently disagreed until a second toggle. Falls
            // back to unmuted if the backend isn't reachable yet at startup.
            let initial_muted = forward_rpc_request(
                &JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(serde_json::json!("tray-privacy-initial")),
                    method: "status.get".into(),
                    params: None,
                },
                &pipe_name,
            )
            .ok()
            .and_then(|response| tray_privacy_muted(&response))
            .unwrap_or(false);
            let _tray = TrayIconBuilder::with_id("audiorouter")
                .icon(audio_router_tray_icon(initial_muted))
                .menu(&menu)
                .tooltip("AudioRouter")
                .on_menu_event(move |app, event| {
                    // The editor window may have been released (closed to
                    // the tray); every action except Close works without it.
                    match event.id().as_ref() {
                        "open" => {
                            if let Err(error) = open_main_window(app) {
                                eprintln!("AudioRouter window could not open: {error}");
                            }
                        }
                        "close" => {
                            // Same path as the window's own close button.
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.close();
                            }
                        }
                        // Play/stop the selected session without the window.
                        "play" | "stop" => {
                            let mut call = backend_call(&pipe_name);
                            let outcome = if event.id().as_ref() == "play" {
                                tray_playback::play_saved_session(&mut call)
                            } else {
                                tray_playback::stop_playing(&mut call)
                            };
                            match outcome {
                                Ok(_) => refresh_tray_status(&status_for_handler, &recordings_for_handler, &pipe_name),
                                Err(reason) => {
                                    let _ = status_for_handler.set_text(reason);
                                }
                            }
                        }
                        "quit" => {
                            let request = JsonRpcRequest {
                                jsonrpc: "2.0".into(),
                                id: Some(serde_json::json!("tray-quit")),
                                method: "system.quit".into(),
                                params: Some(serde_json::json!({
                                    "idempotencyKey": "tray-quit"
                                })),
                            };
                            match forward_rpc_request(&request, &pipe_name) {
                                Ok(response)
                                    if response.result.as_ref().and_then(|result| result.get("state"))
                                        .and_then(serde_json::Value::as_str)
                                        == Some("stopped") => app.exit(0),
                                _ => {
                                    let _ = status_for_handler
                                        .set_text("Quit refused: backend finalization failed");
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
                                    // The menu-item text alone required reopening the
                                    // tray menu to see whether the toggle took effect,
                                    // reported as no visible feedback for a
                                    // safety-relevant latch. The icon itself now
                                    // reflects state at a glance from the taskbar.
                                    if let Some(tray) = app.tray_by_id("audiorouter") {
                                        let _ = tray.set_icon(Some(audio_router_tray_icon(muted)));
                                    }
                                }
                                None => {
                                    let _ =
                                        status_for_handler.set_text("Privacy mute change refused");
                                }
                            }
                        }
                        "refresh-status" => {
                            refresh_tray_status(
                                &status_for_handler,
                                &recordings_for_handler,
                                &pipe_name,
                            );
                        }
                        _ => {}
                    }
                })
                .build(app)?;
            refresh_tray_status(&status, &recordings, &tray_pipe_name);
            app.manage(MainWindowScript(session_script.clone()));
            // A second launch of this program shows this window instead.
            #[cfg(windows)]
            {
                let handle = app.handle().clone();
                instance_windows::listen_for_show_requests(move || {
                    let target = handle.clone();
                    let _ = handle.run_on_main_thread(move || {
                        if let Err(error) = open_main_window(&target) {
                            eprintln!("AudioRouter window could not open: {error}");
                        }
                    });
                });
            }
            // Started at sign-in (`--tray`): no window, so no WebView, until
            // the user opens it from the tray.
            if !starts_in_tray(std::env::args()) {
                open_main_window(app.handle())?;
            }
            // Autoplay (Advanced): play the selected session once the
            // backend answers, with or without the window.
            let database_path = app.state::<ShellState>().database_path.clone();
            // API auto-start (Advanced), with or without the window.
            match auto_start_http_api(&database_path, &tray_pipe_name) {
                Ok(Some(listener)) => {
                    if let Ok(mut slot) = app.state::<HttpApiState>().0.lock() {
                        *slot = Some(listener);
                    }
                }
                Ok(None) => {}
                Err(reason) => eprintln!("local API did not start: {reason}"),
            }
            if shell_settings::load(&shell_settings::path_beside(&database_path)).auto_play {
                let (status, recordings, pipe) = (status.clone(), recordings.clone(), tray_pipe_name.clone());
                std::thread::spawn(move || {
                    let outcome = tray_playback::autoplay(&mut backend_call(&pipe), 40, std::time::Duration::from_millis(500));
                    match outcome {
                        Ok(_) => refresh_tray_status(&status, &recordings, &pipe),
                        Err(reason) => {
                            let _ = status.set_text(reason);
                        }
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the editor never tears down the backend or audio; Quit
            // uses an explicit stop/finalize path. A page with unsaved edits
            // is only hidden; otherwise the WebView is released.
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let unsaved = window.state::<UiUnsaved>().0.load(std::sync::atomic::Ordering::Relaxed);
                if close_action(unsaved) == CloseAction::Hide {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            // The next page starts clean and reports its own edits.
            tauri::WindowEvent::Destroyed => {
                window.state::<UiUnsaved>().0.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building AudioRouter shell")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                if !allow_exit(code) {
                    api.prevent_exit();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_control::ControlPlane;
    use audiorouter_domain::validate_session;

    #[test]
    fn release_page_opens_only_this_repository_for_numeric_tags() {
        assert_eq!(
            release_page_url("v0.0.9").as_deref(),
            Some("https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.9")
        );
        assert_eq!(
            release_page_url("v12.0.100").as_deref(),
            Some("https://github.com/MrDesjardins/audiorouter/releases/tag/v12.0.100")
        );
        for bad in [
            "0.0.9",
            "v0.0",
            "v0.0.9.1",
            "v0.0.9-beta",
            "v0.0.9/../../evil",
            "v0.0.x",
            "v.0.9",
            "v0.0.1234567",
            "https://evil.example/v1.2.3",
            "",
        ] {
            assert_eq!(release_page_url(bad), None, "{bad}");
        }
    }

    /// Only the sign-in registration's own argument starts without a window;
    /// the program name itself never counts.
    #[test]
    fn only_the_tray_argument_starts_without_a_window() {
        let args = |list: &[&str]| {
            list.iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };
        assert!(starts_in_tray(args(&["shell.exe", "--tray"])));
        assert!(!starts_in_tray(args(&["shell.exe"])));
        assert!(!starts_in_tray(args(&["--tray"])), "argv[0] is the program");
        assert!(!starts_in_tray(args(&["shell.exe", "--tray=no", "--traY"])));
    }

    /// Closing to the tray frees the WebView only when nothing would be lost,
    /// and only an explicit Quit (with a code) may end the app.
    #[test]
    fn closing_the_editor_frees_the_webview_unless_edits_are_unsaved() {
        assert_eq!(close_action(false), CloseAction::Release);
        assert_eq!(close_action(true), CloseAction::Hide);
        assert!(
            !allow_exit(None),
            "the last window closing keeps the backend in the tray"
        );
        assert!(allow_exit(Some(0)), "Quit exits after finalizing");
        assert!(
            !UiUnsaved::default()
                .0
                .load(std::sync::atomic::Ordering::Relaxed),
            "a fresh page starts clean"
        );
    }

    /// Setting WebView2 arguments replaces wry's defaults, so they must be
    /// kept beside the heap cap; nothing else (no debugging port) may ship.
    #[test]
    fn webview_arguments_keep_wry_defaults_and_cap_the_js_heap() {
        let arguments = WEBVIEW_BROWSER_ARGS.split_whitespace().collect::<Vec<_>>();
        assert_eq!(
            arguments,
            [
                "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
                "--js-flags=--max-old-space-size=256"
            ]
        );
    }

    /// The grant of the first launch of a fresh install (enrolled just now)
    /// must equal every later launch's: the desktop grant that can accept
    /// the user's device consent. 0.0.1 served a narrower operator grant on
    /// first launch, and the opt-in variable broke the first launch.
    #[test]
    fn a_fresh_install_gets_the_full_desktop_grant_on_its_first_launch() {
        let operator = ("operator".to_owned(), false);
        let first_launch = select_shell_grant(false, Some(&operator)).unwrap().unwrap();
        assert_eq!(first_launch, ClientGrant::for_desktop_shell());
        assert!(first_launch.accepts_device_consent());
        let opted_in = select_shell_grant(true, Some(&operator)).unwrap().unwrap();
        assert_ne!(
            opted_in, first_launch,
            "the opt-in adds device administration"
        );
        // Other roles keep their enrolled grant; the opt-in needs an operator.
        let editor = ("editor".to_owned(), false);
        assert!(select_shell_grant(false, Some(&editor)).unwrap().is_none());
        assert!(select_shell_grant(true, Some(&editor)).is_err());
        assert!(select_shell_grant(true, Some(&("operator".to_owned(), true))).is_err());
    }
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
    fn transition_operation_keys_are_unique_across_sequences() {
        let first = transition_operation_key(0);
        let second = transition_operation_key(1);
        assert_ne!(first, second);
        assert!(first.starts_with("shell-os-transition-"));
        assert!(first.len() < 256);
    }

    #[cfg(windows)]
    #[test]
    fn backend_failure_markers_latch_safe_mode_after_storage_reopen() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let database = std::env::temp_dir().join(format!(
            "audiorouter-shell-recovery-{process}-{suffix}.sqlite",
            process = std::process::id()
        ));
        assert_eq!(record_backend_failure(&database), Some(1));
        assert_eq!(record_backend_failure(&database), Some(2));
        assert_eq!(record_backend_failure(&database), Some(3));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_secs();
        let storage = Storage::open(&database).expect("recovery database reopens");
        let status = storage.recovery_status(now).expect("recovery status");
        assert_eq!(status.recent_crashes, 3);
        assert!(status.safe_mode);
        storage
            .clear_recovery_crashes()
            .expect("safe mode clear persists");
        assert_eq!(recovery_safe_mode(&database), Some(false));
        drop(storage);
        let _ = std::fs::remove_file(&database);
        let _ = std::fs::remove_file(database.with_extension("sqlite-wal"));
        let _ = std::fs::remove_file(database.with_extension("sqlite-shm"));
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
    fn tray_quit_reads_a_coherent_bounded_active_session_set() {
        let response = JsonRpcResponse {
            result: Some(json!({
                "activeSessionCount": 2,
                "activeSessionIds": ["desktop", "voice"]
            })),
            ..JsonRpcResponse::failure(None, -1, "unused")
        };
        assert_eq!(
            tray_active_session_ids(&response),
            Some(vec!["desktop".to_owned(), "voice".to_owned()])
        );

        for result in [
            json!({ "activeSessionCount": 1, "activeSessionIds": ["desktop", "voice"] }),
            json!({ "activeSessionCount": 3, "activeSessionIds": ["a", "b", "c"] }),
            json!({ "activeSessionCount": 2, "activeSessionIds": ["same", "same"] }),
            json!({ "activeSessionCount": 1, "activeSessionIds": [""] }),
        ] {
            let response = JsonRpcResponse {
                result: Some(result),
                ..JsonRpcResponse::failure(None, -1, "unused")
            };
            assert_eq!(tray_active_session_ids(&response), None);
        }
    }

    #[test]
    fn tray_quit_finalizes_active_recorders_for_each_active_session() {
        let response = JsonRpcResponse {
            result: Some(json!([
                { "sessionId": "desktop", "nodeId": "desktop-rec", "state": "recording", "lastFrame": 480 },
                { "sessionId": "voice", "nodeId": null, "state": "paused", "lastFrame": 960 },
                { "sessionId": "desktop", "nodeId": null, "state": "idle", "lastFrame": null }
            ])),
            ..JsonRpcResponse::failure(None, -1, "unused")
        };
        assert_eq!(
            tray_recorders_to_finalize_all(&response),
            Some(vec![
                ("desktop".to_owned(), Some("desktop-rec".to_owned()), 480),
                ("voice".to_owned(), None, 960),
            ])
        );
    }

    #[test]
    fn tray_quit_finalizes_only_active_recorders() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(json!("tray-quit-recorders")),
            result: Some(json!([
                { "sessionId": DESKTOP_SESSION_ID, "nodeId": "mic-rec", "state": "recording", "lastFrame": 480 },
                { "sessionId": DESKTOP_SESSION_ID, "nodeId": null, "state": "idle", "lastFrame": null },
                { "sessionId": "other-session", "nodeId": "other-rec", "state": "recording", "lastFrame": 960 }
            ])),
            error: None,
        };
        assert_eq!(
            tray_recorders_to_finalize_all(&response),
            Some(vec![
                (
                    DESKTOP_SESSION_ID.to_owned(),
                    Some("mic-rec".to_owned()),
                    480
                ),
                (
                    "other-session".to_owned(),
                    Some("other-rec".to_owned()),
                    960
                ),
            ])
        );
    }

    #[test]
    fn tray_quit_rejects_malformed_or_over_capacity_recorder_status() {
        let malformed = JsonRpcResponse {
            result: Some(json!({ "items": [] })),
            ..JsonRpcResponse::failure(None, -1, "malformed")
        };
        assert_eq!(tray_recorders_to_finalize_all(&malformed), None);

        let over_capacity = JsonRpcResponse {
            result: Some(json!([
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null },
                { "sessionId": DESKTOP_SESSION_ID, "state": "idle", "lastFrame": null }
            ])),
            ..malformed.clone()
        };
        assert_eq!(tray_recorders_to_finalize_all(&over_capacity), None);

        let malformed_item = JsonRpcResponse {
            result: Some(json!([{ "sessionId": DESKTOP_SESSION_ID, "state": "recording" }])),
            ..malformed
        };
        assert_eq!(tray_recorders_to_finalize_all(&malformed_item), None);

        let active_without_frame = JsonRpcResponse {
            result: Some(
                json!([{ "sessionId": DESKTOP_SESSION_ID, "state": "recording", "lastFrame": null }]),
            ),
            ..malformed_item
        };
        assert_eq!(tray_recorders_to_finalize_all(&active_without_frame), None);
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

    #[test]
    fn shell_rpc_log_records_graph_counts_without_graph_contents() {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-shell-log-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "sessions.get".into(),
            params: Some(serde_json::json!({"sessionId":"test-session"})),
        };
        let response = Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(serde_json::json!({
                "id":"test-session",
                "revision":7,
                "nodes":[{"id":"private-node-name"}],
                "edges":[{}],
                "runtime":{"detail":"private-runtime-detail"}
            })),
            error: None,
        });
        write_shell_rpc_log(&directory, &request, &response);
        let log = std::fs::read_to_string(directory.join("shell.jsonl")).unwrap();
        let entry: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(entry["method"], "sessions.get");
        assert_eq!(entry["detail"]["revision"], 7);
        assert_eq!(entry["detail"]["nodes"], 1);
        assert_eq!(entry["detail"]["edges"], 1);
        assert!(!log.contains("private-node-name"));
        assert!(!log.contains("private-runtime-detail"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shell_rpc_failure_log_keeps_safe_kind_and_omits_message() {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-shell-error-log-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(2)),
            method: "graph.commit".into(),
            params: Some(serde_json::json!({"secret":"private-name"})),
        };
        let response = Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: None,
            error: Some(audiorouter_protocol::JsonRpcError {
                code: -32001,
                message: "permission denied: GraphWrite private-name".into(),
                data: Some(serde_json::json!({"code":"permissionDenied"})),
            }),
        });
        write_shell_rpc_log(&directory, &request, &response);
        let log = std::fs::read_to_string(directory.join("shell.jsonl")).unwrap();
        let entry: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(entry["detail"]["code"], -32001);
        assert_eq!(entry["detail"]["kind"], "permissionDenied");
        assert!(!log.contains("private-name"));
        assert!(!log.contains("GraphWrite"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shell_logs_audio_operation_build_and_hex_hresult_without_private_payload() {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-shell-audio-log-{}",
            std::process::id()
        ));
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(9)),
            method: "devices.list".into(),
            params: Some(serde_json::json!({"sessionId":{"secret":"private"}})),
        };
        let response = Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: None,
            error: Some(audiorouter_protocol::JsonRpcError {
                code: -32000,
                message: "private device name".into(),
                data: Some(
                    serde_json::json!({"code":"deviceInvalidated","hresult":3758096907_u32,"operation":"inventory.openPropertyStore","retryable":true}),
                ),
            }),
        });
        write_shell_rpc_log(&directory, &request, &response);
        let log = std::fs::read_to_string(directory.join("shell.jsonl")).unwrap();
        let entry: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(entry["detail"]["operation"], "inventory.openPropertyStore");
        assert_eq!(entry["detail"]["hresultHex"], "0xE000020B");
        assert_eq!(entry["version"], env!("CARGO_PKG_VERSION"));
        assert!(entry["buildId"].is_string());
        assert!(!log.contains("private"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shell_panic_log_keeps_location_and_thread_but_no_message() {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-shell-panic-log-{}",
            std::process::id()
        ));
        let location = std::panic::Location::caller();
        let detail = panic_log_detail(Some("audiorouter-control"), Some(location));
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "shell.panic".into(),
            params: None,
        };
        write_shell_rpc_log(
            &directory,
            &request,
            &Ok(JsonRpcResponse::success(None, detail)),
        );
        let log = std::fs::read_to_string(directory.join("shell.jsonl")).unwrap();
        let record: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(record["method"], "shell.panic");
        assert_eq!(record["detail"]["thread"], "audiorouter-control");
        assert_eq!(record["detail"]["line"], location.line());
        assert!(record["detail"]["file"]
            .as_str()
            .unwrap()
            .ends_with("main.rs"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shell_does_not_log_successful_event_polls_but_keeps_poll_failures() {
        for method in ["events.subscribe", "system.diagnostics"] {
            assert_poll_success_skipped_and_failure_kept(method);
        }
    }

    fn assert_poll_success_skipped_and_failure_kept(method: &str) {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-shell-poll-log-{}-{method}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(10)),
            method: method.into(),
            params: None,
        };
        let ok = Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(serde_json::json!({})),
            error: None,
        });
        write_shell_rpc_log(&directory, &request, &ok);
        assert!(!directory.join("shell.jsonl").exists());
        write_shell_rpc_log(&directory, &request, &Err("private transport path".into()));
        let log = std::fs::read_to_string(directory.join("shell.jsonl")).unwrap();
        assert!(log.contains("transportError"));
        assert!(!log.contains("private"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shell_start_failure_log_records_bounded_topology_reason() {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-shell-topology-log-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(3)),
            method: "session.start".into(),
            params: Some(serde_json::json!({"sessionId":"session"})),
        };
        let response = Ok(JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: None,
            error: Some(audiorouter_protocol::JsonRpcError {
                code: -32602,
                message: "native graph rejected: UnsupportedTopology private-path".into(),
                data: Some(serde_json::json!({"code":"invalidRequest"})),
            }),
        });
        write_shell_rpc_log(&directory, &request, &response);
        let log = std::fs::read_to_string(directory.join("shell.jsonl")).unwrap();
        let entry: serde_json::Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(entry["detail"]["reason"], "UnsupportedTopology");
        assert_eq!(entry["detail"]["kind"], "invalidRequest");
        assert!(!log.contains("private-path"));
        std::fs::remove_dir_all(directory).unwrap();
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
