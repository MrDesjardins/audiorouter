//! Tests for `safety.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn sleep_stops_fake_sessions_and_resume_requires_revalidation() {
    let mut plane = ControlPlane::default();
    let mut value = session();
    value.id = EntityId::new("sleep-route");
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();

    let stopped = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("sleep")),
            method: "system.osTransition".into(),
            params: Some(json!({
                "transition": "sleep",
                "idempotencyKey": "os-sleep-1"
            })),
        })
        .result
        .unwrap();
    assert_eq!(stopped["action"], "stopAndRelease");
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([])
    );

    let resumed = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("resume")),
            method: "system.osTransition".into(),
            params: Some(json!({
                "transition": "resume",
                "idempotencyKey": "os-resume-1"
            })),
        })
        .result
        .unwrap();
    assert_eq!(resumed["action"], "revalidateBeforeRestart");
    assert_eq!(resumed["sessionIds"], json!(["sleep-route"]));
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([])
    );
}

#[test]
fn os_transition_idempotency_replays_and_rejects_key_reuse() {
    let mut plane = ControlPlane::default();
    let mut value = session();
    value.id = EntityId::new("idempotent-os-transition");
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();

    let request = |transition: &str| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(transition)),
        method: "system.osTransition".into(),
        params: Some(json!({
            "transition": transition,
            "idempotencyKey": "os-transition-replay"
        })),
    };
    let first = plane.dispatch(request("sleep"));
    let first_result = first.result.clone().expect("initial transition succeeds");
    let replay = plane.dispatch(request("sleep"));
    assert_eq!(replay.result, Some(first_result));
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([])
    );

    let conflict = plane.dispatch(request("resume"));
    assert_eq!(
        conflict.error.as_ref().map(|error| error.code),
        Some(-32602)
    );
    assert!(conflict
        .error
        .as_ref()
        .is_some_and(|error| error.message.contains("idempotency")));
}

#[test]
fn sleep_preserves_native_identity_for_explicit_resume_revalidation() {
    let mut plane = ControlPlane::default();
    let value = session();
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();
    // A marker is sufficient to model a native-owned route in this
    // portable test; no endpoint or driver is opened.
    plane.native_endpoint_session = Some(session_id.clone());

    plane
        .handle_os_transition(os_transition::OsTransition::Sleep)
        .unwrap();
    let resumed = plane
        .handle_os_transition(os_transition::OsTransition::Resume)
        .unwrap();
    assert_eq!(resumed["action"], "revalidateBeforeRestart");
    assert_eq!(resumed["nativeSessionIds"], json!([session_id]));
    assert_eq!(resumed["sessionIds"], json!([]));
}

#[test]
fn runtime_crash_recovery_restarts_only_eligible_fake_sessions() {
    let mut plane = ControlPlane::default();
    let mut running = session();
    running.id = EntityId::new("running");
    let mut recording = session();
    recording.id = EntityId::new("recording");
    plane.insert_session(running).unwrap();
    plane.insert_session(recording).unwrap();
    plane.session_start(&EntityId::new("running")).unwrap();
    plane.session_start(&EntityId::new("recording")).unwrap();

    let recorder = RecorderController::new();
    plane.recorders.insert(EntityId::new("recording"), recorder);
    plane
        .recorders
        .get_mut(&EntityId::new("recording"))
        .unwrap()
        .arm()
        .unwrap();
    plane
        .recorders
        .get_mut(&EntityId::new("recording"))
        .unwrap()
        .start(0)
        .unwrap();

    let decision = plane.recover_after_runtime_crash(100).unwrap();
    assert_eq!(decision.mode, RecoveryMode::RestoreEligible);
    assert_eq!(decision.session_ids, vec![EntityId::new("running")]);
    let status = plane.status_snapshot().unwrap();
    assert_eq!(status["activeSessionIds"], json!(["running"]));
    let events = plane.events.since(0, 500).unwrap();
    let categories = events
        .iter()
        .map(|event| event.category.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        &categories[2..],
        [
            "runtime.started",
            "runtime.started",
            "runtime.crashed",
            "runtime.crashed",
            "runtime.started"
        ]
    );
}

