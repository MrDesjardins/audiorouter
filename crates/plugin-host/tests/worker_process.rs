#[cfg(all(windows, feature = "test-fixtures"))]
use audiorouter_plugin_host::vst2::Vst2EditorThread;
use audiorouter_plugin_host::{
    decode_worker_message, encode_worker_message, inspect_binary, worker_clock_tick,
    EditorParentAuthorizationIssuer, PeArchitecture, PluginFormat, PluginIdentity,
    PluginStateAsset, SharedAudioLayout, SharedAudioTransport, SupervisedWorkerProcess,
    WorkerAudioBusLayout, WorkerFrame, WorkerLatency, WorkerMessage, WorkerProcess,
};
#[cfg(feature = "test-fixtures")]
use audiorouter_plugin_host::{MAX_WORKER_SAMPLE_RATE_HZ, MIN_WORKER_SAMPLE_RATE_HZ};
#[cfg(feature = "test-fixtures")]
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
#[cfg(feature = "test-fixtures")]
use std::process::{Command, Stdio};
#[cfg(feature = "test-fixtures")]
use std::time::Duration;
use std::time::Instant;

#[cfg(all(windows, feature = "test-fixtures"))]
#[link(name = "user32")]
unsafe extern "system" {
    fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: *mut std::ffi::c_void,
        menu: *mut std::ffi::c_void,
        instance: *mut std::ffi::c_void,
        parameter: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn DestroyWindow(window: *mut std::ffi::c_void) -> i32;
}

#[cfg(feature = "test-fixtures")]
fn fixture_worker_path() -> String {
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
    })
}

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
    assert!(worker.describe_parameters().unwrap().is_empty());
    let authorization = EditorParentAuthorizationIssuer::from_key([7; 32])
        .issue(1, std::process::id())
        .unwrap();
    assert!(matches!(
        worker.open_editor(&authorization),
        Err(audiorouter_plugin_host::WorkerProcessError::UnsupportedFeature(
            code
        )) if code == "editorUnavailable"
    ));
    let deadline = worker_clock_tick().saturating_add(10_000);
    let frame = WorkerFrame::new(1, deadline, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap();
    assert_eq!(worker.process(frame.clone(), Vec::new()).unwrap(), frame);
    let latency = WorkerLatency::new(240, 48_000).unwrap();
    assert_eq!(worker.report_latency(latency).unwrap(), latency);
    let asset = PluginStateAsset::new(3, vec![1, 2, 3, 4]).unwrap();
    assert!(matches!(
        worker.restore_state_for_version(asset.clone(), 2),
        Err(audiorouter_plugin_host::WorkerProcessError::State(
            audiorouter_plugin_host::StateError::VersionMismatch
        ))
    ));
    worker.restore_state(asset.clone()).unwrap();
    assert_eq!(worker.save_state().unwrap(), asset);
    assert!(worker.shutdown().unwrap().success());

    // Keep the generic framing helpers exercised in this process-level test.
    let encoded = encode_worker_message(&WorkerMessage::Ready).unwrap();
    assert_eq!(
        decode_worker_message(&encoded).unwrap(),
        WorkerMessage::Ready
    );
}

