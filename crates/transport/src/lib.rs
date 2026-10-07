//! Windows named-pipe transport for the local JSON-RPC boundary.
//!
//! The transport deliberately does not change audio configuration or open an
//! audio endpoint. Native server pipes use an owner-only ACL and validate the
//! connected process SID before reading requests; method-level grants remain a
//! separate authorization boundary in the control plane.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    UnsupportedPlatform,
    InvalidPipeName,
    Windows(String),
    Protocol(String),
    UnexpectedEof,
}

/// Maximum number of requests handled on one persistent local control
/// connection. This keeps subscription/session lifetimes bounded while still
/// matching the control API's maximum page size.
pub const MAX_SESSION_FRAMES: usize = 500;
const BACKEND_DIAGNOSTIC_LIMIT_BYTES: u64 = 5 * 1024 * 1024;

/// Cross-process serialization for bounded diagnostic-file rotation and writes.
/// Names are fixed, same-user, `Local\` objects supplied only by this app.
#[cfg(windows)]
pub struct DiagnosticMutexGuard(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for DiagnosticMutexGuard {
    fn drop(&mut self) {
        // SAFETY: this handle was created by `CreateMutexW` and the successful
        // wait below grants this thread ownership. Drop releases that ownership
        // once, then closes the single owned handle.
        unsafe {
            let _ = windows::Win32::System::Threading::ReleaseMutex(self.0);
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Longest wait for another process's diagnostic write. A log record is
/// dropped rather than blocking the caller behind a stuck writer.
#[cfg(windows)]
const DIAGNOSTIC_MUTEX_WAIT_MS: u32 = 250;

/// Acquire a named, same-logon-session mutex for a short local file operation.
/// The wait occurs only on a control/logging thread, never in an audio callback,
/// and is bounded: `None` means the record should be skipped.
#[cfg(windows)]
pub fn acquire_diagnostic_mutex(name: &str) -> Option<DiagnosticMutexGuard> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{WAIT_ABANDONED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};
    if name.is_empty()
        || name.len() > 96
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return None;
    }
    let wide_name = format!("Local\\{name}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: `wide_name` is NUL-terminated and remains alive for the call;
    // default security gives the mutex the current user's inherited ACL.
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide_name.as_ptr())) }.ok()?;
    // SAFETY: `handle` is the valid owned handle returned above. An abandoned
    // mutex is also acquired and must be released by the returned guard.
    let result = unsafe { WaitForSingleObject(handle, DIAGNOSTIC_MUTEX_WAIT_MS) };
    if result == WAIT_OBJECT_0 || result == WAIT_ABANDONED {
        Some(DiagnosticMutexGuard(handle))
    } else {
        // SAFETY: the wait failed, so close the one valid owned handle.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }
        None
    }
}

/// One backend request and its responses, queued for the logger thread.
struct BackendLogEntry {
    frame: Vec<u8>,
    responses: Vec<Vec<u8>>,
    now_ms: u128,
    /// Opt-in verbose window (`diagnostics.setVerbose`) was on: also log
    /// routine-read successes and the dispatch duration.
    verbose: bool,
    /// Time the control plane spent dispatching the frame.
    duration_micros: u64,
}

/// Queued records before new ones are dropped. A burst larger than this is
/// lost from the log rather than slowing the control plane.
const BACKEND_LOG_QUEUE: usize = 256;

fn backend_log_directory() -> Option<std::path::PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("TEMP").map(std::path::PathBuf::from))
        .map(|root| root.join("AudioRouter").join("logs"))
}

/// Start the thread that writes `backend.jsonl` in `directory`. The control
/// plane only enqueues; all parsing and file I/O happen on this thread, so
/// logging never delays the audio service passes that share the control
/// thread. The thread ends when every sender is dropped.
fn spawn_backend_rpc_logger(
    directory: std::path::PathBuf,
) -> Option<(
    std::sync::mpsc::SyncSender<BackendLogEntry>,
    std::thread::JoinHandle<()>,
)> {
    let (sender, receiver) = std::sync::mpsc::sync_channel::<BackendLogEntry>(BACKEND_LOG_QUEUE);
    let handle = std::thread::Builder::new()
        .name("audiorouter-backend-log".into())
        .spawn(move || {
            for entry in receiver {
                write_backend_rpc_log(&directory, &entry);
            }
        })
        .ok()?;
    Some((sender, handle))
}

/// Control-plane-side request summaries for attended desktop diagnosis.
/// Parameter values and response payloads are deliberately excluded. This
/// only copies the frames into a bounded queue (dropping the record when it
/// is full); the logger thread does the rest.
fn log_backend_rpc(
    frame: &[u8],
    responses: &[Vec<u8>],
    verbose: bool,
    duration: std::time::Duration,
) {
    static SINK: std::sync::OnceLock<Option<std::sync::mpsc::SyncSender<BackendLogEntry>>> =
        std::sync::OnceLock::new();
    let sink = SINK.get_or_init(|| {
        backend_log_directory()
            .and_then(spawn_backend_rpc_logger)
            .map(|(sender, _detached)| sender)
    });
    let Some(sender) = sink else { return };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or_default();
    let _ = sender.try_send(BackendLogEntry {
        frame: frame.to_vec(),
        responses: responses.to_vec(),
        now_ms,
        verbose,
        duration_micros: u64::try_from(duration.as_micros()).unwrap_or(u64::MAX),
    });
}

fn write_backend_rpc_log(directory: &std::path::Path, entry: &BackendLogEntry) {
    use std::io::Write;
    let records = backend_rpc_log_records(
        &entry.frame,
        &entry.responses,
        entry.now_ms,
        entry.verbose.then_some(entry.duration_micros),
    );
    if records.is_empty() {
        return;
    }
    #[cfg(windows)]
    let Some(_cross_process_guard) = acquire_diagnostic_mutex("AudioRouter.BackendDiagnostics") else {
        return;
    };
    if std::fs::create_dir_all(directory).is_err() {
        return;
    }
    let path = directory.join("backend.jsonl");
    if std::fs::metadata(&path)
        .map(|metadata| metadata.len() >= BACKEND_DIAGNOSTIC_LIMIT_BYTES)
        .unwrap_or(false)
    {
        let previous = directory.join("backend.previous.jsonl");
        let _ = std::fs::remove_file(&previous);
        let _ = std::fs::rename(&path, previous);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        for record in records {
            if serde_json::to_writer(&mut file, &record).is_ok() {
                let _ = file.write_all(b"\n");
            }
        }
        let _ = file.flush();
    }
}

/// Privacy-safe commit outcome shared by backend and shell diagnostics. Never
/// include runtime reason strings, which may contain node names or file paths.
pub fn graph_activation_log_summary(result: &serde_json::Value) -> serde_json::Value {
    let state = result
        .pointer("/activation/state")
        .and_then(serde_json::Value::as_str)
        .filter(|state| matches!(*state, "running" | "pending"));
    let native = result
        .pointer("/activation/native/state")
        .and_then(serde_json::Value::as_str)
        .filter(|state| matches!(*state, "applied" | "restarted" | "restartRequired"));
    serde_json::json!({
        "revision": result.get("revision").and_then(serde_json::Value::as_u64),
        "state": state,
        "generation": result.pointer("/activation/generation").and_then(serde_json::Value::as_u64),
        "nativeState": native,
    })
}

/// Fixed safe diagnostic fields shared by shell and backend. Raw messages,
/// request values, paths and arbitrary error data must never enter the log.
pub fn rpc_failure_log_summary(error: &serde_json::Value) -> serde_json::Value {
    let kind = error
        .pointer("/data/code")
        .and_then(serde_json::Value::as_str)
        .filter(|kind| {
            matches!(
                *kind,
                "permissionDenied"
                    | "revisionConflict"
                    | "rateLimited"
                    | "invalidParams"
                    | "notFound"
                    | "unavailable"
                    | "unsupported"
                    | "internalError"
                    | "invalidArgument"
                    | "accessDenied"
                    | "deviceInUse"
                    | "exclusiveModeOnly"
                    | "deviceInvalidated"
                    | "unsupportedFormat"
                    | "serviceUnavailable"
                    | "bufferConstraint"
                    | "other"
                    | "invalidRequest"
                    | "invalidGraph"
                    | "storageFailure"
                    | "corruptDatabase"
                    | "idempotencyConflict"
                    | "deviceUnavailable"
                    | "ambiguousBinding"
                    | "feedbackCycle"
                    | "pluginUnavailable"
                    | "resourceConflict"
                    | "budgetExceeded"
                    | "diskFull"
                    | "resyncRequired"
                    | "restartRequired"
                    | "planExpired"
                    | "planLimitReached"
            )
        });
    let operation = error
        .pointer("/data/operation")
        .and_then(serde_json::Value::as_str)
        .filter(|op| {
            matches!(
                *op,
                "inventory.createEnumerator"
                    | "inventory.enumerateActiveEndpoints"
                    | "inventory.enumerateAllEndpoints"
                    | "inventory.getEndpointCount"
                    | "inventory.getEndpoint"
                    | "inventory.getEndpointId"
                    | "inventory.activateAudioClient"
                    | "inventory.getDevicePeriod"
                    | "inventory.getMixFormat"
                    | "inventory.openPropertyStore"
                    | "inventory.getFriendlyName"
                    | "inventory.getEndpointState"
                    | "IAudioClient::Initialize(render)"
                    | "IAudioClient::Initialize(capture,event-callback)"
                    | "IAudioClient::Initialize(capture,polling)"
                    | "IAudioClient::Initialize(process-loopback)"
                    | "ActivateAudioInterfaceAsync(process-loopback)"
                    | "ActivateAudioInterfaceAsync(process-loopback)/QueryInterface"
            )
        });
    let hresult = error
        .pointer("/data/hresult")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let guidance = match kind {
        Some("deviceInvalidated" | "deviceUnavailable") => {
            Some("Refresh devices and retry the selected endpoint.")
        }
        Some("accessDenied" | "permissionDenied") => {
            Some("Check the required permission and Windows audio privacy settings.")
        }
        Some("deviceInUse") => {
            Some("Check which application owns this endpoint, or choose another endpoint.")
        }
        Some("serviceUnavailable") => Some("Check Windows Audio service availability and retry."),
        Some("unsupportedFormat") => Some("Choose a supported endpoint format."),
        Some("revisionConflict") => {
            Some("Refresh the session and review the retained draft before retrying.")
        }
        Some("restartRequired") => Some("Stop, prepare the route, and Play again."),
        _ => None,
    };
    serde_json::json!({
        "code": error.get("code").and_then(serde_json::Value::as_i64),
        "kind": kind,
        "operation": operation,
        "hresult": hresult,
        "hresultHex": hresult.map(|value| format!("0x{value:08X}")),
        "retryable": error.pointer("/data/retryable").and_then(serde_json::Value::as_bool),
        "guidance": guidance,
    })
}