#[test]
fn runtime_crash_recovery_does_not_restart_native_owned_sessions() {
    let mut plane = ControlPlane::default();
    let mut value = session();
    value.id = EntityId::new("native-owned");
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();

    // A native worker is not constructible in this portable test, but the
    // ownership marker is enough to model the supervisor handoff. A stale
    // marker must fail closed too: recovery must not restart a route that
    // could still require native endpoint ownership.
    plane.native_endpoint_session = Some(session_id.clone());
    let decision = plane.recover_after_runtime_crash(100).unwrap();

    assert_eq!(decision.mode, RecoveryMode::RestoreEligible);
    assert!(decision.session_ids.is_empty());
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([])
    );
    assert!(plane.native_endpoint_session.is_none());
}

#[test]
fn runtime_crash_recovery_enters_safe_mode_without_restarting_sessions() {
    let mut plane = ControlPlane::default();
    let value = session();
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();
    plane.recover_after_runtime_crash(100).unwrap();
    plane.session_start(&session_id).unwrap();
    plane.recover_after_runtime_crash(101).unwrap();
    plane.session_start(&session_id).unwrap();

    let decision = plane.recover_after_runtime_crash(102).unwrap();
    assert_eq!(decision.mode, RecoveryMode::SafeMode);
    assert!(decision.session_ids.is_empty());
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([])
    );
}

#[test]
fn durable_runtime_crash_recovery_applies_policy_and_keeps_safe_mode_latched() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("durable-recovery", storage);
    let value = session();
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();

    let first = plane.recover_after_runtime_crash(100).unwrap();
    assert_eq!(first.mode, RecoveryMode::RestoreEligible);
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([session_id.as_str()])
    );
    plane.recover_after_runtime_crash(101).unwrap();
    plane.session_start(&session_id).unwrap();
    let third = plane.recover_after_runtime_crash(102).unwrap();
    assert_eq!(third.mode, RecoveryMode::SafeMode);
    assert_eq!(
        plane.status_snapshot().unwrap()["recovery"]["safeMode"],
        true
    );
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([])
    );
}

#[test]
fn runtime_crash_recovery_candidates_are_sorted() {
    let mut plane = ControlPlane::default();
    let mut first = session();
    first.id = EntityId::new("z-session");
    let mut second = session();
    second.id = EntityId::new("a-session");
    plane.insert_session(first).unwrap();
    plane.insert_session(second).unwrap();
    plane.session_start(&EntityId::new("z-session")).unwrap();
    plane.session_start(&EntityId::new("a-session")).unwrap();

    let decision = plane.record_runtime_crash(100).unwrap();
    assert_eq!(
        decision.session_ids,
        vec![EntityId::new("a-session"), EntityId::new("z-session")]
    );
}

#[test]
fn clearing_memory_safe_mode_resets_the_crash_tracker() {
    let mut plane = ControlPlane::default();
    plane.record_runtime_crash(100).unwrap();
    plane.record_runtime_crash(101).unwrap();
    assert_eq!(
        plane.record_runtime_crash(102).unwrap().mode,
        RecoveryMode::SafeMode
    );

    let cleared = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(88)),
        method: "recovery.clearSafeMode".into(),
        params: Some(json!({ "idempotencyKey": "recovery-clear-1" })),
    });
    assert_eq!(cleared.result.as_ref().unwrap()["safeMode"], false);
    let replay = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(89)),
        method: "recovery.clearSafeMode".into(),
        params: Some(json!({ "idempotencyKey": "recovery-clear-1" })),
    });
    assert_eq!(replay.result.unwrap(), cleared.result.unwrap());
    let decision = plane.record_runtime_crash(103).unwrap();
    assert_eq!(decision.mode, RecoveryMode::RestoreEligible);
}

#[test]
fn memory_recovery_status_exposes_recent_crashes_and_safe_mode() {
    let mut plane = ControlPlane::default();
    let now = unix_epoch_seconds() as u64;
    plane.record_runtime_crash(now).unwrap();
    plane.record_runtime_crash(now + 1).unwrap();
    plane.record_runtime_crash(now + 2).unwrap();

    let status = plane.status_snapshot().unwrap();
    assert_eq!(status["recovery"]["recentCrashes"], 3);
    assert_eq!(status["recovery"]["safeMode"], true);
    assert_eq!(status["recovery"]["persistence"], "memory");
}

