//! Offline M01 CLI command surface.

use audiorouter_control::{ClientGrant, ControlPlane};
use audiorouter_domain::{inspect_routes, validate_session, EntityId, PermissionScope};
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
        "virtual-devices" => list_subcommand(&command_args, "virtual-devices")?,
        "plugins" => plugins_command(&command_args)?,
        "presets" => presets_command(&command_args)?,
        "apps" => list_subcommand(&command_args, "apps")?,
        "applications" => list_subcommand(&command_args, "applications")?,
        "nodes" => list_subcommand(&command_args, "nodes")?,
        "routes" => routes_subcommand(&command_args)?,
        "history" => history_command(&command_args)?,
        "watch" => watch_command(&command_args)?,
        "graph" => graph_command(&command_args)?,
        "node" => node_command(&command_args)?,
        "session" => session_command(&command_args)?,
        "api" => api_subcommand(&command_args)?,
        "operation" => operation_command(&command_args)?,
        "recordings" => recordings_command(&command_args)?,
        "privacy" => privacy_command(&command_args)?,
        "recovery" => recovery_command(&command_args)?,
        "startup" => startup_command(&command_args)?,
        "backup" => backup_command(&command_args)?,
        "restore" => restore_command(&command_args)?,
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

fn startup_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("get") {
        return Err(CliError::InvalidArguments(
            "usage: startup get [--database <path>]".into(),
        ));
    }
    let request = request("startup.get");
    let response = if args.contains(&"--database") {
        ControlPlane::with_storage("cli", database(args)?).dispatch(request)
    } else {
        ControlPlane::default().dispatch(request)
    };
    response
        .result
        .ok_or_else(|| CliError::InvalidArguments("startup status unavailable".into()))
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

fn plugins_command(args: &[&str]) -> Result<Value, CliError> {
    let action = args.get(1).copied().unwrap_or_default();
    if !matches!(action, "scan" | "inspect") {
        return Err(CliError::InvalidArguments(
            "usage: plugins scan --directory <absolute-path> | plugins inspect --path <absolute-path>".into(),
        ));
    }
    let (method, params) = if action == "scan" {
        let directory = option_value(args, "--directory")?;
        if !std::path::Path::new(directory).is_absolute() {
            return Err(CliError::InvalidArguments(
                "--directory path must be absolute".into(),
            ));
        }
        ("plugins.scan", json!({ "directory": directory }))
    } else {
        let path = option_value(args, "--path")?;
        if !std::path::Path::new(path).is_absolute() {
            return Err(CliError::InvalidArguments("--path must be absolute".into()));
        }
        ("plugins.inspect", json!({ "path": path }))
    };
    let response = ControlPlane::default().dispatch_authorized(
        audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    response.result.ok_or_else(|| {
        CliError::Io(
            response
                .error
                .map_or_else(|| "plugin inspection failed".into(), |error| error.message),
        )
    })
}

fn presets_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("list") {
        return Err(CliError::InvalidArguments("usage: presets list".into()));
    }
    ControlPlane::default()
        .dispatch(request("presets.list"))
        .result
        .ok_or_else(|| CliError::InvalidArguments("presets unavailable".into()))
}

fn backup_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() == Some("prune") {
        let directory = option_value(args, "--directory")?;
        let directory_path = std::path::Path::new(directory);
        if !directory_path.is_absolute() {
            return Err(CliError::InvalidArguments(
                "--directory path must be absolute".into(),
            ));
        }
        let removed = Storage::prune_recovery_backups(directory_path)
            .map_err(|error| CliError::Storage(format!("{error:?}")))?;
        return Ok(json!({
            "directory": directory,
            "retainedDaily": audiorouter_storage::DAILY_RECOVERY_BACKUP_LIMIT,
            "removed": removed.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>()
        }));
    }
    let source = option_value(args, "--database")?;
    let destination = option_value(args, "--output")?;
    let source_path = std::path::Path::new(source);
    let destination_path = std::path::Path::new(destination);
    if !source_path.is_absolute() || !destination_path.is_absolute() {
        return Err(CliError::InvalidArguments(
            "--database and --output paths must be absolute".into(),
        ));
    }
    Storage::open(source_path)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?
        .backup_to(destination_path)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    Ok(json!({ "source": source, "destination": destination, "created": true }))
}

fn restore_command(args: &[&str]) -> Result<Value, CliError> {
    let source = option_value(args, "--backup")?;
    let destination = option_value(args, "--database")?;
    let source_path = std::path::Path::new(source);
    let destination_path = std::path::Path::new(destination);
    if !source_path.is_absolute() || !destination_path.is_absolute() {
        return Err(CliError::InvalidArguments(
            "--backup and --database paths must be absolute".into(),
        ));
    }
    Storage::restore_backup(source_path, destination_path)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    Ok(json!({ "source": source, "destination": destination, "restored": true }))
}

fn operation_command(args: &[&str]) -> Result<Value, CliError> {
    let action = args.get(1).copied();
    if !matches!(action, Some("get" | "cancel")) {
        return Err(CliError::InvalidArguments(
            "usage: operation <get|cancel> <operation-id> --database <path>".into(),
        ));
    }
    let operation_id = positional(args, 2, "operation id")?;
    let response = ControlPlane::with_storage("cli", database(args)?).dispatch(
        audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: if action == Some("cancel") {
                "operations.cancel".into()
            } else {
                "operations.get".into()
            },
            params: Some(json!({ "operationId": operation_id })),
        },
    );
    response
        .result
        .ok_or_else(|| CliError::InvalidArguments("operation not found".into()))
}

