//! Tests for `persistence.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn timestamped_plan_id_allocator_skips_collisions_and_bounds_exhaustion() {
    for prefix in ["startup-plan", "virtual-plan"] {
        let occupied = EntityId::new(format!("{prefix}-123-1"));
        let mut counter = 1;
        let allocated = allocate_timestamped_plan_id(
            prefix,
            123,
            &mut counter,
            |id| id == &occupied,
            "plan IDs exhausted",
        )
        .unwrap();
        assert_eq!(allocated.as_str(), format!("{prefix}-123-2"));
        assert_eq!(counter, 3);

        counter = u64::MAX;
        assert!(matches!(
            allocate_timestamped_plan_id(
                prefix,
                123,
                &mut counter,
                |_| true,
                "plan IDs exhausted",
            ),
            Err(ControlError::InvalidRequest(message)) if message == "plan IDs exhausted"
        ));
        assert_eq!(counter, u64::MAX);
    }
}

#[test]
fn counter_plan_id_allocator_skips_collisions_and_bounds_exhaustion() {
    let occupied = EntityId::new("session-import-1");
    let mut counter = 1;
    let allocated = allocate_counter_plan_id(
        "session-import",
        &mut counter,
        |id| id == &occupied,
        "plan IDs exhausted",
    )
    .unwrap();
    assert_eq!(allocated.as_str(), "session-import-2");
    assert_eq!(counter, 3);

    counter = u64::MAX;
    assert!(matches!(
        allocate_counter_plan_id(
            "session-import",
            &mut counter,
            |_| true,
            "plan IDs exhausted",
        ),
        Err(ControlError::InvalidRequest(message)) if message == "plan IDs exhausted"
    ));
    assert_eq!(counter, u64::MAX);
}

#[test]
fn persisted_plan_duration_rejects_expired_and_caps_far_future_values() {
    let maximum = Duration::from_secs(300);
    assert_eq!(remaining_persisted_plan_duration(99, 100, maximum), None);
    assert_eq!(remaining_persisted_plan_duration(100, 100, maximum), None);
    assert_eq!(
        remaining_persisted_plan_duration(101, 100, maximum),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        remaining_persisted_plan_duration(i64::MAX, 0, maximum),
        Some(maximum)
    );
    assert_eq!(
        remaining_persisted_plan_duration(i64::MIN, i64::MAX, maximum),
        None
    );
}

#[test]
fn storage_startup_restores_all_bounded_sessions() {
    let storage = Storage::open_memory().unwrap();
    for index in 0..audiorouter_domain::MAX_SESSIONS_GLOBAL {
        let mut value = session();
        value.id = EntityId::new(format!("session-{index:03}"));
        value.nodes.clear();
        value.edges.clear();
        storage.save_session(&value).unwrap();
    }

    let plane = ControlPlane::try_with_storage("paged-startup", storage).unwrap();
    let result = plane.sessions_list_page(None, 500).unwrap();
    assert_eq!(
        result["items"].as_array().unwrap().len(),
        audiorouter_domain::MAX_SESSIONS_GLOBAL
    );
    assert!(result["nextCursor"].is_null());
}

