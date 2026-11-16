//! Opt-in real-worker discovery, without an editor or audio device.
#![cfg(windows)]

use audiorouter_plugin_host::{
    inspect_binary, worker_clock_tick, SupervisedWorkerProcess, WorkerFrame,
};
use std::{path::PathBuf, time::Instant};

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST3_WORKER and AUDIOROUTER_TEST_VST3_EFFECT"]
fn real_worker_describes_buses_without_interrupting_processing() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_EFFECT").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let identity = inspect_binary(&plugin, &roots).unwrap();
    let mut process = SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    )
    .unwrap();
    let first = process.describe_buses(Instant::now()).unwrap();
    let layout = first.processing_layout().unwrap();
    assert_eq!(first.inputs[0].native_index, 0);
    assert_eq!(layout.output_buses().len(), 1);
    let frame = WorkerFrame::new(
        1,
        worker_clock_tick().saturating_add(10_000_000),
        2,
        vec![0.0; 512],
    )
    .unwrap();
    let output = process.process(frame, vec![], Instant::now()).unwrap();
    assert!(output.samples.iter().all(|sample| sample.is_finite()));
    assert_eq!(process.describe_buses(Instant::now()).unwrap(), first);
}

#[test]
#[ignore = "requires rebuilt VST3 worker and AUDIOROUTER_TEST_VST3_EFFECT"]
fn a_missing_requested_class_never_substitutes_another_effect() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_EFFECT").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let mut identity = inspect_binary(&plugin, &roots).unwrap();
    // Metadata is not part of the binary fingerprint. The real factory, not
    // stale moduleinfo, must decide whether the requested class still exists.
    identity.metadata.class_ids = vec!["00000000000000000000000000000000".into()];
    let result = SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    );
    assert!(
        result.is_err(),
        "a missing class must refuse instead of loading another effect"
    );
}

#[test]
#[ignore = "requires rebuilt VST3 worker and AUDIOROUTER_VST3_FIXTURE pointing at SDK AGain"]
fn each_again_class_keeps_its_own_bus_identity() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_VST3_FIXTURE").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    for (class_id, input_channels) in [
        ("84E8DE5F92554F5396FAE4133C935A18", vec![2]),
        ("41347FD6FED64094AFBB12B7DBA1D441", vec![2, 1]),
    ] {
        let mut identity = inspect_binary(&plugin, &roots).unwrap();
        identity.metadata.class_ids = vec![class_id.into()];
        let mut process = SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
            &worker,
            &identity,
            &roots,
            2,
            48_000,
            Instant::now(),
        )
        .unwrap();
        assert_eq!(
            process
                .describe_buses(Instant::now())
                .unwrap()
                .processing_layout()
                .unwrap()
                .input_buses(),
            input_channels
        );
        assert!(process
            .verify_sidechain_binding(
                &roots,
                &"f".repeat(64),
                Some(class_id),
                (2, 1),
                Instant::now()
            )
            .is_err());
        assert!(process
            .verify_sidechain_binding(
                &roots,
                &identity.sha256,
                Some("00000000000000000000000000000000"),
                (2, 1),
                Instant::now()
            )
            .is_err());
        let binding = process.verify_sidechain_binding(
            &roots,
            &identity.sha256,
            Some(class_id),
            (2, 1),
            Instant::now(),
        );
        if input_channels.len() == 2 {
            let binding = binding.unwrap();
            assert_eq!(binding.fingerprint(), identity.sha256);
            assert_eq!(binding.class_id(), Some(class_id));
            assert_eq!(binding.native_index(), 1);
            assert_eq!(binding.layout().input_buses(), &[2, 1]);
            let mut wrong_class = identity.clone();
            wrong_class.metadata.class_ids = vec!["84E8DE5F92554F5396FAE4133C935A18".into()];
            assert!(SupervisedWorkerProcess::spawn_verified_bound_vst3(
                &worker,
                &wrong_class,
                &roots,
                &binding,
                48_000,
                Instant::now(),
            )
            .is_err());
            let mut runtime = SupervisedWorkerProcess::spawn_verified_bound_vst3(
                &worker,
                &identity,
                &roots,
                &binding,
                48_000,
                Instant::now(),
            )
            .unwrap();
            // Restart must retain the selected class, not just the module and
            // bus widths. This exercises the same constructor as recovery.
            runtime.record_failure(Instant::now());
            let mut runtime = runtime
                .restart(Instant::now())
                .expect("restart selected class");
            assert_eq!(
                runtime
                    .describe_buses(Instant::now())
                    .unwrap()
                    .processing_layout()
                    .unwrap(),
                *binding.layout()
            );
            let deadline = worker_clock_tick().saturating_add(10_000_000);
            let inputs = binding
                .layout()
                .input_frames(vec![
                    WorkerFrame::new(1, deadline, 2, vec![0.1; 8]).unwrap(),
                    WorkerFrame::new(1, deadline, 1, vec![0.2; 4]).unwrap(),
                ])
                .unwrap();
            let output = runtime
                .process_buses(inputs, vec![], Instant::now())
                .unwrap();
            assert_eq!(output.frames().len(), 1);
            assert!(output.frames()[0]
                .samples
                .iter()
                .all(|value| value.is_finite()));
            assert!(runtime.shutdown().unwrap().success());
            let runtime = SupervisedWorkerProcess::spawn_verified_bound_vst3(
                &worker,
                &identity,
                &roots,
                &binding,
                48_000,
                Instant::now(),
            )
            .unwrap();
            let bridge = audiorouter_plugin_host::sidechain_bridge::SidechainRuntimeBridge::spawn(
                runtime,
                audiorouter_engine::RuntimeGeneration::new(20),
                4,
                4,
            )
            .unwrap();
            let processor: std::sync::Arc<dyn audiorouter_engine::RealtimePluginProcessor> =
                bridge.clone();
            let graph = audiorouter_engine::RuntimeGraph::prepare(
                audiorouter_engine::RuntimeGeneration::new(20),
                vec![audiorouter_engine::ProcessingStage::SidechainPlugin { processor }],
            );
            let mut main = audiorouter_engine::AudioBlock::new(2, 4).unwrap();
            let mut key = audiorouter_engine::AudioBlock::new(1, 4).unwrap();
            key.channel_mut(0).unwrap().fill(0.2);
            let mut heard = false;
            for _ in 0..20 {
                main.channel_mut(0).unwrap().fill(0.1);
                graph.process_with_sidechain(&mut main, Some(&key));
                assert!(main
                    .channel(0)
                    .unwrap()
                    .iter()
                    .all(|sample| sample.is_finite()));
                heard |= main
                    .channel(0)
                    .unwrap()
                    .iter()
                    .any(|sample| sample.abs() > 0.0);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            assert!(heard, "RuntimeGraph must receive the real worker output");
            assert_eq!(key.channel(0).unwrap(), &[0.2; 4]);
            assert_eq!(
                audiorouter_engine::RealtimePluginProcessor::health(bridge.as_ref()).state,
                audiorouter_engine::PluginWorkerState::Running
            );
            assert!(
                audiorouter_engine::RealtimePluginProcessor::latency_samples(bridge.as_ref()) >= 4
            );
            assert!(process
                .verify_sidechain_binding(
                    &roots,
                    &identity.sha256,
                    Some(class_id),
                    (2, 2),
                    Instant::now()
                )
                .is_err());
        } else {
            assert!(
                binding.is_err(),
                "the main-only class must not acquire a key port"
            );
        }
        assert!(process.shutdown().unwrap().success());
    }
}
