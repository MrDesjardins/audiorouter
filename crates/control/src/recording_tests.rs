//! Tests for `recording.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn recorder_branch_plays_without_a_file_and_records_once_one_is_attached() {
    let mut graph = session();
    graph.nodes.push(Node {
        id: EntityId::new("recorder-node"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Recorder".into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![],
    });
    let session_id = graph.id.clone();
    let node_id = EntityId::new("recorder-node");
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    // Play binds the branch before anyone pressed Record: no error, and
    // the audio has nowhere to go yet.
    let bound = plane
        .recorder_tap_set_for_node(&session_id, &node_id)
        .expect("recorder branch binds without a file");
    let block = AudioBlock::new(1, 128).unwrap();
    bound.on_processed_block(0, &block);
    // Record attaches a file worker behind the same, already bound tap.
    let counter = Arc::new(CountingTap::default());
    plane
        .attach_recorder_worker_to_node(
            &session_id,
            node_id.clone(),
            Box::new(SuccessfulTapRecorderWorker {
                tap: counter.clone(),
            }),
        )
        .unwrap();
    bound.on_processed_block(128, &block);
    bound.on_processed_block(256, &block);
    assert_eq!(
        counter.0.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "blocks after Record reach the file"
    );
    // The endpoint-scheduler binding path also succeeds.
    assert!(plane
        .recorder_tap_bindings(&session_id, RuntimeGeneration::new(3))
        .is_ok());
}

