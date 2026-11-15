//! Opt-in real-worker check of read-only VST3 program lists (IUnitInfo).
#![cfg(windows)]

use audiorouter_plugin_host::{inspect_binary, SupervisedWorkerProcess};
use std::{path::PathBuf, time::Instant};

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST3_WORKER and AUDIOROUTER_TEST_VST3_EFFECT"]
fn a_vst3_lists_bounded_program_names_and_refuses_selection() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_EFFECT").unwrap());
    let minimum: usize = std::env::var("AUDIOROUTER_TEST_VST3_PROGRAMS_MIN")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
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
    let programs = process.programs(Instant::now()).unwrap();
    println!("{} programs", programs.len());
    assert!(programs.len() >= minimum, "expected {minimum}+ programs");
    assert!(programs.len() <= 256);
    for (position, program) in programs.iter().enumerate() {
        assert_eq!(program.id as usize, position);
        assert!(!program.name.is_empty() && program.name.len() <= 128);
        assert!(!program.name.chars().any(char::is_control));
    }
    // The listing is repeatable.
    assert_eq!(process.programs(Instant::now()).unwrap(), programs);
    // Selection is not offered for VST3. The live bridge refuses it before the
    // worker is asked; a direct client call gets the worker's refusal, which
    // the supervisor treats as a failed worker, so this is the last call.
    assert!(process.select_program(0, Instant::now()).is_err());
}
