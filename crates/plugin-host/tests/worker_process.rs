use audiorouter_plugin_host::{
    decode_worker_message, encode_worker_message, read_worker_message, write_worker_message,
    WorkerFrame, WorkerLatency, WorkerMessage, WORKER_PROTOCOL_VERSION,
};
use std::io::{BufReader, BufWriter};
use std::process::{Command, Stdio};

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
    let mut child = Command::new(worker_path)
        .args(["--plugin-sha256", &hash, "--channels", "2"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn disposable worker");
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut writer = BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);

    assert_eq!(
        read_worker_message(&mut reader).unwrap(),
        WorkerMessage::Hello {
            protocol_version: WORKER_PROTOCOL_VERSION,
            plugin_sha256: hash,
            channels: 2,
        }
    );
    write_worker_message(&mut writer, &WorkerMessage::Ready).unwrap();
    let frame = WorkerFrame::new(1, 100, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap();
    write_worker_message(
        &mut writer,
        &WorkerMessage::Process {
            frame: frame.clone(),
            parameters: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(
        read_worker_message(&mut reader).unwrap(),
        WorkerMessage::Processed { frame }
    );
    let latency = WorkerLatency::new(240, 48_000).unwrap();
    write_worker_message(&mut writer, &WorkerMessage::Latency(latency)).unwrap();
    assert_eq!(
        read_worker_message(&mut reader).unwrap(),
        WorkerMessage::Latency(latency)
    );
    write_worker_message(&mut writer, &WorkerMessage::Shutdown).unwrap();
    assert!(child.wait().unwrap().success());

    // Keep the generic framing helpers exercised in this process-level test.
    let encoded = encode_worker_message(&WorkerMessage::Ready).unwrap();
    assert_eq!(
        decode_worker_message(&encoded).unwrap(),
        WorkerMessage::Ready
    );
}