#[test]
fn recording_folder_is_chosen_in_the_app_and_unlocks_one_click_recording() {
    let base = std::env::temp_dir().join(format!(
        "audiorouter-root-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut graph = session();
    graph.nodes.push(Node {
        id: EntityId::new("rec"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Recorder".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(json!({ "format": "wavPcm16" })).unwrap(),
        ports: vec![Port {
            name: "in".into(),
            direction: PortDirection::Input,
            channels: 2,
        }],
    });
    let session_id = graph.id.clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    let request = |plane: &mut ControlPlane, method: &str, params: Value| {
        plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        })
    };
    // Nothing approved yet: a suggestion is offered, never applied.
    let root = request(&mut plane, "recordings.getRoot", json!({}))
        .result
        .unwrap();
    assert!(root["root"].is_null());
    assert!(root["suggestedRoot"]
        .as_str()
        .is_some_and(|path| path.ends_with("AudioRouter Recordings")));
    // Record explains where to choose the folder.
    let refused = request(
        &mut plane,
        "recorders.startRecording",
        json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": "early" }),
    );
    assert!(refused.error.unwrap().message.contains("Recording folder"));
    // Unusable folders are refused with plain reasons and change nothing.
    for (path, reason) in [
        ("relative\\folder", "full folder path"),
        ("\\\\server\\share\\rec", "network share"),
    ] {
        let error = request(
            &mut plane,
            "recordings.setRoot",
            json!({ "root": path, "create": true, "idempotencyKey": path }),
        )
        .error
        .unwrap();
        assert!(error.message.contains(reason), "{path}: {}", error.message);
    }
    let missing = base.join("missing");
    let error = request(
        &mut plane,
        "recordings.setRoot",
        json!({ "root": missing, "idempotencyKey": "no-create" }),
    )
    .error
    .unwrap();
    assert!(
        error.message.contains("does not exist"),
        "{}",
        error.message
    );
    assert!(!missing.exists());
    std::fs::create_dir_all(&base).unwrap();
    let file = base.join("file.txt");
    std::fs::write(&file, b"x").unwrap();
    let error = request(
        &mut plane,
        "recordings.setRoot",
        json!({ "root": file, "idempotencyKey": "file" }),
    )
    .error
    .unwrap();
    assert!(error.message.contains("not a folder"), "{}", error.message);
    assert!(request(&mut plane, "recordings.getRoot", json!({}))
        .result
        .unwrap()["root"]
        .is_null());
    // Approve (and create) a folder; a retry with the same key replays.
    let chosen = base.join("My Recordings");
    let set = request(
        &mut plane,
        "recordings.setRoot",
        json!({ "root": chosen, "create": true, "idempotencyKey": "choose" }),
    )
    .result
    .unwrap();
    assert_eq!(set["created"], true);
    assert!(chosen.is_dir());
    let again = request(
        &mut plane,
        "recordings.setRoot",
        json!({ "root": chosen, "create": true, "idempotencyKey": "choose" }),
    )
    .result
    .unwrap();
    assert_eq!(again, set);
    let shown = request(&mut plane, "recordings.getRoot", json!({}))
        .result
        .unwrap();
    assert_eq!(shown["root"], set["root"]);
    assert!(!shown["root"].as_str().unwrap().starts_with("\\\\?\\"));
    // Record now writes into the chosen folder.
    let started = request(
        &mut plane,
        "recorders.startRecording",
        json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": "now" }),
    )
    .result
    .unwrap();
    assert_eq!(started["state"], "recording");
    let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
    assert_eq!(
        path.parent().unwrap().canonicalize().unwrap(),
        chosen.canonicalize().unwrap()
    );
    request(
        &mut plane,
        "recorders.stopRecording",
        json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": "stop" }),
    );
    drop(plane);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn one_click_recording_writes_splits_stops_and_never_blocks_stopping_playback() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-one-click-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let mut graph = session();
    graph.nodes.push(Node {
        id: EntityId::new("rec"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Recorder".into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(json!({ "format": "wavPcm16", "splitMinutes": 10 }))
            .unwrap(),
        ports: vec![Port {
            name: "in".into(),
            direction: PortDirection::Input,
            channels: 2,
        }],
    });
    let session_id = graph.id.clone();
    let node_id = EntityId::new("rec");
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    plane.configure_recording_root(&root).unwrap();
    let call = |plane: &mut ControlPlane, method: &str, key: &str| {
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(
                json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": key }),
            ),
        });
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    // Stop with nothing recording is a harmless no-op (StreamDeck safe).
    assert_eq!(
        call(&mut plane, "recorders.stopRecording", "stop-0")["state"],
        "idle"
    );
    let tap = plane
        .recorder_tap_set_for_node(&session_id, &node_id)
        .unwrap();
    let started = call(&mut plane, "recorders.startRecording", "start-1");
    assert_eq!(started["state"], "recording");
    assert_eq!(started["splitMinutes"], 10);
    // A second Record press while recording keeps the same take.
    assert_eq!(
        call(&mut plane, "recorders.startRecording", "start-again")["alreadyRecording"],
        true
    );
    let mut block = AudioBlock::new(2, 128).unwrap();
    block.channel_mut(0).unwrap().fill(0.25);
    block.channel_mut(1).unwrap().fill(-0.25);
    let mut frame = 1_000_000_u64;
    let mut feed = |plane: &mut ControlPlane, blocks: usize| {
        for _ in 0..blocks {
            tap.on_processed_block(frame, &block);
            frame += 128;
            plane.drain_attached_recorders().unwrap();
        }
    };
    feed(&mut plane, 20);
    // Automatic split (shortened for the test): the WAV splits in place.
    plane
        .recording_splits
        .get_mut(&node_id)
        .unwrap()
        .every_frames = 128 * 10;
    plane.maintain_node_recordings(std::time::Instant::now());
    feed(&mut plane, 20);
    plane.recording_maintained_at = None;
    plane.maintain_node_recordings(std::time::Instant::now());
    feed(&mut plane, 5);
    let stopped = call(&mut plane, "recorders.stopRecording", "stop-1");
    assert_eq!(stopped["state"], "completed");
    assert!(
        stopped["parts"].as_array().unwrap().len() >= 2,
        "split produced parts: {stopped}"
    );
    // Every split part must open in a player, not just be non-empty.
    for entry in std::fs::read_dir(&root).unwrap() {
        assert_playable_wav(&entry.unwrap().path());
    }
    // A new take is a new file; stopping playback finalizes it rather
    // than refusing to stop.
    let second = call(&mut plane, "recorders.startRecording", "start-2");
    assert_ne!(second["path"], started["path"]);
    feed(&mut plane, 4);
    let stop_error = plane
        .session_stop(&session_id)
        .err()
        .map(|error| format!("{error:?}"));
    assert!(
        !stop_error
            .as_deref()
            .unwrap_or("")
            .contains("must be finalized"),
        "{stop_error:?}"
    );
    assert!(
        !plane.recorder_node_workers.contains_key(&node_id),
        "recording finalized by Stop"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Real playback is not a perfect stream. A service stall shorter than
/// the two-second queue (REC-08) must not lose anything. A real gap, a
/// repeated block or a longer stall fails the take, but Stop still
/// succeeds, keeps a playable file up to that point and says why.
#[test]
fn one_click_recording_survives_dropped_and_repeated_blocks_and_stays_playable() {
    // Stalling the control thread no longer stalls the encoder. Deliberate
    // blocked-storage overflow is qualified in threaded_recorder tests.
    for case in ["short stall", "dropped block", "repeated block"] {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-gap-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut graph = session();
        graph.nodes.push(Node {
            id: EntityId::new("rec"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Recorder".into(),
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(json!({ "format": "wavPcm16" })).unwrap(),
            ports: vec![Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 2,
            }],
        });
        let session_id = graph.id.clone();
        let node_id = EntityId::new("rec");
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        plane.configure_recording_root(&root).unwrap();
        let tap = plane
            .recorder_tap_set_for_node(&session_id, &node_id)
            .unwrap();
        let request = |plane: &mut ControlPlane, method: &str, key: &str| {
            plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(
                    json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": key }),
                ),
            })
        };
        let started = request(&mut plane, "recorders.startRecording", "start")
            .result
            .unwrap();
        let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
        let mut block = AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        block.channel_mut(1).unwrap().fill(-0.25);
        let mut frame = 5_000_u64;
        let send = |plane: &mut ControlPlane, frame: &mut u64, blocks: usize, drain: bool| {
            for _ in 0..blocks {
                tap.on_processed_block(*frame, &block);
                *frame += 128;
                if drain {
                    let _ = plane.drain_attached_recorders();
                }
            }
        };
        send(&mut plane, &mut frame, 10, true);
        match case {
            // ~1 s with no service pass: within the queue.
            "short stall" => send(&mut plane, &mut frame, 375, false),
            "dropped block" => frame += 128,
            _ => {
                frame -= 128;
                send(&mut plane, &mut frame, 1, true);
            }
        }
        send(&mut plane, &mut frame, 10, true);
        let stopped = request(&mut plane, "recorders.stopRecording", "stop");
        assert!(
            stopped.error.is_none(),
            "{case}: Stop failed: {:?}",
            stopped.error
        );
        let stopped = stopped.result.unwrap();
        let data = assert_playable_wav(&path);
        if case == "short stall" {
            assert_eq!(stopped["state"], "completed", "{case}: {stopped}");
            // Every block arrived: 10 + 375 + 10 (2 ch × 16-bit).
            assert_eq!(data, 395 * 128 * 4, "{case}");
        } else {
            assert_eq!(stopped["state"], "failed", "{case}: {stopped}");
            assert!(
                stopped["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("audio was lost")
                        && reason.contains("keeps everything")),
                "{case}: {stopped}"
            );
            assert_eq!(stopped["paths"][0], json!(path.to_str().unwrap()), "{case}");
            // The ten blocks before the problem are kept and playable.
            assert!(data >= 10 * 128 * 4, "{case}: only {data} bytes of audio");
        }
        // The node is free for a fresh take.
        let again = request(&mut plane, "recorders.startRecording", "again")
            .result
            .unwrap();
        assert_eq!(again["state"], "recording", "{case}");
        request(&mut plane, "recorders.stopRecording", "again-stop");
        drop(plane);
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn one_click_flac_and_mp3_keep_a_finished_file_after_lost_audio() {
    for format in ["flac24", "mp3"] {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-gap-{format}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut graph = session();
        graph.nodes.push(Node {
            id: EntityId::new("rec"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Recorder".into(),
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(json!({ "format": format })).unwrap(),
            ports: vec![Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 2,
            }],
        });
        let session_id = graph.id.clone();
        let node_id = EntityId::new("rec");
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        plane.configure_recording_root(&root).unwrap();
        let tap = plane
            .recorder_tap_set_for_node(&session_id, &node_id)
            .unwrap();
        let request = |plane: &mut ControlPlane, method: &str, key: &str| {
            plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(
                    json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": key }),
                ),
            })
        };
        let started = request(&mut plane, "recorders.startRecording", "start")
            .result
            .unwrap();
        let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
        let mut block = AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        block.channel_mut(1).unwrap().fill(-0.25);
        let mut frame = 9_000_u64;
        for index in 0..200 {
            if index == 150 {
                frame += 128; // one dropped block
            }
            tap.on_processed_block(frame, &block);
            frame += 128;
            plane.drain_attached_recorders().unwrap();
        }
        let stopped = request(&mut plane, "recorders.stopRecording", "stop");
        assert!(
            stopped.error.is_none(),
            "{format}: Stop failed: {:?}",
            stopped.error
        );
        let stopped = stopped.result.unwrap();
        assert_eq!(stopped["state"], "failed", "{format}: {stopped}");
        assert!(
            std::fs::metadata(&path).unwrap().len() > 1_000,
            "{format}: file too small"
        );
        if format == "flac24" {
            let info = audiorouter_recording::inspect_flac_file(&path).unwrap();
            assert_eq!(info.frames, 150 * 128, "{format}: STREAMINFO frames");
        }
        drop(plane);
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn runtime_crash_recording_returns_one_bounded_memory_recovery_decision() {
    let mut plane = ControlPlane::default();
    let value = session();
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();

    let first = plane.record_runtime_crash(100).unwrap();
    assert_eq!(first.mode, RecoveryMode::RestoreEligible);
    assert_eq!(first.session_ids, vec![session_id.clone()]);

    plane.record_runtime_crash(101).unwrap();
    let third = plane.record_runtime_crash(102).unwrap();
    assert_eq!(third.mode, RecoveryMode::SafeMode);
    assert!(third.session_ids.is_empty());
}

#[test]
fn os_transition_refuses_to_stop_an_active_recording_implicitly() {
    let mut plane = ControlPlane::default();
    let value = session();
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();
    let recorder = RecorderController::new();
    plane.recorders.insert(session_id.clone(), recorder);
    let recorder = plane.recorders.get_mut(&session_id).unwrap();
    recorder.arm().unwrap();
    recorder.start(0).unwrap();

    let result = plane.handle_os_transition(os_transition::OsTransition::Sleep);
    assert!(result.is_err());
    assert_eq!(
        plane.status_snapshot().unwrap()["activeSessionIds"],
        json!([session_id])
    );
}

#[test]
fn runtime_crash_recording_persists_the_safe_mode_decision() {
    let storage = Storage::open_memory().unwrap();
    let mut plane = ControlPlane::with_storage("recovery-supervisor", storage);
    let value = session();
    let session_id = value.id.clone();
    plane.insert_session(value).unwrap();
    plane.session_start(&session_id).unwrap();

    let now = unix_epoch_seconds() as u64;
    plane.record_runtime_crash(now).unwrap();
    plane.record_runtime_crash(now + 1).unwrap();
    let decision = plane.record_runtime_crash(now + 2).unwrap();
    assert_eq!(decision.mode, RecoveryMode::SafeMode);
    assert!(decision.session_ids.is_empty());
    let status = plane.status_snapshot().unwrap();
    assert_eq!(status["recovery"]["safeMode"], true);
    assert_eq!(status["recovery"]["recentCrashes"], 3);
}

#[test]
fn recorder_api_reports_failed_finalization_without_stopping_audio() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let queue = Arc::new(RecordingQueue::new(4).unwrap());
    plane
        .attach_recorder_worker(
            original.id.clone(),
            Box::new(FailingTapRecorderWorker {
                tap: Arc::new(RecorderAudioTap::new(queue)),
            }),
        )
        .unwrap();
    for (index, method) in ["recorders.arm", "recorders.start", "recorders.stop"]
        .iter()
        .enumerate()
    {
        let mut params = json!({"sessionId": original.id,
            "idempotencyKey": format!("failed-finalization-{index}")});
        if *method != "recorders.arm" {
            params["frame"] = json!(0);
        }
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(index)),
            method: (*method).into(),
            params: Some(params),
        });
        assert_eq!(
            response.error.is_some(),
            *method == "recorders.stop",
            "{response:?}"
        );
    }
    assert_eq!(plane.recorders[&original.id].state(), RecorderState::Failed);
    assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Running);
}

