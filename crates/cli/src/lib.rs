//! Offline M01 CLI command surface.

use audiorouter_control::ControlPlane;
use audiorouter_domain::{inspect_routes, EntityId};
use audiorouter_storage::Storage;
use serde_json::{json, Value};

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
        "devices" => list_subcommand(&command_args, "devices")?,
        "apps" => list_subcommand(&command_args, "apps")?,
        "nodes" => list_subcommand(&command_args, "nodes")?,
        "routes" => routes_subcommand(&command_args)?,
        "api" => list_subcommand(&command_args, "api")?,
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

fn list_subcommand(args: &[&str], parent: &str) -> Result<Value, CliError> {
    let expected = if parent == "api" {
        "methods"
    } else if parent == "nodes" {
        "types"
    } else {
        "list"
    };
    if args.get(1).copied() != Some(expected) {
        return Err(CliError::InvalidArguments(format!(
            "usage: {parent} {expected}"
        )));
    }
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
        "nodes" => plane.describe()["nodeTypes"].clone(),
        "api" => plane.describe()["methods"].clone(),
        _ => unreachable!(),
    })
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

fn request(method: &str) -> audiorouter_protocol::JsonRpcRequest {
    audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: None,
    }
}

fn help_value() -> Value {
    json!({ "commands": ["help", "status", "schema", "devices list", "apps list", "nodes types", "routes inspect <session-id> <destination-node> --database <path>", "api methods", "export <session-id> --database <path>", "import <document-path> --database <path>", "export-bundle <session-id> --database <path> --output <path>", "import-bundle <bundle-path> --database <path> --staging <directory>"], "globalOptions": ["--json"], "note": "This M01 CLI reports offline control-plane capabilities; real Windows audio is added in M02." })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_and_json_schema_are_available_offline() {
        let help = run(["help", "--json"]).unwrap();
        assert!(help.contains("devices list"));
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
    }

    #[test]
    fn invalid_and_unknown_commands_are_actionable() {
        assert!(matches!(
            run(["devices", "oops"]),
            Err(CliError::InvalidArguments(_))
        ));
        assert_eq!(run(["nope"]), Err(CliError::UnknownCommand("nope".into())));
    }

    #[test]
    fn import_and_export_use_validated_persistent_storage() {
        let suffix = format!("audiorouter-cli-{}", std::process::id());
        let database = std::env::temp_dir().join(format!("{suffix}.sqlite"));
        let document = std::env::temp_dir().join(format!("{suffix}.json"));
        let _ = std::fs::remove_file(&database);
        std::fs::write(
            &document,
            include_str!("../../../tests/fixtures/valid-session.json"),
        )
        .unwrap();
        let database_arg = database.to_string_lossy().into_owned();
        let document_arg = document.to_string_lossy().into_owned();
        let imported = run([
            "import",
            &document_arg,
            "--database",
            &database_arg,
            "--json",
        ])
        .unwrap();
        assert!(imported.contains("session"));
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
        let _ = std::fs::remove_file(database);
        let _ = std::fs::remove_file(document);
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_file(imported_database);
        let _ = std::fs::remove_dir_all(staging);
    }
}
