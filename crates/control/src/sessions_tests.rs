//! Tests for `sessions.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn sessions_list_supports_stable_cursor_pages() {
    let mut plane = ControlPlane::default();
    for id in ["a", "b", "c"] {
        let mut value = session();
        value.id = EntityId::new(id);
        plane.insert_session(value).unwrap();
    }
    let first = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.list".into(),
            params: Some(json!({ "limit": 2 })),
        })
        .result
        .unwrap();
    assert_eq!(first["items"].as_array().unwrap().len(), 2);
    assert_eq!(first["items"][0]["id"], "a");
    assert_eq!(first["nextCursor"], "b");
    let second = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "sessions.list".into(),
            params: Some(json!({ "cursor": "b", "limit": 2 })),
        })
        .result
        .unwrap();
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert_eq!(second["items"][0]["id"], "c");
    assert!(second["nextCursor"].is_null());
}

#[test]
fn session_file_export_imports_on_another_database_without_replacing_sessions() {
    let root = std::env::temp_dir().join(format!("audiorouter-session-rpc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::create_dir_all(root.join("b")).unwrap();
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        })
    };
    let file = root.join("setup.audiorouter");
    let mut source =
        ControlPlane::with_storage("export", Storage::open(root.join("a/db.sqlite")).unwrap());
    source.create_session(session()).unwrap();
    let exported = call(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "session", "path": file }),
    );
    assert_eq!(exported.result.unwrap()["sessionId"], "session");
    let refused = call(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "session", "path": file }),
    );
    assert!(
        refused.error.unwrap().message.contains("already exists"),
        "never overwrites unasked"
    );
    let before = std::fs::metadata(&file).unwrap().len();
    let replaced = call(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "session", "path": file, "replace": true }),
    );
    assert!(replaced.error.is_none(), "{:?}", replaced.error);
    assert_eq!(std::fs::metadata(&file).unwrap().len(), before);
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        3,
        "no staged file left behind"
    );
    assert!(call(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "session", "path": root.join("a"), "replace": true })
    )
    .error
    .is_some());
    assert!(call(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "session", "path": root.join("x.txt") })
    )
    .error
    .is_some());
    assert!(call(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "session", "path": "relative.audiorouter" })
    )
    .error
    .is_some());

    let mut target =
        ControlPlane::with_storage("import", Storage::open(root.join("b/db.sqlite")).unwrap());
    let first = call(&mut target, "sessions.importFile", json!({ "path": file }))
        .result
        .unwrap();
    assert_eq!(first["session"]["id"], "session");
    assert_eq!(first["renamed"], false);
    let second = call(&mut target, "sessions.importFile", json!({ "path": file }))
        .result
        .unwrap();
    assert_eq!(second["session"]["id"], "session-imported-1");
    assert_eq!(second["session"]["name"], "test (imported)");
    assert_eq!(second["renamed"], true);
    assert_eq!(second["session"]["nodes"], first["session"]["nodes"]);
    drop((source, target));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn the_selected_session_survives_a_backend_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-active-session-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let shell = ClientGrant::for_desktop_shell();
    let call = |plane: &mut ControlPlane, method: &str, params: Value| {
        plane.dispatch_authorized_for_client(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            },
            "active-session-test",
            &shell,
        )
    };
    let session = |id: &str| json!({ "id": id, "name": id, "schemaVersion": 1, "revision": 0, "nodes": [], "edges": [] });
    {
        let mut plane = ControlPlane::with_storage("active-first", Storage::open(&path).unwrap());
        for id in ["a-first", "z-mine"] {
            let created = call(
                &mut plane,
                "sessions.create",
                json!({ "session": session(id), "idempotencyKey": format!("create-{id}") }),
            );
            assert!(created.error.is_none(), "{created:?}");
        }
        let selected = call(
            &mut plane,
            "sessions.active.set",
            json!({ "sessionId": "z-mine", "idempotencyKey": "select" }),
        );
        assert!(selected.error.is_none(), "{selected:?}");
    }
    {
        // Tray Play and autoplay at sign-in play this session, not the first one.
        let mut plane = ControlPlane::with_storage("active-second", Storage::open(&path).unwrap());
        assert_eq!(
            call(&mut plane, "sessions.active.get", json!({}))
                .result
                .unwrap()["sessionId"],
            "z-mine"
        );
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn session_create_and_delete_protect_running_resources() {
    let mut plane = ControlPlane::default();
    let mut created = session();
    created.id = EntityId::new("created");
    let result = plane.create_session(created.clone()).unwrap();
    assert_eq!(result["state"], "stopped");
    let duplicate = plane
        .duplicate_session(
            &created.id,
            EntityId::new("copy"),
            Some("Copied session".into()),
        )
        .unwrap();
    assert_eq!(duplicate["session"]["id"], "copy");
    assert_eq!(duplicate["session"]["name"], "Copied session");
    assert_eq!(duplicate["session"]["revision"], 0);
    assert!(matches!(
        plane.duplicate_session(&created.id, EntityId::new("copy"), None),
        Err(ControlError::InvalidRequest(message))
            if message == "duplicate session ID already exists"
    ));
    assert_eq!(plane.delete_session(&created.id).unwrap()["deleted"], true);
    assert_eq!(
        plane.delete_session(&EntityId::new("copy")).unwrap()["deleted"],
        true
    );
    assert!(matches!(
        plane.delete_session(&created.id),
        Err(ControlError::InvalidRequest(message)) if message == "session not found"
    ));

    let mut running = session();
    running.id = EntityId::new("running");
    plane.create_session(running.clone()).unwrap();
    plane.session_start(&running.id).unwrap();
    assert!(matches!(
        plane.delete_session(&running.id),
        Err(ControlError::InvalidRequest(message)) if message == "stop the session before deleting it"
    ));
    plane.session_stop(&running.id).unwrap();
    assert_eq!(plane.delete_session(&running.id).unwrap()["deleted"], true);
}

#[test]
fn fake_session_lifecycle_is_idempotent_and_stoppable() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let first = plane.session_start(&original.id).unwrap();
    assert_eq!(first["state"], "running");
    assert_eq!(first["generation"], 1);
    let replay = plane.session_start(&original.id).unwrap();
    assert_eq!(replay["generation"], 1);
    assert_eq!(replay["runtime"], "fake");
    assert_eq!(
        plane.session_stop(&original.id).unwrap()["state"],
        "stopped"
    );
    let events = plane.events.since(0, 10).unwrap();
    assert_eq!(events.last().unwrap().resource_revision, original.revision);
    assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
}

