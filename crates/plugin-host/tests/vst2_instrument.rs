//! A VST2 instrument played through the real worker process with MIDI
//! (phase P8b of the full-compatibility plan).
//!
//! The instrument is `tools/vst2-test-instrument`: a sine voice that sounds
//! exactly while a note is held, so the output shows whether MIDI arrived and
//! at which sample. The test builds that DLL with `cargo build` (a few
//! seconds, no network: it has no dependencies) unless
//! `AUDIOROUTER_TEST_VST2_INSTRUMENT` names an existing build of it.
//!
//! Run: `cargo test -p audiorouter-plugin-host --test vst2_instrument`
#![cfg(windows)]

use audiorouter_plugin_host::{
    inspect_binary, MidiEvent, WorkerFrame, WorkerProcess, WorkerProcessError,
};
use std::path::PathBuf;
use std::process::Command;

const FRAMES: usize = 256;
const SAMPLE_RATE: u32 = 48_000;

fn worker_path() -> PathBuf {
    let test_exe = std::env::current_exe().expect("integration test path");
    test_exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("Cargo target directory")
        .join("audiorouter-plugin-worker.exe")
}

/// The instrument DLL, built once for all tests of this file.
fn instrument_dll() -> PathBuf {
    static DLL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DLL.get_or_init(|| {
        if let Some(path) = std::env::var_os("AUDIOROUTER_TEST_VST2_INSTRUMENT") {
            return PathBuf::from(path);
        }
        let manifest =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vst2-test-instrument");
        let status = Command::new(env!("CARGO"))
            .args(["build", "--manifest-path"])
            .arg(manifest.join("Cargo.toml"))
            .status()
            .expect("run cargo build for the test instrument");
        assert!(status.success(), "the test instrument must build");
        manifest
            .join("target/debug/ar_test_instrument.dll")
            .canonicalize()
            .expect("built test instrument")
    })
    .clone()
}

fn silent_block(sequence: u64, channels: u16) -> WorkerFrame {
    WorkerFrame::new(
        sequence,
        audiorouter_plugin_host::worker_clock_tick().saturating_add(10_000_000),
        channels,
        vec![0.0; FRAMES * usize::from(channels)],
    )
    .expect("block")
}

/// Peak of one channel over a range of frames of an interleaved stereo block.
fn peak(frame: &WorkerFrame, from: usize, to: usize) -> f32 {
    (from..to)
        .map(|index| frame.samples[index * 2].abs())
        .fold(0.0, f32::max)
}

fn start(path: &std::path::Path, channels: u16) -> (WorkerProcess, String) {
    let root = path.parent().expect("plugin folder").to_path_buf();
    let identity = inspect_binary(path, std::slice::from_ref(&root)).expect("inspect instrument");
    let worker = WorkerProcess::spawn_vst2_instrument(
        worker_path(),
        path,
        &identity.sha256,
        channels,
        SAMPLE_RATE,
    )
    .expect("start the instrument worker");
    (worker, identity.sha256)
}

#[test]
fn an_instrument_sounds_only_while_its_note_is_held_and_starts_at_the_event_sample() {
    let dll = instrument_dll();
    let (mut worker, _) = start(&dll, 2);

    // No MIDI: silence, finite.
    let quiet = worker.process(silent_block(1, 2), Vec::new()).unwrap();
    assert!(quiet.samples.iter().all(|sample| *sample == 0.0));

    // A note-on at sample 100 of the next block: silent before it, audible after.
    worker
        .send_midi(vec![MidiEvent::new(100, 0x90, 69, 127).unwrap()])
        .unwrap();
    let on = worker.process(silent_block(2, 2), Vec::new()).unwrap();
    assert_eq!(peak(&on, 0, 100), 0.0, "nothing before the note-on sample");
    assert!(
        peak(&on, 101, FRAMES) > 0.1,
        "the held note sounds after its sample"
    );
    assert!(on.samples.iter().all(|sample| sample.is_finite()));

    // With no further events the note keeps sounding (the plugin keeps state).
    let held = worker.process(silent_block(3, 2), Vec::new()).unwrap();
    assert!(peak(&held, 0, FRAMES) > 0.1);

    // Note-off at sample 40: sound until then, silence after.
    worker
        .send_midi(vec![MidiEvent::new(40, 0x80, 69, 0).unwrap()])
        .unwrap();
    let off = worker.process(silent_block(4, 2), Vec::new()).unwrap();
    assert!(peak(&off, 0, 40) > 0.1);
    assert_eq!(peak(&off, 41, FRAMES), 0.0);

    // Velocity scales the level: a soft note is quieter than a hard one.
    worker
        .send_midi(vec![MidiEvent::new(0, 0x90, 69, 32).unwrap()])
        .unwrap();
    let soft = worker.process(silent_block(5, 2), Vec::new()).unwrap();
    let soft_peak = peak(&soft, 1, FRAMES);
    worker
        .send_midi(vec![
            MidiEvent::new(0, 0x80, 69, 0).unwrap(),
            MidiEvent::new(0, 0x90, 69, 127).unwrap(),
        ])
        .unwrap();
    let hard = worker.process(silent_block(6, 2), Vec::new()).unwrap();
    assert!(peak(&hard, 1, FRAMES) > soft_peak * 2.0);

    assert!(worker.shutdown().unwrap().success());
}

#[test]
fn the_instrument_works_on_a_mono_route_and_refuses_midi_it_cannot_apply() {
    let dll = instrument_dll();
    let (mut worker, _) = start(&dll, 1);
    worker
        .send_midi(vec![MidiEvent::new(0, 0x90, 60, 100).unwrap()])
        .unwrap();
    let mono = worker.process(silent_block(1, 1), Vec::new()).unwrap();
    assert!(mono
        .samples
        .iter()
        .skip(1)
        .any(|sample| sample.abs() > 0.05));

    // An event beyond the block ends the worker like any protocol failure.
    worker
        .send_midi(vec![
            MidiEvent::new(FRAMES as u32 + 1, 0x90, 60, 100).unwrap()
        ])
        .unwrap();
    let error = worker.process(silent_block(2, 1), Vec::new());
    assert!(
        matches!(error, Err(WorkerProcessError::Protocol(ref code)) if code.contains("MIDI")),
        "{error:?}"
    );
}

#[test]
fn an_instrument_is_refused_when_loaded_as_an_effect() {
    let dll = instrument_dll();
    let root = dll.parent().unwrap().to_path_buf();
    let identity = inspect_binary(&dll, std::slice::from_ref(&root)).unwrap();
    let as_effect = WorkerProcess::spawn_for_plugin(worker_path(), &dll, &identity.sha256, 2);
    let message = match as_effect {
        Ok(_) => panic!("an instrument must not load as an effect"),
        Err(error) => format!("{error:?}"),
    };
    assert!(message.to_lowercase().contains("instrument"), "{message}");
}
