//! Offline M01 CLI command surface.

use audiorouter_control::ControlPlane;
use audiorouter_domain::{inspect_routes, EntityId};
use audiorouter_storage::Storage;
use serde_json::{json, Value};
use std::io::{BufRead, Read, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputMode {
    Human,
    Json,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CliError {
    InvalidArguments(String),
    UnknownCommand(String),
    Io(String),
    Storage(String),
}

pub fn run<I, S>(args: I) -> Result<String, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let mode = if args.iter().any(|arg| arg == "--json") {
        OutputMode::Json
    } else {
        OutputMode::Human
    };
    let command_args: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| *arg != "--json")
        .collect();
    let command = command_args.first().copied().unwrap_or("help");
    let mut plane = ControlPlane::default();
    let value = match command {
        "help" => help_value(),
        "status" => plane
            .dispatch(status_request())
            .result
            .unwrap_or_else(|| json!({ "error": "status unavailable" })),
        "schema" => plane.describe(),
        "diagnostics" => diagnostics_command(&command_args)?,
        "devices" => list_subcommand(&command_args, "devices")?,
        "apps" => list_subcommand(&command_args, "apps")?,
        "nodes" => list_subcommand(&command_args, "nodes")?,
        "routes" => routes_subcommand(&command_args)?,
        "history" => history_command(&command_args)?,
        "graph" => graph_command(&command_args)?,
        "session" => session_command(&command_args)?,
        "api" => api_subcommand(&command_args)?,
        "operation" => operation_command(&command_args)?,
        "export" => export_session(&command_args)?,
        "import" => import_session(&command_args)?,
        "export-bundle" => export_bundle(&command_args)?,
        "import-bundle" => import_bundle(&command_args)?,
        other => return Err(CliError::UnknownCommand(other.into())),
    };
    match mode {
        OutputMode::Json => serde_json::to_string(&value)
            .map_err(|error| CliError::InvalidArguments(error.to_string())),
        OutputMode::Human => Ok(render_human(command, &value)),
    }
}

fn status_request() -> audiorouter_protocol::JsonRpcRequest {
    audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "status.get".into(),
        params: None,
    }
}

fn diagnostics_command(args: &[&str]) -> Result<Value, CliError> {
    let request = request("system.diagnostics");
    let response = if args.contains(&"--database") {
        ControlPlane::with_storage("cli", database(args)?).dispatch(request)
    } else {
        ControlPlane::default().dispatch(request)
    };
    response
        .result
        .ok_or_else(|| CliError::InvalidArguments("diagnostics unavailable".into()))
}

fn operation_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("get") {
        return Err(CliError::InvalidArguments(
            "usage: operation get <operation-id> --database <path>".into(),
        ));
    }
    let operation_id = positional(args, 2, "operation id")?;
    let response = ControlPlane::with_storage("cli", database(args)?).dispatch(
        audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": operation_id })),
        },
    );
    response
        .result
        .ok_or_else(|| CliError::InvalidArguments("operation not found".into()))
}

fn list_subcommand(args: &[&str], parent: &str) -> Result<Value, CliError> {
    let subcommand = args.get(1).copied();
    let valid = match parent {
        "api" => subcommand == Some("methods"),
        "nodes" => matches!(subcommand, Some("types" | "describe")),
        _ => subcommand == Some("list"),
    };
    if !valid {
        let expected = match parent {
            "api" => "methods",
            "nodes" => "types|describe",
            _ => "list",
        };
        return Err(CliError::InvalidArguments(format!(
            "usage: {parent} {expected}"
        )));
    }
    let expected = subcommand.unwrap();
    let mut plane = ControlPlane::default();
    Ok(match parent {
        "devices" => plane
            .dispatch(request("devices.list"))
            .result
            .unwrap_or_else(|| json!([])),
        "apps" => plane
            .dispatch(request("apps.list"))
            .result
            .unwrap_or_else(|| json!([])),
        "nodes" if matches!(expected, "types" | "describe") => {
            plane.describe()["nodeTypes"].clone()
        }
        "api" => plane.describe()["methods"].clone(),
        _ => unreachable!(),
    })
}

fn api_subcommand(args: &[&str]) -> Result<Value, CliError> {
    match args.get(1).copied() {
        Some("methods") => list_subcommand(args, "api"),
        Some("call") => {
            let method = args.get(2).copied().ok_or_else(|| {
                CliError::InvalidArguments(
                    "usage: api call <method> [<params-json-file|->] [--database <path>]".into(),
                )
            })?;
            let params = match args.get(3).copied() {
                None => None,
                Some("-") => {
                    let mut document = String::new();
                    std::io::stdin()
                        .read_to_string(&mut document)
                        .map_err(|error| CliError::Io(error.to_string()))?;
                    Some(parse_api_params(&document)?)
                }
                Some(path) => {
                    let path = std::path::Path::new(path);
                    if !path.is_absolute() {
                        return Err(CliError::InvalidArguments(
                            "params file path must be absolute".into(),
                        ));
                    }
                    let document = std::fs::read_to_string(path)
                        .map_err(|error| CliError::Io(error.to_string()))?;
                    Some(parse_api_params(&document)?)
                }
            };
            let request = audiorouter_protocol::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params,
            };
            let response = if args.contains(&"--database") {
                let storage = database(args)?;
                ControlPlane::with_storage("cli", storage).dispatch(request)
            } else {
                ControlPlane::default().dispatch(request)
            };
            serde_json::to_value(response)
                .map_err(|error| CliError::InvalidArguments(error.to_string()))
        }
        _ => Err(CliError::InvalidArguments(
            "usage: api methods|call <method> [<params-json-file|->] [--database <path>]".into(),
        )),
    }
}

