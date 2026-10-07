//! Tests for `recorder_workers.rs`.

use super::*;
use crate::test_support::*;

#[test]
fn all_file_formats_drain_queued_audio_before_pause_and_drop_paused_taps() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-e2e-recorders-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    for (index, format) in [
        FileRecorderFormat::Wav(WavFormat::Pcm16),
        FileRecorderFormat::Wav(WavFormat::Pcm24),
        FileRecorderFormat::Wav(WavFormat::Float32),
        FileRecorderFormat::Flac {
            bits_per_sample: 16,
        },
        FileRecorderFormat::Flac {
            bits_per_sample: 24,
        },
        FileRecorderFormat::Mp3,
    ]
    .into_iter()
    .enumerate()
    {
        let config = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id: "synthetic",
            recorder_id: "take",
            sequence: index as u64,
            format,
            channels: 2,
            sample_rate: 48_000,
            dither: false,
            queue_capacity: 4,
            maximum_chunks_per_pass: 1,
        };
        let (path, mut worker) = create_file_recorder_with_config(&policy, &config).unwrap();
        worker.arm().unwrap();
        worker.start(0).unwrap();
        let tap = worker.shared_audio_tap().unwrap();
        let mut block = AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.1);
        block.channel_mut(1).unwrap().fill(-0.1);
        tap.on_processed_block(0, &block);
        // No writer pump between queued input and the lifecycle commands.
        worker.pause(128).unwrap();
        for frame in 128..1152 {
            tap.on_processed_block(frame, &block);
        }
        worker.resume(256).unwrap();
        tap.on_processed_block(256, &block);
        let outcome = worker.finalize(384).unwrap();
        assert_eq!(outcome.state, "completed", "{format:?}");
        assert!(outcome.file_finalized && !outcome.recoverable, "{format:?}");
        let recordings = worker.finalized_recordings();
        assert_eq!(
            recordings
                .iter()
                .map(|recording| recording.frames)
                .sum::<u64>(),
            256,
            "{format:?}"
        );
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        drop(tap);
        drop(worker);
    }
    assert!(
        root.is_absolute()
            && root
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("audiorouter-e2e-recorders-")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn attached_wav_recorder_exposes_one_prebuilt_realtime_tap() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-tap-{}.wav",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let worker = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap();
    let mut plane = ControlPlane::default();
    let session_id = EntityId::new("tap-session");
    plane
        .attach_recorder_worker(session_id.clone(), Box::new(worker))
        .unwrap();

    let taps = plane.recorder_tap_set(&session_id).unwrap();
    assert_eq!(taps.len(), 1);
    assert!(!taps.is_empty());
    assert!(plane
        .recorder_tap_set(&EntityId::new("missing-session"))
        .is_err());

    let _ = std::fs::remove_file(path);
}

