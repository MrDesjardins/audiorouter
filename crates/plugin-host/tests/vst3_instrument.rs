//! A VST3 instrument played through the real native worker with MIDI
//! (phase P8b of the full-compatibility plan). Opt-in: it needs a built
//! worker and an installed instrument.
//!
//! Environment:
//! - `AUDIOROUTER_TEST_VST3_WORKER`: the worker built by
//!   `tools/m06-vst3-worker/build.ps1`.
//! - `AUDIOROUTER_TEST_VST3_INSTRUMENT`: a `.vst3` bundle (for example Surge XT).
//! - `AUDIOROUTER_TEST_VST3_INSTRUMENT_CLASS` (optional): the 32-hex class ID to play.
//!
//! Run: `cargo test -p audiorouter-plugin-host --test vst3_instrument -- --ignored --nocapture`
#![cfg(windows)]

use audiorouter_plugin_host::{inspect_binary, MidiEvent, WorkerFrame, WorkerProcess};
use std::path::PathBuf;

const FRAMES: usize = 256;
const SAMPLE_RATE: u32 = 48_000;

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
}

fn block(sequence: u64, channels: u16) -> WorkerFrame {
    WorkerFrame::new(
        sequence,
        audiorouter_plugin_host::worker_clock_tick().saturating_add(10_000_000),
        channels,
        vec![0.0; FRAMES * usize::from(channels)],
    )
    .expect("block")
}

fn peak(frame: &WorkerFrame) -> f32 {
    frame
        .samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

#[test]
#[ignore = "needs a built VST3 worker and an installed VST3 instrument"]
fn a_vst3_instrument_sounds_when_it_gets_a_note_and_not_before() {
    let worker = env_path("AUDIOROUTER_TEST_VST3_WORKER");
    let plugin = env_path("AUDIOROUTER_TEST_VST3_INSTRUMENT");
    let root = plugin.parent().expect("plugin folder").to_path_buf();
    let identity = inspect_binary(&plugin, std::slice::from_ref(&root)).expect("inspect");
    // Optional: pick one class of a multi-class module (for example mda JX10).
    let class_id = std::env::var("AUDIOROUTER_TEST_VST3_INSTRUMENT_CLASS").ok();
    // The scan itself says what it is, before any worker has refused it. A
    // module that mixes effects and instruments classifies by its first class.
    assert!(
        class_id.is_some() || identity.metadata.instrument,
        "inspection must classify the instrument without loading it"
    );
    let mut process = WorkerProcess::spawn_vst3_instrument(
        &worker,
        &plugin,
        &identity.sha256,
        class_id.as_deref(),
        2,
        SAMPLE_RATE,
    )
    .expect("start the VST3 instrument worker");

    // Let the plugin settle, then check it is silent without a note.
    let mut quiet_peak = 0.0_f32;
    for sequence in 1..=8 {
        let out = process.process(block(sequence, 2), Vec::new()).unwrap();
        assert!(out.samples.iter().all(|sample| sample.is_finite()));
        quiet_peak = quiet_peak.max(peak(&out));
    }
    println!("peak without a note: {quiet_peak}");

    process
        .send_midi(vec![MidiEvent::new(0, 0x90, 60, 110).unwrap()])
        .unwrap();
    let mut played_peak = 0.0_f32;
    for sequence in 9..=60 {
        let out = process.process(block(sequence, 2), Vec::new()).unwrap();
        assert!(out.samples.iter().all(|sample| sample.is_finite()));
        played_peak = played_peak.max(peak(&out));
    }
    println!("peak while a note is held: {played_peak}");
    assert!(
        played_peak > 0.01 && played_peak > quiet_peak * 10.0,
        "the held note must be clearly audible (quiet {quiet_peak}, played {played_peak})"
    );

    process
        .send_midi(vec![MidiEvent::new(0, 0x80, 60, 0).unwrap()])
        .unwrap();
    for sequence in 61..=64 {
        process.process(block(sequence, 2), Vec::new()).unwrap();
    }
    assert!(process.shutdown().unwrap().success());
}

#[test]
#[ignore = "needs a built VST3 worker and an installed VST3 effect"]
fn a_vst3_effect_is_refused_as_an_instrument_with_its_own_reason() {
    let worker = env_path("AUDIOROUTER_TEST_VST3_WORKER");
    let plugin = env_path("AUDIOROUTER_TEST_VST3_EFFECT");
    let root = plugin.parent().expect("plugin folder").to_path_buf();
    let identity = inspect_binary(&plugin, std::slice::from_ref(&root)).expect("inspect");
    assert!(
        !identity.metadata.instrument,
        "inspection must not call an effect an instrument"
    );
    let result = WorkerProcess::spawn_vst3_instrument(
        &worker,
        &plugin,
        &identity.sha256,
        None,
        2,
        SAMPLE_RATE,
    );
    let message = match result {
        Ok(_) => panic!("an effect must not load as an instrument"),
        Err(error) => format!("{error:?}"),
    };
    assert!(
        message.contains("not an instrument") || message.contains("effect"),
        "{message}"
    );
}