#[test]
fn session_stop_refuses_to_orphan_an_active_recorder() {
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let mut recorder = RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(128).unwrap();
    plane.recorders.insert(original.id.clone(), recorder);

    assert!(matches!(
        plane.session_stop(&original.id),
        Err(ControlError::InvalidRequest(message))
            if message == "finalize the active recorder before stopping the session"
    ));
    assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Running);
}

#[test]
fn recorder_api_forwards_lifecycle_boundaries_to_attached_worker() {
    let hooks = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane
        .attach_recorder_worker(
            original.id.clone(),
            Box::new(HookRecorderWorker {
                hooks: hooks.clone(),
            }),
        )
        .unwrap();

    assert!(plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recorders.arm".into(),
            params: Some(json!({
                "sessionId": original.id,
                "idempotencyKey": "hook-arm"
            })),
        })
        .result
        .is_some());
    let repeated_arm = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "recorders.arm".into(),
        params: Some(json!({
            "sessionId": original.id,
            "idempotencyKey": "hook-repeated-arm"
        })),
    });
    assert!(repeated_arm.result.is_none());
    assert!(repeated_arm.error.is_some());
    assert_eq!(*hooks.lock().unwrap(), vec!["arm"]);

    for (id, method, frame) in [
        (3, "recorders.start", Some(0)),
        (4, "recorders.split", Some(128)),
    ] {
        let params = match frame {
            Some(frame) => json!({
                "sessionId": original.id,
                "frame": frame,
                "idempotencyKey": format!("hook-{id}")
            }),
            None => json!({
                "sessionId": original.id,
                "idempotencyKey": format!("hook-{id}")
            }),
        };
        assert!(plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(id)),
                method: method.into(),
                params: Some(params),
            })
            .result
            .is_some());
    }
    assert_eq!(*hooks.lock().unwrap(), vec!["arm", "start:0", "split:128"]);
    assert_eq!(
        plane.recorders[&original.id].checkpoint().last_frame,
        Some(128)
    );
}

