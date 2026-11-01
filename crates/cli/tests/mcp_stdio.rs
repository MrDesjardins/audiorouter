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
    assert_stdio_client("mcp-stdio-client");
}

#[test]
fn local_cowork_identity_interoperates_with_cli_and_observer_writes_are_denied() {
    assert_stdio_client("audiorouter-cowork-local");
}

fn assert_stdio_client(client_id: &str) {
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
        .enroll_client(client_id, ClientRole::Observer)
        .unwrap();
    drop(plane);

    let mut child = Command::new(cli_path())
        .args([
            "mcp",
            "serve",
            "--client-id",
            client_id,
            "--database",
            database.to_str().unwrap(),
            // This test drives internal tools (call_api, plan_graph_change).
            "--advanced-tools",
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

    // Modern MCP is stateless: discovery, catalogs and calls work before any
    // legacy initialize handshake, and retain the same enrolled-client grant.
    let modern_meta = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { "name": "interop-test", "version": "1" }
    });
    let discovered = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "discover", "method": "server/discover",
            "params": { "_meta": modern_meta }
        }),
    );
    assert_eq!(discovered["result"]["supportedVersions"][0], "2026-07-28");
    assert_eq!(discovered["result"]["resultType"], "complete");
    assert_eq!(
        discovered["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "audiorouter"
    );

    let modern_tools = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "modern-tools", "method": "tools/list",
            "params": { "_meta": modern_meta }
        }),
    );
    assert_eq!(
        modern_tools["result"]["tools"].as_array().unwrap().len(),
        89
    );
    assert_eq!(modern_tools["result"]["resultType"], "complete");
    assert_eq!(modern_tools["result"]["cacheScope"], "public");

    let modern_call = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "modern-call", "method": "tools/call",
            "params": {
                "name": "list_processors", "arguments": {}, "_meta": modern_meta
            }
        }),
    );
    assert_eq!(modern_call["result"]["isError"], false);
    assert_eq!(modern_call["result"]["resultType"], "complete");

    // Task tools added for API parity (spec AUTO-18), through the real
    // binary: reads work for this Observer, a Soundboard press is refused.
    for (index, tool) in ["get_status", "list_sounds"].into_iter().enumerate() {
        let read = send(
            &mut input,
            &mut output,
            json!({
                "jsonrpc": "2.0", "id": format!("parity-read-{index}"), "method": "tools/call",
                "params": { "name": tool, "arguments": {}, "_meta": modern_meta }
            }),
        );
        assert_eq!(read["result"]["isError"], false, "{tool}: {read}");
    }
    let denied_press = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "parity-denied-press", "method": "tools/call",
            "params": {
                "name": "trigger_sound", "arguments": { "node": "Board", "slot": 1 },
                "_meta": modern_meta
            }
        }),
    );
    assert_eq!(denied_press["result"]["isError"], true, "{denied_press}");
    assert!(
        denied_press
            .to_string()
            .to_lowercase()
            .contains("permission"),
        "an Observer may not press a Soundboard: {denied_press}"
    );
    assert_eq!(
        modern_call["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "audiorouter"
    );

    let denied_modern_write = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "modern-denied-write", "method": "tools/call",
            "params": {
                "name": "create_session", "arguments": { "name": "must stay denied" },
                "_meta": modern_meta
            }
        }),
    );
    assert_eq!(denied_modern_write["result"]["isError"], true);
    assert_eq!(
        denied_modern_write["result"]["structuredContent"]["error"]["code"],
        -32001
    );

    let modern_resources = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "modern-resources", "method": "resources/list",
            "params": { "_meta": modern_meta }
        }),
    );
    assert_eq!(
        modern_resources["result"]["resources"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(modern_resources["result"]["cacheScope"], "public");

    let modern_resource = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "modern-resource", "method": "resources/read",
            "params": { "uri": "audiorouter://workflow/headless", "_meta": modern_meta }
        }),
    );
    assert_eq!(
        modern_resource["result"]["contents"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(modern_resource["result"]["cacheScope"], "private");

    let missing_capabilities = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "missing-capabilities", "method": "tools/list",
            "params": { "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28"
            } }
        }),
    );
    assert_eq!(missing_capabilities["error"]["code"], -32602);

    let unsupported = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "unsupported", "method": "tools/list",
            "params": { "_meta": {
                "io.modelcontextprotocol/protocolVersion": "1900-01-01",
                "io.modelcontextprotocol/clientCapabilities": {}
            } }
        }),
    );
    assert_eq!(unsupported["error"]["code"], -32022);
    assert_eq!(unsupported["error"]["data"]["supported"][0], "2026-07-28");

    let modern_ping = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0", "id": "modern-ping", "method": "ping",
            "params": { "_meta": modern_meta }
        }),
    );
    assert_eq!(modern_ping["error"]["code"], -32601);

    let initialized = send(
        &mut input,
        &mut output,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "_meta": { "progressToken": "legacy-setup" },
                "capabilities": {},
                "clientInfo": { "name": "interop-test", "version": "1" }
            }
        }),
    );
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");

    let legacy_ping = send(
        &mut input,
        &mut output,
        json!({ "jsonrpc": "2.0", "id": "legacy-ping", "method": "ping" }),
    );
    assert_eq!(legacy_ping["result"], json!({}));

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
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": { "_meta": { "example.com/trace": "legacy-list" } } }),
    );
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 89);

    // A malformed line gets a JSON-RPC parse error and the session continues.
    writeln!(input, "{{ not json").unwrap();
    input.flush().unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let parse_error: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(parse_error["error"]["code"], -32700);

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
    // Every tool call gets its own correlation ID (P2-3).
    let ids = activity
        .iter()
        .map(|entry| entry["requestId"].as_str().expect("requestId").to_owned())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), activity.len());
    assert!(ids
        .iter()
        .all(|id| audiorouter_protocol::diagnostics::is_valid_correlation_id(id)));
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
    assert_pipe_proxy("mcp-pipe-client");
}

#[cfg(windows)]
#[test]
fn local_cowork_stdio_reaches_the_authenticated_pipe_backend() {
    assert_pipe_proxy("audiorouter-cowork-local");
}

#[cfg(windows)]
fn assert_pipe_proxy(client_id: &str) {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let database = std::env::temp_dir().join(format!(
        "audiorouter-mcp-pipe-{}-{stamp}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&database);
    let storage = Storage::open(&database).unwrap();
    let mut plane = ControlPlane::with_storage("mcp-pipe-test", storage);
    plane
        .enroll_client(client_id, ClientRole::Observer)
        .unwrap();
    drop(plane);

    let pipe_name = format!(
        r"\\.\pipe\audiorouter-mcp-pipe-{}-{stamp}",
        std::process::id()
    );
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
            client_id,
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
    // `round_trip` retries for only ~100 ms, which suits a backend that is
    // already running. This one was just spawned: on a busy CI runner its
    // process start, database open and pipe creation can take longer, so
    // keep retrying until it listens, failing early if it exits.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let response = loop {
        match audiorouter_transport::round_trip(&pipe_name, &frame) {
            Ok(response) => break response,
            Err(error) => {
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "backend exited before serving: {error:?}"
                );
                assert!(
                    std::time::Instant::now() < deadline,
                    "backend never listened: {error:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    };
    let response: JsonRpcResponse = decode_frame(&response).unwrap();
    assert!(response.error.is_none());
    assert_eq!(response.result.unwrap()["protocolVersion"]["major"], 1);
    assert!(child.wait().unwrap().success());
    std::fs::remove_file(database).unwrap();
}