fn parse_api_params(document: &str) -> Result<Value, CliError> {
    if document.len() > 4 * 1024 * 1024 {
        return Err(CliError::InvalidArguments(
            "API params exceed the 4 MiB limit".into(),
        ));
    }
    serde_json::from_str(document)
        .map_err(|error| CliError::InvalidArguments(format!("invalid API params JSON: {error}")))
}

fn routes_subcommand(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("inspect") {
        return Err(CliError::InvalidArguments(
            "usage: routes inspect <session-id> <destination-node> --database <path>".into(),
        ));
    }
    let id = args
        .get(2)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments(
                "usage: routes inspect <session-id> <destination-node> --database <path>".into(),
            )
        })?;
    let storage = database(args)?;
    let destination = args
        .get(3)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments(
                "usage: routes inspect <session-id> <destination-node> --database <path>".into(),
            )
        })?;
    let session = storage
        .load_session(&EntityId::new(id))
        .map_err(|error| CliError::Storage(format!("{error:?}")))?
        .ok_or_else(|| CliError::InvalidArguments("session not found".into()))?;
    let inspection = inspect_routes(&session, &EntityId::new(destination)).map_err(|errors| {
        CliError::InvalidArguments(format!("invalid destination or graph: {errors:?}"))
    })?;
    serde_json::to_value(inspection).map_err(|error| CliError::InvalidArguments(error.to_string()))
}

fn history_command(args: &[&str]) -> Result<Value, CliError> {
    let id = args
        .get(1)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments("usage: history <session-id> --database <path>".into())
        })?;
    let limit = args
        .iter()
        .position(|argument| *argument == "--limit")
        .map(|index| {
            args.get(index + 1)
                .copied()
                .ok_or_else(|| CliError::InvalidArguments("--limit requires a value".into()))?
                .parse::<usize>()
                .map_err(|_| CliError::InvalidArguments("--limit must be an integer".into()))
        })
        .transpose()?
        .unwrap_or(100);
    if !(1..=500).contains(&limit) {
        return Err(CliError::InvalidArguments(
            "--limit must be between 1 and 500".into(),
        ));
    }
    let storage = database(args)?;
    let history = storage
        .load_history(&EntityId::new(id), limit)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    serde_json::to_value(history).map_err(|error| CliError::InvalidArguments(error.to_string()))
}

fn graph_command(args: &[&str]) -> Result<Value, CliError> {
    match args.get(1).copied() {
        Some("plan") => graph_plan_command(args),
        Some("inspect") => graph_inspect_command(args),
        Some("apply") => graph_apply_command(args),
        _ => Err(CliError::InvalidArguments(
            "usage: graph <plan|inspect|apply> ...".into(),
        )),
    }
}

fn graph_plan_command(args: &[&str]) -> Result<Value, CliError> {
    let session_id = positional(args, 2, "session id")?;
    let candidate_path = absolute_option(args, "--file")?;
    let output_path = absolute_option(args, "--output")?;
    let base_revision = option_value(args, "--base-revision")?
        .parse::<u64>()
        .map_err(|_| CliError::InvalidArguments("--base-revision must be an integer".into()))?;
    let candidate = read_session(&candidate_path)?;
    if candidate.id != EntityId::new(session_id) {
        return Err(CliError::InvalidArguments(
            "candidate session id does not match the requested session".into(),
        ));
    }
    let storage = database(args)?;
    let mut plane = ControlPlane::with_storage("cli", storage);
    let preview = plane.dispatch(audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "graph.plan".into(),
        params: Some(json!({
            "sessionId": session_id,
            "baseRevision": base_revision,
            "candidate": candidate,
        })),
    });
    if let Some(error) = preview.error {
        return Err(CliError::InvalidArguments(error.message));
    }
    let preview = preview
        .result
        .ok_or_else(|| CliError::InvalidArguments("graph.plan returned no result".into()))?;
    let plan = json!({
        "format": "audiorouter.graph-plan",
        "schemaVersion": 1,
        "sessionId": session_id,
        "baseRevision": base_revision,
        "candidate": candidate,
        "preview": preview,
    });
    write_new_file(&output_path, &serde_json::to_vec_pretty(&plan).unwrap())?;
    Ok(json!({
        "planId": plan["preview"]["planId"],
        "sessionId": session_id,
        "baseRevision": base_revision,
        "output": output_path,
        "dryRun": true,
        "preview": plan["preview"],
    }))
}

fn graph_inspect_command(args: &[&str]) -> Result<Value, CliError> {
    let path = positional_path(args, 2, "plan file")?;
    let plan = read_json_object(&path)?;
    if plan["format"] != "audiorouter.graph-plan" || plan["schemaVersion"] != 1 {
        return Err(CliError::InvalidArguments(
            "unsupported graph plan format".into(),
        ));
    }
    Ok(plan)
}

