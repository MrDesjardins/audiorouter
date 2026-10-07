//! Tests for `status.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn diagnostics_are_redacted_and_report_backend_state() {
    let mut plane = ControlPlane::default();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(9)),
        method: "system.diagnostics".into(),
        params: None,
    });
    let result = response.result.unwrap();
    assert_eq!(result["backend"], "control-plane");
    assert_eq!(result["storage"], "memory");
    assert_eq!(result["nativeAdapter"], "implemented-not-activated");
    assert_eq!(result["nativeAdapterKind"], Value::Null);
    assert_eq!(result["nativeSessionId"], Value::Null);
    assert_eq!(result["schedulerTelemetry"], Value::Null);
    assert_eq!(result["nodeTelemetry"], json!([]));
    assert_eq!(result["redacted"], true);
    assert!(result.get("path").is_none());
}

#[test]
fn audio_status_names_the_attached_worker_kind() {
    assert_eq!(
        ControlPlane::audio_status_for("running", Some("endpoint")),
        ("available", "native endpoint audio is running")
    );
    assert_eq!(
        ControlPlane::audio_status_for("configured-stopped", Some("duplex")),
        (
            "unavailable",
            "native duplex worker is prepared but stopped; start a session explicitly"
        )
    );
    assert_eq!(
        ControlPlane::audio_status_for("running", Some("multi-input")),
        ("available", "native multi-input audio is running")
    );
    assert_eq!(
        ControlPlane::audio_status_for("unknown", None),
        (
            "unavailable",
            "audio is not prepared; in Devices, select exact capture and render endpoints, then Prepare native endpoints and Start session"
        )
    );
}

#[test]
fn status_snapshot_tracks_sessions_and_event_cursor() {
    let mut plane = ControlPlane::default();
    let initial = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(15)),
        method: "status.get".into(),
        params: None,
    });
    assert_eq!(initial.result.as_ref().unwrap()["sessionCount"], 0);
    let value = session();
    plane.insert_session(value.clone()).unwrap();
    let status = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(16)),
        method: "status.get".into(),
        params: None,
    });
    let result = status.result.unwrap();
    assert_eq!(result["sessionCount"], 1);
    assert_eq!(result["activeSessionCount"], 0);
    assert_eq!(
        result["reason"],
        "audio is not prepared; in Devices, select exact capture and render endpoints, then Prepare native endpoints and Start session"
    );
    assert_eq!(result["eventCursor"]["latestSequence"], 1);
}

#[test]
fn status_and_diagnostics_expose_persisted_recovery_safe_mode() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-recovery-status-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let storage = Storage::open(&path).unwrap();
    let timestamp = unix_epoch_seconds() as u64;
    for offset in 0..3 {
        storage.record_recovery_crash(timestamp + offset).unwrap();
    }
    let mut plane = ControlPlane::with_storage("recovery", storage);
    let status = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(17)),
        method: "status.get".into(),
        params: None,
    });
    let status_result = status.result.unwrap();
    assert_eq!(status_result["recovery"]["safeMode"], true);
    assert_eq!(status_result["recovery"]["recentCrashes"], 3);
    let diagnostics = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(18)),
        method: "system.diagnostics".into(),
        params: None,
    });
    let diagnostics_result = diagnostics.result.unwrap();
    assert_eq!(diagnostics_result["recovery"]["safeMode"], true);
    assert_eq!(diagnostics_result["recovery"]["recentCrashes"], 3);
    let cleared = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(19)),
        method: "recovery.clearSafeMode".into(),
        params: Some(json!({ "idempotencyKey": "recovery-clear-2" })),
    });
    assert_eq!(cleared.result.unwrap()["safeMode"], false);
    let status_after = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(20)),
        method: "status.get".into(),
        params: None,
    });
    assert_eq!(status_after.result.unwrap()["recovery"]["recentCrashes"], 0);
    drop(plane);
    let _ = std::fs::remove_file(path);
}

#[test]
fn events_subscribe_replays_filtered_control_state() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "evented".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "event-commit").unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "events.subscribe".into(),
        params: Some(json!({ "sessionId": "session", "afterSequence": 0 })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["backendEpoch"], 1);
    assert_eq!(result["events"].as_array().unwrap().len(), 2);
    assert_eq!(result["events"][1]["operationId"], "event-commit");
}

#[test]
fn events_subscribe_page_cursor_does_not_skip_retained_events() {
    let mut plane = ControlPlane::default();
    for index in 0..=500 {
        plane.events.append(
            index,
            Some(format!("page-{index}")),
            "state.test",
            Some(EntityId::new("session")),
        );
    }

    let first = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(40)),
        method: "events.subscribe".into(),
        params: Some(json!({ "afterSequence": 0, "limit": 500 })),
    });
    let first_result = first.result.unwrap();
    assert_eq!(first_result["events"].as_array().unwrap().len(), 500);
    assert_eq!(first_result["nextSequence"], 500);

    let second = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(41)),
        method: "events.subscribe".into(),
        params: Some(json!({ "afterSequence": 500, "limit": 500 })),
    });
    let second_result = second.result.unwrap();
    assert_eq!(second_result["events"].as_array().unwrap().len(), 1);
    assert_eq!(second_result["events"][0]["resourceRevision"], 500);
    assert_eq!(second_result["events"][0]["operationId"], "page-500");
    assert_eq!(second_result["nextSequence"], 501);
}