#[test]
fn recovery_clear_rpc_preserves_durable_latch_when_journal_is_full() {
    let storage = Storage::open_memory().unwrap();
    for index in 0..audiorouter_storage::MAX_OPERATION_JOURNAL_ENTRIES {
        storage
            .journal_commit(&format!("existing-recovery-{index}"), "test", "{}", 0)
            .unwrap();
    }
    let mut plane = ControlPlane::with_storage("recovery-full-journal", storage);
    let now = unix_epoch_seconds() as u64;
    plane.record_runtime_crash(now).unwrap();
    plane.record_runtime_crash(now + 1).unwrap();
    plane.record_runtime_crash(now + 2).unwrap();

    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(21)),
        method: "recovery.clearSafeMode".into(),
        params: Some(json!({ "idempotencyKey": "recovery-clear-full-journal" })),
    });
    assert!(response.error.is_some());
    let status = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(22)),
            method: "status.get".into(),
            params: None,
        })
        .result
        .unwrap();
    assert_eq!(status["recovery"]["safeMode"], true);
}

#[test]
fn privacy_mute_is_authorized_and_durable_across_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-privacy-mute-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
    let enabled = first.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(19)),
        method: "safety.setPrivacyMute".into(),
        params: Some(json!({ "muted": true, "idempotencyKey": "privacy-1" })),
    });
    assert_eq!(enabled.result.unwrap()["muted"], true);
    let denied = first.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(20)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": false })),
        },
        &ClientGrant::read_only(),
    );
    assert_eq!(denied.error.unwrap().code, -32001);
    // Regression for a desktop-shell tray/UI toggle that was silently
    // refused: the desktop shell grant must be sufficient on its own,
    // without also requiring Capture (which it never holds).
    let shell_toggled = first.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(20)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": false, "idempotencyKey": "privacy-2" })),
        },
        &ClientGrant::for_desktop_shell(),
    );
    assert_eq!(shell_toggled.result.unwrap()["muted"], false);
    let shell_relatched = first.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(20)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": true, "idempotencyKey": "privacy-3" })),
        },
        &ClientGrant::for_desktop_shell(),
    );
    assert_eq!(shell_relatched.result.unwrap()["muted"], true);
    drop(first);
    let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
    let status = second
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(21)),
            method: "status.get".into(),
            params: None,
        })
        .result
        .unwrap();
    assert_eq!(status["privacyMute"]["muted"], true);
    let replay = second.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(22)),
        method: "safety.setPrivacyMute".into(),
        params: Some(json!({ "muted": true, "idempotencyKey": "privacy-1" })),
    });
    assert_eq!(replay.result.unwrap()["muted"], true);
    let _ = std::fs::remove_file(path);
}

#[test]
fn startup_get_reports_unavailable_without_side_effects() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "startup.get".into(),
        params: None,
    });
    let result = response.result.unwrap();
    assert_eq!(result["enabled"], false);
    assert_eq!(result["registration"], "unavailable");
    assert_eq!(
        result["reason"],
        "native sign-in registration is owned by the desktop shell"
    );
}

#[test]
fn startup_plan_and_apply_are_explicitly_fail_closed() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "startup.plan".into(),
        params: Some(json!({ "enabled": true })),
    });
    let plan = response.result.unwrap();
    assert_eq!(plan["enabled"], true);
    assert_eq!(plan["registration"], "unavailable");
    assert_eq!(plan["requiredScopes"], json!(["startupWrite"]));
    let plan_id = plan["planId"].as_str().unwrap().to_owned();

    let apply = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "startup.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-1" })),
    });
    let result = apply.result.unwrap();
    assert_eq!(result["state"], "unavailable");
    assert_eq!(result["registration"], "unavailable");
    assert_eq!(
        result["reason"],
        "native sign-in registration is owned by the desktop shell"
    );
    assert!(
        plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(5)),
                method: "startup.get".into(),
                params: None,
            })
            .result
            .unwrap()["enabled"]
            .as_bool()
            == Some(true)
    );
    let replay = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "startup.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-2" })),
    });
    assert!(replay.error.is_some());
    let retry = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "startup.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-1" })),
    });
    assert_eq!(retry.result.unwrap()["state"], "unavailable");
}