fn graph_apply_command(args: &[&str]) -> Result<Value, CliError> {
    let path = positional_path(args, 2, "plan file")?;
    let key = option_value(args, "--idempotency-key")?;
    if key.len() > 256 || key.is_empty() {
        return Err(CliError::InvalidArguments(
            "--idempotency-key must contain 1..256 characters".into(),
        ));
    }
    let plan = read_json_object(&path)?;
    if plan["format"] != "audiorouter.graph-plan" || plan["schemaVersion"] != 1 {
        return Err(CliError::InvalidArguments(
            "unsupported graph plan format".into(),
        ));
    }
    let session_id = plan["sessionId"]
        .as_str()
        .ok_or_else(|| CliError::InvalidArguments("plan sessionId is required".into()))?;
    let base_revision = plan["baseRevision"]
        .as_u64()
        .ok_or_else(|| CliError::InvalidArguments("plan baseRevision is required".into()))?;
    let candidate: audiorouter_domain::Session = serde_json::from_value(plan["candidate"].clone())
        .map_err(|error| CliError::InvalidArguments(format!("invalid plan candidate: {error}")))?;
    if candidate.id != EntityId::new(session_id) {
        return Err(CliError::InvalidArguments(
            "plan candidate/session mismatch".into(),
        ));
    }
    let storage = database(args)?;
    let mut plane = ControlPlane::with_storage("cli", storage);
    let current = plane
        .dispatch(audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.get".into(),
            params: Some(json!({ "sessionId": session_id })),
        })
        .result
        .ok_or_else(|| CliError::InvalidArguments("session not found".into()))?;
    let current_revision = current["revision"]
        .as_u64()
        .ok_or_else(|| CliError::InvalidArguments("current session has no revision".into()))?;
    if current_revision != base_revision {
        return Err(CliError::InvalidArguments(format!(
            "stale graph plan: expected revision {base_revision}, current revision {}",
            current_revision
        )));
    }
    let planned = plane
        .plan_graph(&EntityId::new(session_id), base_revision, candidate)
        .map_err(|error| CliError::InvalidArguments(format!("graph plan rejected: {error:?}")))?;
    plane
        .commit_graph(&planned, base_revision, key)
        .map_err(|error| CliError::InvalidArguments(format!("graph commit rejected: {error:?}")))
}

fn positional<'a>(args: &'a [&str], index: usize, name: &str) -> Result<&'a str, CliError> {
    args.get(index)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| CliError::InvalidArguments(format!("{name} is required")))
}

fn positional_path(
    args: &[&str],
    index: usize,
    name: &str,
) -> Result<std::path::PathBuf, CliError> {
    let path = std::path::PathBuf::from(positional(args, index, name)?);
    if !path.is_absolute() {
        return Err(CliError::InvalidArguments(format!(
            "{name} path must be absolute"
        )));
    }
    Ok(path)
}

fn read_session(path: &std::path::Path) -> Result<audiorouter_domain::Session, CliError> {
    let document =
        std::fs::read_to_string(path).map_err(|error| CliError::Io(error.to_string()))?;
    serde_json::from_str(&document)
        .map_err(|error| CliError::InvalidArguments(format!("invalid session JSON: {error}")))
}

fn read_json_object(path: &std::path::Path) -> Result<Value, CliError> {
    let document =
        std::fs::read_to_string(path).map_err(|error| CliError::Io(error.to_string()))?;
    let value: Value = serde_json::from_str(&document)
        .map_err(|error| CliError::InvalidArguments(format!("invalid JSON: {error}")))?;
    if !value.is_object() {
        return Err(CliError::InvalidArguments(
            "plan must be a JSON object".into(),
        ));
    }
    Ok(value)
}

fn write_new_file(path: &std::path::Path, contents: &[u8]) -> Result<(), CliError> {
    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| CliError::Io(format!("cannot create {}: {error}", path.display())))?;
    std::io::Write::write_all(&mut file, contents).map_err(|error| CliError::Io(error.to_string()))
}

fn session_command(args: &[&str]) -> Result<Value, CliError> {
    let action = args.get(1).copied().ok_or_else(|| {
        CliError::InvalidArguments(
            "usage: session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path>".into(),
        )
    })?;
    if !matches!(
        action,
        "get" | "list" | "create" | "start" | "stop" | "delete" | "duplicate"
    ) {
        return Err(CliError::InvalidArguments(
            "usage: session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path>".into(),
        ));
    }
    let storage = database(args)?;
    if action == "list" {
        let limit = args
            .iter()
            .position(|argument| *argument == "--limit")
            .map(|index| {
                args.get(index + 1)
                    .copied()
                    .ok_or_else(|| CliError::InvalidArguments("--limit requires a value".into()))?
                    .parse::<usize>()
                    .map_err(|_| CliError::InvalidArguments("--limit must be an integer".into()))
            })
            .transpose()?
            .unwrap_or(100);
        if !(1..=500).contains(&limit) {
            return Err(CliError::InvalidArguments(
                "--limit must be between 1 and 500".into(),
            ));
        }
        return serde_json::to_value(
            storage
                .list_sessions(limit)
                .map_err(|error| CliError::Storage(format!("{error:?}")))?,
        )
        .map_err(|error| CliError::InvalidArguments(error.to_string()));
    }
    if action == "create" {
        let document_path = args
            .get(2)
            .copied()
            .filter(|value| !value.starts_with('-'))
            .ok_or_else(|| {
                CliError::InvalidArguments(
                    "usage: session create <document-path> --database <path>".into(),
                )
            })?;
        let document_path = std::path::Path::new(document_path);
        if !document_path.is_absolute() {
            return Err(CliError::InvalidArguments(
                "document path must be absolute".into(),
            ));
        }
        let document = std::fs::read_to_string(document_path)
            .map_err(|error| CliError::Io(error.to_string()))?;
        let session: audiorouter_domain::Session = serde_json::from_str(&document)
            .map_err(|error| CliError::InvalidArguments(error.to_string()))?;
        let mut plane = ControlPlane::with_storage("cli", storage);
        return plane
            .create_session(session)
            .map_err(|error| CliError::Storage(format!("{error:?}")));
    }
    let id = args
        .get(2)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments(
                "usage: session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path>"
                    .into(),
            )
        })?;
    let id = EntityId::new(id);
    if action == "duplicate" {
        let duplicate_id = args
            .get(3)
            .copied()
            .filter(|value| !value.starts_with('-'))
            .ok_or_else(|| {
                CliError::InvalidArguments(
                    "usage: session duplicate <source-session-id> <new-session-id> --database <path>"
                        .into(),
                )
            })?;
        let mut plane = ControlPlane::with_storage("cli", storage);
        return plane
            .duplicate_session(&id, EntityId::new(duplicate_id), None)
            .map_err(|error| CliError::Storage(format!("{error:?}")));
    }
    if action == "get" {
        let session = storage
            .load_session(&id)
            .map_err(|error| CliError::Storage(format!("{error:?}")))?
            .ok_or_else(|| CliError::InvalidArguments("session not found".into()))?;
        return serde_json::to_value(session)
            .map_err(|error| CliError::InvalidArguments(error.to_string()));
    }
    let mut plane = ControlPlane::with_storage("cli", storage);
    match action {
        "start" => plane.session_start(&id),
        "stop" => plane.session_stop(&id),
        "delete" => plane.delete_session(&id),
        _ => unreachable!(),
    }
    .map_err(|error| CliError::Storage(format!("{error:?}")))
}

