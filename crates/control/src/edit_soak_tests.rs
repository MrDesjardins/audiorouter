//! In-process edit-soak measurement (Q5.4 round 3).
//!
//! The synthetic shell soak's private-byte growth could not be separated from
//! the WebView, the logger and the debug build. This ignored release test runs
//! the same `nodes.set` + `status.get` mix through `ControlPlane::dispatch`
//! against a temp SQLite database and samples the process's private bytes and
//! every retaining structure. It is a measurement, not a CI gate:
//!
//!   cargo test --release -p audiorouter-control --lib edit_soak -- --ignored --nocapture
//!
//! `AUDIOROUTER_EDIT_SOAK_EDITS` overrides the 20,000 default.

use super::*;
use audiorouter_domain::{Edge, Node, Port};
use std::process::Command;

fn gain_node(id: &str) -> Node {
    Node {
        id: EntityId::new(id),
        kind: NodeKind::Gain,
        type_version: 1,
        name: id.to_owned(),
        enabled: true,
        bypass: false,
        parameters: serde_json::Map::from_iter([("gainDb".to_owned(), json!(0.0))]),
        ports: vec![
            Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 2,
            },
            Port {
                name: "out".into(),
                direction: PortDirection::Output,
                channels: 2,
            },
        ],
    }
}

fn session64() -> Session {
    let mut nodes = vec![Node {
        id: EntityId::new("soak-input"),
        kind: NodeKind::PhysicalInput,
        type_version: 1,
        name: "Input".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::Map::new(),
        ports: vec![Port {
            name: "main".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    }];
    let mut edges = Vec::new();
    for index in 0..61 {
        let id = format!("soak-gain-{index}");
        nodes.push(gain_node(&id));
        let (source, source_port) = if index == 0 {
            ("soak-input".to_owned(), "main".to_owned())
        } else {
            (format!("soak-gain-{}", index - 1), "out".to_owned())
        };
        edges.push(Edge {
            id: EntityId::new(format!("soak-edge-{index}")),
            source_node: EntityId::new(source),
            source_port,
            destination_node: EntityId::new(&id),
            destination_port: "in".to_owned(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        });
    }
    nodes.push(Node {
        id: EntityId::new("soak-output"),
        kind: NodeKind::PhysicalOutput,
        type_version: 1,
        name: "Output".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::Map::new(),
        ports: vec![Port {
            name: "main".into(),
            direction: PortDirection::Input,
            channels: 2,
        }],
    });
    edges.push(Edge {
        id: EntityId::new("soak-edge-61"),
        source_node: EntityId::new("soak-gain-60"),
        source_port: "out".into(),
        destination_node: EntityId::new("soak-output"),
        destination_port: "main".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    });
    Session {
        id: EntityId::new("soak-session"),
        name: "Edit soak".into(),
        schema_version: 1,
        revision: 0,
        nodes,
        edges,
    }
}

#[cfg(windows)]
fn private_mib(pid: u32) -> Option<f64> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            &format!("(Get-Process -Id {pid}).PrivateMemorySize64"),
        ])
        .output()
        .ok()?;
    let bytes = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .ok()?;
    (bytes > 0.0).then(|| bytes / 1_048_576.0)
}

#[cfg(not(windows))]
fn private_mib(_pid: u32) -> Option<f64> {
    None
}

fn db_mib(path: &std::path::Path) -> f64 {
    std::fs::metadata(path).map_or(0.0, |metadata| metadata.len() as f64 / 1_048_576.0)
}

fn journal_rows(path: &std::path::Path) -> Option<u64> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .ok()?;
    connection
        .query_row("SELECT COUNT(*) FROM operation_journal", [], |row| {
            row.get(0)
        })
        .ok()
}

fn sample(plane: &ControlPlane, path: &std::path::Path, edits: usize) {
    let notes = plane
        .revision_notes
        .get(&EntityId::new("soak-session"))
        .map_or(0, Vec::len);
    println!(
        "sample=edits:{edits} private_mib={:?} db_mib={:.2} journal={:?} outcomes={} order={} hashes={} events={} history={} notes={}",
        private_mib(std::process::id()),
        db_mib(path),
        journal_rows(path),
        plane.operation_outcomes.len(),
        plane.operation_order.len(),
        plane.idempotency_hashes.len(),
        plane.events.len(),
        plane
            .store
            .history(&EntityId::new("soak-session"), audiorouter_domain::MAX_GRAPH_HISTORY_ENTRIES)
            .len(),
        notes,
    );
}

#[test]
#[ignore = "20,000 dispatch edits; release-mode measurement, run with --ignored"]
fn dispatch_edit_soak_samples_memory_and_journal() {
    let dir = std::env::temp_dir().join(format!("audiorouter-edit-soak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("soak.sqlite");
    let storage = Storage::open(&path).unwrap();
    let mut plane = ControlPlane::try_with_storage("edit-soak", storage).unwrap();
    plane.insert_session(session64()).unwrap();
    let edits = std::env::var("AUDIOROUTER_EDIT_SOAK_EDITS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20_000);
    sample(&plane, &path, 0);
    let mut refused = None;
    for index in 1..=edits {
        let request = |id: u64, method: &str, params: Option<Value>| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: method.into(),
            params,
        };
        let node = format!("soak-gain-{}", (index - 1) % 61);
        let response = plane.dispatch(request(
            (index * 2) as u64,
            "nodes.set",
            Some(json!({
                "sessionId": "soak-session",
                "node": node,
                "parameters": { "gainDb": ((index % 49) as f64) - 24.0 },
                "enabled": index % 7 != 0,
                "idempotencyKey": format!("edit-soak-{index}"),
            })),
        ));
        if let Some(error) = response.error {
            println!("refused_at={index} error={error:?}");
            refused = Some((index, error.message));
            break;
        }
        let status = plane.dispatch(request((index * 2 + 1) as u64, "status.get", None));
        assert!(
            status.error.is_none(),
            "status.get failed at edit {index}: {:?}",
            status.error
        );
        if [1_000, 2_000, 5_000, 10_000, 20_000].contains(&index) {
            sample(&plane, &path, index);
        }
    }
    if let Some((index, message)) = &refused {
        println!("journal refusal at edit {index}: {message}");
    } else {
        sample(&plane, &path, edits);
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        refused.is_none(),
        "a dispatch edit was refused at {refused:?}; the operation journal bound is too small for normal use"
    );
}