#[test]
fn events_subscribe_filters_by_category_and_rejects_unbounded_filters() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let mut candidate = original.clone();
    candidate.name = "evented".into();
    let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
    plane.commit_graph(&plan, 0, "category-filter").unwrap();

    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(6)),
        method: "events.subscribe".into(),
        params: Some(json!({
            "afterSequence": 0,
            "categories": ["graph.committed"]
        })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["events"].as_array().unwrap().len(), 1);
    assert_eq!(result["events"][0]["category"], "graph.committed");

    let too_many_categories = vec!["state.test"; 33];
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(7)),
        method: "events.subscribe".into(),
        params: Some(json!({
            "categories": too_many_categories
        })),
    });
    assert_eq!(response.error.unwrap().code, -32602);
}

#[test]
fn events_subscribe_returns_snapshot_when_cursor_expired() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    for _ in 0..=audiorouter_domain::MAX_RETAINED_EVENTS {
        plane.events.append(0, None, "state.test", None);
    }
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(5)),
        method: "events.subscribe".into(),
        params: Some(json!({ "afterSequence": 1 })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["resyncRequired"], true);
    assert_eq!(
        result["snapshot"]["sessions"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(result["events"].as_array().unwrap().is_empty());
}

#[test]
fn events_subscribe_requires_resync_when_backend_epoch_changes() {
    let mut plane = ControlPlane::default();
    plane.insert_session(session()).unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(6)),
        method: "events.subscribe".into(),
        params: Some(json!({ "backendEpoch": 999, "afterSequence": 0 })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["resyncRequired"], true);
    assert_eq!(result["reason"], "backendEpochChanged");
    assert_eq!(result["backendEpoch"], 1);
    assert_eq!(
        result["snapshot"]["sessions"]["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn durable_control_restart_advances_backend_epoch() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-epoch-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let first_epoch = {
        let mut plane = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "status.get".into(),
                params: None,
            })
            .result
            .unwrap()["eventCursor"]["backendEpoch"]
            .as_u64()
            .unwrap()
    };
    let second_epoch = {
        let mut plane = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "status.get".into(),
                params: None,
            })
            .result
            .unwrap()["eventCursor"]["backendEpoch"]
            .as_u64()
            .unwrap()
    };
    assert_eq!(second_epoch, first_epoch + 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn one_hundred_durable_reconnects_advance_backend_epoch_monotonically() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-reconnect-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let mut previous_epoch = None;

    for index in 0..100 {
        let plane =
            ControlPlane::with_storage(format!("reconnect-{index}"), Storage::open(&path).unwrap());
        let epoch = plane.events.backend_epoch();
        if let Some(previous) = previous_epoch {
            assert_eq!(epoch, previous + 1);
        }
        previous_epoch = Some(epoch);
    }

    let _ = std::fs::remove_file(path);
}

#[test]
fn verbose_diagnostics_switch_needs_session_control_and_reports_the_window() {
    let call = |method: &str, params: Option<Value>| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params,
    };
    let mut plane = ControlPlane::default();
    let now = audiorouter_protocol::diagnostics::unix_time_ms();
    assert!(!plane.verbose_diagnostics_active(now));
    let observer = ClientGrant::for_role(ClientRole::Observer);
    let status = plane
        .dispatch_authorized(call("diagnostics.getVerbose", None), &observer)
        .result
        .unwrap();
    assert_eq!(status["enabled"], false);
    let denied = plane.dispatch_authorized(
        call("diagnostics.setVerbose", Some(json!({"enabled": true}))),
        &observer,
    );
    assert!(denied.error.is_some());
    assert!(!plane.verbose_diagnostics_active(now));
    let on = plane
        .dispatch_authorized(
            call("diagnostics.setVerbose", Some(json!({"enabled": true}))),
            &ClientGrant::for_desktop_shell(),
        )
        .result
        .unwrap();
    assert_eq!(on["enabled"], true);
    assert!(on["remainingSeconds"].as_u64().unwrap() > 3_590);
    assert_eq!(on["maxSeconds"], 3_600);
    assert!(plane.verbose_diagnostics_active(audiorouter_protocol::diagnostics::unix_time_ms()));
    assert!(!plane.verbose_diagnostics_active(on["expiresAtUnixMs"].as_u64().unwrap()));
    for bad in [
        json!({}),
        json!({"enabled": "yes"}),
        json!({"enabled": true, "path": "x"}),
    ] {
        assert!(plane
            .dispatch(call("diagnostics.setVerbose", Some(bad)))
            .error
            .is_some());
    }
    let off = plane
        .dispatch(call(
            "diagnostics.setVerbose",
            Some(json!({"enabled": false})),
        ))
        .result
        .unwrap();
    assert_eq!(off["enabled"], false);
    assert_eq!(off["expiresAtUnixMs"], Value::Null);
}
