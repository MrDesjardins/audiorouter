//! Tests for `recording_library.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn recordings_list_dispatch_exposes_storage_metadata_without_file_actions() {
    let storage = Storage::open_memory().unwrap();
    storage
        .save_recording(&audiorouter_storage::RecordingRecord {
            id: "recording-1".into(),
            session_id: "session".into(),
            recorder_id: "recorder".into(),
            path: "C:\\recordings\\one.wav".into(),
            format: "wav".into(),
            channels: 2,
            sample_rate: 48_000,
            frames: 96_000,
            file_bytes: 384_000,
            start_time: "2026-09-06T00:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: Some("Test".into()),
            artist: None,
            comment: None,
            dither: false,
            conversion: "unknown".into(),
        })
        .unwrap();
    storage
        .save_recording(&audiorouter_storage::RecordingRecord {
            id: "recording-2".into(),
            session_id: "session".into(),
            recorder_id: "recorder".into(),
            path: "C:\\recordings\\two.wav".into(),
            format: "wav".into(),
            channels: 2,
            sample_rate: 48_000,
            frames: 48_000,
            file_bytes: 192_000,
            start_time: "2026-09-06T01:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: None,
            artist: None,
            comment: None,
            dither: true,
            conversion: "targetSampleRate=44100;channels=2;bitsPerSample=16".into(),
        })
        .unwrap();
    let mut plane = ControlPlane::with_storage("recordings", storage);
    let denied = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "recordings.list".into(),
            params: Some(json!({ "sessionId": "session" })),
        },
        &ClientGrant::read_only(),
    );
    assert_eq!(
        denied.error.unwrap().data.unwrap()["code"],
        "permissionDenied"
    );
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(8)),
        method: "recordings.list".into(),
        params: Some(json!({ "sessionId": "session" })),
    });
    let result = response.result.unwrap();
    assert_eq!(result.as_array().unwrap().len(), 2);
    assert_eq!(result[0]["id"], "recording-1");
    assert_eq!(result[0]["missing"], false);
    assert_eq!(result[0]["dither"], false);
    assert_eq!(result[0]["conversion"], "unknown");
    assert_eq!(result[1]["dither"], true);
    assert_eq!(
        result[1]["conversion"],
        "targetSampleRate=44100;channels=2;bitsPerSample=16"
    );
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(10)),
        method: "recordings.list".into(),
        params: Some(json!({ "sessionId": "session", "limit": 1 })),
    });
    let page = response.result.unwrap();
    assert_eq!(page["items"][0]["id"], "recording-1");
    assert_eq!(page["nextCursor"], "recording-1");
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(11)),
        method: "recordings.list".into(),
        params: Some(json!({
            "sessionId": "session",
            "cursor": "recording-1",
            "limit": 1
        })),
    });
    let page = response.result.unwrap();
    assert_eq!(page["items"][0]["id"], "recording-2");
    assert_eq!(page["nextCursor"], Value::Null);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(9)),
        method: "recordings.get".into(),
        params: Some(json!({ "recordingId": "recording-1" })),
    });
    assert_eq!(response.result.unwrap()["title"], "Test");
}

#[test]
fn recording_recovery_dispatch_returns_validated_checkpoint_or_missing() {
    let storage = Storage::open_memory().unwrap();
    let mut recorder = audiorouter_recording::RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(100).unwrap();
    storage
        .save_recording_checkpoint("recovery-recording", &recorder.checkpoint())
        .unwrap();
    let mut plane = ControlPlane::with_storage("recovery", storage);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "recordings.recovery".into(),
        params: Some(json!({ "recordingId": "recovery-recording" })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["status"], "available");
    assert_eq!(result["checkpoint"]["state"], "Recording");
    let missing = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "recordings.recovery".into(),
        params: Some(json!({ "recordingId": "missing" })),
    });
    assert_eq!(missing.result.unwrap()["status"], "missing");
}

#[test]
fn recording_recovery_without_id_lists_bounded_checkpoint_entries() {
    let storage = Storage::open_memory().unwrap();
    let mut recorder = audiorouter_recording::RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(100).unwrap();
    storage
        .save_recording_checkpoint("recovery-a", &recorder.checkpoint())
        .unwrap();
    storage
        .save_recording_checkpoint("recovery-b", &recorder.checkpoint())
        .unwrap();
    let mut plane = ControlPlane::with_storage("recovery-list", storage);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "recordings.recovery".into(),
        params: Some(json!({ "limit": 1 })),
    });
    let result = response.result.unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["recordingId"], "recovery-a");
    assert_eq!(result["items"][0]["status"], "available");
    let cursor = result["nextCursor"].as_str().unwrap();
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "recordings.recovery".into(),
        params: Some(json!({ "cursor": cursor, "limit": 1 })),
    });
    assert_eq!(
        response.result.unwrap()["items"][0]["recordingId"],
        "recovery-b"
    );
}