#[test]
fn commit_log_distinguishes_saved_from_live_applied_without_exposing_reason() {
    let result = serde_json::json!({"revision":123,"activation":{"state":"running","generation":7,"native":{"state":"restartRequired","reason":"private node and plugin path"}}});
    let summary = graph_activation_log_summary(&result);
    assert_eq!(
        summary,
        serde_json::json!({"revision":123,"state":"running","generation":7,"nativeState":"restartRequired"})
    );
    assert!(!summary.to_string().contains("private"));
    assert_eq!(
        graph_activation_log_summary(
            &serde_json::json!({"activation":{"native":{"state":"private"}}})
        )["nativeState"],
        serde_json::Value::Null
    );
}

/// `verbose_duration_micros` is set while the opt-in verbose window is on:
/// routine-read successes are then logged too, each with `durationMs`.
fn backend_rpc_log_records(
    frame: &[u8],
    responses: &[Vec<u8>],
    now_ms: u128,
    verbose_duration_micros: Option<u64>,
) -> Vec<serde_json::Value> {
    use audiorouter_protocol::{decode_rpc_frame, RpcMessage};
    let Ok(message) = decode_rpc_frame(frame) else {
        return Vec::new();
    };
    let requests = match message {
        RpcMessage::Single(request) => vec![request],
        RpcMessage::Batch(requests) => requests,
    };
    // The optional `requestId` correlation member is not part of the typed
    // request; read it from the same (already validated) payload, in order.
    let correlation_ids = frame
        .get(4..)
        .and_then(|payload| serde_json::from_slice::<serde_json::Value>(payload).ok())
        .map(|payload| audiorouter_protocol::diagnostics::payload_correlation_ids(&payload))
        .unwrap_or_default();
    let requests = requests
        .into_iter()
        .enumerate()
        .map(|(index, request)| (request, correlation_ids.get(index).cloned().flatten()))
        .filter(|(request, _)| !is_high_frequency_rpc(&request.method))
        .collect::<Vec<_>>();
    let verbose = verbose_duration_micros.is_some();
    // A batch shares one dispatch; its duration is reported per frame.
    let duration_ms = verbose_duration_micros.map(|micros| micros as f64 / 1000.0);
    let response_values = responses
        .iter()
        .filter_map(|frame| audiorouter_protocol::decode_frame::<serde_json::Value>(frame).ok())
        .collect::<Vec<_>>();
    requests.into_iter().map(|(request, request_id)| {
        let response = response_values.iter().find(|response| response.get("id") == request.id.as_ref());
        let error_code = response.and_then(|value| value.pointer("/error/code")).cloned();
        let error_kind = response
            .and_then(|value| value.pointer("/error/data/code"))
            .and_then(serde_json::Value::as_str)
            .filter(|kind| matches!(*kind, "permissionDenied" | "revisionConflict" | "rateLimited" | "invalidParams" | "notFound" | "unavailable" | "unsupported" | "internalError"))
            .map(str::to_owned);
        let state = response.and_then(|value| value.get("result")).map(|result| if request.method == "devices.list" { device_inventory_log_summary(result) } else if request.method == "graph.commit" { graph_activation_log_summary(result) } else { serde_json::json!({
            "revision": result.get("revision"),
            "nodes": result.get("nodes").and_then(serde_json::Value::as_array).map(Vec::len),
            "edges": result.get("edges").and_then(serde_json::Value::as_array).map(Vec::len),
            "state": result.get("state"),
            "generation": result.get("generation"),
        }) });
        let method = request.method.chars().take(96).collect::<String>();
        let detail = response.and_then(|value| value.get("error")).map(rpc_failure_log_summary);
        let error_kind = detail.as_ref().and_then(|detail| detail.get("kind")).cloned().or_else(|| error_kind.map(serde_json::Value::String));
        let mut record = serde_json::json!({ "timeUnixMs": now_ms, "processId": std::process::id(), "version": env!("CARGO_PKG_VERSION"), "buildId": option_env!("AUDIOROUTER_BUILD_ID").unwrap_or("development"), "requestId": request_id, "method": method, "outcome": if error_code.is_some() { "error" } else { "ok" }, "errorCode": error_code, "errorKind": error_kind, "detail": detail, "summary": state });
        if let Some(duration_ms) = duration_ms {
            record["durationMs"] = serde_json::json!(duration_ms);
            record["verbose"] = serde_json::json!(true);
        }
        record
    }).filter(|record| verbose || record["outcome"] == "error" || !record["method"].as_str().is_some_and(is_routine_read_rpc)).collect()
}

pub fn device_inventory_log_summary(result: &serde_json::Value) -> serde_json::Value {
    let items = result
        .as_array()
        .or_else(|| result.get("items").and_then(serde_json::Value::as_array));
    serde_json::json!({
        "deviceCount": items.map(Vec::len),
        "captureCount": items.map(|items| items.iter().filter(|item| item["direction"] == "capture").count()),
        "renderCount": items.map(|items| items.iter().filter(|item| item["direction"] == "render").count()),
        "inactiveCount": items.map(|items| items.iter().filter(|item| matches!(item["state"].as_str(), Some("disabled" | "unplugged" | "notPresent"))).count()),
        "hasMore": result.get("nextCursor").is_some_and(|cursor| !cursor.is_null()),
    })
}

#[test]
fn failure_logs_preserve_audio_context_and_reject_unbounded_private_fields() {
    let summary = rpc_failure_log_summary(
        &serde_json::json!({"code":-32000,"message":"private device path","data":{"code":"deviceInvalidated","operation":"inventory.openPropertyStore","hresult":3758096907_u32,"retryable":true,"remediation":"private token"}}),
    );
    assert_eq!(summary["operation"], "inventory.openPropertyStore");
    assert_eq!(summary["hresultHex"], "0xE000020B");
    assert_eq!(summary["kind"], "deviceInvalidated");
    assert!(!summary.to_string().contains("private"));
    let hostile = rpc_failure_log_summary(
        &serde_json::json!({"code":{},"data":{"code":"private".repeat(50000),"operation":"private path","hresult":{"audio":"secret"},"retryable":"token"}}),
    );
    assert_eq!(hostile["kind"], serde_json::Value::Null);
    assert_eq!(hostile["operation"], serde_json::Value::Null);
    assert_eq!(hostile["hresult"], serde_json::Value::Null);
    assert!(hostile.to_string().len() < 256);
}

