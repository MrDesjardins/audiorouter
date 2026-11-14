//! Opt-in plugin census (full VST compatibility plan, phase 0).
//!
//! Walks folders of installed plugins (VST2 DLLs and VST3 bundles, 64-bit and
//! 32-bit) and runs each supported effect through the production path: the
//! dispatcher's `plugins.parameters`, then the real isolated worker for the
//! plugin's format and bitness at mono and stereo: start, 100 blocks of a
//! tone, save state, restore, restore into a fresh worker. Every result is
//! recorded, none asserted, so one bad plugin never hides the rest.
//!
//! ```text
//! AUDIOROUTER_CENSUS_DIRS="C:\Program Files\VSTPlugins;C:\Program Files\Common Files\VST3"
//! AUDIOROUTER_CENSUS_SKIP="BC PatchWork"        (substrings; demo plugins that pop dialogs)
//! AUDIOROUTER_CENSUS_REPORT=C:\path\census.json (also writes .md beside it)
//! cargo test -p audiorouter-control plugin_census -- --ignored --nocapture
//! ```
//!
//! It opens no audio device and no plugin editor (the VST2 chain matrix in
//! `crates/plugin-host/tests` covers editors). A plugin that opens a licence
//! or demo dialog can block a worker until the supervisor deadline: skip it.
#![cfg(windows)]

use super::*;
use audiorouter_plugin_host::{
    inspect_binary, worker_clock_tick, PluginCompatibility, PluginFormat, SupervisedWorkerProcess,
    WorkerFrame,
};
use std::path::{Path, PathBuf};

/// One plugin file or bundle found under a census folder.
fn candidates(directory: &Path, found: &mut Vec<PathBuf>, depth: usize) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        match name.as_deref() {
            Some("vst3") => found.push(path),
            Some("dll") if path.is_file() => found.push(path),
            _ if path.is_dir() && depth < 3 => candidates(&path, found, depth + 1),
            _ => {}
        }
    }
}

fn tone(block: u64, channels: u16) -> Vec<f32> {
    (0..128_u64)
        .flat_map(|frame| {
            let value = 0.25
                * (std::f32::consts::TAU * 220.0 * (block * 128 + frame) as f32 / 48_000.0).sin();
            std::iter::repeat_n(value, usize::from(channels))
        })
        .collect()
}

/// Everything one width tells about one plugin; an `Err` is the first step
/// that failed.
fn census_width(
    identity: &audiorouter_plugin_host::PluginIdentity,
    root: &Path,
    channels: u16,
) -> serde_json::Value {
    let kind = PluginWorkerKind::for_identity(identity.format, identity.architecture);
    let executable = match require_plugin_worker(kind) {
        Ok(path) => path,
        Err(error) => {
            return json!({ "channels": channels, "failed": format!("no worker: {error:?}") })
        }
    };
    let spawn = || match kind {
        PluginWorkerKind::Vst2 | PluginWorkerKind::Vst2X86 => {
            SupervisedWorkerProcess::spawn_verified_with_sample_rate(
                &executable,
                identity,
                std::slice::from_ref(&root.to_path_buf()),
                channels,
                48_000,
                Instant::now(),
            )
        }
        PluginWorkerKind::Vst3 | PluginWorkerKind::Vst3X86 => {
            SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
                &executable,
                identity,
                std::slice::from_ref(&root.to_path_buf()),
                channels,
                48_000,
                Instant::now(),
            )
        }
    };
    let mut worker = match spawn() {
        Ok(worker) => worker,
        Err(error) => return json!({ "channels": channels, "failed": format!("start: {error:?}") }),
    };
    let saved = worker.save_state(Instant::now());
    let state_bytes = saved.as_ref().map(|asset| asset.bytes.len()).ok();
    let (mut peak, mut difference, mut non_finite) = (0.0_f32, 0.0_f32, false);
    let mut audio_error = None;
    for block in 0..100_u64 {
        let samples = tone(block, channels);
        let frame = match WorkerFrame::new(
            block + 1,
            worker_clock_tick().saturating_add(10_000),
            channels,
            samples.clone(),
        ) {
            Ok(frame) => frame,
            Err(error) => {
                audio_error = Some(format!("frame: {error:?}"));
                break;
            }
        };
        match worker.process(frame, vec![], Instant::now()) {
            Ok(output) => {
                for (sample, input) in output.samples.into_iter().zip(samples) {
                    non_finite |= !sample.is_finite();
                    peak = peak.max(sample.abs());
                    difference = difference.max((sample - input).abs());
                }
            }
            Err(error) => {
                audio_error = Some(format!("process: {error:?}"));
                break;
            }
        }
    }
    let restore_same = saved
        .as_ref()
        .map_err(|error| format!("{error:?}"))
        .and_then(|asset| {
            worker
                .restore_state(asset.clone(), Instant::now())
                .map_err(|error| format!("{error:?}"))
        });
    let exit_ok = worker.shutdown().map(|status| status.success()).ok();
    let restore_fresh = match (&saved, spawn()) {
        (Ok(asset), Ok(mut fresh)) => {
            let result = fresh
                .restore_state(asset.clone(), Instant::now())
                .map_err(|error| format!("{error:?}"));
            let _ = fresh.shutdown();
            result
        }
        (Err(error), _) => Err(format!("no state to restore: {error:?}")),
        (_, Err(error)) => Err(format!("restart: {error:?}")),
    };
    json!({
        "channels": channels,
        "stateBytes": state_bytes,
        "stateSaved": saved.is_ok(),
        "audioError": audio_error,
        "nonFinite": non_finite,
        "peak": peak,
        "changesSignal": difference > 0.001,
        "silent": peak <= 0.001,
        "restoreSame": restore_same.as_ref().err(),
        "restoreFresh": restore_fresh.as_ref().err(),
        "cleanExit": exit_ok,
    })
}

