//! Offline M01 CLI command surface.

use audiorouter_control::ControlPlane;
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
        "api" => list_subcommand(&command_args, "api")?,
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

fn request(method: &str) -> audiorouter_protocol::JsonRpcRequest {
    audiorouter_protocol::JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: None,
    }
}

fn help_value() -> Value {
    json!({ "commands": ["help", "status", "schema", "devices list", "apps list", "nodes types", "api methods"], "globalOptions": ["--json"], "note": "This M01 CLI reports offline control-plane capabilities; real Windows audio is added in M02." })
}

fn render_human(command: &str, value: &Value) -> String {
    if command == "help" {
        return format!(
            "AudioRouter M01\n{}\n",
            value["commands"]
                .as_array()
                .unwrap()
                .iter()
                .map(Value::as_str)
                .flatten()
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
        assert_eq!(devices, json!([]));
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
}