#[test]
fn live_recorders_list_reports_in_memory_state_and_frame() {
    let mut plane = ControlPlane::default();
    plane.insert_session(session()).unwrap();
    let arm = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "recorders.arm".into(),
        params: Some(json!({ "sessionId": "session", "idempotencyKey": "arm-live-list" })),
    });
    assert_eq!(arm.result.unwrap()["state"], "armed");
    let start = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "recorders.start".into(),
        params: Some(
            json!({ "sessionId": "session", "frame": 128, "idempotencyKey": "start-live-list" }),
        ),
    });
    assert_eq!(start.result.unwrap()["state"], "recording");
    let listed = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "recorders.list".into(),
        params: None,
    });
    assert_eq!(
        listed.result.unwrap(),
        json!([{
            "sessionId": "session",
            "state": "recording",
            "lastFrame": 128
        }])
    );
}

#[test]
fn recording_recycle_preview_never_moves_the_file_and_missing_is_safe() {
    let path = std::env::temp_dir().join(format!("audiorouter-recycle-{}.wav", std::process::id()));
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, b"test recording").unwrap();
    let storage = Storage::open_memory().unwrap();
    storage
        .save_recording(&audiorouter_storage::RecordingRecord {
            id: "recording-recycle".into(),
            session_id: "session".into(),
            recorder_id: "recorder".into(),
            path: path.to_string_lossy().into_owned(),
            format: "wav".into(),
            channels: 1,
            sample_rate: 44_100,
            frames: 10,
            file_bytes: 14,
            start_time: "2026-09-06T00:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: None,
            artist: None,
            comment: None,
            dither: false,
            conversion: "unknown".into(),
        })
        .unwrap();
    let mut plane = ControlPlane::with_storage("recording-recycle", storage);
    let preview = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(11)),
        method: "recordings.recycle".into(),
        params: Some(json!({ "recordingId": "recording-recycle" })),
    });
    assert_eq!(preview.result.unwrap()["preview"], true);
    assert!(path.is_file());
    let preview_events = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(13)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 0, "sessionId": "session" })),
        })
        .result
        .unwrap();
    assert!(preview_events["events"].as_array().unwrap().is_empty());
    std::fs::remove_file(&path).unwrap();
    let missing = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(12)),
        method: "recordings.recycle".into(),
        params: Some(json!({
            "recordingId": "recording-recycle",
            "confirm": true,
            "idempotencyKey": "recycle-missing"
        })),
    });
    assert_eq!(missing.result.unwrap()["reason"], "missing");
}

#[test]
fn recording_metadata_mutation_requires_record_scope_and_preserves_file_path() {
    let storage = Storage::open_memory().unwrap();
    storage
        .save_recording(&audiorouter_storage::RecordingRecord {
            id: "recording-edit".into(),
            session_id: "session".into(),
            recorder_id: "recorder".into(),
            path: "C:\\recordings\\keep.wav".into(),
            format: "wav".into(),
            channels: 1,
            sample_rate: 44_100,
            frames: 10,
            file_bytes: 44,
            start_time: "2026-09-06T00:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: None,
            artist: None,
            comment: None,
            dither: false,
            conversion: "unknown".into(),
        })
        .unwrap();
    let mut plane = ControlPlane::with_storage("recording-edit", storage);
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(10)),
        method: "recordings.setMetadata".into(),
        params: Some(json!({
            "recordingId": "recording-edit",
            "title": "Edited",
            "idempotencyKey": "metadata-edit-1"
        })),
    };
    assert!(plane
        .dispatch_authorized(request.clone(), &ClientGrant::read_only())
        .error
        .is_some());
    let response = plane.dispatch_authorized(
        request,
        &ClientGrant::with_scopes([PermissionScope::Record]),
    );
    assert_eq!(response.result.unwrap()["updated"], true);
    let replay = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(14)),
            method: "recordings.setMetadata".into(),
            params: Some(json!({
                "recordingId": "recording-edit",
                "title": "Edited",
                "idempotencyKey": "metadata-edit-1"
            })),
        },
        &ClientGrant::with_scopes([PermissionScope::Record]),
    );
    assert_eq!(replay.result.unwrap()["updated"], true);
    let conflict = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(15)),
            method: "recordings.setMetadata".into(),
            params: Some(json!({
                "recordingId": "recording-edit",
                "title": "Different",
                "idempotencyKey": "metadata-edit-1"
            })),
        },
        &ClientGrant::with_scopes([PermissionScope::Record]),
    );
    assert!(conflict.error.is_some());
    let record = plane
        .storage
        .as_ref()
        .unwrap()
        .get_recording("recording-edit")
        .unwrap()
        .unwrap();
    assert_eq!(record.path, "C:\\recordings\\keep.wav");
    assert_eq!(record.title.as_deref(), Some("Edited"));
    let events = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(12)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 0, "sessionId": "session" })),
        })
        .result
        .unwrap();
    assert_eq!(events["events"][0]["category"], "recording.metadataChanged");
    let response = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(11)),
            method: "recordings.removeEntry".into(),
            params: Some(json!({
                "recordingId": "recording-edit",
                "idempotencyKey": "remove-recording-edit"
            })),
        },
        &ClientGrant::with_scopes([PermissionScope::Record]),
    );
    assert_eq!(response.result.unwrap()["fileAction"], "none");
    assert!(plane
        .storage
        .as_ref()
        .unwrap()
        .get_recording("recording-edit")
        .unwrap()
        .is_none());
    let events = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(13)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 1, "sessionId": "session" })),
        })
        .result
        .unwrap();
    assert_eq!(events["events"][0]["category"], "recording.entryRemoved");
}