fn recordings_command(args: &[&str]) -> Result<Value, CliError> {
    let action = args.get(1).copied().ok_or_else(|| {
        CliError::InvalidArguments(
            "usage: recordings <list|get|recovery|preview|reveal|set-metadata|rename|remove-entry|recycle> [<recording-id>] --database <path> [--limit N] [--cursor ID]"
                .into(),
        )
    })?;
    let (method, params) = match action {
        "list" => {
            let session_id = args
                .iter()
                .position(|argument| *argument == "--session")
                .map(|index| {
                    args.get(index + 1)
                        .copied()
                        .filter(|value| !value.starts_with('-'))
                        .ok_or_else(|| {
                            CliError::InvalidArguments("--session requires a value".into())
                        })
                })
                .transpose()?;
            let cursor = optional_option_value(args, "--cursor")?;
            let limit = args
                .iter()
                .position(|argument| *argument == "--limit")
                .map(|index| {
                    args.get(index + 1)
                        .copied()
                        .ok_or_else(|| {
                            CliError::InvalidArguments("--limit requires a value".into())
                        })?
                        .parse::<u64>()
                        .map_err(|_| {
                            CliError::InvalidArguments("--limit must be an integer".into())
                        })
                })
                .transpose()?;
            if let Some(limit) = limit {
                if !(1..=500).contains(&limit) {
                    return Err(CliError::InvalidArguments(
                        "--limit must be between 1 and 500".into(),
                    ));
                }
            }
            let paged = cursor.is_some() || limit.is_some();
            let params = if paged {
                Some(json!({
                    "sessionId": session_id,
                    "cursor": cursor,
                    "limit": limit
                }))
            } else {
                session_id.map(|id| json!({ "sessionId": id }))
            };
            ("recordings.list", params)
        }
        "get" => (
            "recordings.get",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?
            })),
        ),
        "recovery" => (
            "recordings.recovery",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?
            })),
        ),
        "preview" => (
            "recordings.preview",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?
            })),
        ),
        "reveal" => (
            "recordings.reveal",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?
            })),
        ),
        "set-metadata" => {
            let recording_id = positional(args, 2, "recording id")?;
            let title = optional_option_value(args, "--title")?;
            let artist = optional_option_value(args, "--artist")?;
            let comment = optional_option_value(args, "--comment")?;
            if title.is_none() && artist.is_none() && comment.is_none() {
                return Err(CliError::InvalidArguments(
                    "set-metadata requires --title, --artist, or --comment".into(),
                ));
            }
            (
                "recordings.setMetadata",
                Some(json!({
                    "recordingId": recording_id,
                    "title": title,
                    "artist": artist,
                    "comment": comment
                })),
            )
        }
        "rename" => (
            "recordings.rename",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?,
                "newPath": absolute_option(args, "--path")?
            })),
        ),
        "remove-entry" => (
            "recordings.removeEntry",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?
            })),
        ),
        "recycle" => (
            "recordings.recycle",
            Some(json!({
                "recordingId": positional(args, 2, "recording id")?,
                "confirm": args.contains(&"--confirm")
            })),
        ),
        _ => return Err(CliError::InvalidArguments(
        "usage: recordings <list|get|recovery|preview|reveal|set-metadata|rename|remove-entry|recycle> [<recording-id>] --database <path> [--limit N] [--cursor ID]"
                .into(),
        )),
    };
    let mut plane = ControlPlane::with_storage("cli", database(args)?);
    let mut params = params;
    if method == "recordings.setMetadata" {
        let recording_id = params
            .as_ref()
            .and_then(|value| value.get("recordingId"))
            .and_then(Value::as_str)
            .ok_or_else(|| CliError::InvalidArguments("recording id is required".into()))?;
        let current = plane
            .dispatch(audiorouter_protocol::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "recordings.get".into(),
                params: Some(json!({ "recordingId": recording_id })),
            })
            .result
            .ok_or_else(|| CliError::InvalidArguments("recording not found".into()))?;
        if let Some(params) = params.as_mut() {
            for field in ["title", "artist", "comment"] {
                if params.get(field).is_some_and(Value::is_null) {
                    params[field] = current[field].clone();
                }
            }
        }
    }
    let response = plane.dispatch(audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params,
    });
    if let Some(result) = response.result {
        Ok(result)
    } else {
        Err(CliError::InvalidArguments(response.error.map_or_else(
            || format!("{method} failed"),
            |error| error.message,
        )))
    }
}

fn privacy_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("mute") {
        return Err(CliError::InvalidArguments(
            "usage: privacy mute <on|off> --database <path>".into(),
        ));
    }
    let muted = match args.get(2).copied() {
        Some("on") => true,
        Some("off") => false,
        _ => {
            return Err(CliError::InvalidArguments(
                "usage: privacy mute <on|off> --database <path>".into(),
            ))
        }
    };
    let mut plane = ControlPlane::with_storage("cli", database(args)?);
    plane
        .dispatch(audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": muted })),
        })
        .result
        .ok_or_else(|| CliError::InvalidArguments("privacy mute update failed".into()))
}