#[test]
fn storage_backed_startup_plan_survives_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-startup-plan-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let plan_id = {
        let mut plane = ControlPlane::with_storage("startup-first", Storage::open(&path).unwrap());
        plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "startup.plan".into(),
                params: Some(json!({ "enabled": true })),
            })
            .result
            .unwrap()["planId"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let mut restarted = ControlPlane::with_storage("startup-second", Storage::open(&path).unwrap());
    let applied = restarted.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "startup.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-restart" })),
    });
    assert_eq!(applied.result.unwrap()["state"], "unavailable");
    assert!(
        restarted
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(5)),
                method: "startup.get".into(),
                params: None,
            })
            .result
            .unwrap()["enabled"]
            .as_bool()
            == Some(true)
    );
    assert!(restarted
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "startup.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-replay" })),
        })
        .error
        .is_some());
    let retry = restarted.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "startup.apply".into(),
        params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-restart" })),
    });
    assert_eq!(retry.result.unwrap()["state"], "unavailable");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn system_quit_stops_running_sessions_and_replays_idempotently() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let request = || JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!("quit")),
        method: "system.quit".into(),
        params: Some(json!({ "idempotencyKey": "quit-once" })),
    };
    let grant = ClientGrant::for_role(ClientRole::Operator);
    let first = plane.dispatch_authorized_for_client(request(), "shell", &grant);
    assert_eq!(first.result.as_ref().unwrap()["state"], "stopped");
    assert_eq!(
        first.result.as_ref().unwrap()["sessions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Stopped);
    let replay = plane.dispatch_authorized_for_client(request(), "shell", &grant);
    assert_eq!(replay.result, first.result);
}

#[test]
fn system_quit_keeps_session_running_when_recorder_finalization_fails() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let mut recorder = RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(128).unwrap();
    plane.recorders.insert(original.id.clone(), recorder);
    let request = || JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!("quit")),
        method: "system.quit".into(),
        params: Some(json!({ "idempotencyKey": "quit-retry" })),
    };
    let grant = ClientGrant::for_role(ClientRole::Operator);
    let failed = plane.dispatch_authorized_for_client(request(), "shell", &grant);
    assert!(failed.result.is_none());
    assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Running);

    plane
        .attach_recorder_worker(original.id.clone(), Box::new(TestRecorderWorker))
        .unwrap();
    let recovered = plane.dispatch_authorized_for_client(request(), "shell", &grant);
    assert_eq!(recovered.result.as_ref().unwrap()["state"], "stopped");
    assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Stopped);
}

#[test]
fn system_quit_finalizes_all_active_node_recorders_before_stopping_session() {
    let mut plane = ControlPlane::default();
    let mut original = session();
    for node_id in ["quit-recorder-a", "quit-recorder-b"] {
        original.nodes.push(Node {
            id: EntityId::new(node_id),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: node_id.into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
    }
    let session_id = original.id.clone();
    plane.insert_session(original).unwrap();
    plane.session_start(&session_id).unwrap();
    let queue = Arc::new(RecordingQueue::new(4).unwrap());
    for node_id in ["quit-recorder-a", "quit-recorder-b"] {
        plane
            .attach_recorder_worker_to_node(
                &session_id,
                EntityId::new(node_id),
                Box::new(SuccessfulTapRecorderWorker {
                    tap: Arc::new(RecorderAudioTap::new(queue.clone())),
                }),
            )
            .unwrap();
        plane
            .control_recorder_node(&EntityId::new(node_id), "recorders.arm", None)
            .unwrap();
        plane
            .control_recorder_node(&EntityId::new(node_id), "recorders.start", Some(0))
            .unwrap();
    }

    let response = plane.dispatch_authorized_for_client(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("quit")),
            method: "system.quit".into(),
            params: Some(json!({ "idempotencyKey": "quit-two-node-recorders" })),
        },
        "shell",
        &ClientGrant::for_role(ClientRole::Operator),
    );

    assert_eq!(response.result.as_ref().unwrap()["state"], "stopped");
    assert_eq!(
        response.result.as_ref().unwrap()["recorders"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(plane.recorder_node_workers.is_empty());
    assert!(plane.recorder_node_states.is_empty());
    assert!(plane.recorder_node_sessions.is_empty());
    assert_eq!(plane.runtimes[&session_id].state(), RuntimeState::Stopped);
}

#[test]
fn startup_mutations_require_the_dedicated_startup_write_scope() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "startup.plan".into(),
        params: Some(json!({ "enabled": true })),
    };
    let denied = ControlPlane::default().dispatch_authorized(
        request.clone(),
        &ClientGrant::for_role(ClientRole::Operator),
    );
    assert_eq!(denied.error.unwrap().code, -32001);

    let allowed = ControlPlane::default().dispatch_authorized(
        request,
        &ClientGrant::with_scopes([PermissionScope::StartupWrite]),
    );
    assert!(allowed.result.is_some());
}