#[test]
fn file_recorder_factory_removes_file_when_attachment_is_rejected() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-factory-rollback-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane
        .attach_recorder_worker(
            original.id.clone(),
            Box::new(HookRecorderWorker {
                hooks: Arc::new(std::sync::Mutex::new(Vec::new())),
            }),
        )
        .unwrap();

    let result = plane.create_and_attach_file_recorder(
        &policy,
        original.id,
        "voice",
        0,
        FileRecorderFormat::Wav(WavFormat::Pcm16),
        1,
        48_000,
        false,
        8,
        1,
    );

    assert!(matches!(
        result,
        Err(ControlError::InvalidRequest(message))
            if message == "recorder worker is already attached"
    ));
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recorder_api_stop_finalizes_attached_wav_before_completion() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-api-stop-{}.wav",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let worker = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 1, 1).unwrap();
    worker
        .try_push(RecordingChunk {
            start_frame: 0,
            samples: vec![0.25, -0.25],
        })
        .unwrap();

    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane
        .attach_recorder_worker(original.id.clone(), Box::new(worker))
        .unwrap();
    for (id, method, frame) in [
        (1, "recorders.arm", None),
        (2, "recorders.start", Some(0)),
        (3, "recorders.stop", Some(2)),
    ] {
        let params = match frame {
            Some(frame) => json!({
                "sessionId": original.id,
                "frame": frame,
                "idempotencyKey": format!("api-stop-{id}")
            }),
            None => json!({
                "sessionId": original.id,
                "idempotencyKey": format!("api-stop-{id}")
            }),
        };
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: method.into(),
            params: Some(params),
        });
        assert!(response.result.is_some(), "{method}: {response:?}");
    }
    assert!(plane.recorder_workers.is_empty());
    assert_eq!(
        audiorouter_recording::inspect_wav_file(&path)
            .unwrap()
            .frames,
        2
    );
    assert_eq!(
        plane.recorders[&original.id].state(),
        RecorderState::Completed
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn recorder_factory_creates_attaches_and_indexes_a_wav_before_arm() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("audiorouter-control-factory-{run_id}"));
    std::fs::create_dir_all(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    let mut plane = ControlPlane::with_storage("factory", Storage::open_memory().unwrap());
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.configure_recording_root(&root).unwrap();
    let invalid = FileRecorderConfig {
        version: FILE_RECORDER_CONFIG_VERSION + 1,
        session_id: original.id.as_str(),
        recorder_id: "invalid",
        sequence: 0,
        format: FileRecorderFormat::Wav(WavFormat::Pcm16),
        channels: 1,
        sample_rate: 48_000,
        dither: false,
        queue_capacity: 8,
        maximum_chunks_per_pass: 1,
    };
    assert!(create_file_recorder_with_config(&policy, &invalid).is_err());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    let config = FileRecorderConfig {
        version: FILE_RECORDER_CONFIG_VERSION,
        session_id: original.id.as_str(),
        recorder_id: "voice",
        sequence: 0,
        format: FileRecorderFormat::Wav(WavFormat::Pcm16),
        channels: 1,
        sample_rate: 48_000,
        dither: false,
        queue_capacity: 8,
        maximum_chunks_per_pass: 1,
    };
    let path = plane
        .create_and_attach_configured_file_recorder(original.id.clone(), &config)
        .unwrap();
    assert!(path.is_file());
    let taps = plane.recorder_tap_set(&original.id).unwrap();
    let mut first_block = AudioBlock::new(1, 2).unwrap();
    first_block
        .channel_mut(0)
        .unwrap()
        .copy_from_slice(&[0.1, 0.2]);
    taps.on_processed_block(0, &first_block);

    // Idle audio is not retained. Start is the admission boundary.
    let queue = plane.recorder_workers[&original.id]
        .shared_recording_queue()
        .unwrap();
    assert_eq!(queue.len(), 0);
    assert_eq!(queue.overruns(), 0);

    for (id, method, frame) in [
        (1, "recorders.arm", None),
        (2, "recorders.start", Some(0)),
        (3, "recorders.split", Some(2)),
    ] {
        let params = match frame {
            Some(frame) => json!({
                "sessionId": original.id,
                "frame": frame,
                "idempotencyKey": format!("factory-{id}")
            }),
            None => json!({
                "sessionId": original.id,
                "idempotencyKey": format!("factory-{id}")
            }),
        };
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: method.into(),
            params: Some(params),
        });
        assert!(response.result.is_some(), "{method}: {response:?}");
        if method == "recorders.start" {
            taps.on_processed_block(0, &first_block);
        }
    }
    let mut second_block = AudioBlock::new(1, 2).unwrap();
    second_block
        .channel_mut(0)
        .unwrap()
        .copy_from_slice(&[0.3, 0.4]);
    taps.on_processed_block(2, &second_block);
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(4)),
        method: "recorders.stop".into(),
        params: Some(json!({
            "sessionId": original.id,
            "frame": 4,
            "idempotencyKey": "factory-4"
        })),
    });
    assert!(response.result.is_some(), "recorders.stop: {response:?}");
    let rows = plane
        .storage
        .as_ref()
        .unwrap()
        .list_recordings(Some(original.id.as_str()))
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .all(|row| row.format == "wav" && row.frames == 2));
    assert_eq!(rows[0].path, path.to_str().unwrap());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recorder_binding_requires_one_validated_node_and_matching_generation() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-binding-{}.wav",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let worker = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap();
    let mut graph = session();
    graph.nodes.push(Node {
        id: EntityId::new("recorder-node"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Recorder".into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![],
    });
    let session_id = graph.id.clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    plane
        .attach_recorder_worker(session_id.clone(), Box::new(worker))
        .unwrap();

    let bindings = plane
        .recorder_tap_bindings(&session_id, RuntimeGeneration::new(7))
        .unwrap();
    let taps = bindings
        .tap_set_for_generation(RuntimeGeneration::new(7), &["recorder-node"])
        .unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(taps.len(), 1);
    assert!(matches!(
        bindings.tap_set_for_generation(RuntimeGeneration::new(8), &["recorder-node"]),
        Err(audiorouter_engine::RecorderTapBindingError::StaleGeneration)
    ));

    let _ = std::fs::remove_file(path);
}

