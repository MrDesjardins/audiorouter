//! Opt-in: a VST3 that only accepts 64-bit samples is refused, never converted.
//!
//! Build the fixture with `tools/m06-vst3-worker/fixtures/build-fixture.ps1`
//! and point `AUDIOROUTER_TEST_VST3_SAMPLE64_FIXTURE` at the resulting
//! `Sample64Only.vst3\Contents\x86_64-win\Sample64Only.vst3`.
#![cfg(windows)]

use audiorouter_plugin_host::{inspect_binary, SupervisedWorkerProcess};
use std::{path::PathBuf, time::Instant};

#[test]
#[ignore = "requires AUDIOROUTER_TEST_VST3_WORKER and AUDIOROUTER_TEST_VST3_SAMPLE64_FIXTURE"]
fn a_plugin_that_only_takes_64_bit_samples_is_refused_with_a_plain_reason() {
    let worker = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_WORKER").unwrap());
    let plugin = PathBuf::from(std::env::var("AUDIOROUTER_TEST_VST3_SAMPLE64_FIXTURE").unwrap());
    let roots = vec![plugin.parent().unwrap().to_path_buf()];
    let identity = inspect_binary(&plugin, &roots).unwrap();
    let error = match SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
        worker,
        &identity,
        &roots,
        2,
        48_000,
        Instant::now(),
    ) {
        Ok(_) => panic!("a 64-bit-only plugin must not load"),
        Err(error) => format!("{error:?}"),
    };
    assert!(
        error.contains("sampleFormatUnsupported") || error.contains("32-bit sample format"),
        "the refusal must name the sample format, got: {error}"
    );
}
