//! Tests for `plugins.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn plugin_worker_resolves_from_the_packaged_resource_directory() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-worker-resource-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let resources = root.join("resources");
    std::fs::create_dir_all(&resources).unwrap();
    let shell = root.join(if cfg!(windows) {
        "audiorouter-shell.exe"
    } else {
        "audiorouter-shell"
    });
    let worker_name = if cfg!(windows) {
        "audiorouter-plugin-worker.exe"
    } else {
        "audiorouter-plugin-worker"
    };
    let worker = resources.join(worker_name);
    std::fs::write(&worker, b"test executable placeholder").unwrap();
    assert_eq!(packaged_plugin_worker_path(&shell), Some(worker.clone()));
    let adjacent = root.join(worker_name);
    std::fs::write(&adjacent, b"adjacent executable placeholder").unwrap();
    assert_eq!(packaged_plugin_worker_path(&shell), Some(adjacent));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn missing_plugin_worker_error_names_search_paths_and_repair_action() {
    let root = std::env::temp_dir().join("audiorouter-worker-error");
    let shell = root.join(if cfg!(windows) {
        "audiorouter-shell.exe"
    } else {
        "audiorouter-shell"
    });
    let message = plugin_worker_unavailable_message(Some(&shell));
    let worker = if cfg!(windows) {
        "audiorouter-plugin-worker.exe"
    } else {
        "audiorouter-plugin-worker"
    };
    assert!(message.contains(&root.join(worker).display().to_string()));
    assert!(message.contains(&root.join("resources").join(worker).display().to_string()));
    assert!(message.contains("rebuild the complete AudioRouter package"));
    assert!(message.contains("AUDIOROUTER_PLUGIN_WORKER_PATH"));
}

/// A plugin setting changed while playing (as in its editor) must come
/// back after Stop and Play. Same environment as the test above; the
/// session must contain a plugin node `AUDIOROUTER_LIVE_PLUGIN_NODE`
/// (default `reaeq`). Privacy mute keeps it silent.
#[cfg(windows)]
#[test]
#[ignore = "live Windows audio devices and plugins"]
fn live_plugin_settings_survive_stop_and_play() {
    let Some(database) = std::env::var_os("AUDIOROUTER_LIVE_PATHS_DATABASE") else {
        return;
    };
    let session_id = std::env::var("AUDIOROUTER_LIVE_PATHS_SESSION")
        .unwrap_or_else(|_| "patrick-main-session".into());
    let node_id = EntityId::new(
        std::env::var("AUDIOROUTER_LIVE_PLUGIN_NODE").unwrap_or_else(|_| "reaeq".into()),
    );
    let storage = audiorouter_storage::Storage::open(std::path::Path::new(&database)).unwrap();
    let mut plane = ControlPlane::with_storage("live-plugin-state", storage);
    let run_id = format!(
        "live-plugin-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: (!params.is_null()).then_some(params),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    let play = |plane: &mut ControlPlane, run: &str| {
        call(
            plane,
            "nativePaths.prepare",
            json!({ "sessionId": session_id }),
        );
        let started = call(
            plane,
            "session.start",
            json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-{run}-start") }),
        );
        let generation = started["generation"].as_u64().unwrap();
        let until = Instant::now() + Duration::from_millis(800);
        while Instant::now() < until {
            call(
                plane,
                "nativeMultiInputs.pump",
                json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        generation
    };
    call(
        &mut plane,
        "safety.setPrivacyMute",
        json!({ "muted": true, "idempotencyKey": format!("{run_id}-mute") }),
    );
    let generation = play(&mut plane, "first");
    let session = EntityId::new(&session_id);
    let bridge = plane.plugin_bridge(&session, &node_id).unwrap();
    let before = bridge.save_state().unwrap();
    // Change settings the way an editor does: on the running instance only.
    // The values differ on every run, so a restored earlier run's values
    // never turn the change into a no-op.
    let seed = 0.1
        + std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_millis() as f32
            / 1000.0
            * 0.6;
    bridge
        .set_parameters(
            (0..4)
                .map(|parameter_id| audiorouter_plugin_host::ParameterEvent {
                    parameter_id,
                    normalized_value: (seed + parameter_id as f32 * 0.05).min(0.95),
                    sample_offset: 0,
                })
                .collect(),
        )
        .unwrap();
    let until = Instant::now() + Duration::from_millis(500);
    while Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
        call(
            &mut plane,
            "nativeMultiInputs.pump",
            json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
        );
    }
    let changed = bridge.save_state().unwrap();
    assert_ne!(
        changed.bytes, before.bytes,
        "the test must actually change the plugin's state"
    );
    drop(bridge);
    call(
        &mut plane,
        "session.stop",
        json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-first-stop") }),
    );
    call(
        &mut plane,
        "nativeEndpoints.detach",
        json!({ "sessionId": session_id }),
    );
    play(&mut plane, "second");
    let restored = plane
        .plugin_bridge(&session, &node_id)
        .unwrap()
        .save_state()
        .unwrap();
    call(
        &mut plane,
        "session.stop",
        json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-second-stop") }),
    );
    // Some plugins re-round a stored double when reloading (ReaEQ moves
    // one frequency by its last bit), so allow a byte or two of drift but
    // require the restore to match the change, not the original state.
    let differing = |left: &[u8], right: &[u8]| {
        left.iter().zip(right).filter(|(a, b)| a != b).count() + left.len().abs_diff(right.len())
    };
    assert!(
        differing(&restored.bytes, &changed.bytes) <= 2
            && differing(&restored.bytes, &before.bytes)
                > differing(&restored.bytes, &changed.bytes),
        "Stop then Play must restore the plugin's settings: restored {:?}, changed {:?}",
        restored.bytes,
        changed.bytes
    );
    eprintln!(
        "verified {} settings survive Stop and Play ({} state bytes)",
        node_id.as_str(),
        restored.bytes.len()
    );
}

#[test]
fn plugin_scan_is_read_only_and_keeps_invalid_candidates_visible() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-plugin-scan-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("candidate.dll"), b"not a PE binary").unwrap();
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.scan".into(),
        params: Some(json!({ "directory": root.to_string_lossy() })),
    };
    let denied = plane.dispatch_authorized(request.clone(), &ClientGrant::read_only());
    assert_eq!(
        denied.error.unwrap().data.unwrap()["code"],
        "permissionDenied"
    );
    let response = plane.dispatch_authorized(
        request,
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(result["directory"], root.to_string_lossy().to_string());
    assert_eq!(result["entries"].as_array().unwrap().len(), 1);
    assert!(result["entries"][0]["identity"].is_null());
    assert!(result["entries"][0]["error"].is_string());
    assert_eq!(result["entries"][0]["errorCode"], "notPe");
    let listed = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "plugins.list".into(),
            params: Some(json!({ "directory": root.to_string_lossy() })),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(listed.result.unwrap(), result);
    let retry_request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "plugins.retry".into(),
        params: Some(json!({
            "directory": root.to_string_lossy(),
            "idempotencyKey": "plugin-retry"
        })),
    };
    let retried = plane.dispatch_authorized(
        retry_request.clone(),
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(
        retried.result.as_ref().unwrap()["entries"],
        result["entries"]
    );
    let replayed = plane.dispatch_authorized(
        retry_request,
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(replayed.result.unwrap(), retried.result.unwrap());
    let inspected = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "plugins.inspect".into(),
            params: Some(json!({ "path": root.join("candidate.dll").to_string_lossy() })),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert!(inspected.error.is_none());
    let inspected_result = inspected.result.unwrap();
    assert!(inspected_result["identity"].is_null());
    assert_eq!(inspected_result["errorCode"], "notPe");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn plugin_placeholder_plan_requires_current_scan_evidence() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.nodes.push(Node {
        id: EntityId::new("plugin"),
        kind: NodeKind::Plugin,
        type_version: 1,
        name: "Unbound plugin".into(),
        enabled: false,
        bypass: false,
        parameters: [
            ("path".into(), json!("C:\\Plugins\\effect.dll")),
            ("format".into(), json!("vst2")),
            (
                "fingerprint".into(),
                json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
            ),
            ("classId".into(), json!("effect-class")),
        ]
        .into_iter()
        .collect(),
        ports: vec![
            Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 1,
            },
            Port {
                name: "out".into(),
                direction: PortDirection::Output,
                channels: 1,
            },
        ],
    });
    let error = plane
        .plan_graph(&original.id, original.revision, candidate)
        .unwrap_err();
    assert!(matches!(
        error,
        ControlError::InvalidRequest(message)
            if message.contains("current explicit scan")
    ));
}