#[test]
fn backend_logger_thread_writes_commits_and_skips_routine_read_successes() {
    let directory = std::env::temp_dir().join(format!(
        "audiorouter-backend-log-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let frame = |method: &str| {
        audiorouter_protocol::encode_frame(
            &serde_json::json!({"jsonrpc":"2.0","id":1,"method":method,"params":{}}),
        )
        .unwrap()
    };
    let ok = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":1,"result":{"revision":3,"nodes":[],"edges":[]}}),
    )
    .unwrap();
    let failed = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"private"}}),
    )
    .unwrap();
    let (sender, handle) = spawn_backend_rpc_logger(directory.clone()).unwrap();
    for (method, response) in [
        ("graph.commit", &ok),
        ("system.diagnostics", &ok),
        ("system.diagnostics", &failed),
    ] {
        sender
            .send(BackendLogEntry {
                frame: frame(method),
                responses: vec![response.clone()],
                now_ms: 1,
                verbose: false,
                duration_micros: 0,
            })
            .unwrap();
    }
    drop(sender);
    handle.join().unwrap();
    let log = std::fs::read_to_string(directory.join("backend.jsonl")).unwrap();
    let lines = log
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2, "{log}");
    assert_eq!(lines[0]["method"], "graph.commit");
    assert_eq!(lines[1]["method"], "system.diagnostics");
    assert_eq!(lines[1]["outcome"], "error");
    assert!(!log.contains("private"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn backend_log_records_carry_only_valid_correlation_ids() {
    let request = |request_id: serde_json::Value| {
        audiorouter_protocol::encode_frame(&serde_json::json!({
            "jsonrpc": "2.0", "id": 9, "method": "graph.commit", "params": {"path": "C:\\private"},
            "requestId": request_id,
        }))
        .unwrap()
    };
    let ok = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":9,"result":{"revision":4}}),
    )
    .unwrap();
    let records = backend_rpc_log_records(
        &request(serde_json::json!("K7Q2M9XD")),
        std::slice::from_ref(&ok),
        1,
        None,
    );
    assert_eq!(records[0]["requestId"], "K7Q2M9XD");
    assert!(records[0].get("durationMs").is_none());
    for hostile in [
        serde_json::json!("has space"),
        serde_json::json!("x".repeat(33)),
        serde_json::json!({"nested": "C:\\private"}),
    ] {
        let records =
            backend_rpc_log_records(&request(hostile), std::slice::from_ref(&ok), 1, None);
        assert_eq!(records[0]["requestId"], serde_json::Value::Null);
        assert!(!records[0].to_string().contains("private"));
    }
    let batch = audiorouter_protocol::encode_frame(&serde_json::json!([
        {"jsonrpc":"2.0","id":1,"method":"nativeEndpoints.pump","requestId":"PUMP"},
        {"jsonrpc":"2.0","id":2,"method":"graph.commit","requestId":"SECOND"},
    ]))
    .unwrap();
    let response = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":2,"result":{"revision":5}}),
    )
    .unwrap();
    let records = backend_rpc_log_records(&batch, &[response], 1, None);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["requestId"], "SECOND");
}

#[test]
fn verbose_backend_logs_add_routine_reads_and_durations() {
    let request = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":3,"method":"system.diagnostics","requestId":"POLL-1"}),
    )
    .unwrap();
    let ok = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":3,"result":{"state":"running"}}),
    )
    .unwrap();
    assert!(backend_rpc_log_records(&request, std::slice::from_ref(&ok), 1, None).is_empty());
    let records = backend_rpc_log_records(&request, &[ok], 1, Some(1_500));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["method"], "system.diagnostics");
    assert_eq!(records[0]["requestId"], "POLL-1");
    assert_eq!(records[0]["durationMs"], 1.5);
    assert_eq!(records[0]["verbose"], true);
    // Native pumps stay out even in verbose mode.
    let pump = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":4,"method":"nativeEndpoints.pump"}),
    )
    .unwrap();
    assert!(backend_rpc_log_records(&pump, &[], 1, Some(10)).is_empty());
}

#[test]
fn device_logs_show_counts_without_names_and_event_errors_remain_visible() {
    let summary = device_inventory_log_summary(
        &serde_json::json!({"items":[{"direction":"capture","state":"active","name":"private mic"},{"direction":"render","state":"unplugged","id":"private id"}],"nextCursor":"private cursor"}),
    );
    assert_eq!(summary["deviceCount"], 2);
    assert_eq!(summary["captureCount"], 1);
    assert_eq!(summary["inactiveCount"], 1);
    assert_eq!(summary["hasMore"], true);
    assert!(!summary.to_string().contains("private"));
    let request = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":4,"method":"events.subscribe","params":{}}),
    )
    .unwrap();
    let ok = audiorouter_protocol::encode_frame(
        &serde_json::json!({"jsonrpc":"2.0","id":4,"result":{}}),
    )
    .unwrap();
    assert!(backend_rpc_log_records(&request, &[ok], 1, None).is_empty());
    let error = audiorouter_protocol::encode_frame(&serde_json::json!({"jsonrpc":"2.0","id":4,"error":{"code":-32000,"data":{"code":"resyncRequired"}}})).unwrap();
    assert_eq!(
        backend_rpc_log_records(&request, &[error], 1, None)[0]["errorKind"],
        "resyncRequired"
    );
}

/// Reads the window repeats on a timer (`system.diagnostics` 20 times a
/// second while playing, `events.subscribe` every second). Their successes
/// would fill a 5 MB log within minutes and rotate out the events a support
/// case needs, so only their failures are logged.
pub fn is_routine_read_rpc(method: &str) -> bool {
    matches!(method, "system.diagnostics" | "events.subscribe")
}

fn is_high_frequency_rpc(method: &str) -> bool {
    matches!(
        method,
        "nativeBridges.heartbeat"
            | "nativeEndpoints.pump"
            | "nativeDuplex.pump"
            | "nativeRenderSources.pump"
            | "nativeMultiInputs.pump"
    )
}

fn validate_response_count(responses: usize) -> Result<(), TransportError> {
    if responses == 0 || responses > MAX_SESSION_FRAMES {
        return Err(TransportError::Protocol(
            "response frame count must be between 1 and 500".into(),
        ));
    }
    Ok(())
}