#[test]
fn concrete_wav_worker_drains_and_finalizes_before_session_stop() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-worker-{}.wav",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let mut worker = WavRecorderWorker::new(file, WavFormat::Float32, 1, 48_000, 8, 1).unwrap();
    worker.arm().unwrap();
    worker.start(0).unwrap();
    let tap = worker.audio_tap();
    let processor = RuntimeProcessor::default();
    processor.publish(RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]));
    let mut block = AudioBlock::new(1, 2).unwrap();
    block
        .channel_mut(0)
        .unwrap()
        .copy_from_slice(&[0.25, -0.25]);
    assert_eq!(
        processor.process_with_tap(&mut block, 0, &tap),
        Some(RuntimeGeneration::new(1))
    );

    let mut plane = ControlPlane::default();
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let mut recorder = RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(0).unwrap();
    recorder.advance(2).unwrap();
    plane.recorders.insert(original.id.clone(), recorder);
    plane
        .attach_recorder_worker_with_identity(
            original.id.clone(),
            FileRecordingIdentity {
                session_id: original.id.as_str().to_owned(),
                recorder_id: "voice".into(),
                path: path.clone(),
            },
            Box::new(worker),
        )
        .unwrap();

    let result = plane.session_stop(&original.id).unwrap();
    assert_eq!(result["recorders"][0]["fileFinalized"], true);
    let info = audiorouter_recording::inspect_wav_file(&path).unwrap();
    assert_eq!(info.frames, 2);
    assert_eq!(info.sample_rate, 48_000);
    assert_eq!(
        plane.recorders[&original.id].state(),
        RecorderState::Completed
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn float_wav_metadata_does_not_claim_requested_dither() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-float-dither-{}.wav",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let mut worker =
        WavRecorderWorker::new_with_dither(file, WavFormat::Float32, 1, 48_000, true, 8, 1)
            .unwrap();
    worker.set_library_identity(FileRecordingIdentity {
        session_id: "float-session".into(),
        recorder_id: "float-recorder".into(),
        path: path.clone(),
    });
    worker.arm().unwrap();
    worker.start(0).unwrap();
    worker
        .try_push(RecordingChunk {
            start_frame: 0,
            samples: vec![0.25],
        })
        .unwrap();
    worker.finalize(1).unwrap();

    let recordings = worker.finalized_recordings();
    assert_eq!(recordings.len(), 1);
    assert!(!recordings[0].dither);
    assert_eq!(
        recordings[0].conversion,
        "targetSampleRate=48000;channels=1;format=Float32"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn recorder_format_defaults_enable_dither_only_for_integer_output() {
    assert!(default_dither_for_format(FileRecorderFormat::Wav(
        WavFormat::Pcm16
    )));
    assert!(default_dither_for_format(FileRecorderFormat::Wav(
        WavFormat::Pcm24
    )));
    assert!(default_dither_for_format(FileRecorderFormat::Flac {
        bits_per_sample: 16,
    }));
    assert!(!default_dither_for_format(FileRecorderFormat::Wav(
        WavFormat::Float32
    )));
    assert!(!default_dither_for_format(FileRecorderFormat::Mp3));
}

#[test]
fn segmented_wav_worker_finalizes_policy_owned_segments() {
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-segmented-worker-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    let mut worker = SegmentedWavRecorderWorker::new(
        policy,
        "session:raw",
        "voice?raw",
        WavFormat::Pcm16,
        1,
        48_000,
        1,
        1,
        2,
    )
    .unwrap();
    worker.arm().unwrap();
    worker.start(0).unwrap();
    worker
        .try_push(RecordingChunk {
            start_frame: 0,
            samples: vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5],
        })
        .unwrap();

    let outcome = worker.finalize(6).unwrap();
    assert_eq!(outcome.state, "completed");
    assert!(outcome.file_finalized);
    let recordings = worker.finalized_recordings();
    assert_eq!(recordings.len(), 3);
    assert!(recordings.iter().all(|recording| {
        recording.session_id == "session:raw"
            && recording.recorder_id == "voice?raw"
            && recording.state == "completed"
            && !recording.missing
            && recording.frames == 2
            && recording.file_bytes > 44
            && !recording.dither
            && recording.conversion == "targetSampleRate=48000;channels=1;format=wav"
    }));
    drop(worker);
    let mut paths = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths.len(), 3);
    for path in &paths {
        assert_eq!(
            audiorouter_recording::inspect_wav_file(path)
                .unwrap()
                .frames,
            2
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn segmented_wav_worker_stop_persists_finalized_library_rows() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "audiorouter-control-library-root-{}-{run_id}",
        std::process::id()
    ));
    let database = std::env::temp_dir().join(format!(
        "audiorouter-control-library-{}-{run_id}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&database);
    std::fs::create_dir(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    let worker = SegmentedWavRecorderWorker::new(
        policy,
        "session",
        "voice",
        WavFormat::Pcm16,
        1,
        48_000,
        2,
        1,
        2,
    )
    .unwrap();
    worker
        .try_push(RecordingChunk {
            start_frame: 0,
            samples: vec![0.25, -0.25],
        })
        .unwrap();

    let original = session();
    let mut plane = ControlPlane::with_storage("library-test", Storage::open(&database).unwrap());
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
                "idempotencyKey": format!("library-{id}")
            }),
            None => json!({
                "sessionId": original.id,
                "idempotencyKey": format!("library-{id}")
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
    drop(plane);

    let storage = Storage::open(&database).unwrap();
    let records = storage.list_recordings(Some("session")).unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].id.starts_with("session-voice-"));
    assert!(records[0].id.ends_with("-0"));
    assert_eq!(records[0].frames, 2);
    assert!(!records[0].missing);
    assert!(std::path::Path::new(&records[0].path).is_file());
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&database);
}