#[test]
fn plugin_inventory_cache_evicts_oldest_scan_roots() {
    let mut plane = ControlPlane::default();
    for index in 0..=MAX_PLUGIN_INVENTORY_ROOTS {
        plane.remember_plugin_inventory(
            format!("C:\\plugin-root-{index}"),
            json!({ "directory": format!("C:\\plugin-root-{index}"), "entries": [] }),
        );
    }

    assert_eq!(plane.plugin_inventories.len(), MAX_PLUGIN_INVENTORY_ROOTS);
    assert!(!plane.plugin_inventories.contains_key("C:\\plugin-root-0"));
    assert!(plane
        .plugin_inventories
        .contains_key(&format!("C:\\plugin-root-{MAX_PLUGIN_INVENTORY_ROOTS}")));
    assert_eq!(
        plane.plugin_inventory_order.len(),
        MAX_PLUGIN_INVENTORY_ROOTS
    );
}

#[test]
fn plugin_scan_directory_failures_have_stable_application_codes() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-plugin-scan-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let response = ControlPlane::default().dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "plugins.scan".into(),
            params: Some(json!({ "directory": root.to_string_lossy() })),
        },
        &ClientGrant::with_scopes([PermissionScope::PluginScan]),
    );
    assert_eq!(response.error.unwrap().data.unwrap()["code"], "invalidRoot");
}

#[test]
fn plugin_inspect_discovery_has_typed_identity_schema() {
    let method = ControlPlane::default().describe()["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|method| method["name"] == "plugins.inspect")
        .unwrap()
        .clone();
    assert_eq!(method["permission"], "pluginScan");
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["type"][1],
        "null"
    );
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["required"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["properties"]["fileBytes"]["maximum"],
        audiorouter_plugin_host::MAX_PLUGIN_BYTES
    );
    assert_eq!(
        method["outputSchema"]["properties"]["identity"]["properties"]["classIds"]["maxItems"],
        256
    );
    assert!(method["outputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "errorCode"));
    assert_eq!(
        method["outputSchema"]["properties"]["errorCode"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
}

#[test]
fn unprepared_plugin_start_explains_http_preparation_for_both_aliases() {
    for method in ["session.start", "sessions.start"] {
        let mut plane = ControlPlane::default();
        let mut saved = session();
        saved.nodes[0].kind = NodeKind::Plugin;
        saved.nodes[0].enabled = true;
        let id = saved.id.clone();
        plane.insert_session(saved.clone()).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(json!({ "sessionId": id, "idempotencyKey": "unprepared-start" })),
        });
        let error = response.error.expect("unprepared plugins must not start");
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("POST /api/v1/nativePaths/prepare"));
        assert!(error.message.contains("DeviceAdministration"));
        assert_eq!(plane.get_session(&id).unwrap(), &saved);
        assert!(!plane
            .runtimes
            .get(&id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running));
    }
}