fn checked_io_count(count: u32, remaining: usize) -> Result<usize, TransportError> {
    let count = usize::try_from(count)
        .map_err(|_| TransportError::Protocol("I/O byte count cannot be represented".into()))?;
    if count == 0 {
        return Err(TransportError::UnexpectedEof);
    }
    if count > remaining {
        return Err(TransportError::Protocol(
            "I/O byte count exceeds the remaining buffer".into(),
        ));
    }
    Ok(count)
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TransportError {}

#[cfg(windows)]
mod windows_pipe {
    use super::{TransportError, MAX_SESSION_FRAMES};
    use audiorouter_protocol::{decode_frame, encode_frame, MAX_FRAME_BYTES};
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_ALREADY_EXISTS, GENERIC_READ, GENERIC_WRITE,
        HANDLE, HLOCAL, INVALID_HANDLE_VALUE,
    };
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    };
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_SHARE_NONE, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
        WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    use windows::Win32::System::Threading::{
        CreateMutexW, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(once(0)).collect()
    }

    fn singleton_name(pipe_name: &str, user_sid: &str) -> Vec<u16> {
        let suffix: String = format!("{user_sid}-{pipe_name}")
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        wide(&format!(r"Local\AudioRouter-{suffix}"))
    }

    fn acquire_singleton(pipe_name: &str) -> Result<Handle, TransportError> {
        let name = singleton_name(pipe_name, &current_user_sid()?);
        let handle =
            unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }.map_err(win_error)?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            drop(Handle(handle));
            return Err(TransportError::Windows(
                "another backend already owns this user pipe".into(),
            ));
        }
        Ok(Handle(handle))
    }

    #[must_use = "the singleton handle must stay alive while serving"]
    pub struct ServerSingleton(Handle);

    impl Drop for ServerSingleton {
        fn drop(&mut self) {
            let _ = &self.0;
        }
    }

    pub fn acquire_server_singleton(pipe_name: &str) -> Result<ServerSingleton, TransportError> {
        Ok(ServerSingleton(acquire_singleton(pipe_name)?))
    }

    fn check_name(name: &str) -> Result<(), TransportError> {
        if !name.starts_with(r"\\.\pipe\") || name.len() <= 9 || name.contains('\0') {
            Err(TransportError::InvalidPipeName)
        } else {
            Ok(())
        }
    }

    fn win_error(error: impl std::fmt::Display) -> TransportError {
        TransportError::Windows(error.to_string())
    }

    fn token_user_sid_string(token: HANDLE) -> Result<String, TransportError> {
        let mut required = 0;
        let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
        if required == 0 {
            return Err(TransportError::Windows(
                "token user information size was zero".into(),
            ));
        }
        let mut buffer = vec![0u8; required as usize];
        unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                buffer.len() as u32,
                &mut required,
            )
        }
        .map_err(win_error)?;
        let user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
        let mut string_sid = windows::core::PWSTR::null();
        unsafe { ConvertSidToStringSidW(user.User.Sid, &mut string_sid) }.map_err(win_error)?;
        if string_sid.is_null() {
            return Err(TransportError::Windows(
                "Windows returned a null SID string".into(),
            ));
        }
        let text = unsafe {
            let mut length = 0;
            while *string_sid.0.add(length) != 0 {
                length += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(string_sid.0, length))
        };
        unsafe { LocalFree(Some(HLOCAL(string_sid.0.cast()))) };
        Ok(text)
    }

    pub fn client_user_sid(client_process_id: u32) -> Result<String, TransportError> {
        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, client_process_id) }
                .map_err(win_error)?;
        let process = Handle(process);
        let mut token = INVALID_HANDLE_VALUE;
        unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) }.map_err(win_error)?;
        let token = Handle(token);
        token_user_sid_string(token.0)
    }

    pub fn current_user_sid() -> Result<String, TransportError> {
        client_user_sid(std::process::id())
    }

    struct SecurityDescriptor(windows::Win32::Security::PSECURITY_DESCRIPTOR);
    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                unsafe { LocalFree(Some(HLOCAL(self.0 .0))) };
            }
        }
    }

    fn owner_only_security() -> Result<SecurityDescriptor, TransportError> {
        let mut descriptor = windows::Win32::Security::PSECURITY_DESCRIPTOR(std::ptr::null_mut());
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                windows::core::w!("D:P(A;;GA;;;OW)"),
                1,
                std::ptr::addr_of_mut!(descriptor),
                None,
            )
        }
        .map_err(win_error)?;
        Ok(SecurityDescriptor(descriptor))
    }

    /// Compare the connected client process token's user SID with this process.
    /// This is a same-user check, not a replacement for a restrictive pipe ACL.
    pub fn client_is_same_user(client_process_id: u32) -> Result<bool, TransportError> {
        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, client_process_id) }
                .map_err(win_error)?;
        let process = Handle(process);
        let mut client_token = INVALID_HANDLE_VALUE;
        unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut client_token) }
            .map_err(win_error)?;
        let client_token = Handle(client_token);

        let current_process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, std::process::id()) }
                .map_err(win_error)?;
        let current_process = Handle(current_process);
        let mut current_token = INVALID_HANDLE_VALUE;
        unsafe { OpenProcessToken(current_process.0, TOKEN_QUERY, &mut current_token) }
            .map_err(win_error)?;
        let current_token = Handle(current_token);

        Ok(token_user_sid_string(client_token.0)? == token_user_sid_string(current_token.0)?)
    }

    struct Handle(HANDLE);
    unsafe impl Send for Handle {}
    impl Drop for Handle {
        fn drop(&mut self) {
            if self.0 != INVALID_HANDLE_VALUE && !self.0.is_invalid() {
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    fn read_exact(handle: HANDLE, mut output: &mut [u8]) -> Result<(), TransportError> {
        while !output.is_empty() {
            let mut count = 0;
            unsafe { ReadFile(handle, Some(output), Some(&mut count), None) }.map_err(win_error)?;
            let count = super::checked_io_count(count, output.len())?;
            output = &mut output[count..];
        }
        Ok(())
    }

    fn write_all(handle: HANDLE, mut input: &[u8]) -> Result<(), TransportError> {
        while !input.is_empty() {
            let mut count = 0;
            unsafe { WriteFile(handle, Some(input), Some(&mut count), None) }.map_err(win_error)?;
            let count = super::checked_io_count(count, input.len())?;
            input = &input[count..];
        }
        Ok(())
    }

    fn read_frame(handle: HANDLE) -> Result<Vec<u8>, TransportError> {
        let mut header = [0u8; 4];
        read_exact(handle, &mut header)?;
        let length = u32::from_le_bytes(header) as usize;
        if length > MAX_FRAME_BYTES {
            return Err(TransportError::Protocol("frame exceeds maximum".into()));
        }
        let mut frame = Vec::with_capacity(4 + length);
        frame.extend_from_slice(&header);
        frame.resize(4 + length, 0);
        read_exact(handle, &mut frame[4..])?;
        Ok(frame)
    }

    /// Pipe instances one server may hold open at once. The production backend
    /// accepts on this many I/O threads so the window, tray, HTTP adapter and
    /// MCP do not queue behind each other's connection; requests are still
    /// dispatched one at a time by the control thread. Every instance of a
    /// name must be created with the same value.
    pub const SERVER_PIPE_INSTANCES: u32 = 4;

    fn accept_client(name: &str) -> Result<(Handle, u32), TransportError> {
        check_name(name)?;
        let name = wide(name);
        let security = owner_only_security()?;
        let attributes = windows::Win32::Security::SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<windows::Win32::Security::SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.0 .0,
            bInheritHandle: false.into(),
        };
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                SERVER_PIPE_INSTANCES,
                (MAX_FRAME_BYTES + 4) as u32,
                (MAX_FRAME_BYTES + 4) as u32,
                0,
                Some(&attributes),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_invalid() {
            return Err(win_error(windows::core::Error::from_thread()));
        }
        let handle = Handle(handle);
        if let Err(error) = unsafe { ConnectNamedPipe(handle.0, None) } {
            if error.code().0 != 0x8007_0217u32 as i32 {
                return Err(win_error(error));
            }
        }
        let mut client_process_id = 0;
        unsafe { GetNamedPipeClientProcessId(handle.0, &mut client_process_id) }
            .map_err(win_error)?;
        if client_process_id == 0 {
            return Err(TransportError::Windows(
                "named pipe returned no client process ID".into(),
            ));
        }
        if !client_is_same_user(client_process_id)? {
            return Err(TransportError::Windows(
                "named pipe client is not the server user".into(),
            ));
        }
        Ok((handle, client_process_id))
    }

    /// Serve exactly one framed request, then disconnect and close the pipe.
    /// The pipe is created with an owner-only ACL and the client SID is checked
    /// before its request is read. Production callers should still review the
    /// deployment account/service model and authorization scopes.
    pub fn serve_once<F>(name: &str, handler: F) -> Result<(), TransportError>
    where
        F: FnOnce(&[u8]) -> Result<Vec<u8>, TransportError>,
    {
        serve_once_with_client(name, |_, frame| handler(frame))
    }

    /// Serve one request and provide the connected client's Windows process ID.
    /// The process ID is an identity input, not authentication by itself; callers
    /// must still validate the process token/SID before allowing sensitive methods.
    pub fn serve_once_with_client<F>(name: &str, handler: F) -> Result<(), TransportError>
    where
        F: FnOnce(u32, &[u8]) -> Result<Vec<u8>, TransportError>,
    {
        serve_once_with_client_optional(name, |client_pid, frame| {
            handler(client_pid, frame).map(Some)
        })
    }

    /// Serve one request where `None` means JSON-RPC notification/no response.
    pub fn serve_once_with_client_optional<F>(name: &str, handler: F) -> Result<(), TransportError>
    where
        F: FnOnce(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
    {
        let (handle, client_process_id) = accept_client(name)?;
        let request = read_frame(handle.0)?;
        if let Some(response) = handler(client_process_id, &request)? {
            write_all(handle.0, &response)?;
            unsafe { FlushFileBuffers(handle.0) }.map_err(win_error)?;
        }
        let _ = unsafe { DisconnectNamedPipe(handle.0) };
        Ok(())
    }

    /// Serve a bounded persistent client session. The same authenticated pipe
    /// connection may carry `frames` requests; it is disconnected afterward so
    /// ownership and shutdown remain deterministic for callers and tests.
    pub fn serve_session<F>(name: &str, frames: usize, mut handler: F) -> Result<(), TransportError>
    where
        F: FnMut(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
    {
        if frames == 0 || frames > MAX_SESSION_FRAMES {
            return Err(TransportError::Protocol(
                "session frame count must be between 1 and 500".into(),
            ));
        }
        let (handle, client_process_id) = accept_client(name)?;
        for _ in 0..frames {
            let request = read_frame(handle.0)?;
            if let Some(response) = handler(client_process_id, &request)? {
                write_all(handle.0, &response)?;
                unsafe { FlushFileBuffers(handle.0) }.map_err(win_error)?;
            }
        }
        let _ = unsafe { DisconnectNamedPipe(handle.0) };
        Ok(())
    }

    /// Serve a fixed number of sequential authenticated connections.
    /// A bounded loop makes lifecycle tests deterministic; a production daemon
    /// can own the outer restart/shutdown policy around `serve_once_with_client`.
    pub fn serve_connections<F>(
        name: &str,
        connections: usize,
        mut handler: F,
    ) -> Result<(), TransportError>
    where
        F: FnMut(u32, &[u8]) -> Result<Vec<u8>, TransportError>,
    {
        let _singleton = acquire_singleton(name)?;
        for _ in 0..connections {
            serve_once_with_client(name, |client_pid, frame| handler(client_pid, frame))?;
        }
        Ok(())
    }

    /// Longest wait for a busy pipe (every server instance serving another
    /// client). Requests are short, so this covers bursts from the window,
    /// tray, HTTP workers and MCP at once.
    const PIPE_BUSY_WAIT: std::time::Duration = std::time::Duration::from_secs(2);
    /// Longest wait for a pipe that does not exist yet. Kept short so callers
    /// learn quickly that no backend is running.
    const PIPE_MISSING_WAIT: std::time::Duration = std::time::Duration::from_millis(100);

    /// Open a client connection to `name` (NUL-terminated UTF-16).
    fn connect_client(name: &[u16]) -> Result<Handle, TransportError> {
        const ERROR_FILE_NOT_FOUND: i32 = 0x8007_0002_u32 as i32;
        const ERROR_PIPE_BUSY: i32 = 0x8007_00E7_u32 as i32;
        let started = std::time::Instant::now();
        loop {
            // SAFETY: `name` is NUL-terminated and outlives the call; the
            // returned handle is owned by the `Handle` wrapper below.
            let result = unsafe {
                CreateFileW(
                    PCWSTR(name.as_ptr()),
                    (GENERIC_READ | GENERIC_WRITE).0,
                    FILE_SHARE_NONE,
                    None,
                    OPEN_EXISTING,
                    Default::default(),
                    None,
                )
            };
            let error = match result {
                Ok(handle) => return Ok(Handle(handle)),
                Err(error) => error,
            };
            let elapsed = started.elapsed();
            match error.code().0 {
                ERROR_PIPE_BUSY if elapsed < PIPE_BUSY_WAIT => {
                    let remaining = (PIPE_BUSY_WAIT - elapsed).as_millis().clamp(1, 50) as u32;
                    // SAFETY: as above; this only waits for a free instance.
                    let _ = unsafe { WaitNamedPipeW(PCWSTR(name.as_ptr()), remaining) };
                }
                ERROR_FILE_NOT_FOUND if elapsed < PIPE_MISSING_WAIT => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                ERROR_PIPE_BUSY | ERROR_FILE_NOT_FOUND => {
                    return Err(TransportError::Windows(
                        "timed out waiting for a free named-pipe instance".into(),
                    ))
                }
                _ => return Err(win_error(error)),
            }
        }
    }

    /// Connect to a local named pipe and exchange one framed message.
    pub fn round_trip(name: &str, request: &[u8]) -> Result<Vec<u8>, TransportError> {
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = connect_client(&name)?;
        write_all(handle.0, request)?;
        read_frame(handle.0)
    }

    /// Exchange one request frame and read exactly `responses` response frames.
    pub fn round_trip_many(
        name: &str,
        request: &[u8],
        responses: usize,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        super::validate_response_count(responses)?;
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = connect_client(&name)?;
        write_all(handle.0, request)?;
        (0..responses).map(|_| read_frame(handle.0)).collect()
    }

    /// Exchange the same framed request repeatedly over one authenticated
    /// connection. This is a deterministic transport primitive for exercising
    /// subscription/reconnect lifetimes; callers can encode distinct frames
    /// with `round_trip_many` or a higher-level client protocol.
    pub fn round_trip_session(
        name: &str,
        request: &[u8],
        frames: usize,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        if frames == 0 || frames > MAX_SESSION_FRAMES {
            return Err(TransportError::Protocol(
                "session frame count must be between 1 and 500".into(),
            ));
        }
        let requests = vec![request; frames];
        round_trip_session_many(name, &requests)
    }

    /// Exchange distinct framed requests over one authenticated connection.
    /// The request count is bounded so a caller cannot hold a pipe forever.
    pub fn round_trip_session_many(
        name: &str,
        requests: &[&[u8]],
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        if requests.is_empty() || requests.len() > MAX_SESSION_FRAMES {
            return Err(TransportError::Protocol(
                "session frame count must be between 1 and 500".into(),
            ));
        }
        check_name(name)?;
        for request in requests {
            if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
                return Err(TransportError::Protocol("invalid request frame".into()));
            }
        }
        let name = wide(name);
        let handle = connect_client(&name)?;
        let mut responses = Vec::with_capacity(requests.len());
        for request in requests {
            write_all(handle.0, request)?;
            responses.push(read_frame(handle.0)?);
        }
        Ok(responses)
    }

    /// Send a notification frame and close after the server has received it.
    pub fn send_oneway(name: &str, request: &[u8]) -> Result<(), TransportError> {
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = connect_client(&name)?;
        write_all(handle.0, request)
    }

    pub fn echo_handler(frame: &[u8]) -> Result<Vec<u8>, TransportError> {
        decode_frame::<serde_json::Value>(frame)
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        encode_frame(&serde_json::json!({"ok": true}))
            .map_err(|e| TransportError::Protocol(e.to_string()))
    }
}

#[cfg(windows)]
pub use windows_pipe::{
    acquire_server_singleton, client_is_same_user, client_user_sid, current_user_sid, echo_handler,
    round_trip, round_trip_many, round_trip_session, round_trip_session_many, send_oneway,
    serve_connections, serve_once, serve_once_with_client, serve_once_with_client_optional,
    serve_session, ServerSingleton, SERVER_PIPE_INSTANCES,
};

#[cfg(windows)]
pub fn serve_control_connections(
    name: &str,
    connections: usize,
    mut plane: audiorouter_control::ControlPlane,
    grant: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    let _singleton = acquire_server_singleton(name)?;
    for _ in 0..connections {
        serve_once_with_client_optional(name, |client_pid, frame| {
            let client_id = client_user_sid(client_pid)?;
            let started = std::time::Instant::now();
            let responses = plane
                .dispatch_frame_authorized_for_client(frame, &client_id, &grant)
                .map_err(|error| TransportError::Protocol(error.to_string()))?;
            let verbose =
                plane.verbose_diagnostics_active(audiorouter_protocol::diagnostics::unix_time_ms());
            log_backend_rpc(frame, &responses, verbose, started.elapsed());
            if responses.is_empty() {
                Ok(None)
            } else {
                let total = responses.iter().map(Vec::len).sum();
                let mut combined = Vec::with_capacity(total);
                for response in responses {
                    combined.extend_from_slice(&response);
                }
                Ok(Some(combined))
            }
        })?;
    }
    Ok(())
}

#[cfg(windows)]
/// Serve a bounded number of authenticated persistent sessions. Each session
/// accepts a fixed number of framed requests before disconnecting, preserving
/// control-plane state across reconnects without introducing an unbounded
/// daemon loop.
pub fn serve_control_sessions(
    name: &str,
    sessions: usize,
    frames_per_session: usize,
    mut plane: audiorouter_control::ControlPlane,
    grant: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    let _singleton = acquire_server_singleton(name)?;
    for _ in 0..sessions {
        serve_session(name, frames_per_session, |client_pid, frame| {
            let client_id = client_user_sid(client_pid)?;
            let responses = plane
                .dispatch_frame_authorized_for_client(frame, &client_id, &grant)
                .map_err(|error| TransportError::Protocol(error.to_string()))?;
            if responses.is_empty() {
                Ok(None)
            } else {
                let total = responses.iter().map(Vec::len).sum();
                let mut combined = Vec::with_capacity(total);
                for response in responses {
                    combined.extend_from_slice(&response);
                }
                Ok(Some(combined))
            }
        })?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn serve_control_connections_as_role(
    name: &str,
    connections: usize,
    plane: audiorouter_control::ControlPlane,
    role: audiorouter_control::ClientRole,
) -> Result<(), TransportError> {
    serve_control_connections(
        name,
        connections,
        plane,
        audiorouter_control::ClientGrant::for_role(role),
    )
}

#[cfg(windows)]
pub fn serve_control_connections_for_current_user(
    name: &str,
    connections: usize,
    plane: audiorouter_control::ControlPlane,
) -> Result<(), TransportError> {
    let sid = current_user_sid()?;
    let grant = plane
        .grant_for_client(&sid)
        .map_err(|error| TransportError::Protocol(format!("enrollment lookup failed: {error:?}")))?
        .ok_or_else(|| TransportError::Windows("current user is not enrolled".into()))?;
    serve_control_connections(name, connections, plane, grant)
}

#[cfg(windows)]
/// Serve authenticated control connections for the lifetime of the hosting
/// process. Unlike the bounded acceptance helper, this is the production
/// backend path; the named-pipe singleton and the control plane remain owned
/// until the process exits or the pipe reports a terminal error.
pub fn serve_control_connections_forever_for_current_user(
    name: &str,
    plane: audiorouter_control::ControlPlane,
) -> Result<(), TransportError> {
    let sid = current_user_sid()?;
    let grant = plane
        .grant_for_client(&sid)
        .map_err(|error| TransportError::Protocol(format!("enrollment lookup failed: {error:?}")))?
        .ok_or_else(|| TransportError::Windows("current user is not enrolled".into()))?;
    serve_control_connections_forever_with_grant(name, plane, grant)
}

/// Interval between backend audio service passes. Well below the common
/// 10 ms WASAPI shared-mode engine period, so a pass is never the reason a
/// render buffer empties; each pass only drains what is already available.
pub const AUDIO_SERVICE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1);

#[cfg(windows)]
/// One received control frame handed from the pipe I/O thread to the thread
/// that owns the control plane.
struct ControlFrame {
    client_pid: u32,
    frame: Vec<u8>,
    reply: std::sync::mpsc::SyncSender<Result<Option<Vec<u8>>, TransportError>>,
}

#[cfg(windows)]
/// Serve the current-user pipe with an explicitly selected process grant.
/// The grant is not persisted; callers must perform any durable enrollment
/// checks before entering this lifetime-serving loop.
///
/// The production backend loop. The control plane stays on the calling
/// thread (it owns thread-affine COM objects), while a separate I/O thread
/// accepts pipe clients and forwards each frame here.
///
/// Between requests this thread pumps every running native worker at
/// [`AUDIO_SERVICE_INTERVAL`]. Prepared native workers only advance when
/// pumped; before this loop owned pumping, the UI pumped them with RPCs, so a
/// throttled WebView timer (minimized or occluded window) or a slow request
/// starved the render buffers and produced audible crackling. UI pump
/// requests remain valid and report diagnostics, but continuity no longer
/// depends on them.
pub fn serve_control_connections_forever_with_grant(
    name: &str,
    plane: audiorouter_control::ControlPlane,
    grant: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    serve_control_connections_forever_observed(name, plane, grant, None)
}

/// Numeric phase timings for native continuity qualification. No client data.
#[cfg(windows)]
#[derive(Clone, Copy, Debug)]
pub struct AudioServicePassTiming {
    pub elapsed: std::time::Duration,
    pub wait: std::time::Duration,
    pub dispatch: std::time::Duration,
    pub service: std::time::Duration,
    pub recorder_drain_micros: u64,
    pub running: bool,
}

/// Internal qualification seam; this is not an application API. The observer
/// must use a bounded channel: full or disconnected channels drop samples.
/// Reporting occurs after pumping, never in a DSP callback. Production uses
/// `None`, so no timing probe runs unless explicitly supplied by a harness.
#[cfg(windows)]
pub fn serve_control_connections_forever_observed(
    name: &str,
    mut plane: audiorouter_control::ControlPlane,
    grant: audiorouter_control::ClientGrant,
    observer: Option<std::sync::mpsc::SyncSender<AudioServicePassTiming>>,
) -> Result<(), TransportError> {
    let _singleton = acquire_server_singleton(name)?;
    let (io, received) = spawn_control_io(name)?;
    let (_scheduling, _capabilities) = audiorouter_windows_audio::AudioServiceThreadGuard::enter();
    plane.mark_audio_service_started();
    // A panic in dispatch or in an audio service pass must end this serve call
    // with an error the caller's supervisor can restart, never leave the
    // process alive with no control backend (2026-10-05 resume defect).
    let served = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        serve_control_plane_frames(&mut plane, &grant, &received, observer.as_ref())
    }));
    match served {
        // Every I/O thread has ended (the receiver disconnected).
        Ok(()) => io.join(),
        Err(_) => {
            drop(received);
            stop_control_io(name, io);
            Err(TransportError::Protocol("control plane panicked".into()))
        }
    }
}