#[test]
fn ephemeral_plan_maps_bound_pending_entries_and_prune_expired_entries() {
    let mut plane = ControlPlane::default();

    for index in 0..MAX_PENDING_PLAN_RECORDS {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(index as u64)),
            method: "startup.plan".into(),
            params: Some(json!({ "enabled": true })),
        });
        assert!(response.result.is_some(), "startup plan {index} failed");
    }
    let startup_overflow = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(MAX_PENDING_PLAN_RECORDS as u64)),
        method: "startup.plan".into(),
        params: Some(json!({ "enabled": true })),
    });
    assert!(startup_overflow
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("too many pending startup plans")));

    for index in 0..MAX_PENDING_PLAN_RECORDS {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!((1000 + index) as u64)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": {
                    "action": "create",
                    "id": "bus-pending",
                    "name": "Pending"
                }
            })),
        });
        assert!(response.result.is_some(), "virtual plan {index} failed");
    }
    let virtual_overflow = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2000)),
        method: "virtualDevices.plan".into(),
        params: Some(json!({
            "operation": {
                "action": "create",
                "id": "bus-pending",
                "name": "Pending"
            }
        })),
    });
    assert!(virtual_overflow.error.as_ref().is_some_and(|error| error
        .message
        .contains("too many pending virtual-device plans")));

    for index in 0..MAX_PENDING_PLAN_RECORDS {
        let mut imported = session();
        imported.id = EntityId::new(format!("import-{index}"));
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!((3000 + index) as u64)),
            method: "sessions.importPlan".into(),
            params: Some(json!({ "session": imported })),
        });
        assert!(response.result.is_some(), "import plan {index} failed");
    }
    let import_overflow = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4000)),
        method: "sessions.importPlan".into(),
        params: Some(json!({ "session": session() })),
    });
    assert!(import_overflow.error.as_ref().is_some_and(|error| error
        .message
        .contains("too many pending session import plans")));

    let existing_startup = plane.startup_plans.keys().next().cloned().unwrap();
    plane.startup_plans.remove(&existing_startup);
    plane.startup_plans.insert(
        EntityId::new("expired-startup"),
        (true, Instant::now() - Duration::from_secs(1)),
    );
    let recovered = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4001)),
        method: "startup.plan".into(),
        params: Some(json!({ "enabled": false })),
    });
    assert!(recovered.result.is_some());
}

#[test]
fn graph_plan_persistence_failure_rolls_back_the_in_memory_plan() {
    let storage = Storage::open_memory().unwrap();
    let original = session();
    let mut plane = ControlPlane::with_storage("plan-rollback", storage);
    plane.insert_session(original.clone()).unwrap();
    for index in 0..audiorouter_domain::MAX_PENDING_GRAPH_PLANS {
        plane
            .storage
            .as_ref()
            .unwrap()
            .save_graph_plan(&GraphPlanRecord {
                id: format!("filled-plan-{index}"),
                session_id: original.id.as_str().into(),
                base_revision: 0,
                candidate: original.clone(),
                expires_at: i64::MAX,
            })
            .unwrap();
    }
    let mut candidate = original.clone();
    candidate.name = "must-not-survive".into();

    let result = plane.plan_graph(&original.id, 0, candidate);
    assert!(
        matches!(result, Err(ControlError::InvalidRequest(_))),
        "{result:?}"
    );
    assert!(matches!(
        plane.commit_graph(&EntityId::new("plan-2"), 0, "rollback-check"),
        Err(ControlError::Store(
            audiorouter_domain::StoreError::PlanNotFound
        ))
    ));
}

#[test]
fn storage_backed_control_persists_session_and_commit() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("persistent-test", storage);
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "persisted-change".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    let result = plane.commit_graph(&plan, 0, "persist-op").unwrap();
    assert_eq!(plane.get_session(&original.id).unwrap().revision, 1);
    assert!(result["revision"] == 1);
}

#[test]
fn operations_get_returns_durable_commit_outcome() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("operation-test", storage);
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "operation-change".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "operation-id").unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(13)),
        method: "operations.get".into(),
        params: Some(json!({ "operationId": "operation-id" })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["status"], "completed");
    assert_eq!(result["durable"], true);
    assert_eq!(result["revision"], 1);
    assert_eq!(result["result"]["revision"], 1);
}

#[test]
fn operations_get_returns_live_memory_outcome_without_claiming_durability() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "memory-operation".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "memory-operation-id").unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(14)),
        method: "operations.get".into(),
        params: Some(json!({ "operationId": "memory-operation-id" })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["status"], "completed");
    assert_eq!(result["durable"], false);
    assert_eq!(result["result"]["revision"], 1);
}

