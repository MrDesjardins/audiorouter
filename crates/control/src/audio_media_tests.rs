//! Tests for `audio_media.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn a_changed_source_setting_drops_its_reused_transport() {
    let mut plane = ControlPlane::default();
    let mut current = session();
    let node = current.nodes[0].clone();
    current.nodes[0].kind = NodeKind::TestSignal;
    current.nodes[0].parameters = serde_json::from_value(json!({"frequencyHz": 880.0})).unwrap();
    let mut previous = current.clone();
    previous.nodes[0].parameters = serde_json::from_value(json!({"frequencyHz": 440.0})).unwrap();
    plane.insert_session(current.clone()).unwrap();
    let source = std::sync::Arc::new(audiorouter_engine::TestSignalSource::new(
        440.0, -18.0, 1_000.0, 48_000,
    ));
    plane.test_signal_sources.insert(
        (current.id.clone(), node.id.clone()),
        std::sync::Arc::clone(&source),
    );

    // A new frequency must rebuild the source: the old tone would keep playing.
    let retained = plane
        .reusable_source_transports(&current, SourceRetention::Unchanged(&previous))
        .unwrap();
    assert!(retained.is_empty());
    // An unchanged source keeps its playback position.
    let retained = plane
        .reusable_source_transports(&current, SourceRetention::Unchanged(&current))
        .unwrap();
    assert_eq!(retained.len(), 1);
    // Compare-only republishes keep every transport; structural edits none.
    assert_eq!(
        plane
            .reusable_source_transports(&current, SourceRetention::All)
            .unwrap()
            .len(),
        1
    );
    assert!(plane
        .reusable_source_transports(&current, SourceRetention::None)
        .is_none());
}

#[test]
fn the_decoded_media_cache_is_bounded_and_cleared_with_its_session() {
    let mut plane = ControlPlane::default();
    let session = session();
    plane.insert_session(session.clone()).unwrap();
    let decoded = |samples: usize| {
        std::sync::Arc::new(DecodedAudio {
            channels: 1,
            sample_rate_hz: 48_000,
            samples: vec![0.0; samples].into(),
        })
    };
    plane
        .audio_media_cache
        .insert((session.id.clone(), "keep".into(), 48_000), decoded(4));
    plane.audio_media_cache.insert(
        (EntityId::new("other"), "drop".into(), 48_000),
        decoded(100),
    );
    plane.trim_audio_media_cache_to(&session.id, 8);
    assert!(plane
        .audio_media_cache
        .contains_key(&(session.id.clone(), "keep".into(), 48_000)));
    assert!(!plane.audio_media_cache.contains_key(&(
        EntityId::new("other"),
        "drop".into(),
        48_000
    )));

    plane.delete_session(&session.id).unwrap();
    assert!(plane.audio_media_cache.is_empty());
}

#[test]
fn media_references_prevent_deleting_board_sounds_and_impulses() {
    for (kind, parameters) in [
        (NodeKind::Soundboard, json!({"slot1MediaId": "used"})),
        (
            NodeKind::Soundboard,
            json!({"slot2Set": ["lib:fixture", "used"]}),
        ),
        (NodeKind::FirFilter, json!({"mediaId": "used"})),
    ] {
        let storage = Storage::open_memory().unwrap();
        storage
            .store_audio_media("used", "sound.wav", "wav", &tiny_test_wav(), None)
            .unwrap();
        let mut plane = ControlPlane::with_storage("media-references", storage);
        let mut saved = session();
        let mut node = saved.nodes[0].clone();
        node.id = EntityId::new("media-user");
        node.kind = kind;
        node.parameters = serde_json::from_value(parameters).unwrap();
        if kind == NodeKind::FirFilter {
            let mut input = node.ports[0].clone();
            input.direction = PortDirection::Input;
            input.name = "in".into();
            node.ports.push(input);
        }
        // Disabled nodes still own their saved assets.
        node.enabled = false;
        saved.nodes.push(node);
        plane.insert_session(saved).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "audioMedia.delete".into(),
            params: Some(json!({"mediaId": "used"})),
        });
        assert!(
            response
                .error
                .expect("referenced media must be retained")
                .message
                .contains("still referenced"),
            "{kind:?}"
        );
        assert!(plane
            .storage
            .as_ref()
            .unwrap()
            .load_audio_media("used")
            .unwrap()
            .is_some());
    }
}

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

#[test]
fn a_missing_audio_file_names_its_node_for_the_canvas() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("missing-media", storage);
    let mut owned = session();
    owned.nodes.push(Node {
        id: EntityId::new("audioFile-1"),
        kind: NodeKind::AudioFile,
        type_version: 1,
        name: "Audio file 1".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(json!({
            "mediaId": "audio-media-3",
            "fileName": "Temporary voice take.wav",
            "loop": false
        }))
        .unwrap(),
        ports: vec![Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    });
    let error = plane.session_audio_media(&owned, 48_000).unwrap_err();
    let ControlError::NodeNotReady { node_id, message } = &error else {
        panic!("expected NodeNotReady, got {error:?}");
    };
    assert_eq!(node_id, "audioFile-1");
    assert_eq!(
        message,
        "\"Audio file 1\" cannot play: AudioRouter no longer has \"Temporary voice take.wav\". Choose the file again in its Properties."
    );
    let response = application_error_response(Some(json!(1)), error);
    let failure = response.error.unwrap();
    assert!(failure.message.starts_with("\"Audio file 1\" cannot play"));
    let data = failure.data.unwrap();
    assert_eq!(data["code"], "nodeNotReady");
    assert_eq!(data["fieldPath"], "nodes/audioFile-1");
    assert_eq!(data["resourceIds"], json!(["audioFile-1"]));
}

#[test]
fn stopping_a_session_releases_its_decoded_media() {
    let mut plane = ControlPlane::default();
    let session = session();
    plane.insert_session(session.clone()).unwrap();
    plane.audio_media_cache.insert(
        (session.id.clone(), "file".into(), 48_000),
        std::sync::Arc::new(DecodedAudio {
            channels: 1,
            sample_rate_hz: 48_000,
            samples: vec![0.0; 16].into(),
        }),
    );
    plane.session_stop(&session.id).unwrap();
    assert!(
        !plane
            .audio_media_cache
            .keys()
            .any(|(owner, _, _)| owner == &session.id),
        "Stop must release the route's decoded Audio File/FIR PCM"
    );
}