/// The pipe I/O threads (one per pipe instance) and their shared stop flag.
#[cfg(windows)]
struct ControlIo {
    threads: Vec<std::thread::JoinHandle<Result<(), TransportError>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(windows)]
impl ControlIo {
    /// Wait for every I/O thread; the first failure is the serve result.
    fn join(self) -> Result<(), TransportError> {
        let mut result = Ok(());
        for thread in self.threads {
            let outcome = thread.join().unwrap_or_else(|_| {
                Err(TransportError::Protocol(
                    "control I/O thread panicked".into(),
                ))
            });
            if result.is_ok() {
                result = outcome;
            }
        }
        result
    }
}

/// Consecutive connection failures after which an I/O thread gives up. Only
/// a persistent fault (the pipe can no longer be created) reaches it; one
/// client that disconnects early or fails the same-user check does not.
#[cfg(windows)]
const MAX_CONSECUTIVE_PIPE_FAILURES: u32 = 50;

/// Start one pipe I/O thread per server instance. Each accepts clients and
/// forwards every frame to the returned receiver, waiting for the control
/// plane's reply, so requests are still handled one at a time. A failed
/// connection is dropped and the thread keeps serving; the threads end when
/// the control plane stops (the stop flag, or a closed receiver).
#[cfg(windows)]
fn spawn_control_io(
    name: &str,
) -> Result<(ControlIo, std::sync::mpsc::Receiver<ControlFrame>), TransportError> {
    use std::sync::atomic::Ordering;
    let (frames, received) = std::sync::mpsc::sync_channel::<ControlFrame>(0);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut threads = Vec::new();
    for index in 0..SERVER_PIPE_INSTANCES {
        let io_name = name.to_owned();
        let frames = frames.clone();
        let stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name(format!("audiorouter-control-io-{index}"))
            .spawn(move || -> Result<(), TransportError> {
                let mut failures = 0_u32;
                while !stop.load(Ordering::Acquire) {
                    let served = serve_once_with_client_optional(&io_name, |client_pid, frame| {
                        let (reply, response) = std::sync::mpsc::sync_channel(1);
                        let stopped = || {
                            stop.store(true, Ordering::Release);
                            TransportError::Protocol("control plane stopped".into())
                        };
                        frames
                            .send(ControlFrame {
                                client_pid,
                                frame: frame.to_vec(),
                                reply,
                            })
                            .map_err(|_| stopped())?;
                        response.recv().map_err(|_| stopped())?
                    });
                    match served {
                        Ok(()) => failures = 0,
                        Err(_) if stop.load(Ordering::Acquire) => break,
                        Err(error) => {
                            failures += 1;
                            if failures >= MAX_CONSECUTIVE_PIPE_FAILURES {
                                return Err(error);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                    }
                }
                Ok(())
            })
            .map_err(|error| TransportError::Windows(format!("control I/O thread: {error}")))?;
        threads.push(thread);
    }
    Ok((ControlIo { threads, stop }, received))
}

/// Wake the I/O threads after the control plane stopped, so they release
/// their pipe instances before a restarted server creates new ones. Threads
/// usually block accepting a client; a connection of our own reaches each
/// one. Bounded: an unresponsive thread is left detached rather than hanging
/// the supervisor.
#[cfg(windows)]
fn stop_control_io(name: &str, io: ControlIo) {
    io.stop.store(true, std::sync::atomic::Ordering::Release);
    let mut threads = io.threads;
    for _ in 0..50 {
        let (finished, running): (Vec<_>, Vec<_>) =
            threads.into_iter().partition(|thread| thread.is_finished());
        for thread in finished {
            let _ = thread.join();
        }
        if running.is_empty() {
            return;
        }
        threads = running;
        for _ in 0..threads.len() {
            let _ = send_oneway(name, &0_u32.to_le_bytes());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// The control-plane side of [`serve_control_connections_forever_observed`]:
/// dispatch received frames and pump native audio between them. Returns when
/// the I/O thread has stopped.
#[cfg(windows)]
fn serve_control_plane_frames(
    plane: &mut audiorouter_control::ControlPlane,
    grant: &audiorouter_control::ClientGrant,
    received: &std::sync::mpsc::Receiver<ControlFrame>,
    observer: Option<&std::sync::mpsc::SyncSender<AudioServicePassTiming>>,
) {
    let origin = std::time::Instant::now();
    loop {
        let wait_start = observer.as_ref().map(|_| std::time::Instant::now());
        let mut dispatch = std::time::Duration::ZERO;
        let wait;
        match received.recv_timeout(AUDIO_SERVICE_INTERVAL) {
            Ok(request) => {
                wait = wait_start.map(|start| start.elapsed()).unwrap_or_default();
                let dispatch_start = observer.as_ref().map(|_| std::time::Instant::now());
                let result =
                    dispatch_control_frame(plane, grant, request.client_pid, &request.frame);
                let _ = request.reply.send(result);
                dispatch = dispatch_start
                    .map(|start| start.elapsed())
                    .unwrap_or_default();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                wait = wait_start.map(|start| start.elapsed()).unwrap_or_default();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
        let service_start = std::time::Instant::now();
        let running = plane.service_running_native_audio(service_start) != 0;
        if let Some(observer) = observer {
            let _ = observer.try_send(AudioServicePassTiming {
                elapsed: service_start.saturating_duration_since(origin),
                wait,
                dispatch,
                service: service_start.elapsed(),
                recorder_drain_micros: plane.audio_service_stats().recorder_drain_micros,
                running,
            });
        }
    }
}

#[cfg(windows)]
fn dispatch_control_frame(
    plane: &mut audiorouter_control::ControlPlane,
    grant: &audiorouter_control::ClientGrant,
    client_pid: u32,
    frame: &[u8],
) -> Result<Option<Vec<u8>>, TransportError> {
    let client_id = client_user_sid(client_pid)?;
    let started = std::time::Instant::now();
    let responses = plane
        .dispatch_frame_authorized_for_client(frame, &client_id, grant)
        .map_err(|error| TransportError::Protocol(error.to_string()))?;
    // Only the window state and a duration are read here; the logger thread
    // does all parsing and file I/O.
    let verbose =
        plane.verbose_diagnostics_active(audiorouter_protocol::diagnostics::unix_time_ms());
    log_backend_rpc(frame, &responses, verbose, started.elapsed());
    if responses.is_empty() {
        Ok(None)
    } else {
        let total = responses.iter().map(Vec::len).sum();
        let mut combined = Vec::with_capacity(total);
        for response in responses {
            combined.extend_from_slice(&response);
        }
        Ok(Some(combined))
    }
}

#[cfg(not(windows))]
pub fn serve_control_sessions(
    _: &str,
    _: usize,
    _: usize,
    _: audiorouter_control::ControlPlane,
    _: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip(_: &str, _: &[u8]) -> Result<Vec<u8>, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_once<F>(_: &str, _: F) -> Result<(), TransportError>
where
    F: FnOnce(&[u8]) -> Result<Vec<u8>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_once_with_client_optional<F>(_: &str, _: F) -> Result<(), TransportError>
where
    F: FnOnce(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn send_oneway(_: &str, _: &[u8]) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip_many(
    _: &str,
    _: &[u8],
    responses: usize,
) -> Result<Vec<Vec<u8>>, TransportError> {
    validate_response_count(responses)?;
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip_session(_: &str, _: &[u8], _: usize) -> Result<Vec<Vec<u8>>, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip_session_many(_: &str, _: &[&[u8]]) -> Result<Vec<Vec<u8>>, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_session<F>(_: &str, _: usize, _: F) -> Result<(), TransportError>
where
    F: FnMut(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn client_is_same_user(_: u32) -> Result<bool, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn client_user_sid(_: u32) -> Result<String, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn current_user_sid() -> Result<String, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections_for_current_user(
    _: &str,
    _: usize,
    _: audiorouter_control::ControlPlane,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections_forever_for_current_user(
    _: &str,
    _: audiorouter_control::ControlPlane,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_connections<F>(_: &str, _: usize, _: F) -> Result<(), TransportError>
where
    F: FnMut(u32, &[u8]) -> Result<Vec<u8>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections(
    _: &str,
    _: usize,
    _: audiorouter_control::ControlPlane,
    _: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections_as_role(
    _: &str,
    _: usize,
    _: audiorouter_control::ControlPlane,
    _: audiorouter_control::ClientRole,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_control::{ClientGrant, ClientRole, ControlPlane};
    use audiorouter_protocol::encode_frame;

    #[test]
    fn backend_diagnostics_keep_method_and_graph_counts_without_rpc_params() {
        let request = encode_frame(&serde_json::json!({
            "jsonrpc":"2.0", "id":7, "method":"sessions.get",
            "params":{"sessionId":"private-session", "secret":"private-value"}
        }))
        .unwrap();
        let response = encode_frame(&serde_json::json!({
            "jsonrpc":"2.0", "id":7,
            "result":{"revision":9, "nodes":[{"id":"one"},{"id":"two"}], "edges":[{"id":"edge"}]}
        }))
        .unwrap();
        let records = backend_rpc_log_records(&request, &[response], 1234, None);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["method"], "sessions.get");
        assert_eq!(records[0]["summary"]["revision"], 9);
        assert_eq!(records[0]["summary"]["nodes"], 2);
        assert_eq!(records[0]["summary"]["edges"], 1);
        assert!(!records[0].to_string().contains("private-value"));
        assert!(!records[0].to_string().contains("private-session"));
    }

    #[test]
    fn backend_diagnostics_keep_safe_error_category_without_error_text() {
        let request = encode_frame(&serde_json::json!({
            "jsonrpc":"2.0", "id":8, "method":"graph.commit",
            "params":{"sessionId":"private-session", "name":"private-name"}
        }))
        .unwrap();
        let response = encode_frame(&serde_json::json!({
            "jsonrpc":"2.0", "id":8,
            "error":{"code":-32001, "message":"permission denied: GraphWrite private-name", "data":{"code":"permissionDenied"}}
        })).unwrap();
        let records = backend_rpc_log_records(&request, &[response], 1234, None);
        assert_eq!(records[0]["outcome"], "error");
        assert_eq!(records[0]["errorCode"], -32001);
        assert_eq!(records[0]["errorKind"], "permissionDenied");
        assert!(!records[0].to_string().contains("private-name"));
        assert!(!records[0].to_string().contains("GraphWrite"));
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_mutex_serializes_concurrent_log_writers() {
        let name = format!("AudioRouter.TestDiagnostics.{}", std::process::id());
        let first = acquire_diagnostic_mutex(&name).expect("first mutex owner");
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished_rx) = std::sync::mpsc::channel();
        let worker_name = name.clone();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let _second = acquire_diagnostic_mutex(&worker_name).expect("second mutex owner");
            finished_tx.send(()).unwrap();
        });
        started_rx.recv().unwrap();
        assert!(finished_rx.try_recv().is_err());
        drop(first);
        finished_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("second writer enters after first releases");
        worker.join().unwrap();
    }

    /// 2026-10-05: a panicking control plane left the I/O thread holding the
    /// only pipe instance, so no restarted server could ever accept again.
    #[cfg(windows)]
    #[test]
    fn stopped_control_plane_releases_the_pipe_for_a_restarted_server() {
        let name = format!(
            r"\\.\pipe\audiorouter-test-io-release-{}",
            std::process::id()
        );
        let (io, received) = spawn_control_io(&name).unwrap();
        // Let the I/O thread block in ConnectNamedPipe, as in production.
        std::thread::sleep(std::time::Duration::from_millis(100));
        drop(received);
        stop_control_io(&name, io);
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_once_with_client_optional(&server_name, |_, frame| Ok(Some(frame.to_vec())))
        });
        let request = encode_frame(&serde_json::json!({"ping": true})).unwrap();
        let response = round_trip(&name, &request).expect("restarted server accepts");
        assert_eq!(response, request);
        server.join().unwrap().unwrap();
    }

    /// Answer every forwarded frame with itself after `delay`, as a slow
    /// control plane would, until the I/O threads stop.
    #[cfg(windows)]
    fn echo_control_plane(
        received: std::sync::mpsc::Receiver<ControlFrame>,
        delay: std::time::Duration,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            for request in received {
                std::thread::sleep(delay);
                let _ = request.reply.send(Ok(Some(request.frame)));
            }
        })
    }

    /// 2026-10-07 review P0-2: one pipe instance plus a ~100 ms client budget
    /// made concurrent clients (window, tray, HTTP, MCP) fail at random.
    #[cfg(windows)]
    #[test]
    fn concurrent_clients_all_reach_a_slow_control_plane() {
        let name = format!(
            r"\\.\pipe\audiorouter-test-io-concurrent-{}",
            std::process::id()
        );
        let (io, received) = spawn_control_io(&name).unwrap();
        // Eight clients at 20 ms each is 160 ms of queued work, well past the
        // old 100 ms budget for the last client in line.
        let plane = echo_control_plane(received, std::time::Duration::from_millis(20));
        std::thread::sleep(std::time::Duration::from_millis(100));
        let clients = (0..8)
            .map(|client| {
                let name = name.clone();
                std::thread::spawn(move || {
                    let request = encode_frame(&serde_json::json!({ "client": client })).unwrap();
                    assert_eq!(round_trip(&name, &request).unwrap(), request);
                })
            })
            .collect::<Vec<_>>();
        for client in clients {
            client.join().unwrap();
        }
        stop_control_io(&name, io);
        plane.join().unwrap();
    }

    /// Before 2026-10-07 one bad connection ended the only I/O thread, which
    /// stopped the control plane and restarted the backend.
    #[cfg(windows)]
    #[test]
    fn a_malformed_client_does_not_stop_the_control_io() {
        let name = format!(
            r"\\.\pipe\audiorouter-test-io-malformed-{}",
            std::process::id()
        );
        let (io, received) = spawn_control_io(&name).unwrap();
        let plane = echo_control_plane(received, std::time::Duration::ZERO);
        std::thread::sleep(std::time::Duration::from_millis(100));
        // A length header far beyond the frame limit fails that connection.
        let _ = send_oneway(&name, &[0xff, 0xff, 0xff, 0xff]);
        let request = encode_frame(&serde_json::json!({"after": "malformed"})).unwrap();
        assert_eq!(round_trip(&name, &request).unwrap(), request);
        assert!(io.threads.iter().all(|thread| !thread.is_finished()));
        stop_control_io(&name, io);
        plane.join().unwrap();
    }

    #[test]
    fn checked_io_count_rejects_zero_and_overreported_bytes() {
        assert_eq!(checked_io_count(2, 4).unwrap(), 2);
        assert_eq!(checked_io_count(0, 4), Err(TransportError::UnexpectedEof));
        assert_eq!(
            checked_io_count(5, 4),
            Err(TransportError::Protocol(
                "I/O byte count exceeds the remaining buffer".into()
            ))
        );
    }

    #[test]
    fn rejects_non_pipe_names_without_touching_the_system() {
        let result = round_trip("not-a-pipe", &[]);
        assert_eq!(result, Err(TransportError::InvalidPipeName));
    }

    #[test]
    fn rejects_unbounded_persistent_sessions_before_platform_access() {
        let request = [0, 0, 0, 0];
        assert_eq!(
            round_trip_session("not-a-pipe", &request, MAX_SESSION_FRAMES + 1),
            Err(TransportError::Protocol(
                "session frame count must be between 1 and 500".into()
            ))
        );
        assert_eq!(
            serve_session("not-a-pipe", MAX_SESSION_FRAMES + 1, |_, _| Ok(None)),
            Err(TransportError::Protocol(
                "session frame count must be between 1 and 500".into()
            ))
        );
    }

    #[test]
    fn rejects_unbounded_multi_response_requests_before_platform_access() {
        let request = [0, 0, 0, 0];
        assert_eq!(
            round_trip_many("not-a-pipe", &request, 0),
            Err(TransportError::Protocol(
                "response frame count must be between 1 and 500".into()
            ))
        );
        assert_eq!(
            round_trip_many("not-a-pipe", &request, MAX_SESSION_FRAMES + 1),
            Err(TransportError::Protocol(
                "response frame count must be between 1 and 500".into()
            ))
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_round_trip() {
        let name = format!(r"\\.\pipe\audiorouter-test-{}", std::process::id());
        let request =
            encode_frame(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"system.describe"}))
                .unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || serve_once(&server_name, echo_handler));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let response = match round_trip(&name, &request) {
            Ok(response) => response,
            Err(error) => panic!(
                "client failed: {error:?}; server: {:?}",
                server.join().unwrap()
            ),
        };
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap()["ok"],
            true
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_dispatches_control_plane_json_rpc() {
        let name = format!(r"\\.\pipe\audiorouter-control-test-{}", std::process::id());
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "system.describe"
        }))
        .unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_connections_as_role(
                &server_name,
                1,
                ControlPlane::new("native-test"),
                ClientRole::Observer,
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let response = round_trip(&name, &request).unwrap();
        let response = serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap();
        assert_eq!(response["id"], 9);
        assert_eq!(response["result"]["protocolVersion"]["major"], 1);
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_applies_grants_before_mutation() {
        let name = format!(r"\\.\pipe\audiorouter-auth-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut plane = ControlPlane::new("native-auth-test");
            serve_once(&server_name, |frame| {
                let responses = plane
                    .dispatch_frame_authorized(frame, &ClientGrant::read_only())
                    .map_err(|error| TransportError::Protocol(error.to_string()))?;
                responses.into_iter().next().ok_or_else(|| {
                    TransportError::Protocol("missing authorization response".into())
                })
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "graph.commit"
        }))
        .unwrap();
        let response = round_trip(&name, &request).unwrap();
        let response = serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap();
        assert_eq!(response["error"]["code"], -32001);
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_notification_has_no_response_and_does_not_block() {
        let name = format!(
            r"\\.\pipe\audiorouter-notification-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_once_with_client_optional(&server_name, |_, frame| {
                let message = audiorouter_protocol::decode_rpc_frame(frame)
                    .map_err(|error| TransportError::Protocol(error.to_string()))?;
                assert!(
                    matches!(message, audiorouter_protocol::RpcMessage::Single(request) if request.is_notification())
                );
                Ok(None)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system.describe"
        }))
        .unwrap();
        send_oneway(&name, &request).unwrap();
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_preserves_all_batch_responses() {
        let name = format!(r"\\.\pipe\audiorouter-batch-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_connections(
                &server_name,
                1,
                ControlPlane::new("native-batch-test"),
                ClientGrant::read_only(),
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!([
            {"jsonrpc":"2.0","id":21,"method":"system.describe"},
            {"jsonrpc":"2.0","id":22,"method":"graph.commit"}
        ]))
        .unwrap();
        let responses = round_trip_many(&name, &request, 2).unwrap();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[0][4..]).unwrap()["id"],
            21
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["id"],
            22
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["error"]
                ["code"],
            -32001
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_rejects_oversized_frame_before_dispatch() {
        let name = format!(
            r"\\.\pipe\audiorouter-oversized-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_connections(
                &server_name,
                1,
                ControlPlane::new("native-oversized-test"),
                ClientGrant::read_only(),
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = ((audiorouter_protocol::MAX_FRAME_BYTES as u32) + 1)
            .to_le_bytes()
            .to_vec();
        let result = round_trip(&name, &request);
        assert!(result.is_err());
        let server_result = server.join().unwrap();
        assert!(matches!(server_result, Err(TransportError::Protocol(_))));
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_exposes_connected_client_process_id() {
        let name = format!(r"\\.\pipe\audiorouter-peer-test-{}", std::process::id());
        let request =
            encode_frame(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"status.get"}))
                .unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_once_with_client(&server_name, |client_pid, frame| {
                assert_eq!(client_pid, std::process::id());
                assert!(client_is_same_user(client_pid).unwrap());
                echo_handler(frame)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        round_trip(&name, &request).unwrap();
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_binds_current_user_enrollment_to_authenticated_sid() {
        let name = format!(r"\\.\pipe\audiorouter-enrolled-test-{}", std::process::id());
        let sid = current_user_sid().unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut plane = ControlPlane::new("native-enrolled-test");
            plane.enroll_client(sid, ClientRole::Observer).unwrap();
            serve_control_connections_for_current_user(&server_name, 1, plane)
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "system.describe"
        }))
        .unwrap();
        let response = round_trip(&name, &request).unwrap();
        let response = serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap();
        assert_eq!(response["id"], 31);
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn authenticated_server_accepts_sequential_connections() {
        let name = format!(r"\\.\pipe\audiorouter-loop-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, 2, |client_pid, frame| {
                assert!(client_is_same_user(client_pid).unwrap());
                echo_handler(frame)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        for id in [1, 2] {
            let request = encode_frame(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "status.get"
            }))
            .unwrap();
            round_trip(&name, &request).unwrap();
        }
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_keeps_one_authenticated_session_for_bounded_frames() {
        let name = format!(r"\\.\pipe\audiorouter-session-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_session(&server_name, 2, |client_pid, frame| {
                assert!(client_is_same_user(client_pid).unwrap());
                echo_handler(frame).map(Some)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 77,
            "method": "status.get"
        }))
        .unwrap();
        let responses = round_trip_session(&name, &request, 2).unwrap();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["ok"],
            true
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_exchanges_distinct_requests_on_one_authenticated_session() {
        let name = format!(
            r"\\.\pipe\audiorouter-distinct-session-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_session(&server_name, 2, |client_pid, frame| {
                assert!(client_is_same_user(client_pid).unwrap());
                let request = audiorouter_protocol::decode_frame::<serde_json::Value>(frame)
                    .map_err(|error| TransportError::Protocol(error.to_string()))?;
                audiorouter_protocol::encode_frame(&serde_json::json!({
                    "id": request["id"]
                }))
                .map(Some)
                .map_err(|error| TransportError::Protocol(error.to_string()))
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let first = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "status.get"
        }))
        .unwrap();
        let second = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 202,
            "method": "system.describe"
        }))
        .unwrap();
        let requests = [first.as_slice(), second.as_slice()];
        let responses = round_trip_session_many(&name, &requests).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[0][4..]).unwrap()["id"],
            101
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["id"],
            202
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_control_session_replays_state_after_a_distinct_mutation_request() {
        let name = format!(
            r"\\.\pipe\audiorouter-control-session-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_sessions(
                &server_name,
                1,
                2,
                ControlPlane::new("persistent-control-test"),
                ClientGrant::for_role(ClientRole::Operator),
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let clear = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 301,
            "method": "recovery.clearSafeMode",
            "params": { "idempotencyKey": "transport-clear-1" }
        }))
        .unwrap();
        let subscribe = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 302,
            "method": "events.subscribe",
            "params": { "afterSequence": 0, "categories": ["recovery.safeModeCleared"] }
        }))
        .unwrap();
        let requests = [clear.as_slice(), subscribe.as_slice()];
        let responses = round_trip_session_many(&name, &requests).unwrap();
        let first = serde_json::from_slice::<serde_json::Value>(&responses[0][4..]).unwrap();
        let second = serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap();
        assert_eq!(first["id"], 301);
        assert_eq!(second["id"], 302);
        assert_eq!(second["result"]["events"].as_array().unwrap().len(), 1);
        assert_eq!(
            second["result"]["events"][0]["category"],
            "recovery.safeModeCleared"
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_backend_is_singleton_per_user_name() {
        let name = format!(
            r"\\.\pipe\audiorouter-singleton-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, 2, |_, frame| echo_handler(frame))
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let competing = serve_connections(&name, 1, |_, frame| echo_handler(frame));
        assert!(matches!(
            competing,
            Err(TransportError::Windows(message)) if message.contains("already owns")
        ));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "status.get"
        }))
        .unwrap();
        round_trip(&name, &request).unwrap();
        round_trip(&name, &request).unwrap();
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_handles_bounded_concurrent_clients() {
        let name = format!(
            r"\\.\pipe\audiorouter-concurrent-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, 8, |_, frame| echo_handler(frame))
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let clients = (0..8)
            .map(|index| {
                let name = name.clone();
                std::thread::spawn(move || {
                    let request = encode_frame(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": index,
                        "method": "status.get"
                    }))
                    .unwrap();
                    let response = round_trip(&name, &request).unwrap();
                    serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let responses = clients
            .into_iter()
            .map(|client| client.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 8);
        assert!(responses.iter().all(|response| response["ok"] == true));
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_handles_high_volume_concurrent_clients_without_corrupting_frames() {
        const CLIENTS: usize = 32;
        let name = format!(
            r"\\.\pipe\audiorouter-high-volume-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, CLIENTS, |_, frame| echo_handler(frame))
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let clients = (0..CLIENTS)
            .map(|index| {
                let name = name.clone();
                std::thread::spawn(move || {
                    let request = encode_frame(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": index,
                        "method": "system.describe"
                    }))
                    .unwrap();
                    let response = round_trip(&name, &request).unwrap();
                    serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let responses = clients
            .into_iter()
            .map(|client| client.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), CLIENTS);
        for response in &responses {
            assert_eq!(response["ok"], true);
        }
        server.join().unwrap().unwrap();
    }
}
