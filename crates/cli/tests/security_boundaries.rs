#![cfg(windows)]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Most bytes of a child's stderr that a failure message carries.
const STDERR_TAIL_BYTES: u64 = 4096;

/// Starts `audiorouter-cli` with its stderr written to `log` (a file, so an
/// unread pipe can never stall the child) and returns the process.
fn spawn_logged(command: &mut Command, log: &std::path::Path) -> Process {
    let file = std::fs::File::create(log).unwrap();
    Process(command.stderr(Stdio::from(file)).spawn().unwrap())
}

/// The last `STDERR_TAIL_BYTES` of a child's stderr log, for a failure message.
fn stderr_tail(log: &std::path::Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = std::fs::File::open(log) else {
        return format!("<no stderr log at {}>", log.display());
    };
    let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let start = length.saturating_sub(STDERR_TAIL_BYTES);
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_to_end(&mut bytes).is_err() {
        return format!("<unreadable stderr log at {}>", log.display());
    }
    let text = String::from_utf8_lossy(&bytes);
    let prefix = if start > 0 { "...(truncated) " } else { "" };
    if text.trim().is_empty() {
        "<stderr empty>".to_owned()
    } else {
        format!("{prefix}{}", text.trim_end())
    }
}

fn enroll(database: &str, id: &str, role: &str, key: &str) {
    let params = json!({ "clientId": id, "role": role, "idempotencyKey": key }).to_string();
    let root = std::path::Path::new(database).parent().unwrap();
    let file = root.join(format!("enroll-{key}.json"));
    std::fs::write(&file, params).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_audiorouter-cli"))
        .args([
            "api",
            "call",
            "clients.authorize",
            file.to_str().unwrap(),
            "--database",
            database,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

/// The stderr logs of the MCP sidecar and the backend it talks to.
struct Logs<'a> {
    mcp: &'a std::path::Path,
    backend: &'a std::path::Path,
}

/// Sends one MCP request; a missing answer reports both stderr tails.
fn call(input: &mut impl Write, output: &mut impl BufRead, message: Value, logs: &Logs) -> Value {
    let wrote = writeln!(input, "{message}").and_then(|()| input.flush());
    let mut response = String::new();
    let read = output.read_line(&mut response);
    serde_json::from_str(&response).unwrap_or_else(|_| {
        panic!(
            "missing MCP response: {response:?} (write {wrote:?}, read {read:?}); MCP stderr: {}; backend stderr: {}",
            stderr_tail(logs.mcp),
            stderr_tail(logs.backend)
        )
    })
}

/// One framed request over the real pipe; a failure reports the backend's stderr tail.
fn pipe_round_trip(pipe: &str, frame: &[u8], backend_log: &std::path::Path) -> Value {
    let bytes = audiorouter_transport::round_trip(pipe, frame).unwrap_or_else(|error| {
        panic!(
            "pipe request failed: {error:?}; backend stderr: {}",
            stderr_tail(backend_log)
        )
    });
    audiorouter_protocol::decode_frame(&bytes).unwrap()
}

#[test]
fn a_failing_child_reports_a_bounded_stderr_tail() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-stderr-tail-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    // A missing database folder makes `backend serve` exit with code 2 and an error line.
    let missing = root.join("no-such-folder").join("review.db");
    let log = root.join("backend.stderr");
    let mut child = spawn_logged(
        Command::new(env!("CARGO_BIN_EXE_audiorouter-cli")).args([
            "backend",
            "serve",
            "--database",
            missing.to_str().unwrap(),
            "--pipe",
            &format!(r"\\.\pipe\audiorouter-stderr-tail-{}", std::process::id()),
            "--connections",
            "1",
        ]),
        &log,
    );
    let status = child.0.wait().unwrap();
    assert_eq!(status.code(), Some(2), "{}", stderr_tail(&log));
    assert!(
        stderr_tail(&log).contains("error:"),
        "the failure message must carry the child's stderr: {}",
        stderr_tail(&log)
    );
    std::fs::write(&log, "x".repeat(STDERR_TAIL_BYTES as usize * 3) + "END").unwrap();
    let tail = stderr_tail(&log);
    assert!(tail.starts_with("...(truncated) ") && tail.ends_with("END"));
    assert!(
        tail.len() <= STDERR_TAIL_BYTES as usize + 32,
        "{}",
        tail.len()
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn real_mcp_pipe_preserves_roles_and_live_revocation() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "audiorouter-security-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    let database = root.join("review.db");
    let database = database.to_str().unwrap();
    let pipe = format!(
        r"\\.\pipe\audiorouter-security-{}-{nonce}",
        std::process::id()
    );
    let sid = audiorouter_transport::current_user_sid().unwrap();
    let observer = format!("observer-{nonce}");
    let editor = format!("editor-{nonce}");
    let operator = format!("operator-{nonce}");
    enroll(database, &sid, "operator", "owner");
    enroll(database, &observer, "observer", "observer");
    enroll(database, &editor, "editor", "editor");
    enroll(database, &operator, "operator", "operator");
    // stderr goes to files so a child that exits early (it once exited with
    // code 2 before answering) leaves its error line in the failure message.
    let backend_log = root.join("backend.stderr");
    let mut backend = spawn_logged(
        Command::new(env!("CARGO_BIN_EXE_audiorouter-cli"))
            .args([
                "backend",
                "serve",
                "--database",
                database,
                "--pipe",
                &pipe,
                "--connections",
                "40",
            ])
            .env("LOCALAPPDATA", &root)
            .stdout(Stdio::null()),
        &backend_log,
    );
    // Real pipe discovery, without activating devices or relying on a fixed sleep.
    let request = audiorouter_protocol::encode_frame(
        &json!({"jsonrpc":"2.0","id":1,"method":"system.describe"}),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while audiorouter_transport::round_trip(&pipe, &request).is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "backend failed to start: {:?}; backend stderr: {}",
            backend.0.try_wait(),
            stderr_tail(&backend_log)
        );
        std::thread::yield_now();
    }
    let mutation = audiorouter_protocol::encode_frame(&json!({"jsonrpc":"2.0","id":9,"method":"system.quit","params":{"idempotencyKey":"raw-must-not-quit"}})).unwrap();
    let raw = pipe_round_trip(&pipe, &mutation, &backend_log);
    assert_eq!(
        raw["error"]["data"]["code"], "permissionDenied",
        "raw same-user callers cannot inherit the server grant"
    );
    let forged = audiorouter_protocol::encode_frame(&json!({"pipeClient":{"id":operator,"token":"0".repeat(64)},"payload":{"jsonrpc":"2.0","id":10,"method":"system.quit","params":{"idempotencyKey":"forged"}}})).unwrap();
    let refused = pipe_round_trip(&pipe, &forged, &backend_log);
    assert_eq!(refused["error"]["data"]["code"], "permissionDenied");
    {
        // The original exploit must fail with the default tool set too.
        let mcp_log = root.join("mcp-default-observer.stderr");
        let logs = Logs {
            mcp: &mcp_log,
            backend: &backend_log,
        };
        let mut mcp = spawn_logged(
            Command::new(env!("CARGO_BIN_EXE_audiorouter-cli"))
                .args([
                    "mcp",
                    "serve",
                    "--database",
                    database,
                    "--client-id",
                    &observer,
                    "--pipe",
                    &pipe,
                ])
                .env("LOCALAPPDATA", &root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped()),
            &mcp_log,
        );
        let mut input = mcp.0.stdin.take().unwrap();
        let mut output = BufReader::new(mcp.0.stdout.take().unwrap());
        call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
            &logs,
        );
        let denied = call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_session","arguments":{"name":"Default Observer"}}}),
            &logs,
        );
        assert_eq!(
            denied["result"]["structuredContent"]["error"]["data"]["code"], "permissionDenied",
            "{denied}"
        );
    }
    // create_session also selects the created session, so its complete
    // workflow requires Operator (GraphWrite plus SessionControl).
    for (identity, allowed) in [(&observer, false), (&editor, false), (&operator, true)] {
        let mcp_log = root.join(format!("mcp-{identity}.stderr"));
        let logs = Logs {
            mcp: &mcp_log,
            backend: &backend_log,
        };
        let mut mcp = spawn_logged(
            Command::new(env!("CARGO_BIN_EXE_audiorouter-cli"))
                .args([
                    "mcp",
                    "serve",
                    "--database",
                    database,
                    "--client-id",
                    identity,
                    "--pipe",
                    &pipe,
                    "--advanced-tools",
                ])
                .env("LOCALAPPDATA", &root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped()),
            &mcp_log,
        );
        let mut input = mcp.0.stdin.take().unwrap();
        let mut output = BufReader::new(mcp.0.stdout.take().unwrap());
        call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
            &logs,
        );
        let response = call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_session","arguments":{"name":"Security regression"}}}),
            &logs,
        );
        assert_eq!(response["result"]["isError"], !allowed, "{response}");
        for (method, params) in [
            (
                "devices.setAccess",
                json!({"allowed":true,"idempotencyKey":"deny-devices"}),
            ),
            (
                "recordings.setRoot",
                json!({"root":root,"idempotencyKey":"deny-root"}),
            ),
            ("startup.plan", json!({"enabled":true})),
            ("obs.connect", json!({})),
            (
                "sessions.approveExport",
                json!({"path":root.join("bad.audiorouter")}),
            ),
        ] {
            let denied = call(
                &mut input,
                &mut output,
                json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"call_api","arguments":{"method":method,"params":params}}}),
                &logs,
            );
            assert_eq!(
                denied["result"]["structuredContent"]["error"]["data"]["code"], "permissionDenied",
                "{method}: {denied}"
            );
        }
        if allowed {
            let file = root.join("revoke.json");
            std::fs::write(
                &file,
                json!({"clientId":identity,"idempotencyKey":"revoke-editor"}).to_string(),
            )
            .unwrap();
            let result = Command::new(env!("CARGO_BIN_EXE_audiorouter-cli"))
                .args([
                    "api",
                    "call",
                    "clients.revoke",
                    file.to_str().unwrap(),
                    "--database",
                    database,
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let denied = call(
                &mut input,
                &mut output,
                json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"create_session","arguments":{"name":"Revoked"}}}),
                &logs,
            );
            assert_eq!(
                denied["result"]["isError"], true,
                "live sidecar must lose authority: {denied}"
            );
        }
    }
}