#[test]
fn memory_operation_retention_evicts_in_insertion_order() {
    let mut plane = ControlPlane::default();
    for index in 0..=MAX_MEMORY_OPERATION_OUTCOMES {
        plane.remember_operation_outcome(
            &format!("operation-{index}"),
            json!({ "index": index }),
            "test.operation",
            None,
        );
    }
    assert!(!plane.operation_outcomes.contains_key("operation-0"));
    assert!(plane.operation_outcomes.contains_key("operation-1"));
    assert!(plane
        .operation_outcomes
        .contains_key(&format!("operation-{MAX_MEMORY_OPERATION_OUTCOMES}")));
    assert_eq!(plane.operation_order.len(), MAX_MEMORY_OPERATION_OUTCOMES);
}

#[test]
fn operations_cancel_reports_completed_operations_without_undoing_them() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "cancel-check".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "cancel-check-key").unwrap();
    let missing_key = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(15)),
        method: "operations.cancel".into(),
        params: Some(json!({ "operationId": "cancel-check-key" })),
    });
    assert_eq!(missing_key.error.unwrap().code, -32602);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(16)),
        method: "operations.cancel".into(),
        params: Some(json!({
            "operationId": "cancel-check-key",
            "idempotencyKey": "cancel-request-key"
        })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["status"], "completed");
    assert_eq!(result["cancelled"], false);
    assert_eq!(result["reason"], "alreadyCompleted");
    assert_eq!(plane.get_session(&original.id).unwrap().revision, 1);
}

#[test]
fn durable_commit_replays_before_plan_lookup_after_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-idempotency-restart-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let original = session();
    let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
    first.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "durable-change".into();
    let plan = first.plan_graph(&original.id, 0, candidate).unwrap();
    first.commit_graph(&plan, 0, "restart-key").unwrap();
    drop(first);

    let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
    let replay = second.commit_graph(&plan, 0, "restart-key").unwrap();
    assert_eq!(replay["idempotentReplay"], true);
    assert_eq!(replay["revision"], 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn durable_uncommitted_plan_survives_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-plan-restart-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let original = session();
    let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
    first.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "survives-restart".into();
    let plan = first.plan_graph(&original.id, 0, candidate).unwrap();
    drop(first);

    let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
    let committed = second.commit_graph(&plan, 0, "restart-plan-key").unwrap();
    assert_eq!(committed["revision"], 1);
    assert_eq!(committed["idempotentReplay"], false);
    assert_eq!(
        second
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(15)),
                method: "sessions.get".into(),
                params: Some(json!({ "sessionId": "session" })),
            })
            .result
            .unwrap()["name"],
        "survives-restart"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn restart_does_not_reuse_a_durable_graph_plan_id() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-plan-id-restart-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let original = session();
    let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
    first.insert_session(original.clone()).unwrap();
    let first_plan = first.plan_graph(&original.id, 0, original.clone()).unwrap();
    assert_eq!(first_plan.as_str(), "plan-1");
    drop(first);

    let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
    let mut candidate = original.clone();
    candidate.name = "second-plan".into();
    let second_plan = second.plan_graph(&original.id, 0, candidate).unwrap();
    assert_eq!(second_plan.as_str(), "plan-2");
    let storage = second.storage.as_ref().unwrap();
    assert_eq!(
        storage
            .load_graph_plan("plan-1")
            .unwrap()
            .unwrap()
            .candidate
            .name,
        "test"
    );
    assert_eq!(
        storage
            .load_graph_plan("plan-2")
            .unwrap()
            .unwrap()
            .candidate
            .name,
        "second-plan"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn corrupt_database_error_has_non_retryable_recovery_code() {
    let response = application_error_response(
        Some(json!(1)),
        ControlError::CorruptDatabase("integrity check failed".into()),
    );
    let error = response.error.unwrap();
    let data = error.data.unwrap();
    assert_eq!(data["code"], "corruptDatabase");
    assert_eq!(data["retryable"], false);
}

#[test]
fn storage_error_mapping_does_not_leak_os_paths() {
    let mapped = storage_error(StorageError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        r"C:\private\recordings\secret.wav",
    )));
    assert_eq!(
        mapped,
        ControlError::Storage("storage I/O operation failed".into())
    );
    let response = application_error_response(Some(json!(1)), mapped);
    let message = response.error.unwrap().message;
    assert!(!message.contains("secret.wav"));
    assert!(!message.contains("C:\\private"));
}