#[cfg(feature = "test-fixtures")]
#[test]
fn multi_bus_fixture_worker_negotiates_and_echoes_a_complete_bus_set() {
    let hash = "e".repeat(64);
    let mut child = Command::new(fixture_worker_path())
        .args([
            "--plugin-sha256",
            &hash,
            "--channels",
            "2",
            "--input-buses",
            "2,1",
            "--output-buses",
            "2,1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn multi-bus fixture worker");
    let stdin = child.stdin.take().expect("multi-bus worker stdin");
    let stdout = child.stdout.take().expect("multi-bus worker stdout");
    let mut writer = BufWriter::new(stdin);
    let mut reader = BufReader::new(stdout);

    let hello = audiorouter_plugin_host::read_worker_message(&mut reader).unwrap();
    let layout = WorkerAudioBusLayout::new(&[2, 1], &[2, 1]).unwrap();
    assert_eq!(
        hello,
        WorkerMessage::HelloBuses {
            protocol_version: audiorouter_plugin_host::WORKER_PROTOCOL_VERSION,
            plugin_sha256: hash,
            layout: layout.clone(),
        }
    );
    audiorouter_plugin_host::write_worker_message(&mut writer, &WorkerMessage::Ready).unwrap();

    let frames = layout
        .input_frames(vec![
            WorkerFrame::new(7, 100, 2, vec![0.1, 0.2, 0.3, 0.4]).unwrap(),
            WorkerFrame::new(7, 100, 1, vec![0.5, 0.6]).unwrap(),
        ])
        .unwrap();
    audiorouter_plugin_host::write_worker_message(
        &mut writer,
        &WorkerMessage::ProcessBuses {
            layout: layout.clone(),
            frames: frames.frames().to_vec(),
            parameters: Vec::new(),
        },
    )
    .unwrap();
    let response = audiorouter_plugin_host::read_worker_message(&mut reader).unwrap();
    assert_eq!(
        response,
        WorkerMessage::ProcessedBuses {
            layout: layout.clone(),
            frames: frames.frames().to_vec(),
        }
    );
    audiorouter_plugin_host::write_worker_message(&mut writer, &WorkerMessage::Shutdown).unwrap();
    drop(writer);
    assert!(child.wait().unwrap().success());
}

#[cfg(feature = "test-fixtures")]
#[test]
fn disposable_worker_rejects_invalid_sample_rate_before_spawn() {
    for sample_rate_hz in [MIN_WORKER_SAMPLE_RATE_HZ - 1, MAX_WORKER_SAMPLE_RATE_HZ + 1] {
        assert!(matches!(
            WorkerProcess::spawn_with_sample_rate(
                fixture_worker_path(),
                &"d".repeat(64),
                2,
                sample_rate_hz,
            ),
            Err(audiorouter_plugin_host::WorkerProcessError::Protocol(message))
                if message == "invalid sample rate"
        ));
    }
}

#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires AUDIOROUTER_VST2_FIXTURE pointing to an approved local VST2 DLL"]
fn verified_worker_loads_and_processes_an_opt_in_vst2_fixture() {
    let plugin_path = PathBuf::from(
        std::env::var("AUDIOROUTER_VST2_FIXTURE")
            .expect("set AUDIOROUTER_VST2_FIXTURE for the opt-in VST2 acceptance"),
    );
    let root = plugin_path
        .parent()
        .expect("VST2 fixture parent")
        .to_path_buf();
    let identity = inspect_binary(&plugin_path, std::slice::from_ref(&root))
        .expect("inspect VST2 fixture without loading it");
    assert_eq!(identity.format, PluginFormat::Vst2);
    assert_eq!(identity.architecture, PeArchitecture::X64);
    let worker_path = fixture_worker_path();
    let sample_rate_hz = std::env::var("AUDIOROUTER_VST2_SAMPLE_RATE")
        .ok()
        .map(|value| {
            value
                .parse::<u32>()
                .expect("valid AUDIOROUTER_VST2_SAMPLE_RATE")
        })
        .unwrap_or(44_100);
    let mut worker = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
        worker_path,
        &identity,
        std::slice::from_ref(&root),
        2,
        sample_rate_hz,
        Instant::now(),
    )
    .expect("load VST2 fixture in the isolated worker");
    let descriptors = worker
        .describe_parameters(Instant::now())
        .expect("describe VST2 parameters");
    assert!(!descriptors.is_empty());
    let editor = worker
        .describe_editor(Instant::now())
        .expect("describe VST2 editor capability");
    assert!(editor.width <= 4096 && editor.height <= 4096);
    match worker.save_state(Instant::now()) {
        Ok(state) => {
            assert!(state.bytes.len() <= 512 * 1024);
            worker
                .restore_state(state, Instant::now())
                .expect("restore VST2 state");
        }
        Err(audiorouter_plugin_host::WorkerProcessError::UnsupportedFeature(message)) => {
            assert!(message.contains("StateUnsupported"));
        }
        Err(error) => panic!("unexpected VST2 state result: {error:?}"),
    }
    let mut samples = vec![0.0; 256];
    samples[2] = 0.1;
    samples[3] = -0.1;
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 2, samples).unwrap();
    let processed = worker
        .process(
            frame,
            vec![audiorouter_plugin_host::ParameterEvent {
                parameter_id: descriptors[0].parameter_id,
                normalized_value: descriptors[0].default_value,
                sample_offset: 0,
            }],
            Instant::now(),
        )
        .expect("VST2 worker processing");
    assert!(processed.samples.iter().all(|sample| sample.is_finite()));
    let latency = worker
        .report_latency(
            WorkerLatency::new(0, sample_rate_hz).unwrap(),
            Instant::now(),
        )
        .expect("query VST2 latency");
    assert!(latency.samples <= sample_rate_hz * 10);
    assert!(worker.shutdown().unwrap().success());
}

