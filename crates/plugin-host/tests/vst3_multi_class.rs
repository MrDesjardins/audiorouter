//! Opt-in: one VST3 module with two effect classes. The worker must load
//! exactly the class that was asked for, and one class's saved state must not
//! be accepted by the other.
//!
//! Build with `tools/m06-vst3-worker/fixtures/build-fixture.ps1 -Name TwoClasses`
//! and set `AUDIOROUTER_TEST_VST3_TWOCLASSES_FIXTURE` to the
//! `TwoClasses.vst3\Contents\x86_64-win\TwoClasses.vst3` binary.
#![cfg(windows)]

use audiorouter_plugin_host::{
    inspect_binary, worker_clock_tick, SupervisedWorkerProcess, WorkerFrame,
};
use std::{path::PathBuf, time::Instant};

/// Class IDs as the four FUID words written in `twoclasses.cpp` (the worker
/// accepts either byte order), with the gain each class applies.
fn full_ids() -> [(&'static str, &'static str, f32); 2] {
    [
        ("Quiet", "41525433515549455400000000000001", 0.5),
        ("Loud", "415254334C4F55440000000000000002", 2.0),
    ]
}

fn start(class_id: &str) -> Result<SupervisedWorkerProcess, String> {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_TWOCLASSES_FIXTURE").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let mut identity = inspect_binary(&plugin, &roots).unwrap();
    identity.metadata.class_ids = vec![class_id.to_owned()];
    SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    )
    .map_err(|error| format!("{error:?}"))
}

fn first_output(process: &mut SupervisedWorkerProcess) -> f32 {
    let mut last = 0.0;
    for sequence in 1..=4 {
        let frame = WorkerFrame::new(
            sequence,
            worker_clock_tick().saturating_add(10_000_000),
            2,
            vec![0.25; 512],
        )
        .unwrap();
        let output = process.process(frame, vec![], Instant::now()).unwrap();
        last = output.samples[0];
    }
    last
}

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST3_WORKER and AUDIOROUTER_TEST_VST3_TWOCLASSES_FIXTURE"]
fn each_class_of_a_module_loads_alone_and_keeps_its_own_state() {
    let ids = full_ids();
    for (name, id, gain) in ids {
        let mut process = start(id).unwrap_or_else(|error| panic!("{name}: {error}"));
        let heard = first_output(&mut process);
        assert!(
            (heard - 0.25 * gain).abs() < 1.0e-4,
            "{name} must apply its own gain {gain}, heard {heard}"
        );
    }
    // A class that is not in the module is refused, never replaced.
    assert!(start("00000000000000000000000000000000").is_err());
    // State written by one class is refused by the other and accepted by itself.
    let mut quiet = start(ids[0].1).unwrap();
    let state = quiet.save_state(Instant::now()).unwrap();
    quiet
        .restore_state(state.clone(), Instant::now())
        .expect("a class takes back its own state");
    let mut loud = start(ids[1].1).unwrap();
    assert!(
        loud.restore_state(state, Instant::now()).is_err(),
        "another class's state must not be accepted"
    );
}