#[test]
fn independent_recorder_nodes_bind_distinct_workers_and_taps() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let paths = [
        std::env::temp_dir().join(format!("audiorouter-control-node-a-{run_id}.wav")),
        std::env::temp_dir().join(format!("audiorouter-control-node-b-{run_id}.wav")),
    ];
    let make_worker = |path: &std::path::Path| {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap()
    };
    let mut graph = session();
    for (id, name) in [("recorder-a", "A"), ("recorder-b", "B")] {
        graph.nodes.push(Node {
            id: EntityId::new(id),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: name.into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
    }
    let session_id = graph.id.clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    plane
        .attach_recorder_worker_to_node(
            &session_id,
            EntityId::new("recorder-a"),
            Box::new(make_worker(&paths[0])),
        )
        .unwrap();
    plane
        .attach_recorder_worker_to_node(
            &session_id,
            EntityId::new("recorder-b"),
            Box::new(make_worker(&paths[1])),
        )
        .unwrap();

    let bindings = plane
        .recorder_tap_bindings(&session_id, RuntimeGeneration::new(9))
        .unwrap();
    assert_eq!(bindings.len(), 2);
    let taps = bindings
        .tap_set_for_generation(RuntimeGeneration::new(9), &["recorder-a", "recorder-b"])
        .unwrap();
    assert_eq!(taps.len(), 2);
    for node_id in ["recorder-a", "recorder-b"] {
        let arm = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(format!("{node_id}-arm"))),
            method: "recorders.arm".into(),
            params: Some(json!({
                "sessionId": session_id,
                "nodeId": node_id,
                "idempotencyKey": format!("node-{node_id}-arm"),
            })),
        });
        assert!(arm.error.is_none(), "recorders.arm: {arm:?}");
        let repeated_arm = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(format!("{node_id}-repeated-arm"))),
            method: "recorders.arm".into(),
            params: Some(json!({
                "sessionId": session_id,
                "nodeId": node_id,
                "idempotencyKey": format!("node-{node_id}-repeated-arm"),
            })),
        });
        assert!(repeated_arm.result.is_none());
        assert!(repeated_arm.error.is_some());
        for (index, method, frame) in [
            (2, "recorders.start", Some(0)),
            (3, "recorders.stop", Some(0)),
        ] {
            let mut params = json!({
                "sessionId": session_id,
                "nodeId": node_id,
                "idempotencyKey": format!("node-{node_id}-{index}"),
            });
            if let Some(frame) = frame {
                params["frame"] = json!(frame);
            }
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(format!("{node_id}-{index}"))),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.error.is_none(), "{method}: {response:?}");
            if method == "recorders.stop" {
                assert_eq!(response.result.unwrap()["state"], "completed");
            }
        }
    }
    assert!(plane.recorder_node_workers.is_empty());
    assert!(plane.session_stop(&session_id).is_ok());
    assert!(plane
        .attach_recorder_worker_to_node(
            &session_id,
            EntityId::new("missing"),
            Box::new(make_worker(
                &std::env::temp_dir()
                    .join(format!("audiorouter-control-node-missing-{run_id}.wav"))
            )),
        )
        .is_err());

    plane.delete_session(&session_id).unwrap();
    assert!(plane.recorder_node_workers.is_empty());

    for path in paths {
        let _ = std::fs::remove_file(path);
    }
    let _ = std::fs::remove_file(
        std::env::temp_dir().join(format!("audiorouter-control-node-missing-{run_id}.wav")),
    );
}

