//! Opt-in acceptance matrix for installed VST2 effects in a chain (issue #136).
//!
//! It loads one to three installed x64 VST2 effects as one supervised chain,
//! in several orders and at mono and stereo widths, and for each combination
//! records whether the chain starts, exposes its parameters, accepts a
//! changed parameter, keeps processing audio, saves and restores its state,
//! and opens, shows and closes every member's native editor while audio runs.
//!
//! Nothing here touches an audio device: the audio is a generated tone sent
//! through the real worker process and the real runtime bridge. The editor
//! windows are children of a hidden test window unless
//! `AUDIOROUTER_CHAIN_SHOW_EDITORS=1` is set.
//!
//! Run (Windows, plugins installed):
//!   cargo test -p audiorouter-plugin-host --all-features --test vst2_chain_matrix \
//!     -- --ignored --nocapture
//!
//! Environment:
//! - `AUDIOROUTER_CHAIN_PLUGINS`: `KEY=path;KEY=path` replaces the default
//!   plugin paths. Keys: BC (Blue Cat PatchWork), PP (Pitchproof x64),
//!   RC (ReaComp), EQ (ReaEQ).
//! - `AUDIOROUTER_CHAIN_FILTER`: only run combinations whose name contains it.
//! - `AUDIOROUTER_CHAIN_SHOW_EDITORS=1`: show the editor host window and keep
//!   each editor open for `AUDIOROUTER_CHAIN_HOLD_MS` (default 1500).
//! - `AUDIOROUTER_CHAIN_REPORT`: report path (JSON); a `.md` copy is written
//!   beside it. Default: `target/<profile>/vst2-chain-matrix.json`.
#![cfg(windows)]

use audiorouter_engine::{AudioBlock, RealtimePluginProcessor};
use audiorouter_plugin_host::{
    inspect_binary, EditorParentAuthorizationIssuer, ParameterDescriptor, ParameterEvent,
    PluginCompatibility, PluginRuntimeBridge, PluginStateAsset, SupervisedWorkerProcess,
    WorkerChainPlugin,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const FRAMES: usize = 128;
const SAMPLE_RATE: u32 = 48_000;
/// Blocks sent per run: about a second of audio at the test's pace.
const RUN_BLOCKS: usize = 400;

#[link(name = "user32")]
unsafe extern "system" {
    fn CreateWindowExW(
        extended_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: *mut c_void,
        menu: *mut c_void,
        instance: *mut c_void,
        parameter: *mut c_void,
    ) -> *mut c_void;
    fn DestroyWindow(window: *mut c_void) -> i32;
    fn GetClientRect(window: *mut c_void, rect: *mut [i32; 4]) -> i32;
    fn GetWindow(window: *mut c_void, command: u32) -> *mut c_void;
    fn PeekMessageW(
        message: *mut [usize; 6],
        window: *mut c_void,
        minimum: u32,
        maximum: u32,
        remove: u32,
    ) -> i32;
    fn TranslateMessage(message: *const [usize; 6]) -> i32;
    fn DispatchMessageW(message: *const [usize; 6]) -> isize;
}

fn worker_path() -> String {
    std::env::var("CARGO_BIN_EXE_audiorouter_plugin_worker").unwrap_or_else(|_| {
        std::env::current_exe()
            .expect("test path")
            .parent()
            .and_then(|deps| deps.parent())
            .expect("target directory")
            .join("audiorouter-plugin-worker.exe")
            .to_string_lossy()
            .into_owned()
    })
}

/// A window whose thread pumps messages, like the desktop shell's editor host.
/// Host client size, newest child client size, and that child's handle.
type EditorSizes = (Option<(i32, i32)>, Option<(i32, i32)>, usize);

struct EditorHostWindow {
    handle: usize,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl EditorHostWindow {
    fn open(visible: bool) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = std::sync::mpsc::channel();
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let title: Vec<u16> = "AudioRouter VST2 chain matrix\0".encode_utf16().collect();
            // WS_OVERLAPPEDWINDOW | WS_VISIBLE, or hidden.
            let style = if visible {
                0x00CF_0000 | 0x1000_0000
            } else {
                0
            };
            // SAFETY: NUL-terminated buffers outlive the synchronous call.
            let window = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    title.as_ptr(),
                    style,
                    100,
                    100,
                    640,
                    480,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            sender.send(window as usize).expect("send window");
            let mut message = [0_usize; 6];
            while !thread_stop.load(Ordering::Acquire) {
                // SAFETY: standard non-blocking pump for this thread's windows;
                // `message` outlives each synchronous call.
                unsafe {
                    while PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, 1) != 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            // SAFETY: created on this thread; the editors are closed.
            unsafe { DestroyWindow(window) };
        });
        let handle = receiver.recv().expect("editor host window");
        assert_ne!(handle, 0, "editor host window creation failed");
        Self {
            handle,
            stop,
            thread: Some(thread),
        }
    }

    /// Client size of the host window, the topmost child (the newest editor
    /// window) and that child's handle (0 when there is none).
    fn sizes(&self) -> EditorSizes {
        let size = |window: *mut c_void| {
            let mut rect = [0_i32; 4];
            // SAFETY: `rect` is a live writable RECT; a stale handle fails.
            (unsafe { GetClientRect(window, &mut rect) } != 0)
                .then_some((rect[2] - rect[0], rect[3] - rect[1]))
        };
        let parent = self.handle as *mut c_void;
        // SAFETY: GW_CHILD (5) only queries the window tree.
        let child = unsafe { GetWindow(parent, 5) };
        (
            size(parent),
            (!child.is_null()).then(|| size(child)).flatten(),
            child as usize,
        )
    }
}