fn request(method: &str) -> audiorouter_protocol::JsonRpcRequest {
    audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: None,
    }
}

fn help_value() -> Value {
    json!({ "commands": ["help", "status", "diagnostics [--database <path>]", "schema", "devices list", "apps list", "nodes types", "nodes describe", "routes inspect <session-id> <destination-node> --database <path>", "history <session-id> --database <path> [--limit N]", "graph plan <session-id> --base-revision <n> --file <candidate.json> --output <plan.json> --database <path>", "graph inspect <plan.json>", "graph apply <plan.json> --idempotency-key <key> --database <path>", "operation get <operation-id> --database <path>", "session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path>", "api methods", "api call <method> [<params-json-file|->] [--database <path>]", "mcp serve --client-id <enrolled-client> --database <path> [--pipe \\\\.\\pipe\\audiorouter]", "export <session-id> --database <path>", "import <document-path> --database <path>", "export-bundle <session-id> --database <path> --output <path>", "import-bundle <bundle-path> --database <path> --staging <directory>"], "globalOptions": ["--json"], "note": "Graph plans are versioned local files; apply revalidates the current revision before committing. The local MCP stdio adapter is pinned to protocol 2025-06-18 and requires an enrolled client." })
}

fn option_value<'a>(args: &'a [&str], option: &str) -> Result<&'a str, CliError> {
    let index = args
        .iter()
        .position(|argument| *argument == option)
        .ok_or_else(|| CliError::InvalidArguments(format!("{option} is required")))?;
    args.get(index + 1)
        .copied()
        .filter(|value| !value.is_empty() && !value.starts_with('-'))
        .ok_or_else(|| CliError::InvalidArguments(format!("{option} requires a value")))
}

fn database(args: &[&str]) -> Result<Storage, CliError> {
    let path = option_value(args, "--database")?;
    if !std::path::Path::new(path).is_absolute() {
        return Err(CliError::InvalidArguments(
            "--database path must be absolute".into(),
        ));
    }
    Storage::open(path).map_err(|error| CliError::Storage(format!("{error:?}")))
}

fn export_session(args: &[&str]) -> Result<Value, CliError> {
    let id = args
        .get(1)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments("usage: export <session-id> --database <path>".into())
        })?;
    let storage = database(args)?;
    let document = storage
        .export_session(&EntityId::new(id))
        .map_err(|error| CliError::Storage(format!("{error:?}")))?
        .ok_or_else(|| CliError::InvalidArguments("session not found".into()))?;
    serde_json::from_str(&document).map_err(|error| CliError::InvalidArguments(error.to_string()))
}

fn import_session(args: &[&str]) -> Result<Value, CliError> {
    let path = args
        .get(1)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments("usage: import <document-path> --database <path>".into())
        })?;
    let document =
        std::fs::read_to_string(path).map_err(|error| CliError::Io(error.to_string()))?;
    let storage = database(args)?;
    let session = storage
        .import_session(&document)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    serde_json::to_value(session).map_err(|error| CliError::InvalidArguments(error.to_string()))
}

fn absolute_option(args: &[&str], option: &str) -> Result<std::path::PathBuf, CliError> {
    let value = option_value(args, option)?;
    let path = std::path::PathBuf::from(value);
    if !path.is_absolute() {
        return Err(CliError::InvalidArguments(format!(
            "{option} path must be absolute"
        )));
    }
    Ok(path)
}

fn export_bundle(args: &[&str]) -> Result<Value, CliError> {
    let id = args
        .get(1)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments(
                "usage: export-bundle <session-id> --database <path> --output <path>".into(),
            )
        })?;
    let output = absolute_option(args, "--output")?;
    let storage = database(args)?;
    storage
        .export_bundle(&EntityId::new(id), &output)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    Ok(json!({ "sessionId": id, "output": output }))
}

