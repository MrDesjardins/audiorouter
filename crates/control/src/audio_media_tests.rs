//! Tests for `audio_media.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn test_signal_transport_controls_only_the_prepared_source() {
    let mut plane = ControlPlane::default();
    let owned = session();
    plane.insert_session(owned.clone()).unwrap();
    plane.session_start(&owned.id).unwrap();
    let node_id = EntityId::new("tone");
    let source = std::sync::Arc::new(audiorouter_engine::TestSignalSource::new(
        440.0, -18.0, 1_000.0, 48_000,
    ));
    plane.test_signal_sources.insert(
        (owned.id.clone(), node_id.clone()),
        std::sync::Arc::clone(&source),
    );
    let request = |action| json!({ "sessionId": owned.id, "nodeId": node_id, "action": action });
    assert_eq!(
        plane
            .dispatch_audio_source_transport(Some(request("status")))
            .unwrap()["state"],
        "stopped"
    );
    assert_eq!(
        plane
            .dispatch_audio_source_transport(Some(request("play")))
            .unwrap()["state"],
        "playing"
    );
    assert_eq!(
        plane
            .dispatch_audio_source_transport(Some(request("stop")))
            .unwrap()["state"],
        "stopped"
    );
    assert!(plane
        .dispatch_audio_source_transport(Some(request("pause")))
        .is_err());
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"][0],
        owned.id.as_str()
    );
}

#[test]
fn audio_media_api_uploads_and_decodes_a_bounded_wav() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("audio-upload", storage);
    let wav = tiny_test_wav();
    let begin = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "audioMedia.beginUpload".into(),
            params: Some(json!({"fileName":"voice.wav", "sizeBytes":wav.len()})),
        })
        .result
        .unwrap();
    let upload_id = begin["uploadId"].as_str().unwrap();
    let chunk = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "audioMedia.uploadChunk".into(),
        params: Some(
            json!({"uploadId":upload_id, "chunkIndex":0, "dataBase64":encode_test_base64(&wav)}),
        ),
    });
    assert_eq!(chunk.result.unwrap()["receivedBytes"], json!(wav.len()));
    let finish = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "audioMedia.finishUpload".into(),
            params: Some(json!({"uploadId":upload_id})),
        })
        .result
        .unwrap();
    assert_eq!(finish["format"], "wav");
    assert_eq!(finish["channels"], 1);
    assert_eq!(finish["durationMs"], 1);
    assert!(finish["mediaId"]
        .as_str()
        .unwrap()
        .starts_with("audio-media-"));
}

#[test]
fn temporary_take_import_is_bounded_expires_and_removes_only_its_own_file() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("temporary-take", storage);
    let wav = {
        let samples = [0i16; 48];
        let mut bytes = Vec::with_capacity(36 + samples.len() * 2);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36u32 + (samples.len() * 2) as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&48_000u32.to_le_bytes());
        bytes.extend_from_slice(&96_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&((samples.len() * 2) as u32).to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    };
    let path = std::env::temp_dir().join(format!(
        "audiorouter-temporary-take-{}.wav",
        std::process::id()
    ));
    std::fs::write(&path, &wav).unwrap();
    plane
        .storage
        .as_ref()
        .unwrap()
        .save_recording(&audiorouter_storage::RecordingRecord {
            id: "session-audio-file-take-test-run".into(),
            session_id: "session".into(),
            recorder_id: "audio-file-take-test".into(),
            path: path.to_string_lossy().into_owned(),
            format: "wav".into(),
            channels: 1,
            sample_rate: 48_000,
            frames: 48,
            file_bytes: wav.len() as u64,
            start_time: "2026-09-22T00:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: None,
            artist: None,
            comment: None,
            dither: false,
            conversion: "temporary take".into(),
        })
        .unwrap();
    let denied = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(0)),
            method: "audioMedia.importTemporaryRecording".into(),
            params: Some(json!({"recordingId":"session-audio-file-take-test-run"})),
        },
        &ClientGrant::read_only(),
    );
    assert_eq!(denied.error.unwrap().code, -32001);
    assert!(
        path.exists(),
        "a denied import must not touch the temporary file"
    );
    let result = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "audioMedia.importTemporaryRecording".into(),
            params: Some(json!({"recordingId":"session-audio-file-take-test-run"})),
        },
        &ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::Record]),
    );
    assert!(
        result.error.is_none(),
        "temporary take import error: {:?}",
        result.error
    );
    let result = result.result.unwrap();
    assert_eq!(result["format"], "wav");
    assert_eq!(result["sourceRemoved"], true);
    assert!(result["expiresAt"].as_i64().unwrap() > unix_epoch_seconds());
    assert!(!path.exists());
    let media_id = result["mediaId"].as_str().unwrap();
    assert!(plane
        .storage
        .as_ref()
        .unwrap()
        .load_audio_media(media_id)
        .unwrap()
        .is_some());
    assert!(plane
        .storage
        .as_ref()
        .unwrap()
        .get_recording("session-audio-file-take-test-run")
        .unwrap()
        .is_none());

    let ordinary = path.with_file_name("audiorouter-ordinary-recording.wav");
    std::fs::write(&ordinary, &wav).unwrap();
    plane
        .storage
        .as_ref()
        .unwrap()
        .save_recording(&audiorouter_storage::RecordingRecord {
            id: "ordinary-recording-run".into(),
            session_id: "session".into(),
            recorder_id: "normal-recorder".into(),
            path: ordinary.to_string_lossy().into_owned(),
            format: "wav".into(),
            channels: 1,
            sample_rate: 48_000,
            frames: 48,
            file_bytes: wav.len() as u64,
            start_time: "2026-09-22T00:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: None,
            artist: None,
            comment: None,
            dither: false,
            conversion: "test".into(),
        })
        .unwrap();
    let rejected = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "audioMedia.importTemporaryRecording".into(),
        params: Some(json!({"recordingId":"ordinary-recording-run"})),
    });
    assert!(rejected.error.is_some());
    assert!(ordinary.exists(), "ordinary recordings are not consumed");
    let _ = std::fs::remove_file(ordinary);
}