impl Drop for EditorHostWindow {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Plugin {
    key: &'static str,
    path: PathBuf,
    /// Parameters, read once through a single-plugin worker.
    parameters: Vec<ParameterDescriptor>,
}

fn default_paths() -> Vec<(&'static str, PathBuf)> {
    let reaplugs = r"C:\Program Files\VSTPlugins\ReaPlugs";
    let mut paths = vec![
        (
            "BC",
            PathBuf::from(r"C:\Program Files\VSTPlugins\BC PatchWork VST.dll"),
        ),
        (
            "PP",
            PathBuf::from(r"C:\Program Files\Common Files\VST3\Pitchproof\pitchproof-x64.dll"),
        ),
        (
            "RC",
            PathBuf::from(format!(r"{reaplugs}\reacomp-standalone.dll")),
        ),
        (
            "EQ",
            PathBuf::from(format!(r"{reaplugs}\reaeq-standalone.dll")),
        ),
        (
            "GT",
            PathBuf::from(format!(r"{reaplugs}\reagate-standalone.dll")),
        ),
        (
            "DL",
            PathBuf::from(format!(r"{reaplugs}\readelay-standalone.dll")),
        ),
        (
            "XC",
            PathBuf::from(format!(r"{reaplugs}\reaxcomp-standalone.dll")),
        ),
        (
            "FR",
            PathBuf::from(format!(r"{reaplugs}\reafir_standalone.dll")),
        ),
    ];
    if let Ok(overrides) = std::env::var("AUDIOROUTER_CHAIN_PLUGINS") {
        for entry in overrides.split(';').filter(|entry| !entry.is_empty()) {
            if let Some((key, path)) = entry.split_once('=') {
                if let Some(slot) = paths.iter_mut().find(|(known, _)| *known == key) {
                    slot.1 = PathBuf::from(path);
                }
            }
        }
    }
    paths
}

fn roots_of(path: &std::path::Path) -> Vec<PathBuf> {
    vec![path.parent().expect("plugin folder").to_path_buf()]
}

/// Load a plugin's parameter list through its own worker.
fn discover(key: &'static str, path: PathBuf) -> Result<Plugin, String> {
    let roots = roots_of(&path);
    let identity = inspect_binary(&path, &roots).map_err(|error| format!("inspect: {error:?}"))?;
    if identity.compatibility() != PluginCompatibility::SupportedVst2X64Gated {
        return Err(format!(
            "not a supported x64 VST2 effect: {:?}",
            identity.compatibility()
        ));
    }
    let mut worker = SupervisedWorkerProcess::spawn_verified_with_sample_rate(
        worker_path(),
        &identity,
        &roots,
        2,
        SAMPLE_RATE,
        Instant::now(),
    )
    .map_err(|error| format!("launch: {error}"))?;
    let parameters = worker
        .describe_parameters(Instant::now())
        .map_err(|error| format!("parameters: {error:?}"))?;
    let _ = worker.shutdown_with_timeout(Duration::from_secs(3));
    Ok(Plugin {
        key,
        path,
        parameters,
    })
}

fn start_bridges(
    members: &[&Plugin],
    channels: u16,
    states: &[Option<PluginStateAsset>],
) -> Result<Vec<Arc<PluginRuntimeBridge>>, String> {
    let mut plugins = Vec::new();
    for member in members {
        let roots = roots_of(&member.path);
        let identity = inspect_binary(&member.path, &roots)
            .map_err(|error| format!("inspect {}: {error:?}", member.key))?;
        plugins.push(WorkerChainPlugin {
            path: identity.path,
            sha256: identity.sha256,
            configured_roots: roots,
        });
    }
    let now = Instant::now();
    let mut worker = if members.len() > 1 {
        SupervisedWorkerProcess::spawn_verified_chain(
            worker_path(),
            &plugins,
            channels,
            SAMPLE_RATE,
            now,
        )
    } else {
        let identity = inspect_binary(&plugins[0].path, &plugins[0].configured_roots)
            .map_err(|error| format!("inspect: {error:?}"))?;
        SupervisedWorkerProcess::spawn_verified_with_sample_rate(
            worker_path(),
            &identity,
            &plugins[0].configured_roots,
            channels,
            SAMPLE_RATE,
            now,
        )
    }
    .map_err(|error| format!("launch: {error}"))?;
    for (index, state) in states.iter().enumerate() {
        if let Some(asset) = state {
            if members.len() > 1 {
                worker
                    .select_instance(index, Instant::now())
                    .map_err(|error| format!("select {index}: {error:?}"))?;
            }
            worker
                .restore_state(asset.clone(), Instant::now())
                .map_err(|error| format!("restore state {index}: {error}"))?;
        }
    }
    if members.len() > 1 {
        PluginRuntimeBridge::start_chain(worker, usize::from(channels), FRAMES, 8, members.len())
    } else {
        PluginRuntimeBridge::start(worker, usize::from(channels), FRAMES, 8)
            .map(|bridge| vec![bridge])
    }
    .map_err(|error| format!("bridge: {error:?}"))
}

struct RunStats {
    finite: bool,
    peak: f32,
    /// Largest difference from the dry tone over the last blocks.
    max_difference: f32,
    failed: bool,
}

/// Send a tone through every bridge in order, as the engine does.
fn run_audio(bridges: &[Arc<PluginRuntimeBridge>], channels: usize, blocks: usize) -> RunStats {
    let mut block = AudioBlock::new(channels, FRAMES).expect("block");
    let mut phase = 0_usize;
    let mut stats = RunStats {
        finite: true,
        peak: 0.0,
        max_difference: 0.0,
        failed: false,
    };
    for index in 0..blocks {
        let dry: Vec<f32> = (0..FRAMES)
            .map(|frame| {
                0.25 * (2.0 * std::f32::consts::PI * 440.0 * (phase + frame) as f32
                    / SAMPLE_RATE as f32)
                    .sin()
            })
            .collect();
        phase += FRAMES;
        for channel in 0..channels {
            block
                .channel_mut(channel)
                .expect("channel")
                .copy_from_slice(&dry);
        }
        for bridge in bridges {
            bridge.process(&mut block);
        }
        // Skip the start-up latency of the worker pipeline.
        if index >= blocks / 2 {
            for channel in 0..channels {
                for (sample, dry) in block.channel(channel).expect("channel").iter().zip(&dry) {
                    stats.finite &= sample.is_finite();
                    stats.peak = stats.peak.max(sample.abs());
                    stats.max_difference = stats.max_difference.max((sample - dry).abs());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    stats.failed = bridges.iter().any(|bridge| bridge.failed());
    stats
}

fn reasons(bridges: &[Arc<PluginRuntimeBridge>]) -> Vec<String> {
    bridges
        .iter()
        .filter_map(|bridge| bridge.failure_reason())
        .collect()
}

fn stats_json(stats: &RunStats) -> Value {
    json!({
        "finite": stats.finite,
        "peak": stats.peak,
        "differsFromDryTone": stats.max_difference > 1.0e-3,
        "silent": stats.peak < 1.0e-6,
        "failed": stats.failed,
    })
}

/// One combination at one channel width. Returns the report entry and the
/// problems found (a step that should work and did not).
fn run_combination(
    name: &str,
    members: &[&Plugin],
    channels: u16,
    window: &EditorHostWindow,
    hold: Duration,
) -> (Value, Vec<String>) {
    let mut problems = Vec::new();
    let mut steps = serde_json::Map::new();
    let label = format!("{name} x{channels}ch");
    let fail = |problems: &mut Vec<String>, step: &str, why: String| {
        problems.push(format!("{label}: {step}: {why}"));
    };

    // 1. Start the chain.
    let bridges = match start_bridges(members, channels, &vec![None; members.len()]) {
        Ok(bridges) => bridges,
        Err(error) => {
            fail(&mut problems, "start", error.clone());
            steps.insert("start".into(), json!({ "ok": false, "error": error }));
            return (
                json!({ "name": name, "channels": channels, "steps": steps }),
                problems,
            );
        }
    };
    steps.insert("start".into(), json!({ "ok": true }));

    // 2. Baseline audio.
    let baseline = run_audio(&bridges, usize::from(channels), RUN_BLOCKS);
    steps.insert("audio".into(), stats_json(&baseline));
    if baseline.failed || !baseline.finite {
        fail(
            &mut problems,
            "audio",
            format!("failed={} reasons={:?}", baseline.failed, reasons(&bridges)),
        );
    }

    // 3. Configure: move each member's first parameter away from its default.
    let mut before = Vec::new();
    for bridge in &bridges {
        before.push(bridge.save_state().ok().map(|state| state.sha256));
    }
    let mut configured = Vec::new();
    for (bridge, member) in bridges.iter().zip(members) {
        let Some(parameter) = member.parameters.first() else {
            configured.push(json!({ "plugin": member.key, "parameters": 0 }));
            continue;
        };
        let value = if parameter.default_value < 0.5 {
            0.8
        } else {
            0.2
        };
        let accepted = bridge
            .set_parameters(vec![ParameterEvent {
                parameter_id: parameter.parameter_id,
                normalized_value: value,
                sample_offset: 0,
            }])
            .is_ok();
        configured.push(json!({
            "plugin": member.key,
            "parameters": member.parameters.len(),
            "parameter": parameter.title,
            "accepted": accepted,
        }));
        if !accepted {
            fail(
                &mut problems,
                "configure",
                format!("{} refused a value", member.key),
            );
        }
    }
    let after_configure = run_audio(&bridges, usize::from(channels), RUN_BLOCKS / 2);
    steps.insert(
        "configure".into(),
        json!({ "members": configured, "audio": stats_json(&after_configure) }),
    );
    if after_configure.failed {
        fail(
            &mut problems,
            "configure",
            format!("worker failed: {:?}", reasons(&bridges)),
        );
    }

    // 4. Save state, and check the parameter change reached it.
    let mut states: Vec<Option<PluginStateAsset>> = Vec::new();
    let mut state_report = Vec::new();
    for (index, (bridge, member)) in bridges.iter().zip(members).enumerate() {
        match bridge.save_state() {
            Ok(state) => {
                state_report.push(json!({
                    "plugin": member.key,
                    "bytes": state.bytes.len(),
                    "changedByParameter": before[index].as_deref() != Some(state.sha256.as_str()),
                }));
                states.push(Some(state));
            }
            Err(error) => {
                fail(
                    &mut problems,
                    "saveState",
                    format!("{}: {error}", member.key),
                );
                state_report.push(json!({ "plugin": member.key, "error": error }));
                states.push(None);
            }
        }
    }
    steps.insert("saveState".into(), Value::Array(state_report));

    // 5. Editors: open each member's window while audio runs, then close it.
    let authorization = EditorParentAuthorizationIssuer::from_key([9; 32])
        .issue(window.handle as u64, std::process::id())
        .expect("editor authorization");
    let mut editors = Vec::new();
    let mut window_child_before = window.sizes().2;
    for (bridge, member) in bridges.iter().zip(members) {
        let opened = bridge.open_editor(authorization.clone());
        if let Err(error) = &opened {
            // A plugin without an editor says so; that is not a defect.
            editors.push(json!({ "plugin": member.key, "opened": false, "error": error }));
            if !error.contains("editorUnavailable") && !error.contains("UnsupportedFeature") {
                fail(
                    &mut problems,
                    "editor",
                    format!("{} open: {error}", member.key),
                );
            }
            continue;
        }
        // Only a window that did not exist before this open counts: a plugin
        // may leave its window behind after close, and that must not be
        // mistaken for the next member's editor.
        let child_before = window_child_before;
        let started = Instant::now();
        let mut sizes = (None, None, child_before);
        let mut appeared_after = None;
        while started.elapsed() < Duration::from_secs(5) {
            let _ = run_audio(&bridges, usize::from(channels), 20);
            let now = window.sizes();
            if now.2 != child_before && now.2 != 0 {
                sizes = now;
                appeared_after.get_or_insert(started.elapsed());
                if started.elapsed() >= appeared_after.unwrap_or_default() + hold {
                    break;
                }
            } else {
                sizes.0 = now.0;
            }
        }
        let while_open = run_audio(&bridges, usize::from(channels), 40);
        let closed = bridge.close_editor();
        let after_close = run_audio(&bridges, usize::from(channels), 40);
        std::thread::sleep(Duration::from_millis(300));
        let left_behind = window.sizes().2 == sizes.2 && sizes.2 != child_before;
        window_child_before = window.sizes().2;
        let child_visible = appeared_after.is_some()
            && sizes
                .1
                .is_some_and(|(width, height)| width > 1 && height > 1);
        if !child_visible {
            fail(
                &mut problems,
                "editor",
                format!("{} showed no editor window", member.key),
            );
        }
        if let Err(error) = &closed {
            fail(
                &mut problems,
                "editorClose",
                format!("{}: {error}", member.key),
            );
        }
        if while_open.failed || after_close.failed {
            fail(
                &mut problems,
                "editorAudio",
                format!(
                    "{} worker failed around its editor: {:?}",
                    member.key,
                    reasons(&bridges)
                ),
            );
        }
        editors.push(json!({
            "plugin": member.key,
            "opened": true,
            "hostClient": sizes.0.map(|(w, h)| json!([w, h])),
            "editorClient": sizes.1.map(|(w, h)| json!([w, h])),
            "appearedAfterMs": appeared_after.map(|time| time.as_millis() as u64),
            "windowLeftAfterClose": left_behind,
            "editorShown": child_visible,
            "closed": closed.is_ok(),
            "audioWhileOpenOk": !while_open.failed,
            "audioAfterCloseOk": !after_close.failed,
        }));
    }
    steps.insert("editors".into(), Value::Array(editors));

    // 6. Restore the saved states into a fresh chain and run again.
    let still_running = !bridges.iter().any(|bridge| bridge.failed());
    drop(bridges);
    if still_running && states.iter().all(Option::is_some) {
        match start_bridges(members, channels, &states) {
            Ok(restored) => {
                let stats = run_audio(&restored, usize::from(channels), RUN_BLOCKS / 2);
                let resaved = restored
                    .iter()
                    .map(|bridge| bridge.save_state().ok().map(|state| state.bytes.len()))
                    .collect::<Vec<_>>();
                if stats.failed || !stats.finite || resaved.iter().any(Option::is_none) {
                    fail(
                        &mut problems,
                        "restore",
                        format!("failed={} reasons={:?}", stats.failed, reasons(&restored)),
                    );
                }
                steps.insert("restore".into(), json!({ "ok": !stats.failed, "audio": stats_json(&stats), "stateBytes": resaved }));
            }
            Err(error) => {
                fail(&mut problems, "restore", error.clone());
                steps.insert("restore".into(), json!({ "ok": false, "error": error }));
            }
        }
    } else {
        steps.insert(
            "restore".into(),
            json!({ "skipped": "an earlier step failed" }),
        );
    }

    (
        json!({ "name": name, "channels": channels, "steps": steps }),
        problems,
    )
}

fn markdown(report: &Value) -> String {
    let mut text = String::from("# VST2 chain matrix\n\n");
    text.push_str("| Combination | Ch | Start | Audio | Differs from dry | Editors shown | Restore |\n|---|---|---|---|---|---|---|\n");
    for run in report["runs"].as_array().into_iter().flatten() {
        if run["skipped"].is_string() {
            text.push_str(&format!(
                "| {} | - | skipped | - | - | - | - |\n",
                run["name"].as_str().unwrap_or("?")
            ));
            continue;
        }
        let steps = &run["steps"];
        let editors = steps["editors"].as_array();
        let shown = editors.map_or("-".to_owned(), |list| {
            format!(
                "{}/{}",
                list.iter().filter(|e| e["editorShown"] == true).count(),
                list.len()
            )
        });
        let yes = |value: &Value| match value {
            Value::Bool(true) => "yes",
            Value::Bool(false) => "NO",
            _ => "-",
        };
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            run["name"].as_str().unwrap_or("?"),
            run["channels"],
            yes(&steps["start"]["ok"]),
            if steps["audio"]["failed"] == false {
                "ok"
            } else {
                "FAILED"
            },
            yes(&steps["audio"]["differsFromDryTone"]),
            shown,
            yes(&steps["restore"]["ok"]),
        ));
    }
    text.push_str("\nProblems:\n");
    let problems = report["problems"].as_array().cloned().unwrap_or_default();
    if problems.is_empty() {
        text.push_str("- none\n");
    }
    for problem in problems {
        text.push_str(&format!("- {}\n", problem.as_str().unwrap_or("?")));
    }
    text
}

#[test]
#[ignore = "requires installed x64 VST2 effects (Blue Cat PatchWork, Pitchproof, ReaPlugs) and a Windows desktop"]
fn installed_vst2_effects_load_run_configure_and_show_editors_in_chains() {
    let show = std::env::var("AUDIOROUTER_CHAIN_SHOW_EDITORS").is_ok_and(|v| v == "1");
    let hold = Duration::from_millis(
        std::env::var("AUDIOROUTER_CHAIN_HOLD_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(1_500),
    );
    let filter = std::env::var("AUDIOROUTER_CHAIN_FILTER").unwrap_or_default();

    let mut plugins: HashMap<&'static str, Plugin> = HashMap::new();
    let mut skipped = Vec::new();
    // `AUDIOROUTER_CHAIN_SKIP=BC` leaves a plugin out (for example the demo
    // plugin whose "limiting" dialog pops up in its own window).
    let skip = std::env::var("AUDIOROUTER_CHAIN_SKIP").unwrap_or_default();
    for (key, path) in default_paths() {
        if skip.split(',').any(|skipped| skipped == key) {
            println!("{key}: left out by AUDIOROUTER_CHAIN_SKIP");
            continue;
        }
        match discover(key, path.clone()) {
            Ok(plugin) => {
                println!(
                    "{key}: {} parameter(s) at {}",
                    plugin.parameters.len(),
                    path.display()
                );
                plugins.insert(key, plugin);
            }
            Err(error) => {
                println!("{key}: skipped ({error})");
                skipped.push(
                    json!({ "plugin": key, "path": path.display().to_string(), "reason": error }),
                );
            }
        }
    }
    assert!(
        plugins.contains_key("BC") || skip.split(',').any(|skipped| skipped == "BC"),
        "Blue Cat PatchWork is not installed or not loadable: {skipped:?}"
    );

    // Instruments are refused up front, with a reason, rather than loaded.
    let synth = PathBuf::from(r"C:\Program Files\VSTPlugins\BC PatchWork Synth VST.dll");
    let synth_result = inspect_binary(&synth, &roots_of(&synth))
        .map(|identity| format!("{:?}", identity.compatibility()))
        .unwrap_or_else(|error| format!("inspect failed: {error:?}"));

    let combinations: &[&[&str]] = &[
        &["BC"],
        &["PP"],
        &["RC"],
        &["BC", "RC"],
        &["RC", "BC"],
        &["BC", "PP"],
        &["PP", "BC"],
        &["BC", "EQ"],
        &["BC", "RC", "EQ"],
        &["RC", "BC", "EQ"],
        &["RC", "EQ", "BC"],
        &["BC", "PP", "RC"],
        &["PP", "RC", "BC"],
        // The same plugin twice: two instances of one DLL in one chain.
        &["BC", "RC", "BC"],
        // Combinations without the demo plugin.
        &["EQ"],
        &["GT"],
        &["DL"],
        &["XC"],
        &["FR"],
        &["RC", "EQ"],
        &["EQ", "RC"],
        &["PP", "RC"],
        &["RC", "PP"],
        &["GT", "RC"],
        &["DL", "EQ"],
        &["FR", "XC"],
        &["RC", "EQ", "XC"],
        &["GT", "RC", "EQ"],
        &["PP", "RC", "EQ"],
        &["RC", "PP", "EQ"],
        &["DL", "XC", "PP"],
        &["FR", "GT", "DL"],
        // The same plugin twice and three times in one chain.
        &["RC", "EQ", "RC"],
        &["EQ", "EQ", "EQ"],
    ];
    let window = EditorHostWindow::open(show);
    let mut runs = Vec::new();
    let mut problems = Vec::new();
    for combination in combinations {
        let name = combination.join(">");
        if !filter.is_empty() && !name.contains(&filter) {
            continue;
        }
        let Some(members) = combination
            .iter()
            .map(|key| plugins.get(key))
            .collect::<Option<Vec<_>>>()
        else {
            runs.push(json!({ "name": name, "skipped": "a plugin is not installed" }));
            continue;
        };
        for channels in [1_u16, 2] {
            let started = Instant::now();
            let (run, found) = run_combination(&name, &members, channels, &window, hold);
            println!(
                "{name} x{channels}: {} problem(s) in {:.1}s",
                found.len(),
                started.elapsed().as_secs_f32()
            );
            runs.push(run);
            problems.extend(found);
        }
    }
    drop(window);

    let report = json!({
        "plugins": plugins.values().map(|plugin| json!({
            "key": plugin.key,
            "path": plugin.path.display().to_string(),
            "parameters": plugin.parameters.len(),
        })).collect::<Vec<_>>(),
        "skippedPlugins": skipped,
        "instrumentRefusal": { "BC PatchWork Synth": synth_result },
        "runs": runs,
        "problems": problems,
    });
    let path = std::env::var("AUDIOROUTER_CHAIN_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_exe()
                .expect("test path")
                .parent()
                .and_then(|deps| deps.parent())
                .expect("target directory")
                .join("vst2-chain-matrix.json")
        });
    std::fs::write(&path, serde_json::to_string_pretty(&report).expect("json")).expect("report");
    std::fs::write(path.with_extension("md"), markdown(&report)).expect("markdown report");
    println!("{}\nreport: {}", markdown(&report), path.display());
    assert!(
        problems.is_empty(),
        "{} problem(s); see the report",
        problems.len()
    );
}