fn census_one(path: &Path, root: &Path, plane: &mut ControlPlane) -> serde_json::Value {
    let identity = match inspect_binary(path, std::slice::from_ref(&root.to_path_buf())) {
        Ok(identity) => identity,
        Err(error) => return json!({ "path": path, "refused": format!("{error:?}") }),
    };
    let compatibility = identity.compatibility();
    let mut result = json!({
        "path": path,
        "format": match identity.format { PluginFormat::Vst3 => "vst3", PluginFormat::Vst2 => "vst2", _ => "unknown" },
        "architecture": format!("{:?}", identity.architecture),
        "compatibility": format!("{compatibility:?}"),
    });
    if !matches!(
        compatibility,
        PluginCompatibility::SupportedVst3X64
            | PluginCompatibility::SupportedVst2X64Gated
            | PluginCompatibility::SupportedVst3X86
            | PluginCompatibility::SupportedVst2X86
    ) {
        result["skipped"] = json!("not a supported effect");
        return result;
    }
    let response = plane.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: "plugins.parameters".into(),
        params: Some(json!({ "path": path })),
    });
    result["parameters"] = match (&response.error, &response.result) {
        (None, Some(value)) => json!(value["parameters"].as_array().map_or(0, Vec::len)),
        (Some(error), _) => json!({ "error": error.message }),
        _ => json!({ "error": "no result" }),
    };
    result["widths"] = json!([
        census_width(&identity, root, 1),
        census_width(&identity, root, 2)
    ]);
    result
}

#[test]
#[ignore = "opt-in census of installed plugins; see the module documentation"]
fn plugin_census() {
    let directories = std::env::var("AUDIOROUTER_CENSUS_DIRS").expect("AUDIOROUTER_CENSUS_DIRS");
    let skip = std::env::var("AUDIOROUTER_CENSUS_SKIP").unwrap_or_default();
    let mut plane = ControlPlane::default();
    let mut rows = Vec::new();
    for directory in directories.split(';').filter(|entry| !entry.is_empty()) {
        let root = PathBuf::from(directory);
        // `plugins.parameters` only accepts a path from a current scan.
        let scan = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "plugins.scan".into(),
            params: Some(json!({ "directory": root })),
        });
        if let Some(error) = scan.error {
            eprintln!("census: scan of {directory} failed: {}", error.message);
        }
        let mut found = Vec::new();
        candidates(&root, &mut found, 0);
        found.sort();
        for path in found {
            let text = path.display().to_string();
            if skip
                .split(',')
                .filter(|part| !part.is_empty())
                .any(|part| text.contains(part))
            {
                rows.push(
                    json!({ "path": text, "skipped": "left out by AUDIOROUTER_CENSUS_SKIP" }),
                );
                continue;
            }
            eprintln!("census: {text}");
            rows.push(census_one(&path, &root, &mut plane));
        }
    }
    let report = json!({ "plugins": rows });
    let destination = std::env::var("AUDIOROUTER_CENSUS_REPORT").map_or_else(
        |_| std::env::temp_dir().join("plugin-census.json"),
        PathBuf::from,
    );
    std::fs::write(&destination, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    std::fs::write(destination.with_extension("md"), census_markdown(&report)).unwrap();
    eprintln!("{}", census_markdown(&report));
    eprintln!("report: {}", destination.display());
}