fn import_bundle(args: &[&str]) -> Result<Value, CliError> {
    let input = args
        .get(1)
        .copied()
        .filter(|value| !value.starts_with('-'))
        .ok_or_else(|| {
            CliError::InvalidArguments(
                "usage: import-bundle <bundle-path> --database <path> --staging <directory>".into(),
            )
        })?;
    let input = std::path::Path::new(input);
    if !input.is_absolute() {
        return Err(CliError::InvalidArguments(
            "bundle path must be absolute".into(),
        ));
    }
    let staging = absolute_option(args, "--staging")?;
    let storage = database(args)?;
    let session = storage
        .import_bundle(input, &staging)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    serde_json::to_value(session).map_err(|error| CliError::InvalidArguments(error.to_string()))
}

fn render_human(command: &str, value: &Value) -> String {
    if command == "help" {
        return format!(
            "AudioRouter M01\n{}\n",
            value["commands"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

/// Run the local MCP stdio adapter using the pinned 2025-06-18 protocol.
/// stdout is reserved for newline-delimited JSON-RPC; diagnostics go to stderr.
pub fn run_mcp_stdio(args: &[String]) -> Result<(), CliError> {
    let database_path = option_value_owned(args, "--database")?;
    let client_id = option_value_owned(args, "--client-id")?;
    let pipe_name = args
        .iter()
        .position(|value| value == "--pipe")
        .and_then(|index| args.get(index + 1))
        .cloned();
    let database_path = std::path::PathBuf::from(database_path);
    if !database_path.is_absolute() {
        return Err(CliError::InvalidArguments(
            "--database path must be absolute".into(),
        ));
    }
    let storage =
        Storage::open(database_path).map_err(|error| CliError::Storage(format!("{error:?}")))?;
    let mut plane = ControlPlane::with_storage("mcp", storage);
    let grant = plane
        .grant_for_client(&client_id)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?
        .ok_or_else(|| {
            CliError::InvalidArguments("client is not enrolled or has been revoked".into())
        })?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    let mut initialized = false;
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| CliError::Io(error.to_string()))?;
        if line.len() > 4 * 1024 * 1024 {
            return Err(CliError::InvalidArguments(
                "MCP message exceeds the 4 MiB limit".into(),
            ));
        }
        let message: Value = serde_json::from_str(&line)
            .map_err(|error| CliError::InvalidArguments(format!("invalid MCP JSON: {error}")))?;
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = message.get("id").cloned();
        let response = match method {
            "initialize" => {
                let requested = message["params"]["protocolVersion"]
                    .as_str()
                    .unwrap_or_default();
                if requested != "2025-06-18" && requested != "2025-03-26" {
                    Some(mcp_error(id, -32602, "unsupported MCP protocol version"))
                } else {
                    initialized = true;
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "protocolVersion": "2025-06-18",
                            "capabilities": { "tools": { "listChanged": false }, "resources": { "subscribe": false, "listChanged": false } },
                            "serverInfo": { "name": "audiorouter", "version": env!("CARGO_PKG_VERSION") }
                        }
                    }))
                }
            }
            "notifications/initialized" => None,
            "tools/list" if initialized => Some(json!({
                "jsonrpc": "2.0", "id": id, "result": { "tools": mcp_tools() }
            })),
            "tools/call" if initialized => Some(mcp_tool_call(
                &mut plane,
                &client_id,
                &grant,
                pipe_name.as_deref(),
                &message,
            )),
            "resources/list" if initialized => Some(json!({
                "jsonrpc": "2.0", "id": id, "result": { "resources": mcp_resources() }
            })),
            "resources/read" if initialized => Some(mcp_resource_read(
                &mut plane,
                &client_id,
                &grant,
                pipe_name.as_deref(),
                &message,
            )),
            _ => Some(mcp_error(
                id,
                -32002,
                if initialized {
                    "unsupported MCP method"
                } else {
                    "MCP session is not initialized"
                },
            )),
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut stdout, &response)
                .map_err(|error| CliError::Io(error.to_string()))?;
            stdout
                .write_all(b"\n")
                .map_err(|error| CliError::Io(error.to_string()))?;
            stdout
                .flush()
                .map_err(|error| CliError::Io(error.to_string()))?;
        }
    }
    Ok(())
}

fn option_value_owned(args: &[String], option: &str) -> Result<String, CliError> {
    let index = args
        .iter()
        .position(|value| value == option)
        .ok_or_else(|| CliError::InvalidArguments(format!("{option} is required")))?;
    args.get(index + 1)
        .filter(|value| !value.is_empty() && !value.starts_with('-'))
        .cloned()
        .ok_or_else(|| CliError::InvalidArguments(format!("{option} requires a value")))
}

