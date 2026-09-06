use audiorouter_control::{ClientRole, ControlPlane};
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
    let database = std::env::temp_dir().join(format!(
        "audiorouter-mcp-stdio-{}.sqlite",
        std::process::id()
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
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 24);

    let resources = send(
        &mut input,
        &mut output,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
    );
    assert_eq!(
        resources["result"]["resources"].as_array().unwrap().len(),
        3
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

    drop(input);
    assert!(child.wait().unwrap().success());
    std::fs::remove_file(database).unwrap();
}