fn recovery_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("clear-safe-mode") {
        return Err(CliError::InvalidArguments(
            "usage: recovery clear-safe-mode --database <path>".into(),
        ));
    }
    let response = ControlPlane::with_storage("cli", database(args)?).dispatch(
        audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recovery.clearSafeMode".into(),
            params: None,
        },
    );
    response
        .result
        .ok_or_else(|| CliError::InvalidArguments("recovery clear failed".into()))
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
            "usage: {parent} {expected} [--limit N] [--cursor ID]"
        )));
    }
    let expected = subcommand.unwrap();
    let mut plane = ControlPlane::default();
    Ok(match parent {
        "devices" | "virtual-devices" => {
            let cursor = optional_option_value(args, "--cursor")?;
            let limit = optional_option_value(args, "--limit")?
                .map(|value| {
                    value.parse::<u64>().map_err(|_| {
                        CliError::InvalidArguments("--limit must be an integer".into())
                    })
                })
                .transpose()?;
            if let Some(limit) = limit {
                if !(1..=500).contains(&limit) {
                    return Err(CliError::InvalidArguments(
                        "--limit must be between 1 and 500".into(),
                    ));
                }
            }
            let method = if parent == "devices" {
                "devices.list"
            } else {
                "virtualDevices.list"
            };
            let mut device_request = request(method);
            if cursor.is_some() || limit.is_some() {
                device_request.params = Some(json!({ "cursor": cursor, "limit": limit }));
            }
            plane
                .dispatch(device_request)
                .result
                .unwrap_or_else(|| json!([]))
        }
        "apps" | "applications" => plane
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
            CliError::InvalidArguments(
                "usage: history <session-id> --database <path> [--limit N] [--cursor REVISION]"
                    .into(),
            )
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
    if let Some(cursor) = optional_option_value(args, "--cursor")? {
        if limit > 100 {
            return Err(CliError::InvalidArguments(
                "cursor history pages require --limit between 1 and 100".into(),
            ));
        }
        let mut plane = ControlPlane::with_storage("cli", storage);
        let response = plane.dispatch(audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "graph.history".into(),
            params: Some(json!({
                "sessionId": id,
                "cursor": cursor,
                "limit": limit
            })),
        });
        return response.result.ok_or_else(|| {
            CliError::InvalidArguments(
                response
                    .error
                    .map_or_else(|| "history unavailable".into(), |error| error.message),
            )
        });
    }
    let history = storage
        .load_history(&EntityId::new(id), limit)
        .map_err(|error| CliError::Storage(format!("{error:?}")))?;
    serde_json::to_value(history).map_err(|error| CliError::InvalidArguments(error.to_string()))
}

fn watch_command(args: &[&str]) -> Result<Value, CliError> {
    let session_id = positional(args, 1, "session id")?;
    let after_sequence = args
        .iter()
        .position(|argument| *argument == "--after")
        .map(|index| {
            args.get(index + 1)
                .copied()
                .ok_or_else(|| CliError::InvalidArguments("--after requires a value".into()))?
                .parse::<u64>()
                .map_err(|_| CliError::InvalidArguments("--after must be an integer".into()))
        })
        .transpose()?
        .unwrap_or(0);
    let limit = args
        .iter()
        .position(|argument| *argument == "--limit")
        .map(|index| {
            args.get(index + 1)
                .copied()
                .ok_or_else(|| CliError::InvalidArguments("--limit requires a value".into()))?
                .parse::<u64>()
                .map_err(|_| CliError::InvalidArguments("--limit must be an integer".into()))
        })
        .transpose()?
        .unwrap_or(100);
    if !(1..=500).contains(&limit) {
        return Err(CliError::InvalidArguments(
            "--limit must be between 1 and 500".into(),
        ));
    }
    let response = ControlPlane::with_storage("cli", database(args)?).dispatch(
        audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "events.subscribe".into(),
            params: Some(json!({
                "sessionId": session_id,
                "afterSequence": after_sequence,
                "limit": limit
            })),
        },
    );
    response.result.ok_or_else(|| {
        CliError::InvalidArguments(
            response
                .error
                .map_or_else(|| "watch failed".into(), |error| error.message),
        )
    })
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

fn node_command(args: &[&str]) -> Result<Value, CliError> {
    if args.get(1).copied() != Some("set") {
        return Err(CliError::InvalidArguments(
            "usage: node set <session-id> <node-id> <parameter> --value <json-scalar> [--idempotency-key <key>] [--dry-run] --database <path>".into(),
        ));
    }
    let session_id = positional(args, 2, "session id")?;
    let node_id = positional(args, 3, "node id")?;
    let parameter = positional(args, 4, "parameter")?;
    let raw_value = option_value_allow_negative(args, "--value")?;
    let value: Value = serde_json::from_str(raw_value).map_err(|error| {
        CliError::InvalidArguments(format!("--value must be valid JSON: {error}"))
    })?;
    if !value.is_boolean() && !value.is_number() && !value.is_string() {
        return Err(CliError::InvalidArguments(
            "--value must be a JSON boolean, number, or string".into(),
        ));
    }
    let dry_run = args.contains(&"--dry-run");
    let key = optional_option_value(args, "--idempotency-key")?;
    if !dry_run && key.is_none() {
        return Err(CliError::InvalidArguments(
            "--idempotency-key is required unless --dry-run is used".into(),
        ));
    }
    if let Some(key) = key {
        if key.is_empty() || key.len() > 256 {
            return Err(CliError::InvalidArguments(
                "--idempotency-key must contain 1..256 characters".into(),
            ));
        }
    }
    let storage = database(args)?;
    let mut candidate = storage
        .load_session(&EntityId::new(session_id))
        .map_err(|error| CliError::Storage(format!("{error:?}")))?
        .ok_or_else(|| CliError::InvalidArguments("session not found".into()))?;
    let node = candidate
        .nodes
        .iter_mut()
        .find(|node| node.id == EntityId::new(node_id))
        .ok_or_else(|| CliError::InvalidArguments("node not found".into()))?;
    node.parameters.insert(parameter.into(), value);
    let base_revision = candidate.revision;
    let candidate_id = candidate.id.clone();
    if dry_run {
        validate_session(&candidate).map_err(|errors| {
            CliError::InvalidArguments(format!("node update rejected: {errors:?}"))
        })?;
        return Ok(json!({
            "dryRun": true,
            "sessionId": candidate_id,
            "baseRevision": base_revision,
            "candidate": candidate
        }));
    }
    let mut plane = ControlPlane::with_storage("cli", storage);
    plane
        .dispatch(audiorouter_protocol::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.get".into(),
            params: Some(json!({ "sessionId": candidate_id })),
        })
        .result
        .ok_or_else(|| CliError::InvalidArguments("session not found".into()))?;
    let plan = plane
        .plan_graph(&candidate_id, base_revision, candidate)
        .map_err(|error| CliError::InvalidArguments(format!("node update rejected: {error:?}")))?;
    plane
        .commit_graph(&plan, base_revision, key.expect("validated above"))
        .map_err(|error| {
            CliError::InvalidArguments(format!("node update commit rejected: {error:?}"))
        })
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

