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
    let minimum_units: usize = std::env::var("AUDIOROUTER_TEST_VST3_UNITS_MIN")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let mut identity = inspect_binary(&plugin, &roots).unwrap();
    if let Ok(class_id) = std::env::var("AUDIOROUTER_TEST_VST3_CLASS_ID") {
        identity.metadata.class_ids = vec![class_id];
    }
    let mut process = SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    )
    .unwrap();
    let listing = process.programs(Instant::now()).unwrap();
    println!(
        "{} programs, {} units",
        listing.programs.len(),
        listing.units.len()
    );
    assert!(
        listing.programs.len() >= minimum,
        "expected {minimum}+ programs"
    );
    assert!(
        listing.units.len() >= minimum_units,
        "expected {minimum_units}+ units"
    );
    assert!(listing.programs.len() <= 256);
    assert!(listing.units.len() <= 256);
    for (position, program) in listing.programs.iter().enumerate() {
        assert_eq!(program.id as usize, position);
        assert!(!program.name.is_empty() && program.name.len() <= 128);
        assert!(!program.name.chars().any(char::is_control));
    }
    for unit in &listing.units {
        assert!(unit.name.len() <= 128);
        assert!(unit.program_count <= 256);
        assert!(unit.programs.len() <= unit.program_count as usize);
        assert!(unit
            .programs
            .iter()
            .all(|program| !program.name.is_empty() && program.name.len() <= 128));
    }
    // The listing is repeatable.
    assert_eq!(process.programs(Instant::now()).unwrap(), listing);
    // Selection is not offered for VST3. The live bridge refuses it before the
    // worker is asked; a direct client call gets the worker's refusal, which
    // the supervisor treats as a failed worker, so this is the last call.
    assert!(process.select_program(0, Instant::now()).is_err());
}
