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

fn call(input: &mut impl Write, output: &mut impl BufRead, message: Value) -> Value {
    writeln!(input, "{message}").unwrap();
    input.flush().unwrap();
    let mut response = String::new();
    output.read_line(&mut response).unwrap();
    serde_json::from_str(&response).unwrap_or_else(|_| panic!("missing MCP response: {response}"))
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
    let mut backend = Process(
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
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
            "backend failed to start: {:?}",
            backend.0.try_wait()
        );
        std::thread::yield_now();
    }
    let mutation = audiorouter_protocol::encode_frame(&json!({"jsonrpc":"2.0","id":9,"method":"system.quit","params":{"idempotencyKey":"raw-must-not-quit"}})).unwrap();
    let raw: Value = audiorouter_protocol::decode_frame(
        &audiorouter_transport::round_trip(&pipe, &mutation).unwrap(),
    )
    .unwrap();
    assert_eq!(
        raw["error"]["data"]["code"], "permissionDenied",
        "raw same-user callers cannot inherit the server grant"
    );
    let forged = audiorouter_protocol::encode_frame(&json!({"pipeClient":{"id":operator,"token":"0".repeat(64)},"payload":{"jsonrpc":"2.0","id":10,"method":"system.quit","params":{"idempotencyKey":"forged"}}})).unwrap();
    let refused: Value = audiorouter_protocol::decode_frame(
        &audiorouter_transport::round_trip(&pipe, &forged).unwrap(),
    )
    .unwrap();
    assert_eq!(refused["error"]["data"]["code"], "permissionDenied");
    {
        // The original exploit must fail with the default tool set too.
        let mut mcp = Process(
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
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut input = mcp.0.stdin.take().unwrap();
        let mut output = BufReader::new(mcp.0.stdout.take().unwrap());
        call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
        );
        let denied = call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_session","arguments":{"name":"Default Observer"}}}),
        );
        assert_eq!(
            denied["result"]["structuredContent"]["error"]["data"]["code"], "permissionDenied",
            "{denied}"
        );
    }
    // create_session also selects the created session, so its complete
    // workflow requires Operator (GraphWrite plus SessionControl).
    for (identity, allowed) in [(&observer, false), (&editor, false), (&operator, true)] {
        let mut mcp = Process(
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
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let mut input = mcp.0.stdin.take().unwrap();
        let mut output = BufReader::new(mcp.0.stdout.take().unwrap());
        call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}),
        );
        let response = call(
            &mut input,
            &mut output,
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_session","arguments":{"name":"Security regression"}}}),
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
            assert!(result.status.success());
            let denied = call(
                &mut input,
                &mut output,
                json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"create_session","arguments":{"name":"Revoked"}}}),
            );
            assert_eq!(
                denied["result"]["isError"], true,
                "live sidecar must lose authority: {denied}"
            );
        }
    }
}