fn mcp_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn mcp_tools() -> Value {
    json!([
        { "name": "describe_capabilities", "description": "Read AudioRouter capabilities and schemas.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "list_devices", "description": "List authoritative audio endpoint descriptors.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "list_applications", "description": "List discoverable application identities.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "get_session", "description": "Read one session by opaque identifier.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "string", "minLength": 1 } }, "required": ["sessionId"], "additionalProperties": false } },
        { "name": "inspect_routes", "description": "Inspect desired upstream route provenance.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "string" }, "destinationNode": { "type": "string" } }, "required": ["sessionId", "destinationNode"], "additionalProperties": false } },
        { "name": "get_operation", "description": "Read an idempotent operation outcome.", "inputSchema": { "type": "object", "properties": { "operationId": { "type": "string" } }, "required": ["operationId"], "additionalProperties": false } },
        { "name": "list_recordings", "description": "List persisted recording metadata without reading audio content.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": ["string", "null"] } }, "additionalProperties": false } },
        { "name": "get_recording", "description": "Read one persisted recording metadata resource without reading audio content.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "remove_recording_entry", "description": "Remove recording library metadata without deleting the file.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "plan_graph_change", "description": "Validate and preview a complete graph candidate without committing it.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "string" }, "baseRevision": { "type": "integer", "minimum": 0 }, "candidate": { "type": "object" } }, "required": ["sessionId", "baseRevision", "candidate"], "additionalProperties": false } },
        { "name": "apply_graph_change", "description": "Commit a previously planned graph change with stale-plan and idempotency checks.", "inputSchema": { "type": "object", "properties": { "planId": { "type": "string" }, "baseRevision": { "type": "integer", "minimum": 0 }, "idempotencyKey": { "type": "string" } }, "required": ["planId", "baseRevision", "idempotencyKey"], "additionalProperties": false } },
        { "name": "control_session", "description": "Start or stop one session through the authorized lifecycle API.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "string" }, "action": { "enum": ["start", "stop"] } }, "required": ["sessionId", "action"], "additionalProperties": false } },
        { "name": "call_api", "description": "Call one validated permitted AudioRouter API method.", "inputSchema": { "type": "object", "properties": { "method": { "type": "string" }, "params": { "type": ["object", "null"] } }, "required": ["method"], "additionalProperties": false } }
    ])
}

fn mcp_resources() -> Value {
    json!([
        { "uri": "audiorouter://capabilities", "name": "AudioRouter capabilities", "description": "Current backend capabilities, method schemas, and limits.", "mimeType": "application/json" },
        { "uri": "audiorouter://diagnostics", "name": "Redacted diagnostics", "description": "Current redacted backend diagnostic snapshot.", "mimeType": "application/json" },
        { "uri": "audiorouter://workflow/headless", "name": "Headless workflow", "description": "Safe plan, apply, confirmation, and recovery guidance.", "mimeType": "text/plain" }
    ])
}

fn mcp_tool_call(
    plane: &mut ControlPlane,
    client_id: &str,
    grant: &audiorouter_control::ClientGrant,
    pipe_name: Option<&str>,
    message: &Value,
) -> Value {
    let id = message.get("id").cloned();
    let name = message["params"]["name"].as_str().unwrap_or_default();
    let arguments = message["params"]["arguments"].clone();
    let (method, params) = match name {
        "describe_capabilities" => ("system.describe", None),
        "list_devices" => ("devices.list", None),
        "list_applications" => ("apps.list", None),
        "get_session" => ("sessions.get", Some(arguments)),
        "inspect_routes" => ("routes.inspect", Some(arguments)),
        "get_operation" => ("operations.get", Some(arguments)),
        "list_recordings" => ("recordings.list", Some(arguments)),
        "get_recording" => ("recordings.get", Some(arguments)),
        "remove_recording_entry" => ("recordings.removeEntry", Some(arguments)),
        "plan_graph_change" => ("graph.plan", Some(arguments)),
        "apply_graph_change" => ("graph.commit", Some(arguments)),
        "control_session" => {
            let action = arguments["action"].as_str().unwrap_or_default();
            let method = match action {
                "start" => "sessions.start",
                "stop" => "sessions.stop",
                _ => return mcp_tool_error(id, "control_session action must be start or stop"),
            };
            let params = json!({ "sessionId": arguments["sessionId"] });
            return mcp_dispatch_tool(plane, client_id, grant, pipe_name, id, method, Some(params));
        }
        "call_api" => {
            let method = arguments["method"].as_str().unwrap_or_default();
            let params = arguments.get("params").cloned();
            if method.is_empty() {
                return mcp_tool_error(id, "call_api requires a method");
            }
            return mcp_dispatch_tool(plane, client_id, grant, pipe_name, id, method, params);
        }
        _ => return mcp_tool_error(id, "unknown tool"),
    };
    mcp_dispatch_tool(plane, client_id, grant, pipe_name, id, method, params)
}

fn mcp_resource_read(
    plane: &mut ControlPlane,
    client_id: &str,
    grant: &audiorouter_control::ClientGrant,
    pipe_name: Option<&str>,
    message: &Value,
) -> Value {
    let id = message.get("id").cloned();
    let uri = message["params"]["uri"].as_str().unwrap_or_default();
    let (mime_type, text) = match uri {
        "audiorouter://capabilities" | "audiorouter://diagnostics" => {
            let method = if uri.ends_with("capabilities") {
                "system.describe"
            } else {
                "system.diagnostics"
            };
            let payload = match mcp_api_value(
                plane,
                client_id,
                grant,
                pipe_name,
                Some(json!(1)),
                method,
                None,
            ) {
                Ok(payload) => payload,
                Err(error) => return mcp_error(id, -32003, &error),
            };
            ("application/json", serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()))
        }
        "audiorouter://workflow/headless" => (
            "text/plain",
            "Read capabilities and the current session revision; plan the smallest change; inspect warnings; apply with a unique idempotency key; confirm operation status and events; on conflict, reread and replan. Imports are stopped and never arm recording or install drivers automatically.".into(),
        ),
        _ => return mcp_error(id, -32602, "unknown resource URI"),
    };
    json!({ "jsonrpc": "2.0", "id": id, "result": { "contents": [{ "uri": uri, "mimeType": mime_type, "text": text }] } })
}