#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires the repository-owned non-finite VST2 fixture"]
fn verified_worker_rejects_nonfinite_vst2_output() {
    let plugin_path = PathBuf::from(
        std::env::var("AUDIOROUTER_VST2_FIXTURE")
            .expect("set AUDIOROUTER_VST2_FIXTURE for the VST2 invalid-output acceptance"),
    );
    let root = plugin_path
        .parent()
        .expect("VST2 fixture parent")
        .to_path_buf();
    let identity = inspect_binary(&plugin_path, std::slice::from_ref(&root))
        .expect("inspect VST2 invalid-output fixture");
    let mut worker = SupervisedWorkerProcess::spawn_verified(
        fixture_worker_path(),
        &identity,
        std::slice::from_ref(&root),
        2,
        Instant::now(),
    )
    .expect("spawn VST2 invalid-output worker");
    let frame = WorkerFrame::new(
        1,
        worker_clock_tick().saturating_add(10_000),
        2,
        vec![0.5, -0.5, 0.0, 0.0],
    )
    .unwrap();
    let error = worker
        .process(frame, Vec::new(), Instant::now())
        .unwrap_err();
    assert!(
        matches!(
            &error,
            audiorouter_plugin_host::WorkerProcessError::Protocol(message)
                if message.contains("vst2Processing") && message.contains("NonFiniteOutput")
        ),
        "unexpected non-finite VST2 error: {error:?}"
    );
    assert_eq!(worker.state(), audiorouter_plugin_host::WorkerState::Failed);
    assert_eq!(
        worker.failure_diagnostic().unwrap().last_failure,
        Some(audiorouter_plugin_host::WorkerFailureReason::Immediate)
    );
}

#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires a repository-owned crashing or hanging VST2 fixture"]
fn verified_worker_contains_native_vst2_fault() {
    let plugin_path = PathBuf::from(
        std::env::var("AUDIOROUTER_VST2_FIXTURE")
            .expect("set AUDIOROUTER_VST2_FIXTURE for the VST2 fault acceptance"),
    );
    let root = plugin_path
        .parent()
        .expect("VST2 fixture parent")
        .to_path_buf();
    let identity = inspect_binary(&plugin_path, std::slice::from_ref(&root))
        .expect("inspect VST2 fault fixture");
    let mut worker = SupervisedWorkerProcess::spawn_verified(
        fixture_worker_path(),
        &identity,
        std::slice::from_ref(&root),
        2,
        Instant::now(),
    )
    .expect("spawn VST2 fault worker");
    let frame = WorkerFrame::new(
        1,
        worker_clock_tick().saturating_add(10_000),
        2,
        vec![0.5, -0.5, 0.0, 0.0],
    )
    .unwrap();
    assert!(worker.process(frame, Vec::new(), Instant::now()).is_err());
    assert_eq!(worker.state(), audiorouter_plugin_host::WorkerState::Failed);
    assert_eq!(worker.failure_diagnostic().unwrap().failure_count, 1);
}