fn optional_option_value<'a>(args: &'a [&str], option: &str) -> Result<Option<&'a str>, CliError> {
    let Some(index) = args.iter().position(|argument| *argument == option) else {
        return Ok(None);
    };
    args.get(index + 1)
        .copied()
        .filter(|value| !value.is_empty() && !value.starts_with('-'))
        .map(Some)
        .ok_or_else(|| CliError::InvalidArguments(format!("{option} requires a value")))
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
            "usage: session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path> [--limit N] [--cursor ID]".into(),
        )
    })?;
    if !matches!(
        action,
        "get" | "list" | "create" | "start" | "stop" | "delete" | "duplicate"
    ) {
        return Err(CliError::InvalidArguments(
            "usage: session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path> [--limit N] [--cursor ID]".into(),
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
        if let Some(cursor) = optional_option_value(args, "--cursor")? {
            let mut plane = ControlPlane::with_storage("cli", storage);
            let response = plane.dispatch(audiorouter_protocol::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "sessions.list".into(),
                params: Some(json!({ "cursor": cursor, "limit": limit })),
            });
            return response.result.ok_or_else(|| {
                CliError::InvalidArguments(
                    response
                        .error
                        .map_or_else(|| "session list unavailable".into(), |error| error.message),
                )
            });
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
                "usage: session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path> [--limit N] [--cursor ID]"
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
    let mut value = json!({ "commands": ["help", "status", "diagnostics [--database <path>]", "schema", "devices list [--limit N] [--cursor ID]", "apps list", "applications list", "nodes types", "nodes describe", "routes inspect <session-id> <destination-node> --database <path>", "history <session-id> --database <path> [--limit N] [--cursor REVISION]", "graph plan <session-id> --base-revision <n> --file <candidate.json> --output <plan.json> --database <path>", "graph inspect <plan.json>", "graph apply <plan.json> --idempotency-key <key> --database <path>", "node set <session-id> <node-id> <parameter> --value <json-scalar> [--idempotency-key <key>] [--dry-run] --database <path>", "operation get <operation-id> --database <path>", "session <get|list|create|start|stop|delete|duplicate> [<session-id>] --database <path> [--limit N] [--cursor ID]", "api methods", "api call <method> [<params-json-file|->] [--database <path>]", "mcp serve --client-id <enrolled-client> --database <path> [--pipe \\\\.\\pipe\\audiorouter]", "export <session-id> --database <path>", "import <document-path> --database <path>", "export-bundle <session-id> --database <path> --output <path>", "import-bundle <bundle-path> --database <path> --staging <directory>"], "globalOptions": ["--json"], "note": "Graph plans are versioned local files; apply revalidates the current revision before committing. The local MCP stdio adapter is pinned to protocol 2025-06-18 and requires an enrolled client." });
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(4, json!("backup --database <path> --output <new-path>"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(5, json!("virtual-devices list [--limit N] [--cursor ID]"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(6, json!("plugins scan --directory <absolute-path>"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(7, json!("plugins inspect --path <absolute-path>"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(7, json!("presets list"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(5, json!("backup prune --directory <path>"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(6, json!("restore --backup <path> --database <new-path>"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(3, json!("startup get [--database <path>]"));
    value["commands"].as_array_mut().unwrap().insert(
        7,
        json!("watch <session-id> --database <path> [--after N] [--limit N]"),
    );
    value["commands"].as_array_mut().unwrap().insert(
        15,
        json!("recordings list|get|recovery|preview|reveal|set-metadata|rename|remove-entry|recycle [<recording-id>] --database <path> [--limit N] [--cursor ID]"),
    );
    value["commands"].as_array_mut().unwrap().insert(
        14,
        json!("operation <get|cancel> <operation-id> --database <path>"),
    );
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(14, json!("privacy mute <on|off> --database <path>"));
    value["commands"]
        .as_array_mut()
        .unwrap()
        .insert(15, json!("recovery clear-safe-mode --database <path>"));
    value
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

fn option_value_allow_negative<'a>(args: &'a [&str], option: &str) -> Result<&'a str, CliError> {
    let index = args
        .iter()
        .position(|argument| *argument == option)
        .ok_or_else(|| CliError::InvalidArguments(format!("{option} is required")))?;
    args.get(index + 1)
        .copied()
        .filter(|value| !value.is_empty())
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
        { "name": "get_startup", "description": "Read sign-in startup capability without changing startup.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "list_devices", "description": "List authoritative audio endpoint descriptors. Optional cursor/limit fields return bounded pages.", "inputSchema": { "type": "object", "properties": { "cursor": { "type": ["string", "null"], "minLength": 1 }, "limit": { "type": "integer", "minimum": 1, "maximum": 500 } }, "additionalProperties": false } },
        { "name": "list_virtual_devices", "description": "List managed virtual bus desired state without activating endpoints. Optional cursor/limit fields return bounded pages.", "inputSchema": { "type": "object", "properties": { "cursor": { "type": ["string", "null"], "minLength": 1 }, "limit": { "type": "integer", "minimum": 1, "maximum": 500 } }, "additionalProperties": false } },
        { "name": "plan_virtual_device", "description": "Validate a managed virtual bus lifecycle operation without applying it.", "inputSchema": { "type": "object", "properties": { "operation": { "type": "object" } }, "required": ["operation"], "additionalProperties": false } },
        { "name": "apply_virtual_device", "description": "Apply a validated managed virtual bus lifecycle plan.", "inputSchema": { "type": "object", "properties": { "planId": { "type": "string", "minLength": 1 }, "idempotencyKey": { "type": "string", "minLength": 1 } }, "required": ["planId", "idempotencyKey"], "additionalProperties": false } },
        { "name": "list_applications", "description": "List discoverable application identities and observed Windows audio-session activity.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "get_session", "description": "Read one session by opaque identifier.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "string", "minLength": 1 } }, "required": ["sessionId"], "additionalProperties": false } },
        { "name": "inspect_routes", "description": "Inspect desired upstream route provenance.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "string" }, "destinationNode": { "type": "string" } }, "required": ["sessionId", "destinationNode"], "additionalProperties": false } },
        { "name": "get_operation", "description": "Read an idempotent operation outcome.", "inputSchema": { "type": "object", "properties": { "operationId": { "type": "string" } }, "required": ["operationId"], "additionalProperties": false } },
        { "name": "cancel_operation", "description": "Request cancellation; completed operations are never undone.", "inputSchema": { "type": "object", "properties": { "operationId": { "type": "string", "minLength": 1 } }, "required": ["operationId"], "additionalProperties": false } },
        { "name": "list_recordings", "description": "List persisted recording metadata without reading audio content; requires recording scope. Optional cursor/limit fields return bounded pages.", "inputSchema": { "type": "object", "properties": { "sessionId": { "type": ["string", "null"] }, "cursor": { "type": ["string", "null"], "minLength": 1 }, "limit": { "type": "integer", "minimum": 1, "maximum": 500 } }, "additionalProperties": false } },
        { "name": "get_recording", "description": "Read one persisted recording metadata resource without reading audio content; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "get_recording_recovery", "description": "Read a validated recorder recovery checkpoint without audio content; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "preview_recording", "description": "Inspect recording file metadata without decoding audio; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "reveal_recording", "description": "Reveal a recording in the operating system file browser; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "set_recording_metadata", "description": "Update recording metadata without changing its audio or path; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 }, "title": { "type": ["string", "null"], "maxLength": 256 }, "artist": { "type": ["string", "null"], "maxLength": 256 }, "comment": { "type": ["string", "null"], "maxLength": 256 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "rename_recording", "description": "Rename a recording within its approved directory; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 }, "newPath": { "type": "string", "minLength": 1 } }, "required": ["recordingId", "newPath"], "additionalProperties": false } },
        { "name": "set_privacy_mute", "description": "Latch or clear process-local privacy mute; requires capture scope.", "inputSchema": { "type": "object", "properties": { "muted": { "type": "boolean" } }, "required": ["muted"], "additionalProperties": false } },
        { "name": "clear_recovery_safe_mode", "description": "Clear the latched crash-recovery safe mode after stability is confirmed; requires session-control scope.", "inputSchema": { "type": "object", "additionalProperties": false } },
        { "name": "remove_recording_entry", "description": "Remove recording library metadata without deleting the file.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 } }, "required": ["recordingId"], "additionalProperties": false } },
        { "name": "recycle_recording", "description": "Preview or explicitly recycle a recording through the OS Recycle Bin; requires recording scope.", "inputSchema": { "type": "object", "properties": { "recordingId": { "type": "string", "minLength": 1 }, "confirm": { "type": "boolean" } }, "required": ["recordingId"], "additionalProperties": false } },
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
        "get_startup" => ("startup.get", None),
        "list_devices" => ("devices.list", Some(arguments)),
        "list_virtual_devices" => ("virtualDevices.list", Some(arguments)),
        "plan_virtual_device" => ("virtualDevices.plan", Some(arguments)),
        "apply_virtual_device" => ("virtualDevices.apply", Some(arguments)),
        "list_applications" => ("apps.list", None),
        "get_session" => ("sessions.get", Some(arguments)),
        "inspect_routes" => ("routes.inspect", Some(arguments)),
        "get_operation" => ("operations.get", Some(arguments)),
        "cancel_operation" => ("operations.cancel", Some(arguments)),
        "list_recordings" => ("recordings.list", Some(arguments)),
        "get_recording" => ("recordings.get", Some(arguments)),
        "get_recording_recovery" => ("recordings.recovery", Some(arguments)),
        "preview_recording" => ("recordings.preview", Some(arguments)),
        "reveal_recording" => ("recordings.reveal", Some(arguments)),
        "set_recording_metadata" => ("recordings.setMetadata", Some(arguments)),
        "rename_recording" => ("recordings.rename", Some(arguments)),
        "set_privacy_mute" => ("safety.setPrivacyMute", Some(arguments)),
        "clear_recovery_safe_mode" => ("recovery.clearSafeMode", None),
        "remove_recording_entry" => ("recordings.removeEntry", Some(arguments)),
        "recycle_recording" => ("recordings.recycle", Some(arguments)),
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
        assert!(help.contains("virtual-devices list"));
        assert!(help.contains("plugins scan"));
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
        assert!(apps.as_array().unwrap().iter().all(|application| {
            application.get("audioActivity").is_some()
                && application.get("captureCapability").is_some()
                && application.get("audioSessionCount").is_some()
                && application.get("activeAudioSessionCount").is_some()
                && application.get("captureSessionCount").is_some()
                && application.get("audioDisplayNames").is_some()
        }));
        let applications: Value =
            serde_json::from_str(&run(["applications", "list", "--json"]).unwrap()).unwrap();
        assert!(!applications.as_array().unwrap().is_empty());
        assert!(applications.as_array().unwrap().iter().all(|application| {
            application["processId"].as_u64().is_some_and(|pid| pid > 0)
                && application["executable"].as_str().is_some()
                && application["audioActivity"].is_string()
                && application["captureCapability"].is_string()
        }));
        let virtual_devices: Value =
            serde_json::from_str(&run(["virtual-devices", "list", "--json"]).unwrap()).unwrap();
        assert!(virtual_devices.as_array().unwrap().is_empty());
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
    fn presets_list_exposes_the_read_only_voice_catalog() {
        let presets: Value =
            serde_json::from_str(&run(["presets", "list", "--json"]).unwrap()).unwrap();
        let voice_chains = presets["voiceChains"].as_array().unwrap();
        assert_eq!(voice_chains.len(), 2);
        assert_eq!(voice_chains[0]["id"], "voiceNeutral");
        assert_eq!(voice_chains[1]["id"], "voiceGateAndCompression");
        assert!(voice_chains
            .iter()
            .all(|preset| preset["name"].as_str().is_some()
                && preset["description"].as_str().is_some()));
        let eq = presets["eq"].as_array().unwrap();
        assert_eq!(
            eq.iter()
                .map(|preset| preset["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["voiceNeutral", "hum50Hz", "hum60Hz"]
        );
    }

    #[test]
    fn recording_commands_use_control_dispatch_and_preserve_file_entries() {
        let database = std::env::temp_dir().join(format!(
            "audiorouter-cli-recordings-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database);
        let storage = Storage::open(&database).unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-cli".into(),
                session_id: "session-cli".into(),
                recorder_id: "recorder-1".into(),
                path: "C:\\Audio\\recording.wav".into(),
                format: "wav".into(),
                channels: 2,
                sample_rate: 48_000,
                frames: 128,
                file_bytes: 512,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "complete".into(),
                missing: false,
                title: Some("CLI test".into()),
                artist: None,
                comment: None,
            })
            .unwrap();
        drop(storage);
        let database = database.to_string_lossy().into_owned();
        let listed: Value = serde_json::from_str(
            &run(["recordings", "list", "--database", &database, "--json"]).unwrap(),
        )
        .unwrap();
        assert_eq!(listed[0]["id"], "recording-cli");
        let paged: Value = serde_json::from_str(
            &run([
                "recordings",
                "list",
                "--session",
                "session-cli",
                "--limit",
                "1",
                "--database",
                &database,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(paged["items"][0]["id"], "recording-cli");
        assert_eq!(paged["nextCursor"], Value::Null);
        let fetched: Value = serde_json::from_str(
            &run([
                "recordings",
                "get",
                "recording-cli",
                "--database",
                &database,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fetched["path"], "C:\\Audio\\recording.wav");
        let preview: Value = serde_json::from_str(
            &run([
                "recordings",
                "preview",
                "recording-cli",
                "--database",
                &database,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(preview["preview"]["status"], "missing");
        let updated: Value = serde_json::from_str(
            &run([
                "recordings",
                "set-metadata",
                "recording-cli",
                "--title",
                "Updated title",
                "--database",
                &database,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(updated["updated"], true);
        let fetched: Value = serde_json::from_str(
            &run([
                "recordings",
                "get",
                "recording-cli",
                "--database",
                &database,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fetched["title"], "Updated title");
        assert_eq!(fetched["path"], "C:\\Audio\\recording.wav");
        let muted: Value = serde_json::from_str(
            &run(["privacy", "mute", "on", "--database", &database, "--json"]).unwrap(),
        )
        .unwrap();
        assert_eq!(muted["muted"], true);
        let removed: Value = serde_json::from_str(
            &run([
                "recordings",
                "remove-entry",
                "recording-cli",
                "--database",
                &database,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(removed["fileAction"], "none");
        assert!(Storage::open(&database)
            .unwrap()
            .get_recording("recording-cli")
            .unwrap()
            .is_none());
        let _ = std::fs::remove_file(database);
    }

    #[test]
    fn invalid_and_unknown_commands_are_actionable() {
        assert!(matches!(
            run(["devices", "oops"]),
            Err(CliError::InvalidArguments(_))
        ));
        assert!(matches!(
            run(["devices", "list", "--limit", "0"]),
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
        assert!(matches!(
            run(["node", "oops"]),
            Err(CliError::InvalidArguments(_))
        ));
        assert!(matches!(
            run(["plugins", "scan", "--directory", "relative"]),
            Err(CliError::InvalidArguments(_))
        ));
    }

    #[test]
    fn plugin_scan_cli_keeps_invalid_candidates_visible() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-cli-plugin-scan-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("candidate.vst3"), b"not a portable executable").unwrap();
        let directory = root.to_string_lossy().into_owned();
        let scanned: Value = serde_json::from_str(
            &run(["plugins", "scan", "--directory", &directory, "--json"]).unwrap(),
        )
        .unwrap();
        assert_eq!(scanned["entries"].as_array().unwrap().len(), 1);
        assert!(scanned["entries"][0]["identity"].is_null());
        assert!(scanned["entries"][0]["error"].is_string());
        let path = root.join("candidate.vst3").to_string_lossy().into_owned();
        let inspected: Value =
            serde_json::from_str(&run(["plugins", "inspect", "--path", &path, "--json"]).unwrap())
                .unwrap();
        assert_eq!(inspected["path"], path);
        assert!(inspected["identity"].is_null());
        assert!(inspected["error"].is_string());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn node_set_uses_plan_commit_and_preserves_revision_safety() {
        let suffix = format!("audiorouter-cli-node-set-{}", std::process::id());
        let database = std::env::temp_dir().join(format!("{suffix}.sqlite"));
        let _ = std::fs::remove_file(&database);
        let fixture = include_str!("../../../tests/fixtures/valid-session.json")
            .replace("session-fixture", "node-set-session")
            .replace("\"kind\": \"physicalInput\"", "\"kind\": \"gain\"");
        let database_arg = database.to_string_lossy().into_owned();
        let storage = Storage::open(&database).unwrap();
        let session: audiorouter_domain::Session = serde_json::from_str(&fixture).unwrap();
        storage.save_session(&session).unwrap();
        drop(storage);
        let committed: Value = serde_json::from_str(
            &run([
                "node",
                "set",
                "node-set-session",
                "input",
                "gainDb",
                "--value",
                "-6",
                "--idempotency-key",
                "node-set-1",
                "--database",
                &database_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(committed["revision"], 1);
        let session = Storage::open(&database)
            .unwrap()
            .load_session(&EntityId::new("node-set-session"))
            .unwrap()
            .unwrap();
        assert_eq!(session.nodes[0].parameters["gainDb"], -6);
        assert_eq!(session.revision, 1);
        let dry_run: Value = serde_json::from_str(
            &run([
                "node",
                "set",
                "node-set-session",
                "input",
                "gainDb",
                "--value",
                "-3",
                "--dry-run",
                "--database",
                &database_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(dry_run["dryRun"], true);
        assert_eq!(dry_run["candidate"]["nodes"][0]["parameters"]["gainDb"], -3);
        let unchanged = Storage::open(&database)
            .unwrap()
            .load_session(&EntityId::new("node-set-session"))
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.revision, 1);
        assert_eq!(unchanged.nodes[0].parameters["gainDb"], -6);
        let _ = std::fs::remove_file(database);
    }

    #[test]
    fn session_list_exposes_backend_cursor_pages() {
        let database = std::env::temp_dir().join(format!(
            "audiorouter-cli-session-page-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database);
        let storage = Storage::open(&database).unwrap();
        for id in ["page-a", "page-b"] {
            let mut session: audiorouter_domain::Session =
                serde_json::from_str(include_str!("../../../tests/fixtures/valid-session.json"))
                    .unwrap();
            session.id = EntityId::new(id);
            storage.save_session(&session).unwrap();
        }
        drop(storage);
        let database_arg = database.to_string_lossy().into_owned();
        let page: Value = serde_json::from_str(
            &run([
                "session",
                "list",
                "--cursor",
                "page-a",
                "--limit",
                "1",
                "--database",
                &database_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(page["items"][0]["id"], "page-b");
        assert_eq!(page["nextCursor"], "page-b");
        let _ = std::fs::remove_file(database);
    }

    #[test]
    fn history_exposes_revision_cursor_pages() {
        let database = std::env::temp_dir().join(format!(
            "audiorouter-cli-history-page-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database);
        let storage = Storage::open(&database).unwrap();
        let mut first: audiorouter_domain::Session =
            serde_json::from_str(include_str!("../../../tests/fixtures/valid-session.json"))
                .unwrap();
        first.id = EntityId::new("history-page");
        storage.save_session(&first).unwrap();
        let mut second = first.clone();
        second.revision = 1;
        second.name = "Updated".into();
        storage.save_session(&second).unwrap();
        drop(storage);
        let database_arg = database.to_string_lossy().into_owned();
        let page: Value = serde_json::from_str(
            &run([
                "history",
                "history-page",
                "--cursor",
                "2",
                "--limit",
                "1",
                "--database",
                &database_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(page["items"][0]["revision"], 1);
        assert_eq!(page["nextCursor"], "1");
        let _ = std::fs::remove_file(database);
    }

    #[test]
    fn diagnostics_convenience_command_uses_read_only_dispatch() {
        let diagnostics: Value =
            serde_json::from_str(&run(["diagnostics", "--json"]).unwrap()).unwrap();
        assert_eq!(diagnostics["redacted"], true);
        assert_eq!(diagnostics["audio"]["state"], "unavailable");
    }

    #[test]
    fn startup_convenience_command_reports_capability_without_registration() {
        let startup: Value =
            serde_json::from_str(&run(["startup", "get", "--json"]).unwrap()).unwrap();
        assert_eq!(startup["enabled"], false);
        assert_eq!(startup["registration"], "unavailable");
    }

    #[test]
    fn backup_and_restore_commands_round_trip_without_overwriting() {
        let suffix = format!("audiorouter-cli-recovery-{}", std::process::id());
        let source = std::env::temp_dir().join(format!("{suffix}-source.sqlite"));
        let backup = std::env::temp_dir().join(format!("{suffix}-backup.sqlite"));
        let restored = std::env::temp_dir().join(format!("{suffix}-restored.sqlite"));
        for path in [&source, &backup, &restored] {
            let _ = std::fs::remove_file(path);
        }
        let source_arg = source.to_string_lossy().into_owned();
        let backup_arg = backup.to_string_lossy().into_owned();
        let restored_arg = restored.to_string_lossy().into_owned();
        let created: Value = serde_json::from_str(
            &run([
                "backup",
                "--database",
                &source_arg,
                "--output",
                &backup_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(created["created"], true);
        let restored_result: Value = serde_json::from_str(
            &run([
                "restore",
                "--backup",
                &backup_arg,
                "--database",
                &restored_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(restored_result["restored"], true);
        assert!(restored.is_file());
        assert!(matches!(
            run([
                "restore",
                "--backup",
                &backup_arg,
                "--database",
                &restored_arg,
                "--json"
            ]),
            Err(CliError::Storage(_))
        ));
        for path in [&source, &backup, &restored] {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn backup_prune_command_retains_daily_limit_and_pre_migration_files() {
        let directory =
            std::env::temp_dir().join(format!("audiorouter-cli-retention-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory).unwrap();
        for index in 1..=11 {
            std::fs::write(
                directory.join(format!("audiorouter-backup-202609{:02}.sqlite", index)),
                [index as u8],
            )
            .unwrap();
        }
        let pre_migration = directory.join("audiorouter-pre-migration-20260901.sqlite");
        std::fs::write(&pre_migration, b"preserve").unwrap();
        let directory_arg = directory.to_string_lossy().into_owned();
        let result: Value = serde_json::from_str(
            &run(["backup", "prune", "--directory", &directory_arg, "--json"]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result["retainedDaily"],
            audiorouter_storage::DAILY_RECOVERY_BACKUP_LIMIT
        );
        assert_eq!(result["removed"].as_array().unwrap().len(), 1);
        assert!(!directory
            .join("audiorouter-backup-20260901.sqlite")
            .exists());
        assert!(directory
            .join("audiorouter-backup-20260911.sqlite")
            .exists());
        assert!(pre_migration.exists());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn recording_recovery_command_reads_checkpoint_without_audio_access() {
        let database = std::env::temp_dir().join(format!(
            "audiorouter-cli-recording-recovery-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database);
        let storage = Storage::open(&database).unwrap();
        let mut recorder = audiorouter_recording::RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(10).unwrap();
        storage
            .save_recording_checkpoint("recovery-cli", &recorder.checkpoint())
            .unwrap();
        let database_arg = database.to_string_lossy().into_owned();
        let result: Value = serde_json::from_str(
            &run([
                "recordings",
                "recovery",
                "recovery-cli",
                "--database",
                &database_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["status"], "available");
        assert_eq!(result["checkpoint"]["state"], "Recording");
        let _ = std::fs::remove_file(database);
    }

    #[test]
    fn watch_command_reads_bounded_event_cursor_without_audio_access() {
        let database = std::env::temp_dir().join(format!(
            "audiorouter-cli-watch-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database);
        let storage = Storage::open(&database).unwrap();
        let session: audiorouter_domain::Session =
            serde_json::from_str(include_str!("../../../tests/fixtures/valid-session.json"))
                .unwrap();
        storage.save_session(&session).unwrap();
        let database_arg = database.to_string_lossy().into_owned();
        let result: Value = serde_json::from_str(
            &run([
                "watch",
                "session-fixture",
                "--database",
                &database_arg,
                "--after",
                "0",
                "--limit",
                "10",
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result["events"].as_array().unwrap().len(), 0);
        assert_eq!(result["nextSequence"], 0);
        assert!(matches!(
            run([
                "watch",
                "session-fixture",
                "--database",
                &database_arg,
                "--limit",
                "501",
                "--json",
            ]),
            Err(CliError::InvalidArguments(_))
        ));
        let _ = std::fs::remove_file(database);
    }

    #[test]
    fn recovery_clear_command_clears_only_the_safe_mode_latch() {
        let database = std::env::temp_dir().join(format!(
            "audiorouter-cli-recovery-clear-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database);
        let storage = Storage::open(&database).unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for offset in 0..3 {
            storage.record_recovery_crash(now + offset).unwrap();
        }
        let database_arg = database.to_string_lossy().into_owned();
        let cleared: Value = serde_json::from_str(
            &run([
                "recovery",
                "clear-safe-mode",
                "--database",
                &database_arg,
                "--json",
            ])
            .unwrap(),
        )
        .unwrap();
        assert_eq!(cleared["safeMode"], false);
        assert_eq!(cleared["recentCrashes"], 0);
        let status = Storage::open(&database)
            .unwrap()
            .recovery_status(now)
            .unwrap();
        assert_eq!(status.recent_crashes, 0);
        assert!(!status.safe_mode);
        let _ = std::fs::remove_file(database);
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
        let startup = mcp_tool_call(
            &mut plane,
            "mcp-test",
            &grant,
            None,
            &json!({
                "id": 9,
                "params": { "name": "get_startup", "arguments": {} }
            }),
        );
        assert_eq!(startup["result"]["isError"], false);
        let startup_content = startup["result"]["content"][0]["text"].as_str().unwrap();
        let startup_payload: Value = serde_json::from_str(startup_content).unwrap();
        assert_eq!(startup_payload["result"]["registration"], "unavailable");
        let recovery = mcp_tool_call(
            &mut plane,
            "mcp-test",
            &grant,
            None,
            &json!({
                "id": 10,
                "params": {
                    "name": "get_recording_recovery",
                    "arguments": { "recordingId": "missing" }
                }
            }),
        );
        assert_eq!(recovery["result"]["isError"], true);
        let recovery_content = recovery["result"]["content"][0]["text"].as_str().unwrap();
        let recovery_payload: Value = serde_json::from_str(recovery_content).unwrap();
        assert!(recovery_payload["error"].is_object());
        let operator =
            audiorouter_control::ClientGrant::for_role(audiorouter_control::ClientRole::Operator);
        let cleared = mcp_tool_call(
            &mut plane,
            "mcp-test",
            &operator,
            None,
            &json!({
                "id": 11,
                "params": { "name": "clear_recovery_safe_mode", "arguments": {} }
            }),
        );
        assert_eq!(cleared["result"]["isError"], false);
        let denied_clear = mcp_tool_call(
            &mut plane,
            "mcp-test",
            &grant,
            None,
            &json!({
                "id": 12,
                "params": { "name": "clear_recovery_safe_mode", "arguments": {} }
            }),
        );
        assert_eq!(denied_clear["result"]["isError"], true);
        assert_eq!(mcp_tools().as_array().unwrap().len(), 26);
        let tools = mcp_tools();
        let list_recordings = tools
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "list_recordings")
            .unwrap();
        assert_eq!(
            list_recordings["inputSchema"]["properties"]["limit"]["maximum"],
            500
        );
        assert_eq!(
            list_recordings["inputSchema"]["properties"]["cursor"]["type"],
            json!(["string", "null"])
        );
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