fn mcp_dispatch_tool(
    plane: &mut ControlPlane,
    client_id: &str,
    grant: &audiorouter_control::ClientGrant,
    pipe_name: Option<&str>,
    id: Option<Value>,
    method: &str,
    params: Option<Value>,
) -> Value {
    let payload = match mcp_api_value(
        plane,
        client_id,
        grant,
        pipe_name,
        id.clone(),
        method,
        params,
    ) {
        Ok(payload) => payload,
        Err(error) => return mcp_error(id, -32003, &error),
    };
    let is_error = payload.get("error").is_some();
    json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{ "type": "text", "text": serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()) }], "isError": is_error, "structuredContent": payload } })
}

fn mcp_api_value(
    plane: &mut ControlPlane,
    client_id: &str,
    grant: &audiorouter_control::ClientGrant,
    pipe_name: Option<&str>,
    request_id: Option<Value>,
    method: &str,
    params: Option<Value>,
) -> Result<Value, String> {
    let request = audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: request_id,
        method: method.into(),
        params,
    };
    if let Some(pipe_name) = pipe_name {
        let frame = audiorouter_protocol::encode_frame(&request)
            .map_err(|error| format!("cannot encode backend request: {error}"))?;
        let response = audiorouter_transport::round_trip(pipe_name, &frame)
            .map_err(|error| format!("backend pipe request failed: {error}"))?;
        return audiorouter_protocol::decode_frame::<Value>(&response)
            .map_err(|error| format!("invalid backend response: {error}"));
    }
    let response = plane.dispatch_authorized_for_client(request, client_id, grant);
    serde_json::to_value(response)
        .map_err(|error| format!("response serialization failed: {error}"))
}