#[test]
fn failed_node_recorder_does_not_stop_healthy_sibling() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let healthy_path =
        std::env::temp_dir().join(format!("audiorouter-control-isolation-{run_id}.wav"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&healthy_path)
        .unwrap();
    let healthy = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap();
    let queue = Arc::new(RecordingQueue::new(4).unwrap());
    let failing = FailingTapRecorderWorker {
        tap: Arc::new(RecorderAudioTap::new(queue)),
    };
    let mut graph = session();
    for id in ["failed-recorder", "healthy-recorder"] {
        graph.nodes.push(Node {
            id: EntityId::new(id),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
    }
    let session_id = graph.id.clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    plane
        .attach_recorder_worker_to_node(
            &session_id,
            EntityId::new("failed-recorder"),
            Box::new(failing),
        )
        .unwrap();
    plane
        .attach_recorder_worker_to_node(
            &session_id,
            EntityId::new("healthy-recorder"),
            Box::new(healthy),
        )
        .unwrap();
    for node_id in ["failed-recorder", "healthy-recorder"] {
        plane
            .control_recorder_node(&EntityId::new(node_id), "recorders.arm", None)
            .unwrap();
        plane
            .control_recorder_node(&EntityId::new(node_id), "recorders.start", Some(0))
            .unwrap();
    }
    assert!(plane
        .control_recorder_node(&EntityId::new("failed-recorder"), "recorders.stop", Some(0),)
        .is_err());
    assert_eq!(
        plane.recorder_node_states[&EntityId::new("failed-recorder")].state(),
        RecorderState::Failed
    );
    let listed = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recorders.list".into(),
            params: None,
        })
        .result
        .unwrap();
    assert!(listed
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["nodeId"] == "failed-recorder" && entry["state"] == "failed"));
    assert!(plane
        .control_recorder_node(
            &EntityId::new("healthy-recorder"),
            "recorders.stop",
            Some(0),
        )
        .is_ok());
    assert!(!plane
        .recorder_node_workers
        .contains_key(&EntityId::new("healthy-recorder")));
    assert!(plane
        .recorder_node_workers
        .contains_key(&EntityId::new("failed-recorder")));
    plane.delete_session(&session_id).unwrap();
    let _ = std::fs::remove_file(healthy_path);
}

