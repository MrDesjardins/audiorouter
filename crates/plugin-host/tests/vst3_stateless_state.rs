//! Regression (2026-11-18): a DPF VST3 plugin with no parameters and no states
//! (for example ProM) saves a single NUL byte and then refuses that same byte
//! in `setState`. The worker accepts exactly that sentinel, so such a plugin
//! restores in the same worker and in a fresh one. Opt-in: it needs the real
//! worker and a stateless DPF plugin.
//!
//! Environment:
//! - `AUDIOROUTER_TEST_VST3_NATIVE_WORKER`: the worker from
//!   `tools/m06-vst3-worker/build.ps1`.
//! - `AUDIOROUTER_TEST_VST3_STATELESS`: a `.vst3` bundle such as DPF's ProM.
//!
//! Run: `cargo test -p audiorouter-plugin-host --test vst3_stateless_state -- --ignored`
#![cfg(windows)]

use audiorouter_plugin_host::{inspect_binary, SupervisedWorkerProcess};
use std::{path::PathBuf, time::Instant};

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

#[test]
#[ignore = "needs the built VST3 worker and a DPF plugin without parameters (ProM)"]
fn a_stateless_dpf_plugin_restores_its_own_saved_state() {
    let worker = env_path("AUDIOROUTER_TEST_VST3_NATIVE_WORKER");
    let plugin = env_path("AUDIOROUTER_TEST_VST3_STATELESS");
    let roots = [plugin.parent().expect("plugin folder").to_path_buf()];
    let identity = inspect_binary(&plugin, &roots).expect("inspect");
    let spawn = || {
        SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
            &worker,
            &identity,
            &roots,
            2,
            48_000,
            Instant::now(),
        )
        .expect("start the VST3 worker")
    };
    let mut first = spawn();
    let saved = first.save_state(Instant::now()).expect("save state");
    first
        .restore_state(saved.clone(), Instant::now())
        .expect("restore into the same worker");
    let mut fresh = spawn();
    fresh
        .restore_state(saved, Instant::now())
        .expect("restore into a fresh worker");
    assert!(first.shutdown().unwrap().success());
    assert!(fresh.shutdown().unwrap().success());
}