#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires the repository-owned VST2 chunk-state fixture"]
fn verified_worker_applies_restored_vst2_chunk_state() {
    let plugin_path = PathBuf::from(
        std::env::var("AUDIOROUTER_VST2_FIXTURE")
            .expect("set AUDIOROUTER_VST2_FIXTURE for the VST2 state acceptance"),
    );
    assert!(plugin_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                "audiorouter-vst2-state-fixture.dll" | "audiorouter-vst2-legacy-main-fixture.dll"
            )
        }));
    let root = plugin_path
        .parent()
        .expect("VST2 fixture parent")
        .to_path_buf();
    let identity = inspect_binary(&plugin_path, std::slice::from_ref(&root))
        .expect("inspect VST2 state fixture without loading it");
    let worker_path = fixture_worker_path();
    let mut worker = SupervisedWorkerProcess::spawn_verified(
        worker_path,
        &identity,
        std::slice::from_ref(&root),
        2,
        Instant::now(),
    )
    .expect("load VST2 state fixture in the isolated worker");
    let descriptors = worker
        .describe_parameters(Instant::now())
        .expect("describe VST2 state fixture parameters");
    assert_eq!(descriptors.len(), 1);
    let saved = worker
        .save_state(Instant::now())
        .expect("save VST2 chunk state");
    assert_eq!(saved.bytes.len(), std::mem::size_of::<f32>());

    let changed = worker
        .process(
            WorkerFrame::new(
                1,
                worker_clock_tick().saturating_add(10_000),
                2,
                vec![1.0, -1.0, 0.0, 0.0],
            )
            .unwrap(),
            vec![audiorouter_plugin_host::ParameterEvent {
                parameter_id: descriptors[0].parameter_id,
                normalized_value: 0.25,
                sample_offset: 0,
            }],
            Instant::now(),
        )
        .expect("process changed VST2 state");
    assert!((changed.samples[0] - 0.25).abs() < f32::EPSILON);

    worker
        .restore_state(saved, Instant::now())
        .expect("restore VST2 chunk state");
    let restored = worker
        .process(
            WorkerFrame::new(
                2,
                worker_clock_tick().saturating_add(10_000),
                2,
                vec![1.0, -1.0, 0.0, 0.0],
            )
            .unwrap(),
            Vec::new(),
            Instant::now(),
        )
        .expect("process restored VST2 state");
    assert!((restored.samples[0] - 0.5).abs() < f32::EPSILON);
    assert!(worker.shutdown().unwrap().success());
}