#[test]
fn mixed_legacy_and_node_recorders_share_the_active_capacity_limit() {
    let mut graph = session();
    graph.nodes.push(Node {
        id: EntityId::new("capacity-recorder"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Capacity recorder".into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![],
    });
    let session_id = graph.id.clone();
    let mut plane = ControlPlane::default();
    plane.insert_session(graph).unwrap();
    for index in 0..MAX_ACTIVE_RECORDERS {
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(0).unwrap();
        plane
            .recorders
            .insert(EntityId::new(format!("legacy-{index}")), recorder);
    }
    let queue = Arc::new(RecordingQueue::new(4).unwrap());
    plane
        .attach_recorder_worker_to_node(
            &session_id,
            EntityId::new("capacity-recorder"),
            Box::new(FailingTapRecorderWorker {
                tap: Arc::new(RecorderAudioTap::new(queue)),
            }),
        )
        .unwrap();
    assert!(plane
        .control_recorder_node(&EntityId::new("capacity-recorder"), "recorders.arm", None,)
        .is_err());
    assert_eq!(
        plane.recorder_node_states[&EntityId::new("capacity-recorder")].state(),
        RecorderState::Idle
    );
    plane.delete_session(&session_id).unwrap();
}

#[test]
fn recorder_create_api_is_idempotent_and_does_not_arm() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("audiorouter-control-create-api-{run_id}"));
    std::fs::create_dir_all(&root).unwrap();
    let mut plane = ControlPlane::with_storage("create-api", Storage::open_memory().unwrap());
    plane.configure_recording_root(&root).unwrap();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "recorders.create".into(),
        params: Some(json!({
            "sessionId": original.id,
            "recorderId": "voice",
            "format": "wavPcm24",
            "sequence": 0,
            "channels": 1,
            "sampleRate": 48000,
            "dither": true,
            "queueCapacity": 8,
            "maximumChunksPerPass": 1,
            "idempotencyKey": "create-api-1"
        })),
    };
    let first = plane.dispatch(request.clone());
    let first_result = first.result.clone().unwrap();
    assert_eq!(first_result["state"], "idle");
    assert_eq!(first_result["armed"], false);
    assert_eq!(first_result["format"], "wavPcm24");
    let second = plane.dispatch(request);
    assert_eq!(second.result.unwrap(), first_result);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(plane.recorders.len(), 0);
    let duplicate = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "recorders.create".into(),
        params: Some(json!({
            "sessionId": original.id,
            "recorderId": "voice",
            "format": "wavPcm24",
            "sequence": 1,
            "channels": 1,
            "sampleRate": 48000,
            "queueCapacity": 8,
            "maximumChunksPerPass": 1,
            "idempotencyKey": "create-api-duplicate"
        })),
    };
    let failed = plane.dispatch(duplicate);
    assert!(failed.error.is_some());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recorder_create_schema_allows_format_aware_dither_default() {
    let schema = method_input_schema("recorders.create");
    let required = schema["required"].as_array().unwrap();
    assert!(!required.iter().any(|value| value == "dither"));
    assert_eq!(
        schema["properties"]["dither"]["description"],
        "Optional; defaults to TPDF for integer WAV/FLAC and false for WAV Float32 or MP3."
    );
}

#[test]
fn recorder_create_api_can_target_a_recorder_node_and_replay_idempotently() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("audiorouter-control-node-create-{run_id}"));
    std::fs::create_dir_all(&root).unwrap();
    let mut plane = ControlPlane::with_storage("node-create", Storage::open_memory().unwrap());
    plane.configure_recording_root(&root).unwrap();
    let mut original = session();
    original.nodes.push(Node {
        id: EntityId::new("capture-recorder"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Capture recorder".into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![],
    });
    original.nodes.push(Node {
        id: EntityId::new("desktop-recorder"),
        kind: NodeKind::Recorder,
        type_version: 1,
        name: "Desktop recorder".into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![],
    });
    plane.insert_session(original.clone()).unwrap();
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "recorders.create".into(),
        params: Some(json!({
            "sessionId": original.id,
            "nodeId": "capture-recorder",
            "recorderId": "capture",
            "format": "wavPcm16",
            "sequence": 0,
            "channels": 1,
            "sampleRate": 48000,
            "queueCapacity": 8,
            "maximumChunksPerPass": 1,
            "idempotencyKey": "node-create-1"
        })),
    };
    let first = plane.dispatch(request.clone());
    let first_result = first.result.clone().unwrap();
    assert_eq!(first_result["nodeId"], "capture-recorder");
    assert_eq!(first_result["state"], "idle");
    assert_eq!(plane.recorder_node_workers.len(), 1);
    assert_eq!(plane.dispatch(request).result.unwrap(), first_result);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let second = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(2)),
        method: "recorders.create".into(),
        params: Some(json!({
            "sessionId": original.id,
            "nodeId": "desktop-recorder",
            "recorderId": "desktop",
            "format": "wavFloat32",
            "sequence": 0,
            "channels": 1,
            "sampleRate": 48000,
            "queueCapacity": 8,
            "maximumChunksPerPass": 1,
            "idempotencyKey": "node-create-2"
        })),
    });
    assert!(second.error.is_none(), "{second:?}");
    assert_eq!(second.result.unwrap()["nodeId"], "desktop-recorder");
    assert_eq!(plane.recorder_node_workers.len(), 2);
    let listed = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "recorders.list".into(),
            params: None,
        })
        .result
        .unwrap();
    let listed = listed.as_array().unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed
        .iter()
        .any(|entry| entry["nodeId"] == "capture-recorder"));
    assert!(listed
        .iter()
        .any(|entry| entry["nodeId"] == "desktop-recorder"));

    for (node_id, recorder_id) in [
        ("capture-recorder", "capture"),
        ("desktop-recorder", "desktop"),
    ] {
        for (index, method, frame) in [
            (2, "recorders.arm", None),
            (3, "recorders.start", Some(0)),
            (4, "recorders.stop", Some(0)),
        ] {
            let mut params = json!({
                "sessionId": original.id,
                "nodeId": node_id,
                "idempotencyKey": format!("node-create-{recorder_id}-{index}"),
            });
            if let Some(frame) = frame {
                params["frame"] = json!(frame);
            }
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(index)),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.error.is_none(), "{method}: {response:?}");
        }
    }
    assert!(plane.recorder_node_workers.is_empty());
    let recordings = plane
        .dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(20)),
            method: "recordings.list".into(),
            params: Some(json!({ "sessionId": original.id })),
        })
        .result
        .unwrap();
    let recordings = recordings.as_array().unwrap();
    assert_eq!(recordings.len(), 2);
    assert!(recordings
        .iter()
        .all(|recording| recording["state"] == "completed"));
    assert!(recordings
        .iter()
        .any(|recording| recording["recorderId"] == "capture"));
    assert!(recordings
        .iter()
        .any(|recording| recording["recorderId"] == "desktop"));
    assert!(recordings
        .iter()
        .any(|recording| recording["nodeId"] == "capture-recorder"));
    assert!(recordings
        .iter()
        .any(|recording| recording["nodeId"] == "desktop-recorder"));
    assert_eq!(
        recordings
            .iter()
            .find(|recording| recording["recorderId"] == "capture")
            .unwrap()["dither"],
        true
    );
    assert_eq!(
        recordings
            .iter()
            .find(|recording| recording["recorderId"] == "desktop")
            .unwrap()["dither"],
        false
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recorder_arm_enforces_the_eight_recorder_global_limit() {
    let mut plane = ControlPlane::default();
    for index in 0..=MAX_ACTIVE_RECORDERS {
        let id = EntityId::new(format!("recorder-session-{index}"));
        let mut graph = session();
        graph.id = id.clone();
        plane.insert_session(graph).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(index)),
            method: "recorders.arm".into(),
            params: Some(json!({
                "sessionId": id,
                "idempotencyKey": format!("arm-{index}")
            })),
        });
        if index < MAX_ACTIVE_RECORDERS {
            assert!(response.result.is_some(), "arm {index}: {response:?}");
        } else {
            assert!(matches!(
                response.error,
                Some(error) if error.message.contains("active recorder limit reached")
            ));
        }
    }
}