fn census_markdown(report: &serde_json::Value) -> String {
    let mut text = String::from(
        "| Plugin | Format | Bits | Params | Mono | Stereo | Notes |\n|---|---|---|---|---|---|---|\n",
    );
    for row in report["plugins"].as_array().into_iter().flatten() {
        let name = Path::new(row["path"].as_str().unwrap_or("?"))
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        if let Some(reason) = row["refused"].as_str().or_else(|| row["skipped"].as_str()) {
            text.push_str(&format!("| {name} | - | - | - | - | - | {reason} |\n"));
            continue;
        }
        let width = |index: usize| -> String {
            let width = &row["widths"][index];
            if let Some(failed) = width["failed"].as_str() {
                return format!("FAILED {failed}");
            }
            let mut flags = Vec::new();
            if width["audioError"].is_string() {
                flags.push("audio error");
            }
            if width["nonFinite"] == true {
                flags.push("non-finite");
            }
            if width["silent"] == true {
                flags.push("silent");
            }
            if width["changesSignal"] == false {
                flags.push("pass-through");
            }
            if !width["restoreSame"].is_null() || !width["restoreFresh"].is_null() {
                flags.push("restore failed");
            }
            if width["stateSaved"] == false {
                flags.push("no state");
            }
            if flags.is_empty() {
                "ok".into()
            } else {
                flags.join(", ")
            }
        };
        text.push_str(&format!(
            "| {name} | {} | {} | {} | {} | {} | |\n",
            row["format"].as_str().unwrap_or("?"),
            row["architecture"].as_str().unwrap_or("?"),
            row["parameters"],
            width(0),
            width(1),
        ));
    }
    text
}

/// Regression (2026-11-13): an instrument whose scan metadata says nothing
/// (Surge XT, PatchWork Synth) was listed as a supported effect. After the
/// first load attempt, the worker's refusal must change the next scan.
#[test]
#[ignore = "requires AUDIOROUTER_TEST_INSTRUMENT (an instrument plugin) and the packaged workers"]
fn an_instrument_refused_at_load_is_listed_as_an_instrument_afterwards() {
    let path = PathBuf::from(std::env::var_os("AUDIOROUTER_TEST_INSTRUMENT").unwrap());
    let root = path.parent().unwrap().to_path_buf();
    let mut plane = ControlPlane::default();
    let mut call = |method: &str, params: serde_json::Value| {
        plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        })
    };
    let compatibility = |response: &JsonRpcResponse| {
        response.result.as_ref().unwrap()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| {
                entry["path"]
                    .as_str()
                    .is_some_and(|text| Path::new(text) == path)
            })
            .map(|entry| {
                entry["identity"]["compatibility"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .unwrap()
    };
    let first = call("plugins.scan", json!({ "directory": root }));
    assert!(
        compatibility(&first).starts_with("supported"),
        "precondition: the scan cannot see an instrument: {}",
        compatibility(&first)
    );
    let refused = call("plugins.parameters", json!({ "path": path }));
    let message = refused
        .error
        .expect("the worker refuses an instrument")
        .message;
    assert!(message.contains("instrument"), "{message}");
    let second = call("plugins.scan", json!({ "directory": root }));
    assert_eq!(compatibility(&second), "unsupportedInstrument");
}
