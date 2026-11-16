//! The VST3 worker's parameter-snapshot fallback (phase P3 of the
//! full-compatibility plan): when a plugin refuses a saved state, the worker
//! puts the saved parameter values back instead of reporting that state is
//! unsupported. Opt-in: it needs a built worker and an installed effect with
//! at least one writable parameter.
//!
//! Environment:
//! - `AUDIOROUTER_TEST_VST3_WORKER`: the worker built by
//!   `tools/m06-vst3-worker/build.ps1`.
//! - `AUDIOROUTER_TEST_VST3_EFFECT`: a `.vst3` effect bundle (for example
//!   Dragonfly Plate Reverb).
//!
//! Run: `cargo test -p audiorouter-plugin-host --test vst3_state_fallback -- --ignored --nocapture`
#![cfg(windows)]

use audiorouter_plugin_host::{
    inspect_binary, ParameterEvent, PluginIdentity, PluginStateAsset, SupervisedWorkerProcess,
    WorkerFrame, WorkerProcessError,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const FRAMES: usize = 256;
const SAMPLE_RATE: u32 = 48_000;
const ENVELOPE_V2: [u8; 8] = [b'A', b'R', b'S', b'T', b'2', 0, 0, 0];

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

fn block(sequence: u64) -> WorkerFrame {
    WorkerFrame::new(
        sequence,
        audiorouter_plugin_host::worker_clock_tick().saturating_add(10_000_000),
        2,
        vec![0.0; FRAMES * 2],
    )
    .expect("block")
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

/// The `(parameter id, normalized value)` pairs at the end of an `ARST2` state.
fn snapshot(bytes: &[u8]) -> Vec<(u32, f64)> {
    assert_eq!(&bytes[..8], &ENVELOPE_V2, "the worker saves envelope 2");
    let sections_end = 16 + read_u32(bytes, 8) as usize + read_u32(bytes, 12) as usize;
    let count = read_u32(bytes, sections_end) as usize;
    (0..count)
        .map(|index| {
            let at = sections_end + 4 + index * 12;
            let bits =
                u64::from(read_u32(bytes, at + 4)) | (u64::from(read_u32(bytes, at + 8)) << 32);
            (read_u32(bytes, at), f64::from_bits(bits))
        })
        .collect()
}

/// An `ARST2` state whose component section no plugin wrote, followed by the
/// given parameter values.
fn damaged_state(values: &[(u32, f64)]) -> Vec<u8> {
    let component = [0xFF_u8; 7];
    let mut bytes = ENVELOPE_V2.to_vec();
    bytes.extend_from_slice(&(component.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&component);
    bytes.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for (id, value) in values {
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes
}

fn start(worker: &Path, identity: &PluginIdentity, root: &Path) -> SupervisedWorkerProcess {
    SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
        worker,
        identity,
        &[root.to_path_buf()],
        2,
        SAMPLE_RATE,
        Instant::now(),
    )
    .expect("start the VST3 worker")
}

#[test]
#[ignore = "needs a built VST3 worker and an installed VST3 effect with parameters"]
fn a_refused_state_falls_back_to_the_saved_parameter_values() {
    let worker = env_path("AUDIOROUTER_TEST_VST3_WORKER");
    let plugin = env_path("AUDIOROUTER_TEST_VST3_EFFECT");
    let root = plugin.parent().expect("plugin folder").to_path_buf();
    let identity = inspect_binary(&plugin, std::slice::from_ref(&root)).expect("inspect");

    // Move one parameter well away from its default and save.
    let mut first = start(&worker, &identity, &root);
    let defaults = snapshot(
        &first
            .save_state(Instant::now())
            .expect("save defaults")
            .bytes,
    );
    assert!(
        !defaults.is_empty(),
        "the test plugin must have a writable parameter"
    );
    let (parameter, default) = defaults[0];
    let target = if default > 0.5 { 0.125_f32 } else { 0.875_f32 };
    first
        .process(
            block(1),
            vec![ParameterEvent::new(parameter, target, 0).unwrap()],
            Instant::now(),
        )
        .expect("set the parameter");
    for sequence in 2..=4 {
        first
            .process(block(sequence), Vec::new(), Instant::now())
            .unwrap();
    }
    let saved = first.save_state(Instant::now()).expect("save");
    let saved_values = snapshot(&saved.bytes);
    let saved_value = saved_values
        .iter()
        .find(|(id, _)| *id == parameter)
        .unwrap()
        .1;
    println!("parameter {parameter}: default {default}, set {target}, saved {saved_value}");
    assert!(
        (saved_value - f64::from(target)).abs() < 0.02,
        "the saved snapshot holds the value that was set"
    );
    drop(first);

    // The plugin really refuses the damaged component state: with no values to
    // fall back on, the worker reports the limitation.
    let mut refused = start(&worker, &identity, &root);
    let without_values = PluginStateAsset::new(saved.version, damaged_state(&[])).unwrap();
    let error = refused
        .restore_state(without_values, Instant::now())
        .expect_err("a damaged state with no parameter values cannot restore");
    assert!(
        matches!(error, WorkerProcessError::UnsupportedFeature(_)),
        "expected the state-unsupported report, got {error:?}"
    );
    drop(refused);

    // With the values beside it, a fresh plugin gets its parameters back.
    let mut second = start(&worker, &identity, &root);
    let with_values = PluginStateAsset::new(saved.version, damaged_state(&saved_values)).unwrap();
    second
        .restore_state(with_values, Instant::now())
        .expect("the fallback restores the parameters");
    for sequence in 1..=3 {
        let out = second
            .process(block(sequence), Vec::new(), Instant::now())
            .unwrap();
        assert!(out.samples.iter().all(|sample| sample.is_finite()));
    }
    let restored = snapshot(&second.save_state(Instant::now()).expect("save again").bytes);
    let restored_value = restored.iter().find(|(id, _)| *id == parameter).unwrap().1;
    println!("restored {restored_value}");
    assert!(
        (restored_value - saved_value).abs() < 0.02,
        "the parameter came back from the snapshot (saved {saved_value}, restored {restored_value})"
    );
}