#[test]
fn recorder_lifecycle_preserves_frame_boundaries_without_audio_access() {
    let mut plane = ControlPlane::default();
    plane.create_session(session()).unwrap();
    let grant = ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::Record]);
    for (method, params) in [
        (
            "recorders.arm",
            json!({"sessionId": "session", "idempotencyKey": "arm-lifecycle"}),
        ),
        (
            "recorders.start",
            json!({"sessionId": "session", "frame": 10, "idempotencyKey": "start-lifecycle"}),
        ),
        (
            "recorders.pause",
            json!({"sessionId": "session", "frame": 20, "idempotencyKey": "pause-lifecycle"}),
        ),
        (
            "recorders.resume",
            json!({"sessionId": "session", "frame": 30, "idempotencyKey": "resume-lifecycle"}),
        ),
        (
            "recorders.split",
            json!({"sessionId": "session", "frame": 40, "idempotencyKey": "split-lifecycle"}),
        ),
        (
            "recorders.stop",
            json!({"sessionId": "session", "frame": 50, "idempotencyKey": "stop-lifecycle"}),
        ),
    ] {
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(method)),
                method: method.into(),
                params: Some(params),
            },
            &grant,
        );
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
    }
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(7)),
        method: "recorders.stop".into(),
        params: Some(
            json!({"sessionId": "session", "frame": 50, "idempotencyKey": "stop-unauthed"}),
        ),
    });
    assert!(response.error.is_some());
}

#[test]
fn recorder_idempotency_replays_and_rejects_hash_conflicts() {
    let mut plane = ControlPlane::default();
    plane.create_session(session()).unwrap();
    let grant = ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::Record]);
    let request = |frame| JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(frame)),
        method: "recorders.start".into(),
        params: Some(json!({
            "sessionId": "session",
            "frame": frame,
            "idempotencyKey": "start-once"
        })),
    };
    let arm = plane.dispatch_authorized(
        JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recorders.arm".into(),
            params: Some(json!({
                "sessionId": "session",
                "idempotencyKey": "arm-once"
            })),
        },
        &grant,
    );
    assert!(arm.error.is_none());
    let first = plane.dispatch_authorized(request(10), &grant);
    let first_result = first.result.clone().unwrap();
    let replay = plane.dispatch_authorized(request(10), &grant);
    assert_eq!(replay.result.unwrap(), first_result);
    let conflict = plane.dispatch_authorized(request(11), &grant);
    assert_eq!(
        conflict.error.unwrap().message,
        "idempotency key is already used for a different request"
    );
}

#[test]
fn recorder_checkpoint_survives_control_restart() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-recorder-control-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let mut plane = ControlPlane::with_storage("recorder-first", Storage::open(&path).unwrap());
        plane.insert_session(session()).unwrap();
        assert!(plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "recorders.arm".into(),
                params: Some(json!({"sessionId": "session", "idempotencyKey": "arm-restart"})),
            })
            .result
            .is_some());
        assert!(plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "recorders.start".into(),
                params: Some(
                    json!({"sessionId": "session", "frame": 128, "idempotencyKey": "start-restart"})
                ),
            })
            .result
            .is_some());
    }
    let mut restarted =
        ControlPlane::with_storage("recorder-second", Storage::open(&path).unwrap());
    let response = restarted.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(3)),
        method: "recorders.pause".into(),
        params: Some(json!({
            "sessionId": "session",
            "frame": 256,
            "idempotencyKey": "pause-restart"
        })),
    });
    assert_eq!(response.result.unwrap()["state"], "paused");
    let _ = std::fs::remove_file(path);
}
