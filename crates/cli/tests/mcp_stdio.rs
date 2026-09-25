use audiorouter_control::{ClientRole, ControlPlane};
use audiorouter_protocol::{decode_frame, encode_frame, JsonRpcRequest, JsonRpcResponse};
use audiorouter_storage::Storage;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn cli_path() -> String {
    std::env::var("CARGO_BIN_EXE_audiorouter").unwrap_or_else(|_| {
        let test_exe = std::env::current_exe().expect("integration test path");
        test_exe
            .parent()
            .and_then(|deps| deps.parent())
            .expect("Cargo target directory")
            .join(if cfg!(windows) {
                "audiorouter-cli.exe"
            } else {
                "audiorouter-cli"
            })
            .to_string_lossy()
            .into_owned()
    })
}

fn send(input: &mut impl Write, output: &mut impl BufRead, message: Value) -> Value {
    writeln!(input, "{}", serde_json::to_string(&message).unwrap()).unwrap();
    input.flush().unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    assert!(!line.is_empty(), "MCP server closed stdout");
    serde_json::from_str(&line).unwrap()
}

#[test]
fn mcp_stdio_client_interoperates_with_cli_process() {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let local_app_data = std::env::temp_dir().join(format!(
        "audiorouter-mcp-stdio-local-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&local_app_data).unwrap();
    let database = std::env::temp_dir().join(format!(
        "audiorouter-mcp-stdio-{}-{stamp}.sqlite",
        std::process::id(),
    ));
    let _ = std::fs::remove_file(&database);
    let storage = Storage::open(&database).unwrap();
    let mut plane = ControlPlane::with_storage("mcp-stdio-test", storage);
    plane
        .enroll_client("mcp-stdio-client", ClientRole::Observer)
        .unwrap();
    drop(plane);

    let mut child = Command::new(cli_path())
        .args([
            "mcp",
            "serve",
            "--client-id",
            "mcp-stdio-client",
            "--database",
            database.to_str().unwrap(),
        ])
        .env("LOCALAPPDATA", &local_app_data)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn CLI MCP server");
    let mut input = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut output = BufReader::new(stdout);

    let initialized = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "interop-test", "version": "1" }
            }
        }),
    );
    assert_eq!(initialized["result"]["protocolVersion"], "2025-06-18");

    writeln!(
        input,
        "{}",
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .unwrap()
    )
    .unwrap();
    input.flush().unwrap();

    let tools = send(
        &mut input,
        &mut output,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    );
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 58);

    let processors = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "list_processors", "arguments": {} }
        }),
    );
    assert_eq!(processors["result"]["isError"], false);
    assert_eq!(
        processors["result"]["structuredContent"]["result"][0]["id"],
        "graphicEq"
    );

    let invalid_devices = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "tools/call",
            "params": { "name": "list_devices", "arguments": { "limit": 0 } }
        }),
    );
    assert_eq!(invalid_devices["result"]["isError"], true);
    assert_eq!(
        invalid_devices["result"]["structuredContent"]["error"]["message"],
        "limit must be between 1 and 500"
    );
    let error_text = invalid_devices["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let error_payload: Value = serde_json::from_str(error_text).unwrap();
    assert_eq!(
        error_payload["error"]["message"],
        "limit must be between 1 and 500"
    );

    let resources = send(
        &mut input,
        &mut output,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
    );
    assert_eq!(
        resources["result"]["resources"].as_array().unwrap().len(),
        5
    );

    let nodes = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "resources/read",
            "params": { "uri": "audiorouter://nodes" }
        }),
    );
    assert_eq!(
        nodes["result"]["contents"][0]["mimeType"],
        "application/json"
    );

    let sessions = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 32,
            "method": "resources/read",
            "params": { "uri": "audiorouter://sessions" }
        }),
    );
    assert_eq!(
        sessions["result"]["contents"][0]["mimeType"],
        "application/json"
    );

    let diagnostics = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "resources/read",
            "params": { "uri": "audiorouter://diagnostics" }
        }),
    );
    assert_eq!(
        diagnostics["result"]["contents"][0]["mimeType"],
        "application/json"
    );

    let startup = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": { "name": "get_startup", "arguments": {} }
        }),
    );
    assert_eq!(startup["result"]["isError"], false);

    let denied = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "plan_graph_change",
                "arguments": {
                    "sessionId": "missing",
                    "baseRevision": 0,
                    "candidate": {}
                }
            }
        }),
    );
    assert_eq!(denied["result"]["isError"], true);

    let redaction = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "tools/call",
            "params": { "name": "call_api", "arguments": { "method": "nodes.describe", "params": { "kind": "gain", "secret": "private-audio-fixture" } } }
        }),
    );
    assert_eq!(redaction["result"]["isError"], true);

    drop(input);
    assert!(child.wait().unwrap().success());
    let activity_path = local_app_data
        .join("AudioRouter")
        .join("logs")
        .join("mcp-activity.jsonl");
    let activity_text = std::fs::read_to_string(&activity_path).unwrap();
    let activity = activity_text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let denied_activity = activity
        .iter()
        .find(|entry| entry["tool"] == "list_devices" && entry["outcome"] == "error")
        .expect("the actual MCP tools/call is recorded");
    assert_eq!(denied_activity["errorKind"], "toolError");
    assert_eq!(denied_activity["argumentFields"], json!(["limit"]));
    assert!(!activity_text.contains("\"limit\":0"));
    let redacted_activity = activity
        .iter()
        .find(|entry| entry["tool"] == "call_api")
        .expect("the actual call_api request is recorded");
    assert_eq!(
        redacted_activity["argumentFields"],
        json!(["method", "params"])
    );
    assert!(!activity_text.contains("private-audio-fixture"));
    std::fs::remove_file(database).unwrap();
    let _ = std::fs::remove_file(std::env::temp_dir().join(format!(
        "audiorouter-mcp-stdio-{}-{stamp}.sqlite-wal",
        std::process::id(),
    )));
    let _ = std::fs::remove_file(std::env::temp_dir().join(format!(
        "audiorouter-mcp-stdio-{}-{stamp}.sqlite-shm",
        std::process::id(),
    )));
    std::fs::remove_file(activity_path).unwrap();
    std::fs::remove_dir(local_app_data.join("AudioRouter").join("logs")).unwrap();
    std::fs::remove_dir(local_app_data.join("AudioRouter")).unwrap();
    std::fs::remove_dir(local_app_data).unwrap();
}

