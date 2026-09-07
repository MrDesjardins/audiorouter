use audiorouter_plugin_host::{
    decode_worker_message, encode_worker_message, worker_clock_tick, PeArchitecture, PluginFormat,
    PluginIdentity, SharedAudioLayout, SharedAudioTransport, SupervisedWorkerProcess, WorkerFrame,
    WorkerLatency, WorkerMessage, WorkerProcess,
};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn disposable_worker_process_round_trips_control_and_audio_frames() {
    let hash = "d".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let mut worker = WorkerProcess::spawn(worker_path, &hash, 2).expect("spawn worker client");
    let deadline = worker_clock_tick().saturating_add(10_000);
    let frame = WorkerFrame::new(1, deadline, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap();
    assert_eq!(worker.process(frame.clone(), Vec::new()).unwrap(), frame);
    let latency = WorkerLatency::new(240, 48_000).unwrap();
    assert_eq!(worker.report_latency(latency).unwrap(), latency);
    assert!(worker.shutdown().unwrap().success());

    // Keep the generic framing helpers exercised in this process-level test.
    let encoded = encode_worker_message(&WorkerMessage::Ready).unwrap();
    assert_eq!(
        decode_worker_message(&encoded).unwrap(),
        WorkerMessage::Ready
    );
}

#[test]
fn supervised_worker_refreshes_heartbeat_on_successful_processing() {
    let hash = "f".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let identity = PluginIdentity {
        path: PathBuf::from("effect.vst3"),
        binary_path: PathBuf::from("effect.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
    };
    let start = Instant::now();
    let mut worker = SupervisedWorkerProcess::spawn(worker_path, &identity, 1, start)
        .expect("spawn supervised worker");
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 1, vec![0.5]).unwrap();
    assert_eq!(
        worker.process(frame.clone(), Vec::new(), start).unwrap(),
        frame
    );
    assert_eq!(
        worker.state(),
        audiorouter_plugin_host::WorkerState::Running
    );
    let mut worker = worker
        .poll_and_restart(start + audiorouter_plugin_host::WORKER_HEARTBEAT_TIMEOUT)
        .expect("healthy worker must not be replaced");
    assert_eq!(
        worker.state(),
        audiorouter_plugin_host::WorkerState::Running
    );
    let next = WorkerFrame::new(
        2,
        worker_clock_tick().saturating_add(10_000),
        1,
        vec![-0.25],
    )
    .unwrap();
    assert_eq!(
        worker.process(next.clone(), Vec::new(), start).unwrap(),
        next
    );
    assert!(worker.shutdown().unwrap().success());
}

#[test]
fn supervised_worker_fails_closed_after_heartbeat_timeout() {
    let hash = "a".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let identity = PluginIdentity {
        path: PathBuf::from("effect.vst3"),
        binary_path: PathBuf::from("effect.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
    };
    let start = Instant::now();
    let mut worker = SupervisedWorkerProcess::spawn(worker_path, &identity, 1, start)
        .expect("spawn supervised worker");
    assert_eq!(
        worker.poll(
            start
                + audiorouter_plugin_host::WORKER_HEARTBEAT_TIMEOUT
                + std::time::Duration::from_millis(1)
        ),
        audiorouter_plugin_host::WorkerState::Failed
    );
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 1, vec![0.5]).unwrap();
    let error = worker
        .process(frame.clone(), Vec::new(), start)
        .unwrap_err();
    assert!(
        matches!(error, audiorouter_plugin_host::WorkerProcessError::Protocol(message) if message.contains("not running under supervision"))
    );
    let mut replacement = worker
        .poll_and_restart(start)
        .expect("recover failed worker");
    assert_eq!(
        replacement.state(),
        audiorouter_plugin_host::WorkerState::Running
    );
    assert_eq!(
        replacement
            .process(frame.clone(), Vec::new(), start)
            .unwrap(),
        frame
    );
    assert!(replacement.shutdown().unwrap().success());
}

#[test]
fn supervised_worker_accepts_outer_process_failure_reports() {
    let hash = "b".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let identity = PluginIdentity {
        path: PathBuf::from("effect.vst3"),
        binary_path: PathBuf::from("effect.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
    };
    let start = Instant::now();
    let mut worker = SupervisedWorkerProcess::spawn(worker_path, &identity, 1, start)
        .expect("spawn supervised worker");
    assert_eq!(
        worker.record_failure(start),
        audiorouter_plugin_host::WorkerState::Failed
    );
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 1, vec![0.5]).unwrap();
    let error = worker.process(frame, Vec::new(), start).unwrap_err();
    assert!(
        matches!(error, audiorouter_plugin_host::WorkerProcessError::Protocol(message) if message.contains("not running under supervision"))
    );
    assert!(worker.shutdown().unwrap().success());
}

#[test]
fn supervised_worker_replacement_preserves_quarantine_history() {
    let hash = "c".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let identity = PluginIdentity {
        path: PathBuf::from("effect.vst3"),
        binary_path: PathBuf::from("effect.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
    };
    let start = Instant::now();
    let mut worker = SupervisedWorkerProcess::spawn(worker_path.clone(), &identity, 1, start)
        .expect("spawn supervised worker");
    assert_eq!(
        worker.record_failure(start),
        audiorouter_plugin_host::WorkerState::Failed
    );
    let mut replacement = worker.restart(start).expect("spawn replacement");
    assert_eq!(
        replacement.record_failure(start),
        audiorouter_plugin_host::WorkerState::Failed
    );
    let mut final_worker = replacement.restart(start).expect("spawn final replacement");
    assert_eq!(
        final_worker.record_failure(start),
        audiorouter_plugin_host::WorkerState::Quarantined
    );
    assert_eq!(
        final_worker.state(),
        audiorouter_plugin_host::WorkerState::Quarantined
    );
    assert!(final_worker.shutdown().unwrap().success());
}

#[test]
fn disposable_worker_process_round_trips_shared_audio_frames() {
    let hash = "e".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let stem = format!("audiorouter-worker-shared-{}", std::process::id());
    let input_path = std::env::temp_dir().join(format!("{stem}-input"));
    let output_path = std::env::temp_dir().join(format!("{stem}-output"));
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);
    let transport = SharedAudioTransport::create(
        &input_path,
        &output_path,
        SharedAudioLayout::new(2).unwrap(),
    )
    .expect("create shared transport");
    let identity = PluginIdentity {
        path: PathBuf::from("effect.vst3"),
        binary_path: PathBuf::from("effect.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
    };
    let start = Instant::now();
    let mut worker =
        SupervisedWorkerProcess::spawn_shared(worker_path, &identity, 2, transport, start)
            .expect("spawn worker");
    let frame = WorkerFrame::new(
        1,
        worker_clock_tick().saturating_add(10_000),
        2,
        vec![0.25, -0.25, 0.0, 0.1],
    )
    .unwrap();
    assert_eq!(
        worker
            .process_shared(frame.clone(), Vec::new(), start)
            .unwrap(),
        frame
    );
    assert_eq!(
        worker.record_failure(start),
        audiorouter_plugin_host::WorkerState::Failed
    );
    let mut replacement = worker.restart(start).expect("restart shared worker");
    let replacement_frame = WorkerFrame::new(
        2,
        worker_clock_tick().saturating_add(10_000),
        2,
        vec![0.5, -0.5, 0.2, -0.2],
    )
    .unwrap();
    assert_eq!(
        replacement
            .process_shared(replacement_frame.clone(), Vec::new(), start)
            .unwrap(),
        replacement_frame
    );
    assert!(replacement.shutdown().unwrap().success());
    std::fs::remove_file(input_path).unwrap();
    std::fs::remove_file(output_path).unwrap();
}

#[test]
fn disposable_worker_rejects_duplicate_sequence_frames() {
    let hash = "f".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let mut worker = WorkerProcess::spawn(worker_path, &hash, 1).expect("spawn worker");
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 1, vec![0.25]).unwrap();
    assert_eq!(worker.process(frame.clone(), Vec::new()).unwrap(), frame);
    let error = worker.process(frame, Vec::new()).unwrap_err();
    assert!(matches!(
        error,
        audiorouter_plugin_host::WorkerProcessError::Protocol(code)
            if code.contains("SequenceRegression")
    ));
}

#[test]
fn disposable_worker_rejects_expired_deadline_frames() {
    let hash = "1".repeat(64);
    let worker_path =
        std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
            let test_exe = std::env::current_exe().expect("integration test path");
            test_exe
                .parent()
                .and_then(|deps| deps.parent())
                .expect("Cargo target directory")
                .join(if cfg!(windows) {
                    "audiorouter-plugin-worker.exe"
                } else {
                    "audiorouter-plugin-worker"
                })
                .to_string_lossy()
                .into_owned()
        });
    let mut worker = WorkerProcess::spawn(worker_path, &hash, 1).expect("spawn worker");
    let frame = WorkerFrame::new(1, 0, 1, vec![0.25]).unwrap();
    let error = worker.process(frame, Vec::new()).unwrap_err();
    assert!(matches!(
        error,
        audiorouter_plugin_host::WorkerProcessError::Protocol(code)
            if code.contains("DeadlineExpired")
    ));
}