#[test]
fn session_runtime_label_distinguishes_native_attachment() {
    assert_eq!(session_runtime_label(false), "fake");
    assert_eq!(session_runtime_label(true), "native");
}

#[test]
fn session_stop_uses_attached_worker_and_reports_finalization() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let mut recorder = RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(128).unwrap();
    plane.recorders.insert(original.id.clone(), recorder);
    plane
        .attach_recorder_worker(original.id.clone(), Box::new(TestRecorderWorker))
        .unwrap();

    let result = plane.session_stop(&original.id).unwrap();
    assert_eq!(result["state"], "stopped");
    assert_eq!(result["recorders"][0]["state"], "completed");
    assert_eq!(result["recorders"][0]["fileFinalized"], true);
    assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Stopped);
    assert_eq!(
        plane.recorders[&original.id].state(),
        RecorderState::Completed
    );
    assert!(!plane.recorder_workers.contains_key(&original.id));
}

#[test]
fn draft_preview_requires_prepared_native_route_and_does_not_save_candidate() {
    let mut plane = ControlPlane::default();
    let saved = session();
    plane.insert_session(saved.clone()).unwrap();
    let mut candidate = saved.clone();
    candidate.name = "temporary audition".into();
    let error = plane
        .session_preview_start(&saved.id, &candidate)
        .unwrap_err();
    let ControlError::InvalidRequest(message) = error else {
        panic!("expected native preview preparation guidance");
    };
    assert!(message.contains("prepared single-endpoint audio route"));
    assert_eq!(plane.get_session(&saved.id).unwrap(), &saved);
    assert!(!plane
        .runtimes
        .get(&saved.id)
        .is_some_and(|runtime| runtime.state() == RuntimeState::Running));

    let request = json!({
        "sessionId": saved.id,
        "candidate": candidate,
        "idempotencyKey": "preview-contract"
    });
    assert!(validate_method_params("session.start", Some(&request)).is_ok());
}

#[test]
fn meter_reset_requires_session_control_and_a_prepared_meter() {
    let mut plane = ControlPlane::default();
    let mut saved = session();
    saved.nodes[1].kind = NodeKind::Meter;
    let node_id = saved.nodes[1].id.clone();
    let id = saved.id.clone();
    plane.insert_session(saved.clone()).unwrap();
    let request = || JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "meters.reset".into(),
        params: Some(json!({"sessionId":id,"nodeId":node_id})),
    };
    let read_only = ClientGrant::read_only();
    assert_eq!(
        plane
            .dispatch_authorized(request(), &read_only)
            .error
            .unwrap()
            .code,
        -32001
    );
    assert!(plane
        .dispatch(request())
        .error
        .unwrap()
        .message
        .contains("not prepared"));
    assert_eq!(plane.get_session(&id).unwrap(), &saved);
    assert!(validate_method_params(
        "meters.reset",
        Some(&json!({"sessionId":id,"nodeId":node_id,"unexpected":true}))
    )
    .is_err());
}