#[test]
fn concrete_buffered_flac_worker_finalizes_before_session_stop() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-worker-{}.flac",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let mut worker =
        BufferedFlacRecorderWorker::new_with_dither(file, 1, 48_000, 16, true, 8, 1).unwrap();
    worker.set_library_identity(FileRecordingIdentity {
        session_id: "session".into(),
        recorder_id: "voice".into(),
        path: path.clone(),
    });
    worker.arm().unwrap();
    worker.start(0).unwrap();
    worker
        .try_push(RecordingChunk {
            start_frame: 0,
            samples: vec![0.25, -0.25],
        })
        .unwrap();

    let mut plane =
        ControlPlane::with_storage("buffered-flac-metadata", Storage::open_memory().unwrap());
    let original = session();
    plane.insert_session(original.clone()).unwrap();
    plane.session_start(&original.id).unwrap();
    let mut recorder = RecorderController::new();
    recorder.arm().unwrap();
    recorder.start(0).unwrap();
    recorder.advance(2).unwrap();
    plane.recorders.insert(original.id.clone(), recorder);
    plane
        .attach_recorder_worker(original.id.clone(), Box::new(worker))
        .unwrap();

    let result = plane.session_stop(&original.id).unwrap();
    assert_eq!(result["recorders"][0]["fileFinalized"], true);
    let info = audiorouter_recording::inspect_flac_file(&path).unwrap();
    assert_eq!(info.frames, 2);
    assert_eq!(info.sample_rate, 48_000);
    let finalized = plane
        .storage
        .as_ref()
        .unwrap()
        .list_recordings(Some("session"))
        .unwrap();
    assert_eq!(finalized.len(), 1);
    assert!(finalized[0].dither);
    assert_eq!(
        finalized[0].conversion,
        "targetSampleRate=48000;channels=1;bitsPerSample=16"
    );
    assert_eq!(
        plane.recorders[&original.id].state(),
        RecorderState::Completed
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn concrete_streaming_flac_worker_writes_frames_before_finalize() {
    let path = std::env::temp_dir().join(format!(
        "audiorouter-control-streaming-worker-{}.flac",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    let mut worker = StreamingFlacRecorderWorker::new(file, 1, 48_000, 16, false, 8, 1).unwrap();
    worker.set_library_identity(FileRecordingIdentity {
        session_id: "session".into(),
        recorder_id: "voice".into(),
        path: path.clone(),
    });
    worker.arm().unwrap();
    worker.start(0).unwrap();
    worker
        .try_push(RecordingChunk {
            start_frame: 0,
            samples: vec![0.25, -0.25],
        })
        .unwrap();
    assert_eq!(worker.drain_pending(1).unwrap(), 1);
    let outcome = worker.finalize(2).unwrap();
    assert_eq!(outcome.state, "completed");
    let info = audiorouter_recording::inspect_flac_file(&path).unwrap();
    assert_eq!(info.frames, 2);
    let rows = worker.finalized_recordings();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].format, "flac");
    assert_eq!(rows[0].frames, 2);
    assert_eq!(rows[0].file_bytes, info.file_bytes);
    assert!(!rows[0].missing);
    let _ = std::fs::remove_file(path);
}

#[test]
fn recorder_factory_selects_incremental_flac_worker() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("audiorouter-control-streaming-factory-{run_id}"));
    std::fs::create_dir_all(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    let config = FileRecorderConfig {
        version: FILE_RECORDER_CONFIG_VERSION,
        session_id: "session",
        recorder_id: "voice",
        sequence: 0,
        format: FileRecorderFormat::Flac {
            bits_per_sample: 16,
        },
        channels: 1,
        sample_rate: 48_000,
        dither: true,
        queue_capacity: 8,
        maximum_chunks_per_pass: 1,
    };
    let (path, mut worker) = create_file_recorder_with_config(&policy, &config).unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() >= 4);
    worker.arm().unwrap();
    worker.start(0).unwrap();
    let tap = worker.shared_audio_tap().unwrap();
    let mut block = AudioBlock::new(1, 2).unwrap();
    block
        .channel_mut(0)
        .unwrap()
        .copy_from_slice(&[0.25, -0.25]);
    tap.on_processed_block(0, &block);
    // Service reads progress without waiting; Finalize is the durable barrier.
    worker.drain_pending(1).unwrap();
    assert_eq!(worker.finalize(2).unwrap().state, "completed");
    assert_eq!(
        audiorouter_recording::inspect_flac_file(&path)
            .unwrap()
            .frames,
        2
    );
    let finalized = worker.finalized_recordings();
    assert_eq!(finalized.len(), 1);
    assert!(finalized[0].dither);
    assert_eq!(
        finalized[0].conversion,
        "targetSampleRate=48000;channels=1;bitsPerSample=16"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recorder_factory_selects_mp3_worker_and_persists_metadata() {
    let run_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("audiorouter-control-mp3-factory-{run_id}"));
    std::fs::create_dir_all(&root).unwrap();
    let policy = RecordingPathPolicy::new(&root).unwrap();
    let config = FileRecorderConfig {
        version: FILE_RECORDER_CONFIG_VERSION,
        session_id: "session",
        recorder_id: "voice",
        sequence: 0,
        format: FileRecorderFormat::Mp3,
        channels: 1,
        sample_rate: 48_000,
        dither: false,
        queue_capacity: 8,
        maximum_chunks_per_pass: 1,
    };
    let (path, mut worker) = create_file_recorder_with_config(&policy, &config).unwrap();
    worker.arm().unwrap();
    worker.start(0).unwrap();
    let tap = worker.shared_audio_tap().unwrap();
    let mut block = AudioBlock::new(1, 4).unwrap();
    block
        .channel_mut(0)
        .unwrap()
        .copy_from_slice(&[0.25, -0.25, 0.1, -0.1]);
    tap.on_processed_block(0, &block);
    worker.drain_pending(1).unwrap();
    assert_eq!(worker.finalize(4).unwrap().state, "completed");
    assert_eq!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("mp3")
    );
    assert!(std::fs::metadata(&path).unwrap().len() > 128);
    let finalized = worker.finalized_recordings();
    assert_eq!(finalized.len(), 1);
    assert_eq!(finalized[0].format, "mp3");
    assert_eq!(finalized[0].frames, 4);
    assert!(!finalized[0].dither);
    assert!(finalized[0].conversion.contains("bitrateKbps=192"));
    let _ = std::fs::remove_dir_all(root);
}
