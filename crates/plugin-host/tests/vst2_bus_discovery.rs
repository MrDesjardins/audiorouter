//! Real VST2 discovery without an editor or audio endpoint.
#![cfg(windows)]
use audiorouter_plugin_host::{
    inspect_binary, worker_clock_tick, ParameterEvent, SupervisedWorkerProcess, WorkerFrame,
    WorkerProcessError,
};
use std::{path::PathBuf, time::Instant};

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST2_WORKER and AUDIOROUTER_TEST_VST2_EFFECT"]
fn real_vst2_worker_describes_buses_and_keeps_processing() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_EFFECT").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let identity = inspect_binary(&plugin, &roots).unwrap();
    let mut process = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    )
    .unwrap();
    let before = process.describe_buses(Instant::now()).unwrap();
    before.processing_layout().unwrap();
    let frame = WorkerFrame::new(
        1,
        worker_clock_tick().saturating_add(10_000_000),
        2,
        vec![0.0; 512],
    )
    .unwrap();
    assert!(process
        .process(frame, vec![], Instant::now())
        .unwrap()
        .samples
        .iter()
        .all(|value| value.is_finite()));
    assert_eq!(process.describe_buses(Instant::now()).unwrap(), before);
}

/// Build tools/vst2-test-instrument with --features bus-fixture into its own
/// target directory and pass that DLL as AUDIOROUTER_TEST_VST2_BUS_FIXTURE.
#[test]
#[ignore = "requires rebuilt worker and AUDIOROUTER_TEST_VST2_BUS_FIXTURE"]
fn labeled_pins_are_discovered_but_unlabeled_surround_is_refused() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST2_BUS_FIXTURE").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let identity = inspect_binary(&plugin, &roots).unwrap();
    let mut process = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    )
    .unwrap();
    let buses = process
        .describe_buses(Instant::now())
        .expect("opcode 33 must reach the fixture");
    assert_eq!(buses.processing_layout().unwrap().input_buses(), &[2, 2]);
    assert_eq!(buses.inputs[1].native_index, 2);
    assert_eq!(buses.inputs[1].name, "Key L");
    let binding = process
        .verify_sidechain_binding(&roots, &identity.sha256, None, (2, 2), Instant::now())
        .unwrap();
    assert_eq!(binding.native_index(), 2);
    assert_eq!(binding.class_id(), None);
    let frame = WorkerFrame::new(
        1,
        worker_clock_tick().saturating_add(10_000_000),
        2,
        vec![0.0; 512],
    )
    .unwrap();
    // This fixture's parameter changes only the pin labels. The same four
    // channels must no longer be classified as main + sidechain afterward.
    process
        .process(
            frame,
            vec![ParameterEvent::new(0, 0.0, 0).unwrap()],
            Instant::now(),
        )
        .unwrap();
    assert!(
        matches!(process.describe_buses(Instant::now()), Err(WorkerProcessError::Protocol(code)) if code == "unsupportedVst2BusLayout")
    );
}