fn mcp_tool_error(id: Option<Value>, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": { "isError": true, "content": [{ "type": "text", "text": message }] } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_and_json_schema_are_available_offline() {
        let help = run(["help", "--json"]).unwrap();
        assert!(help.contains("devices list"));
        assert!(help.contains("operation get"));
        let schema: Value = serde_json::from_str(&run(["schema", "--json"]).unwrap()).unwrap();
        assert_eq!(schema["protocolVersion"]["major"], 1);
    }

    #[test]
    fn list_commands_use_discovery_and_do_not_fake_devices() {
        let devices: Value =
            serde_json::from_str(&run(["devices", "list", "--json"]).unwrap()).unwrap();
        assert!(devices
            .as_array()
            .unwrap()
            .iter()
            .all(|device| { device["state"] == "active" && device["id"].as_str().is_some() }));
        let apps: Value = serde_json::from_str(&run(["apps", "list", "--json"]).unwrap()).unwrap();
        assert!(!apps.as_array().unwrap().is_empty());
        let nodes: Value =
            serde_json::from_str(&run(["nodes", "types", "--json"]).unwrap()).unwrap();
        assert!(nodes
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["availability"]["status"] == "unavailable"));
        let descriptions: Value =
            serde_json::from_str(&run(["nodes", "describe", "--json"]).unwrap()).unwrap();
        assert_eq!(
            descriptions.as_array().unwrap().len(),
            nodes.as_array().unwrap().len()
        );
    }

    #[test]
    fn invalid_and_unknown_commands_are_actionable() {
        assert!(matches!(
            run(["devices", "oops"]),
            Err(CliError::InvalidArguments(_))
        ));
        assert!(matches!(
            run(["nodes", "oops"]),
            Err(CliError::InvalidArguments(_))
        ));
        assert_eq!(run(["nope"]), Err(CliError::UnknownCommand("nope".into())));
        assert!(matches!(
            run(["operation", "oops"]),
            Err(CliError::InvalidArguments(_))
        ));
    }

    #[test]
    fn diagnostics_convenience_command_uses_read_only_dispatch() {
        let diagnostics: Value =
            serde_json::from_str(&run(["diagnostics", "--json"]).unwrap()).unwrap();
        assert_eq!(diagnostics["redacted"], true);
        assert_eq!(diagnostics["audio"]["state"], "unavailable");
    }

    #[test]
    fn generic_api_call_uses_the_shared_dispatcher() {
        let response: Value =
            serde_json::from_str(&run(["api", "call", "status.get", "--json"]).unwrap()).unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"]["audio"], "unavailable");

        let path =
            std::env::temp_dir().join(format!("audiorouter-cli-api-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"protocolVersion":{"major":1,"minor":0}}"#).unwrap();
        let path_string = path.to_string_lossy().into_owned();
        let response: Value = serde_json::from_str(
            &run(["api", "call", "system.handshake", &path_string, "--json"]).unwrap(),
        )
        .unwrap();
        assert_eq!(response["result"]["compatible"], true);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn import_and_export_use_validated_persistent_storage() {
        let suffix = format!("audiorouter-cli-{}", std::process::id());
        let database = std::env::temp_dir().join(format!("{suffix}.sqlite"));
        let document = std::env::temp_dir().join(format!("{suffix}.json"));
        let created_database = std::env::temp_dir().join(format!("{suffix}-created.sqlite"));
        let created_document = std::env::temp_dir().join(format!("{suffix}-created.json"));
        let _ = std::fs::remove_file(&database);
        let _ = std::fs::remove_file(&created_database);
        std::fs::write(
            &document,
            include_str!("../../../tests/fixtures/valid-session.json"),
        )
        .unwrap();
        let create_document = include_str!("../../../tests/fixtures/valid-session.json")
            .replace("session-fixture", "session-created");
        std::fs::write(&created_document, create_document).unwrap();
        let database_arg = database.to_string_lossy().into_owned();
        let document_arg = document.to_string_lossy().into_owned();
        let created_database_arg = created_database.to_string_lossy().into_owned();
        let created_document_arg = created_document.to_string_lossy().into_owned();
        let created = run([
            "session",
            "create",
            &created_document_arg,
            "--database",
            &created_database_arg,
            "--json",
        ])
        .unwrap();
        assert!(created.contains("session-created"));
        let imported = run([
            "import",
            &document_arg,
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        assert!(imported.contains("session"));
        let history = run([
            "history",
            "session-fixture",
            "--database",
            &database_arg,
            "--limit",
            "1",
            "--json",
        ])
        .unwrap();
        let history: Value = serde_json::from_str(&history).unwrap();
        assert_eq!(history.as_array().unwrap().len(), 1);
        let exported = run([
            "export",
            "session-fixture",
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        let exported: Value = serde_json::from_str(&exported).unwrap();
        assert_eq!(exported["id"], "session-fixture");
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let imported_database = std::env::temp_dir().join(format!("{suffix}-imported.sqlite"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_file(&imported_database);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        let bundle_arg = bundle.to_string_lossy().into_owned();
        let staging_arg = staging.to_string_lossy().into_owned();
        let imported_database_arg = imported_database.to_string_lossy().into_owned();
        run([
            "export-bundle",
            "session-fixture",
            "--database",
            &database_arg,
            "--output",
            &bundle_arg,
            "--json",
        ])
        .unwrap();
        let imported_bundle = run([
            "import-bundle",
            &bundle_arg,
            "--database",
            &imported_database_arg,
            "--staging",
            &staging_arg,
            "--json",
        ])
        .unwrap();
        assert!(imported_bundle.contains("session-fixture"));
        let duplicate_id = "session-fixture-copy";
        let duplicated = run([
            "session",
            "duplicate",
            "session-fixture",
            duplicate_id,
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        assert!(duplicated.contains(duplicate_id));
        let deleted = run([
            "session",
            "delete",
            "session-fixture",
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        assert!(deleted.contains("\"deleted\":true"));
        run([
            "session",
            "delete",
            duplicate_id,
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        let _ = std::fs::remove_file(database);
        let _ = std::fs::remove_file(document);
        let _ = std::fs::remove_file(created_database);
        let _ = std::fs::remove_file(created_document);
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_file(imported_database);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn graph_plan_file_is_inspectable_and_apply_rechecks_revision() {
        let suffix = format!("audiorouter-cli-plan-{}", std::process::id());
        let database = std::env::temp_dir().join(format!("{suffix}.sqlite"));
        let candidate_path = std::env::temp_dir().join(format!("{suffix}-candidate.json"));
        let plan_path = std::env::temp_dir().join(format!("{suffix}-plan.json"));
        for path in [&database, &candidate_path, &plan_path] {
            let _ = std::fs::remove_file(path);
        }
        let candidate_document = include_str!("../../../tests/fixtures/valid-session.json")
            .replace("session-fixture", "session-plan");
        std::fs::write(&candidate_path, candidate_document).unwrap();
        let database_arg = database.to_string_lossy().into_owned();
        let candidate_arg = candidate_path.to_string_lossy().into_owned();
        let plan_arg = plan_path.to_string_lossy().into_owned();
        run([
            "session",
            "create",
            &candidate_arg,
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        let first = run([
            "graph",
            "plan",
            "session-plan",
            "--base-revision",
            "0",
            "--file",
            &candidate_arg,
            "--output",
            &plan_arg,
            "--database",
            &database_arg,
            "--json",
        ]);
        assert!(first.is_ok(), "graph plan failed: {first:?}");
        let inspected: Value =
            serde_json::from_str(&run(["graph", "inspect", &plan_arg, "--json"]).unwrap()).unwrap();
        assert_eq!(inspected["format"], "audiorouter.graph-plan");
        assert!(run([
            "graph",
            "apply",
            &plan_arg,
            "--idempotency-key",
            "cli-plan-apply",
            "--database",
            &database_arg,
            "--json",
        ])
        .is_ok());
        let _ = std::fs::remove_file(database);
        let _ = std::fs::remove_file(candidate_path);
        let _ = std::fs::remove_file(plan_path);
    }

    #[test]
    fn mcp_tools_use_the_authorized_control_dispatcher() {
        let mut plane = ControlPlane::default();
        let session: audiorouter_domain::Session =
            serde_json::from_str(include_str!("../../../tests/fixtures/valid-session.json"))
                .unwrap();
        plane.insert_session(session).unwrap();
        let grant = audiorouter_control::ClientGrant::read_only();
        let response = mcp_tool_call(
            &mut plane,
            "mcp-test",
            &grant,
            None,
            &json!({
                "id": 7,
                "params": {
                    "name": "get_session",
                    "arguments": { "sessionId": "session-fixture" }
                }
            }),
        );
        assert_eq!(response["result"]["isError"], false);
        let content = response["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(content).unwrap();
        assert_eq!(payload["id"], 7);
        assert_eq!(mcp_tools().as_array().unwrap().len(), 13);
        assert_eq!(mcp_resources().as_array().unwrap().len(), 3);
        let denied = mcp_tool_call(
            &mut plane,
            "mcp-test",
            &grant,
            None,
            &json!({
                "id": 8,
                "params": {
                    "name": "plan_graph_change",
                    "arguments": { "sessionId": "session-fixture", "baseRevision": 0, "candidate": {} }
                }
            }),
        );
        assert_eq!(denied["result"]["isError"], true);
    }
}