#[cfg(windows)]
#[test]
fn mcp_pipe_proxy_interoperates_with_authenticated_backend() {
    let database = std::env::temp_dir().join(format!(
        "audiorouter-mcp-pipe-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&database);
    let storage = Storage::open(&database).unwrap();
    let mut plane = ControlPlane::with_storage("mcp-pipe-test", storage);
    plane
        .enroll_client("mcp-pipe-client", ClientRole::Observer)
        .unwrap();
    drop(plane);

    let pipe_name = format!(r"\\.\pipe\audiorouter-mcp-pipe-{}", std::process::id());
    let backend = std::thread::spawn({
        let pipe_name = pipe_name.clone();
        move || {
            audiorouter_transport::serve_control_connections_as_role(
                &pipe_name,
                1,
                ControlPlane::new("mcp-pipe-backend"),
                ClientRole::Observer,
            )
            .unwrap();
        }
    });

    let mut child = Command::new(cli_path())
        .args([
            "mcp",
            "serve",
            "--client-id",
            "mcp-pipe-client",
            "--database",
            database.to_str().unwrap(),
            "--pipe",
            &pipe_name,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn CLI MCP pipe proxy");
    let mut input = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut output = BufReader::new(stdout);

    let initialized = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "pipe-interop-test", "version": "1" }
            }
        }),
    );
    assert_eq!(initialized["result"]["protocolVersion"], "2025-06-18");

    let startup = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "get_startup", "arguments": {} }
        }),
    );
    assert_eq!(startup["result"]["isError"], false);
    assert_eq!(
        startup["result"]["structuredContent"]["result"]["enabled"],
        false
    );

    drop(input);
    assert!(child.wait().unwrap().success());
    backend.join().unwrap();
    std::fs::remove_file(database).unwrap();
}

#[cfg(windows)]
#[test]
fn bounded_backend_command_interoperates_with_authenticated_pipe_client() {
    let database =
        std::env::temp_dir().join(format!("audiorouter-backend-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&database);
    let storage = Storage::open(&database).unwrap();
    let mut plane = ControlPlane::with_storage("backend-test", storage);
    let sid = audiorouter_transport::current_user_sid().unwrap();
    plane.enroll_client(sid, ClientRole::Observer).unwrap();
    drop(plane);

    let pipe_name = format!(r"\\.\pipe\audiorouter-backend-{}", std::process::id());
    let mut child = Command::new(cli_path())
        .args([
            "backend",
            "serve",
            "--database",
            database.to_str().unwrap(),
            "--pipe",
            &pipe_name,
            "--connections",
            "1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn bounded backend");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "system.describe".into(),
        params: None,
    };
    let frame = encode_frame(&request).unwrap();
    let response = audiorouter_transport::round_trip(&pipe_name, &frame).unwrap();
    let response: JsonRpcResponse = decode_frame(&response).unwrap();
    assert!(response.error.is_none());
    assert_eq!(response.result.unwrap()["protocolVersion"]["major"], 1);
    assert!(child.wait().unwrap().success());
    std::fs::remove_file(database).unwrap();
}