#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires an editor-capable local VST2 DLL and a Windows desktop"]
fn dedicated_vst2_editor_thread_bounds_a_nonreturning_native_editor() {
    let plugin_path = PathBuf::from(
        std::env::var("AUDIOROUTER_VST2_FIXTURE")
            .expect("set AUDIOROUTER_VST2_FIXTURE for the VST2 editor acceptance"),
    );
    let parent_class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let parent_title: Vec<u16> = "AudioRouter VST2 acceptance\0".encode_utf16().collect();
    // SAFETY: The class/title buffers are NUL-terminated and live through the
    // synchronous Win32 call. A zero style keeps this parent hidden.
    let parent = unsafe {
        CreateWindowExW(
            0,
            parent_class.as_ptr(),
            parent_title.as_ptr(),
            0,
            0,
            0,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert!(!parent.is_null(), "hidden editor parent creation failed");
    let editor = Vst2EditorThread::spawn(&plugin_path).expect("spawn VST2 editor thread");
    let wrong_owner = editor.open(
        parent as usize,
        std::process::id().saturating_add(1),
        "acceptance-token",
    );
    assert!(
        wrong_owner
            .as_ref()
            .is_err_and(|error| error.contains("not valid")),
        "an HWND bound to another owner process must be rejected: {wrong_owner:?}"
    );
    let result = editor
        .open(parent as usize, std::process::id(), "acceptance-token")
        .and_then(|_| editor.close());
    drop(editor);
    // SAFETY: The handle was returned by CreateWindowExW and is no longer
    // needed after the editor has closed.
    unsafe { assert_ne!(DestroyWindow(parent), 0) };
    let error = result.expect_err("the ReaPlugs editor must remain bounded");
    assert!(
        error.contains("timed out"),
        "unexpected editor result: {error}"
    );
}

#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires an editor-capable local VST2 DLL and a Windows desktop"]
fn supervised_vst2_editor_timeout_kills_the_worker_and_records_failure() {
    let plugin_path = PathBuf::from(
        std::env::var("AUDIOROUTER_VST2_FIXTURE")
            .expect("set AUDIOROUTER_VST2_FIXTURE for the VST2 editor acceptance"),
    );
    let root = plugin_path
        .parent()
        .expect("VST2 fixture parent")
        .to_path_buf();
    let identity = inspect_binary(&plugin_path, std::slice::from_ref(&root))
        .expect("inspect VST2 fixture before spawning");
    let parent_class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let parent_title: Vec<u16> = "AudioRouter VST2 worker acceptance\0"
        .encode_utf16()
        .collect();
    // SAFETY: The class/title buffers are NUL-terminated and live through the
    // synchronous Win32 call. A zero style keeps this parent hidden.
    let parent = unsafe {
        CreateWindowExW(
            0,
            parent_class.as_ptr(),
            parent_title.as_ptr(),
            0,
            0,
            0,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert!(!parent.is_null(), "hidden editor parent creation failed");
    let mut worker = SupervisedWorkerProcess::spawn_verified(
        fixture_worker_path(),
        &identity,
        std::slice::from_ref(&root),
        2,
        Instant::now(),
    )
    .expect("spawn VST2 worker");
    let authorization = audiorouter_plugin_host::EditorParentAuthorizationIssuer::from_key([7; 32])
        .issue(parent as u64, std::process::id())
        .expect("editor authorization");
    let result = worker.open_editor(&authorization, Instant::now());
    // SAFETY: The handle was returned by CreateWindowExW and the worker has
    // been terminated before the parent is destroyed.
    unsafe { assert_ne!(DestroyWindow(parent), 0) };
    assert!(result.is_err(), "a nonreturning editor must fail closed");
    assert_eq!(worker.state(), audiorouter_plugin_host::WorkerState::Failed);
    assert_eq!(worker.into_supervisor().failure_count(), 1);
}

#[cfg(feature = "test-fixtures")]
#[test]
fn disposable_worker_process_round_trips_non_empty_parameter_descriptors() {
    let mut worker =
        WorkerProcess::spawn_fixture(fixture_worker_path(), &"e".repeat(64), 1, "descriptors")
            .expect("spawn descriptor fixture");
    let descriptors = worker.describe_parameters().expect("describe parameters");
    assert_eq!(descriptors.len(), 2);
    assert_eq!(descriptors[0].title, "Mix");
    assert_eq!(descriptors[0].parameter_id, 1);
    assert_eq!(descriptors[1].title, "Output");
    assert_eq!(descriptors[1].minimum, 0.0);
    assert!(worker
        .shutdown()
        .expect("shutdown descriptor fixture")
        .success());
}

#[cfg(feature = "test-fixtures")]
#[test]
fn disposable_worker_process_exercises_dynamic_latency_updates() {
    let mut worker =
        WorkerProcess::spawn_fixture(fixture_worker_path(), &"f".repeat(64), 1, "latency")
            .expect("spawn latency fixture");
    let first = worker
        .report_latency(WorkerLatency::new(128, 48_000).unwrap())
        .expect("first latency report");
    assert_eq!(first.samples, 192);
    let second = worker.report_latency(first).expect("second latency report");
    assert_eq!(second.samples, 256);
    assert_eq!(second.sample_rate_hz, 48_000);
    assert!(worker
        .shutdown()
        .expect("shutdown latency fixture")
        .success());
}

#[test]
fn verified_supervised_launch_rechecks_the_scanned_plugin_identity() {
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
    let root = std::env::temp_dir().join(format!("audiorouter-identity-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let plugin_path = root.join("effect.vst3");
    std::fs::copy(&worker_path, &plugin_path).unwrap();
    let identity = inspect_binary(&plugin_path, std::slice::from_ref(&root)).unwrap();
    let now = Instant::now();
    let mut worker = SupervisedWorkerProcess::spawn_verified(
        &worker_path,
        &identity,
        std::slice::from_ref(&root),
        1,
        now,
    )
    .expect("verified worker launch");
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 1, vec![0.25]).unwrap();
    assert_eq!(
        worker.process(frame.clone(), Vec::new(), now).unwrap(),
        frame
    );
    assert!(worker.shutdown().unwrap().success());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn verified_supervised_launch_rejects_missing_identity_before_spawning() {
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
    let root = std::env::temp_dir().join(format!(
        "audiorouter-missing-identity-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let identity = PluginIdentity {
        path: root.join("missing.vst3"),
        binary_path: root.join("missing.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: "0".repeat(64),
        metadata: Default::default(),
    };
    let result = SupervisedWorkerProcess::spawn_verified(
        worker_path,
        &identity,
        std::slice::from_ref(&root),
        1,
        Instant::now(),
    );
    assert!(matches!(
        result,
        Err(audiorouter_plugin_host::WorkerProcessError::PluginIdentity(
            audiorouter_plugin_host::IdentityVerificationError::Inspection(
                audiorouter_plugin_host::InspectionError::Missing
            )
        ))
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn supervised_worker_round_trips_versioned_state_without_restarting() {
    let hash = "7".repeat(64);
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
        metadata: Default::default(),
    };
    let now = Instant::now();
    let mut worker = SupervisedWorkerProcess::spawn(worker_path, &identity, 1, now)
        .expect("spawn supervised worker");
    let asset = PluginStateAsset::new(8, vec![9, 8, 7]).unwrap();
    assert!(matches!(
        worker.restore_state_for_version(asset.clone(), 7, now),
        Err(audiorouter_plugin_host::WorkerProcessError::State(
            audiorouter_plugin_host::StateError::VersionMismatch
        ))
    ));
    assert_eq!(
        worker.state(),
        audiorouter_plugin_host::WorkerState::Running
    );
    worker
        .restore_state_for_version(asset.clone(), 8, now)
        .unwrap();
    assert_eq!(worker.save_state(now).unwrap(), asset);
    assert!(worker.shutdown().unwrap().success());
}

#[cfg(feature = "test-fixtures")]
#[test]
fn supervised_worker_fixture_preserves_dynamic_latency_and_descriptors() {
    let hash = "8".repeat(64);
    let worker_path = fixture_worker_path();
    let identity = PluginIdentity {
        path: PathBuf::from("effect.vst3"),
        binary_path: PathBuf::from("effect.vst3"),
        format: PluginFormat::Vst3,
        architecture: PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
        metadata: Default::default(),
    };
    let now = Instant::now();
    let mut worker =
        SupervisedWorkerProcess::spawn_fixture(worker_path, &identity, 1, "latency", now)
            .expect("spawn supervised latency fixture");
    let first = worker
        .report_latency(WorkerLatency::new(128, 48_000).unwrap(), now)
        .expect("supervised latency report");
    assert_eq!(first.samples, 192);
    assert_eq!(
        worker.state(),
        audiorouter_plugin_host::WorkerState::Running
    );
    assert!(worker.shutdown().unwrap().success());

    let mut descriptor_worker = SupervisedWorkerProcess::spawn_fixture(
        fixture_worker_path(),
        &identity,
        1,
        "descriptors",
        now,
    )
    .expect("spawn supervised descriptor fixture");
    let descriptors = descriptor_worker
        .describe_parameters(now)
        .expect("supervised parameter description");
    assert_eq!(descriptors.len(), 2);
    assert_eq!(descriptors[1].parameter_id, 2);
    assert!(descriptor_worker.shutdown().unwrap().success());
}

#[test]
fn disposable_worker_rejects_a_runtime_sample_rate_change() {
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
    let mut worker = WorkerProcess::spawn(worker_path, &hash, 1).expect("spawn worker client");
    let initial = WorkerLatency::new(128, 48_000).unwrap();
    assert_eq!(worker.report_latency(initial).unwrap(), initial);

    let rejected = worker.report_latency(WorkerLatency::new(128, 44_100).unwrap());
    assert!(
        matches!(
            &rejected,
            Err(audiorouter_plugin_host::WorkerProcessError::Protocol(code))
                if code.starts_with("session:InvalidLatency")
        ),
        "unexpected rejection: {rejected:?}"
    );
    assert!(!worker.shutdown().unwrap().success());
}

#[test]
fn disposable_worker_fails_closed_when_state_is_unavailable() {
    let hash = "6".repeat(64);
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
    let mut worker = WorkerProcess::spawn(worker_path, &hash, 1).expect("spawn worker client");
    assert!(matches!(
        worker.save_state(),
        Err(audiorouter_plugin_host::WorkerProcessError::Protocol(code))
            if code == "stateUnavailable"
    ));
    assert!(!worker.shutdown().unwrap().success());
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
        metadata: Default::default(),
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
        metadata: Default::default(),
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
    assert_eq!(
        worker.failure_diagnostic().unwrap().last_failure,
        Some(audiorouter_plugin_host::WorkerFailureReason::HeartbeatTimeout)
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
        metadata: Default::default(),
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
    let status = worker.shutdown().unwrap();
    assert!(!status.success());
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
        metadata: Default::default(),
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
    let (error, supervisor) = match final_worker.poll_and_restart(start) {
        Ok(_) => panic!("quarantine must prevent automatic replacement"),
        Err(result) => result,
    };
    assert!(
        matches!(
            error,
            audiorouter_plugin_host::WorkerProcessError::Quarantined
        ),
        "unexpected quarantine error: {error:?}"
    );
    assert_eq!(
        supervisor.state(),
        audiorouter_plugin_host::WorkerState::Quarantined
    );
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
        metadata: Default::default(),
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

#[cfg(feature = "test-fixtures")]
#[test]
fn controlled_worker_crash_is_reaped_without_host_panic() {
    let worker = WorkerProcess::spawn_fixture(fixture_worker_path(), &"2".repeat(64), 1, "crash")
        .expect("spawn crash fixture");
    std::thread::sleep(Duration::from_millis(20));
    let status = worker.shutdown().expect("reap crashed fixture");
    assert!(!status.success());
}

#[cfg(feature = "test-fixtures")]
#[test]
fn controlled_worker_hang_is_killed_by_bounded_shutdown() {
    let worker = WorkerProcess::spawn_fixture(fixture_worker_path(), &"3".repeat(64), 1, "hang")
        .expect("spawn hang fixture");
    assert!(matches!(
        worker.shutdown_with_timeout(Duration::from_millis(20)),
        Err(audiorouter_plugin_host::WorkerProcessError::Timeout)
    ));
}

#[cfg(feature = "test-fixtures")]
#[test]
fn controlled_worker_invalid_output_is_rejected_at_reader_boundary() {
    let mut worker =
        WorkerProcess::spawn_fixture(fixture_worker_path(), &"4".repeat(64), 1, "invalid-output")
            .expect("spawn invalid-output fixture");
    let frame =
        WorkerFrame::new(1, worker_clock_tick().saturating_add(10_000), 1, vec![0.25]).unwrap();
    assert!(matches!(
        worker.process(frame, Vec::new()),
        Err(audiorouter_plugin_host::WorkerProcessError::Message(_))
    ));
    let _ = worker.shutdown();
}

#[cfg(feature = "test-fixtures")]
#[test]
fn supervised_worker_contains_a_real_hang_fixture() {
    let hash = "5".repeat(64);
    let identity = audiorouter_plugin_host::PluginIdentity {
        path: PathBuf::from("fixture.vst3"),
        binary_path: PathBuf::from("fixture.vst3"),
        format: audiorouter_plugin_host::PluginFormat::Vst3,
        architecture: audiorouter_plugin_host::PeArchitecture::X64,
        file_bytes: 1,
        sha256: hash,
        metadata: Default::default(),
    };
    let now = Instant::now();
    let worker =
        SupervisedWorkerProcess::spawn_fixture(fixture_worker_path(), &identity, 1, "hang", now)
            .expect("spawn supervised hang fixture");
    let mut worker = worker;
    assert_eq!(
        worker.poll(
            now + audiorouter_plugin_host::WORKER_HEARTBEAT_TIMEOUT + Duration::from_millis(1)
        ),
        audiorouter_plugin_host::WorkerState::Failed
    );
    let status = worker.shutdown().expect("reap supervised hang fixture");
    assert!(!status.success());
}
