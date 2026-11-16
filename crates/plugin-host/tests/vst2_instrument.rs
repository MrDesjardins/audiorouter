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
        // Its own target folder, and none of the parent's build environment:
        // a coverage run (cargo llvm-cov) sets a target directory, compiler
        // flags and profile files that must not reach this tiny cdylib.
        let target = manifest.join("target");
        let mut build = Command::new(env!("CARGO"));
        build
            .args(["build", "--manifest-path"])
            .arg(manifest.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target);
        for (name, _) in std::env::vars_os() {
            let name = name.to_string_lossy().into_owned();
            if matches!(
                name.as_str(),
                "RUSTFLAGS"
                    | "CARGO_ENCODED_RUSTFLAGS"
                    | "CARGO_TARGET_DIR"
                    | "LLVM_PROFILE_FILE"
                    | "RUSTC_WORKSPACE_WRAPPER"
            ) || name.starts_with("CARGO_LLVM_COV")
            {
                build.env_remove(name);
            }
        }
        let status = build
            .status()
            .expect("run cargo build for the test instrument");
        assert!(status.success(), "the test instrument must build");
        target
            .join("debug/ar_test_instrument.dll")
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

mod through_the_runtime_bridge {
    use super::*;
    use audiorouter_engine::{AudioBlock, RealtimePluginProcessor};
    use audiorouter_plugin_host::{PluginRuntimeBridge, SupervisedWorkerProcess};
    use std::time::{Duration, Instant};

    const QUANTUM: usize = 128;

    fn start_bridge() -> std::sync::Arc<PluginRuntimeBridge> {
        let dll = instrument_dll();
        let root = dll.parent().unwrap().to_path_buf();
        let identity = inspect_binary(&dll, std::slice::from_ref(&root)).unwrap();
        let worker = SupervisedWorkerProcess::spawn_instrument_verified(
            worker_path(),
            &identity,
            std::slice::from_ref(&root),
            2,
            SAMPLE_RATE,
            Instant::now(),
        )
        .expect("start the supervised instrument worker");
        PluginRuntimeBridge::start_instrument(worker, 2, QUANTUM, 2).expect("start bridge")
    }

    /// Runs the engine-side processor for a while and returns the peak.
    fn run_for(bridge: &PluginRuntimeBridge, quanta: usize) -> f32 {
        let mut peak = 0.0_f32;
        let mut samples = vec![0.0_f32; QUANTUM * 2];
        for _ in 0..quanta {
            // The engine hands an instrument a silent block.
            let mut block = AudioBlock::new(2, QUANTUM).unwrap();
            bridge.process(&mut block);
            block.copy_to_interleaved(&mut samples).unwrap();
            peak = samples
                .iter()
                .fold(peak, |peak, sample| peak.max(sample.abs()));
            std::thread::sleep(Duration::from_millis(3));
        }
        peak
    }

    #[test]
    fn notes_pushed_to_the_queue_are_heard_and_released_by_a_panic() {
        let bridge = start_bridge();
        let queue = bridge.midi_queue();
        assert!(run_for(&bridge, 40) < 1e-6, "silent until a note arrives");

        queue.push(&[MidiEvent::new(0, 0x90, 69, 120).unwrap()]);
        assert_eq!(queue.held_notes(), 1);
        assert!(run_for(&bridge, 40) > 0.1, "the held note sounds");

        // The panic button: release everything, then the output falls silent.
        assert_eq!(queue.release_all_notes(), 1);
        assert_eq!(queue.held_notes(), 0);
        run_for(&bridge, 20);
        assert!(run_for(&bridge, 40) < 1e-6, "silent after the release");
        assert!(!bridge.failed(), "{:?}", bridge.failure_reason());
    }

    #[test]
    fn a_route_without_midi_stays_silent_and_healthy() {
        let bridge = start_bridge();
        assert!(run_for(&bridge, 60) < 1e-6);
        assert!(!bridge.failed());
        assert_eq!(
            bridge.health().state,
            audiorouter_engine::PluginWorkerState::Running
        );
    }
}

mod the_midi_queue {
    use audiorouter_plugin_host::{MidiEvent, MidiQueue};

    #[test]
    fn it_tracks_held_notes_and_turns_all_notes_off_into_explicit_releases() {
        let queue = MidiQueue::default();
        queue.push(&[
            MidiEvent::new(0, 0x90, 60, 100).unwrap(),
            MidiEvent::new(0, 0x90, 64, 100).unwrap(),
            MidiEvent::new(0, 0x91, 67, 100).unwrap(),
        ]);
        assert_eq!(queue.held_notes(), 3);
        // A note-on with velocity zero is a note-off.
        queue.push(&[MidiEvent::new(0, 0x90, 64, 0).unwrap()]);
        assert_eq!(queue.held_notes(), 2);
        // CC 123 on channel 0 releases that channel only.
        queue.push(&[MidiEvent::new(0, 0xB0, 123, 0).unwrap()]);
        assert_eq!(queue.held_notes(), 1);
        assert_eq!(queue.release_all_notes(), 1);
        assert_eq!(queue.held_notes(), 0);
        assert_eq!(queue.release_all_notes(), 0);
    }

    #[test]
    fn it_is_bounded_and_skips_malformed_messages() {
        let queue = MidiQueue::default();
        let bad = MidiEvent {
            sample_offset: 0,
            status: 0xF8,
            data1: 0,
            data2: 0,
        };
        assert_eq!(queue.push(&[bad]), 0);
        let note = MidiEvent::new(0, 0x90, 60, 90).unwrap();
        let burst = vec![note; MidiQueue::CAPACITY * 2];
        // An overflowing batch is refused instead of evicting an older
        // note-off; the queue also clears its now-stale held state.
        assert_eq!(queue.push(&burst), 0);
        assert_eq!(queue.held_notes(), 0);
        assert_eq!(queue.release_all_notes(), 0);
    }
}
