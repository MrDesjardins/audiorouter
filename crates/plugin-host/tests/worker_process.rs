use audiorouter_plugin_host::{
    decode_worker_message, encode_worker_message, WorkerFrame, WorkerLatency, WorkerMessage,
    WorkerProcess,
};

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
    let frame = WorkerFrame::new(1, 100, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap();
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