#[test]
fn keyed_session_lifecycle_replays_and_conflicts_durably() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("lifecycle-idempotency", storage);
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let start = || JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.start".into(),
        params: Some(json!({ "sessionId": "session", "idempotencyKey": "start-1" })),
    };
    let first = plane.dispatch(start()).result.unwrap();
    let replay = plane.dispatch(start()).result.unwrap();
    assert_eq!(first, replay);
    let mut other = session();
    other.id = EntityId::new("other");
    plane.insert_session(other).unwrap();
    let same_key = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "sessions.start".into(),
        params: Some(json!({ "sessionId": "other", "idempotencyKey": "start-1" })),
    });
    assert!(same_key.error.is_some());
    let stop = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "sessions.stop".into(),
        params: Some(json!({ "sessionId": "session", "idempotencyKey": "stop-1" })),
    });
    assert_eq!(stop.result.as_ref().unwrap()["state"], "stopped");
    let stop_replay = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "sessions.stop".into(),
        params: Some(json!({ "sessionId": "session", "idempotencyKey": "stop-1" })),
    });
    assert_eq!(stop.result.unwrap(), stop_replay.result.unwrap());
}

#[test]
fn commit_reactivates_a_running_fake_session_as_one_generation() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 1);
    let mut candidate = original.clone();
    candidate.name = "live-edit".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    let result = plane.commit_graph(&plan, 0, "live-edit-op").unwrap();
    assert_eq!(result["activation"]["state"], "running");
    assert_eq!(result["activation"]["generation"], 2);
    assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
}

#[test]
fn session_lifecycle_requires_an_existing_session() {
    let mut plane = ControlPlane::default();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "session.start".into(),
        params: Some(json!({ "sessionId": "missing" })),
    };
    assert_eq!(plane.dispatch(request).error.unwrap().code, -32602);
}

#[test]
fn session_start_enforces_the_two_active_session_limit() {
    let mut plane = ControlPlane::default();
    for id in ["one", "two", "three"] {
        let mut graph = session();
        graph.id = EntityId::new(id);
        plane.insert_session(graph).unwrap();
    }
    plane.session_start(&EntityId::new("one")).unwrap();
    plane.session_start(&EntityId::new("two")).unwrap();
    assert!(matches!(
        plane.session_start(&EntityId::new("three")),
        Err(ControlError::InvalidRequest(message)) if message == "active session limit reached"
    ));
}

#[test]
fn draft_preview_requires_graph_write_in_addition_to_session_control() {
    let saved = session();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(94)),
        method: "session.start".into(),
        params: Some(json!({
            "sessionId": saved.id,
            "candidate": saved,
            "idempotencyKey": "preview-permission"
        })),
    };
    let denied = ControlPlane::default().dispatch_authorized(
        request,
        &ClientGrant::with_scopes([PermissionScope::SessionControl]),
    );
    let error = denied.error.unwrap();
    assert_eq!(error.code, -32001);
    assert_eq!(error.data.unwrap()["code"], "permissionDenied");
}

#[test]
fn session_import_plan_and_commit_are_validated_and_idempotent() {
    let mut plane = ControlPlane::default();
    let mut imported = session();
    imported.id = EntityId::new("imported-session");
    let duplicate_candidate = imported.clone();
    let planned = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "sessions.importPlan".into(),
        params: Some(json!({"session": imported})),
    });
    let plan = planned.result.unwrap();
    assert_eq!(plan["session"]["id"], "imported-session");
    let commit = |id| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(id)),
        method: "sessions.importCommit".into(),
        params: Some(json!({
            "planId": plan["planId"],
            "idempotencyKey": "import-once"
        })),
    };
    let first = plane.dispatch(commit(2));
    assert_eq!(first.result.as_ref().unwrap()["state"], "stopped");
    let replay = plane.dispatch(commit(3));
    assert_eq!(replay.result.unwrap(), first.result.unwrap());
    let duplicate = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "sessions.importPlan".into(),
        params: Some(json!({"session": duplicate_candidate})),
    });
    assert!(duplicate.error.is_some());
}
