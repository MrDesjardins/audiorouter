#![cfg(all(windows, feature = "test-fixtures"))]
use audiorouter_plugin_host::{
    inspect_binary, worker_clock_tick, ParameterEvent, SupervisedWorkerProcess, WorkerFrame,
};
use std::{path::PathBuf, time::Instant};
fn fixture_worker_path() -> String {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("audiorouter-plugin-worker.exe")
        .to_string_lossy()
        .into_owned()
}

/// The repository's native VST2 fixture implements y = normalized_mix * x.
/// Test that transfer law through the isolated production worker, including
/// stereo isolation and frame-offset automation, rather than just checking
/// that a parameter change makes some sample different.
#[cfg(all(windows, feature = "test-fixtures"))]
#[test]
#[ignore = "requires the approved repository audiorouter-vst2-state-fixture.dll"]
fn native_vst2_fixture_matches_exact_gain_and_sample_offset_automation() {
    let path = PathBuf::from(std::env::var("AUDIOROUTER_VST2_FIXTURE").unwrap());
    assert_eq!(
        path.file_name().unwrap(),
        "audiorouter-vst2-state-fixture.dll"
    );
    let root = path.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
    for rate in [44_100, 48_000, 96_000] {
        let mut worker = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
            fixture_worker_path(),
            &identity,
            std::slice::from_ref(&root),
            2,
            rate,
            Instant::now(),
        )
        .unwrap();
        let descriptors = worker.describe_parameters(Instant::now()).unwrap();
        assert_eq!(descriptors.len(), 1);
        let input: Vec<f32> = (0..256)
            .map(|index| {
                if index % 2 == 0 {
                    (index as f32 - 128.0) / 512.0
                } else {
                    (64.0 - index as f32) / 256.0
                }
            })
            .collect();
        for (sequence, gain) in [0.0, 0.25, 0.5, 0.75, 1.0].into_iter().enumerate() {
            let result = worker
                .process(
                    WorkerFrame::new(
                        sequence as u64 + 1,
                        worker_clock_tick().saturating_add(10_000),
                        2,
                        input.clone(),
                    )
                    .unwrap(),
                    vec![ParameterEvent {
                        parameter_id: descriptors[0].parameter_id,
                        normalized_value: gain,
                        sample_offset: 0,
                    }],
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(result.samples.len(), input.len());
            let error = result
                .samples
                .iter()
                .zip(&input)
                .map(|(actual, input)| (actual - input * gain).abs())
                .fold(0.0_f32, f32::max);
            assert!(
                error < 1e-6,
                "rate {rate}, gain {gain}, sample error {error}"
            );
            if let Some(root) = std::env::var_os("AUDIOROUTER_SIGNAL_ARTIFACTS") {
                let root = PathBuf::from(root).join(format!("native-vst2-{rate}-{gain}"));
                std::fs::create_dir_all(&root).unwrap();
                std::fs::write(root.join("samples.json"), serde_json::to_vec_pretty(&serde_json::json!({
                    "sampleRate":rate,"channels":2,"gain":gain,"input":input,"output":result.samples,
                    "maxSampleError":error,"tolerance":1e-6
                })).unwrap()).unwrap();
            }
        }
        let result = worker
            .process(
                WorkerFrame::new(
                    6,
                    worker_clock_tick().saturating_add(10_000),
                    2,
                    input.clone(),
                )
                .unwrap(),
                vec![
                    ParameterEvent {
                        parameter_id: descriptors[0].parameter_id,
                        normalized_value: 0.25,
                        sample_offset: 0,
                    },
                    ParameterEvent {
                        parameter_id: descriptors[0].parameter_id,
                        normalized_value: 0.75,
                        sample_offset: 64,
                    },
                ],
                Instant::now(),
            )
            .unwrap();
        for (index, (actual, input)) in result.samples.iter().zip(&input).enumerate() {
            let expected = input * if index / 2 < 64 { 0.25 } else { 0.75 };
            assert!(
                (actual - expected).abs() < 1e-6,
                "rate {rate}, interleaved sample {index}: {actual} != {expected}"
            );
        }
        if let Some(root) = std::env::var_os("AUDIOROUTER_SIGNAL_ARTIFACTS") {
            let root = PathBuf::from(root).join(format!("native-vst2-{rate}"));
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join("samples.json"),serde_json::to_vec_pretty(&serde_json::json!({"sampleRate":rate,"channels":2,"input":input,"output":result.samples,"gains":[0.25,0.75],"changeAtFrame":64,"tolerance":1e-6})).unwrap()).unwrap();
        }
    }
}

#[test]
#[ignore = "requires approved SDK AGain module and repository native VST3 worker"]
fn native_vst3_again_matches_exact_gain_and_bypass_transfer() {
    let path = PathBuf::from(std::env::var("AUDIOROUTER_VST3_FIXTURE").unwrap());
    assert_eq!(path.file_name().unwrap(), "again.vst3");
    let worker_path = PathBuf::from(std::env::var("AUDIOROUTER_VST3_NATIVE_WORKER").unwrap());
    let root = path.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
    for rate in [44_100, 48_000, 96_000] {
        let mut worker = SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
            &worker_path,
            &identity,
            std::slice::from_ref(&root),
            2,
            rate,
            Instant::now(),
        )
        .unwrap();
        let input: Vec<f32> = (0..256)
            .map(|index| {
                if index % 2 == 0 {
                    (index as f32 - 128.0) / 512.0
                } else {
                    (64.0 - index as f32) / 256.0
                }
            })
            .collect();
        for (sequence, (gain, bypass)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .into_iter()
            .flat_map(|gain| [(gain, false), (gain, true)])
            .enumerate()
        {
            // SDK againparamids.h: gain=0, read-only meter=1, bypass=2.
            let result = worker
                .process(
                    WorkerFrame::new(
                        sequence as u64 + 1,
                        worker_clock_tick().saturating_add(10_000),
                        2,
                        input.clone(),
                    )
                    .unwrap(),
                    vec![
                        ParameterEvent {
                            parameter_id: 0,
                            normalized_value: gain,
                            sample_offset: 0,
                        },
                        ParameterEvent {
                            parameter_id: 2,
                            normalized_value: if bypass { 1.0 } else { 0.0 },
                            sample_offset: 0,
                        },
                    ],
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(result.samples.len(), input.len());
            let effective_gain = if bypass { 1.0 } else { gain };
            let error = result
                .samples
                .iter()
                .zip(&input)
                .map(|(actual, input)| (actual - input * effective_gain).abs())
                .fold(0.0_f32, f32::max);
            assert!(error < 1e-6, "AGain {rate}/{gain}/{bypass}: {error}");
            if let Some(root) = std::env::var_os("AUDIOROUTER_SIGNAL_ARTIFACTS") {
                let root = PathBuf::from(root).join(format!("native-vst3-{rate}-{gain}-{bypass}"));
                std::fs::create_dir_all(&root).unwrap();
                std::fs::write(root.join("samples.json"),serde_json::to_vec_pretty(&serde_json::json!({"sampleRate":rate,"channels":2,"gain":gain,"bypass":bypass,"input":input,"output":result.samples,"maxSampleError":error,"tolerance":1e-6})).unwrap()).unwrap();
            }
        }
    }
}
