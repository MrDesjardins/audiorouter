//! Opt-in synthetic timing of quick requests that arrive while a bundle
//! export or import is running, over the production serve loop.
//!
//! The control plane handles one request at a time (the pipe I/O threads hand
//! each frame to the single control thread). A bundle export or import is one
//! request that can take seconds, so a quick request sent meanwhile waits for
//! it. This test measures that wait on a private pipe, with a poller that
//! sends `diagnostics.getVerbose` (a `quick` method in `METHOD_DEADLINES`,
//! so the UI gives it 5 s) every 20 ms. No audio device is used, so it says
//! nothing about audio continuity; it measures how long the control thread
//! is held, which is the delay every other control request sees.
//!
//! Run from the repository root (release mode, Windows only):
//!
//! ```text
//! $env:CARGO_BUILD_JOBS = 6
//! cargo test -p audiorouter-transport --release --locked --test service_timing_pipe -- --ignored --exact pipe_quick_requests_wait_behind_bundle_work --nocapture
//! ```
//!
//! Environment (all optional): `AUDIOROUTER_TIMING_CSV` (raw rows),
//! `AUDIOROUTER_TIMING_RUNS` (default 3), `AUDIOROUTER_TIMING_SCALE` (divides
//! blob sizes and counts, default 1; use 8 for a smoke run). It uses a pipe
//! named after the process ID, never the default control pipe, and
//! temporary databases under `target/service-timing-pipe-<pid>`. The serve
//! threads keep those databases open until the process exits and have no stop
//! request, so the test cannot delete them (about 1 to 2 GiB per run): remove
//! `target/service-timing-pipe-*` afterwards, when no run is in progress.
#![cfg(windows)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use audiorouter_control::{ClientGrant, ControlPlane};
use audiorouter_domain::{EntityId, Node, NodeKind, PermissionScope, Port, PortDirection, Session};
use audiorouter_storage::Storage;
use serde_json::{json, Value};

const BLOB_BYTES: usize = 16 * 1024 * 1024 - 1024;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn wav(total: usize, compressible: bool, seed: u64) -> Vec<u8> {
    let data = total - 44;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&48_000u32.to_le_bytes());
    out.extend_from_slice(&96_000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    if compressible {
        let period: Vec<u8> = (0..2048)
            .flat_map(|index| {
                let phase = f64::from(index) / 2048.0 * std::f64::consts::TAU;
                ((phase.sin() * 8000.0) as i16).to_le_bytes()
            })
            .collect();
        while out.len() < total {
            let take = (total - out.len()).min(period.len());
            out.extend_from_slice(&period[..take]);
        }
    } else {
        let mut state = seed | 1;
        while out.len() < total {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let bytes = state.to_le_bytes();
            let take = (total - out.len()).min(8);
            out.extend_from_slice(&bytes[..take]);
        }
    }
    out
}

fn node(id: &str, kind: NodeKind, direction: PortDirection, parameters: Value) -> Node {
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(parameters).unwrap(),
        ports: vec![Port {
            name: if kind == NodeKind::AudioFile {
                "out".into()
            } else {
                "main".into()
            },
            direction,
            channels: 2,
        }],
    }
}

fn media_session(id: &str, media: &[String]) -> Session {
    let mut nodes: Vec<Node> = media
        .iter()
        .enumerate()
        .map(|(index, media_id)| {
            node(
                &format!("audio-{index}"),
                NodeKind::AudioFile,
                PortDirection::Output,
                json!({ "mediaId": media_id, "fileName": "fixture.wav", "loop": false }),
            )
        })
        .collect();
    nodes.push(node(
        "out",
        NodeKind::PhysicalOutput,
        PortDirection::Input,
        json!({}),
    ));
    Session {
        id: EntityId::new(id),
        name: id.into(),
        schema_version: 1,
        revision: 0,
        nodes,
        edges: Vec::new(),
    }
}

/// A database holding `compressible` + `noise` 16 MiB blobs (prefix-named),
/// one session referencing each group, and `fillers` one-node sessions.
fn build_library(
    database: &std::path::Path,
    prefix: &str,
    compressible: usize,
    noise: usize,
    fillers: usize,
) {
    let storage = Storage::open(database).unwrap();
    let blob = (BLOB_BYTES / env_usize("AUDIOROUTER_TIMING_SCALE", 1)).max(64 * 1024);
    let mut compressible_ids = Vec::new();
    let mut noise_ids = Vec::new();
    for index in 0..compressible {
        let id = format!("{prefix}-c-{index:02}");
        storage
            .store_audio_media(&id, "fixture.wav", "wav", &wav(blob, true, 0), None)
            .unwrap();
        compressible_ids.push(id);
    }
    for index in 0..noise {
        let id = format!("{prefix}-r-{index:02}");
        storage
            .store_audio_media(
                &id,
                "fixture.wav",
                "wav",
                &wav(blob, false, 0x9e37_79b9 + index as u64),
                None,
            )
            .unwrap();
        noise_ids.push(id);
    }
    if compressible > 0 {
        storage
            .save_session(&media_session(
                &format!("{prefix}-compressible"),
                &compressible_ids,
            ))
            .unwrap();
    }
    if noise > 0 {
        storage
            .save_session(&media_session(&format!("{prefix}-noise"), &noise_ids))
            .unwrap();
    }
    for index in 0..fillers {
        storage
            .save_session(&Session {
                id: EntityId::new(format!("{prefix}-filler-{index:03}")),
                name: format!("Filler {index}"),
                schema_version: 1,
                revision: 0,
                nodes: vec![node(
                    "only",
                    NodeKind::Gain,
                    PortDirection::Output,
                    json!({}),
                )],
                edges: Vec::new(),
            })
            .unwrap();
    }
}

fn call(pipe: &str, method: &str, params: Value) -> Result<Value, String> {
    let request = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let frame =
        audiorouter_protocol::encode_frame(&request).map_err(|error| format!("{error:?}"))?;
    let response =
        audiorouter_transport::round_trip(pipe, &frame).map_err(|error| format!("{error:?}"))?;
    let response: Value =
        audiorouter_protocol::decode_frame(&response).map_err(|error| format!("{error:?}"))?;
    if !response["error"].is_null() {
        return Err(response["error"].to_string());
    }
    Ok(response["result"].clone())
}

fn grant() -> ClientGrant {
    ClientGrant::with_scopes([
        PermissionScope::Read,
        PermissionScope::GraphWrite,
        PermissionScope::SessionControl,
        PermissionScope::FileExport,
    ])
}

/// Start the production serve loop on a private pipe and wait until it answers.
fn serve(database: std::path::PathBuf, pipe: String) {
    let server_pipe = pipe.clone();
    std::thread::spawn(move || {
        let plane =
            ControlPlane::try_with_storage("service-timing", Storage::open(database).unwrap())
                .unwrap();
        let _ = audiorouter_transport::serve_control_connections_forever_with_grant(
            &server_pipe,
            plane,
            grant(),
        );
    });
    for _ in 0..200 {
        if call(&pipe, "diagnostics.getVerbose", Value::Null).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the timing backend did not start");
}

/// A request to send: a label, the method and its parameters.
type HeavyRequest = (String, String, Value);

/// One scenario to run: its name, the pipe to use and the heavy requests.
type Plan<'a> = (&'a str, String, Vec<HeavyRequest>);

/// One poll: send offset (ms), latency (ms) and the error text if it failed.
type Poll = (f64, f64, Option<String>);

/// One heavy request: start and end offsets (ms from the scenario start).
struct Heavy {
    name: String,
    start: f64,
    end: f64,
}

/// Run `heavy` requests (each on its own client thread, started together)
/// while a poller sends quick requests every 20 ms. Returns the poll
/// latencies in ms (with their send offsets) and the heavy timings. A 2 s
/// idle lead-in and lead-out frame the heavy window.
fn scenario(pipe: &str, pollers: usize, heavy: Vec<HeavyRequest>) -> (Vec<Poll>, Vec<Heavy>) {
    let origin = Instant::now();
    let stop = Arc::new(AtomicBool::new(false));
    let pollers: Vec<_> = (0..pollers)
        .map(|_| {
            let poller_pipe = pipe.to_owned();
            let poller_stop = stop.clone();
            std::thread::spawn(move || {
                let mut polls = Vec::new();
                while !poller_stop.load(Ordering::Acquire) {
                    let sent = origin.elapsed();
                    let started = Instant::now();
                    let error = call(&poller_pipe, "diagnostics.getVerbose", Value::Null).err();
                    polls.push((millis(sent), millis(started.elapsed()), error));
                    std::thread::sleep(Duration::from_millis(20));
                }
                polls
            })
        })
        .collect();
    std::thread::sleep(Duration::from_secs(2));
    let workers: Vec<_> = heavy
        .into_iter()
        .map(|(name, method, params)| {
            let pipe = pipe.to_owned();
            std::thread::spawn(move || {
                if method == "sessions.exportFile" {
                    // The Save-dialog approval is a separate quick request;
                    // it is expired by queueing, so it is made just before.
                    call(
                        &pipe,
                        "sessions.approveExport",
                        json!({ "path": params["path"], "replace": false }),
                    )
                    .unwrap();
                }
                let start = millis(origin.elapsed());
                let result = call(&pipe, &method, params);
                let end = millis(origin.elapsed());
                assert!(result.is_ok(), "{name}: {result:?}");
                Heavy { name, start, end }
            })
        })
        .collect();
    let heavy: Vec<Heavy> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    std::thread::sleep(Duration::from_secs(2));
    stop.store(true, Ordering::Release);
    let polls = pollers
        .into_iter()
        .flat_map(|poller| poller.join().unwrap())
        .collect();
    (polls, heavy)
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * fraction).round() as usize]
}

#[test]
#[ignore = "opt-in timing harness; builds ~600 MiB of synthetic databases and bundles"]
fn pipe_quick_requests_wait_behind_bundle_work() {
    let scale = env_usize("AUDIOROUTER_TIMING_SCALE", 1);
    let runs = env_usize("AUDIOROUTER_TIMING_RUNS", 3);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("service-timing-pipe-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    // Export approval refuses the `\\?\` verbatim form `canonicalize` returns.
    let root = std::path::PathBuf::from(
        root.canonicalize()
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\"),
    );
    // Import staging uses a fixed %TEMP% name; keep this process's apart.
    std::fs::create_dir_all(root.join("temp")).unwrap();
    std::env::set_var("TEMP", root.join("temp"));
    std::env::set_var("TMP", root.join("temp"));
    let header = "run,scenario,heavyRequests,heavyMs,pollsDuring,pollMedianMs,pollP99Ms,pollMaxMs,pollsOver5s,idlePollMedianMs,idlePollMaxMs,failedPolls";
    let mut lines = Vec::new();
    for run in 0..runs {
        let run_root = root.join(format!("run-{run}"));
        std::fs::create_dir_all(&run_root).unwrap();
        // Bundles to import come from a separate factory database.
        let factory = run_root.join("factory.sqlite");
        build_library(&factory, "imp", (15 / scale).max(2), (6 / scale).max(2), 0);
        let factory_pipe = format!(
            r"\\.\pipe\audiorouter-service-timing-f{run}-{}",
            std::process::id()
        );
        serve(factory.clone(), factory_pipe.clone());
        let mut bundles = Vec::new();
        for session in ["imp-compressible", "imp-noise"] {
            let path = run_root.join(format!("{session}.audiorouter"));
            call(
                &factory_pipe,
                "sessions.approveExport",
                json!({ "path": path, "replace": false }),
            )
            .unwrap();
            call(
                &factory_pipe,
                "sessions.exportFile",
                json!({ "sessionId": session, "path": path }),
            )
            .unwrap();
            bundles.push(path);
        }

        // Backend 1: the large library, for export scenarios.
        let large = run_root.join("large.sqlite");
        build_library(
            &large,
            "lib",
            (15 / scale).max(2),
            (10 / scale).max(2),
            (100 / scale).max(4),
        );
        let large_pipe = format!(
            r"\\.\pipe\audiorouter-service-timing-l{run}-{}",
            std::process::id()
        );
        serve(large, large_pipe.clone());
        // Backend 2: a smaller library that imports and exports at once.
        let mixed = run_root.join("mixed.sqlite");
        // 60 fillers keep the 128-node global cap clear of the four imports.
        build_library(&mixed, "mix", 0, (6 / scale).max(2), (60 / scale).max(4));
        let mixed_pipe = format!(
            r"\\.\pipe\audiorouter-service-timing-m{run}-{}",
            std::process::id()
        );
        serve(mixed, mixed_pipe.clone());

        let export = |_pipe: &str, session: &str, name: &str| {
            let path = run_root.join(format!("{name}.audiorouter"));
            (
                name.to_owned(),
                "sessions.exportFile".to_owned(),
                json!({ "sessionId": session, "path": path }),
            )
        };
        let import = |bundle: &std::path::Path, name: &str| {
            (
                name.to_owned(),
                "sessions.importFile".to_owned(),
                json!({ "path": bundle }),
            )
        };
        // Alternate which export runs first so a warm cache favours neither.
        let mut plan: Vec<Plan> = vec![
            ("idle", large_pipe.clone(), vec![]),
            (
                "export-compressible",
                large_pipe.clone(),
                vec![export(
                    &large_pipe,
                    "lib-compressible",
                    &format!("pipe-c-{run}"),
                )],
            ),
            (
                "export-noise",
                large_pipe.clone(),
                vec![export(&large_pipe, "lib-noise", &format!("pipe-n-{run}"))],
            ),
            (
                "import-compressible",
                mixed_pipe.clone(),
                vec![import(&bundles[0], "import-c")],
            ),
            (
                "import-noise",
                mixed_pipe.clone(),
                vec![import(&bundles[1], "import-n")],
            ),
        ];
        if run % 2 == 1 {
            plan.swap(1, 2);
        }
        for (name, pipe, heavy) in plan {
            let heavy_count = heavy.len();
            let (polls, heavy) = scenario(&pipe, 1, heavy);
            lines.push(summarize(run, name, heavy_count, &polls, &heavy));
        }
        // Export and import queued together on the mixed backend. By now
        // that database already holds `imp-compressible`'s media, so this
        // second import restores less than the first did.
        let combined = vec![
            export(&mixed_pipe, "mix-noise", &format!("pipe-mix-{run}")),
            import(&bundles[0], "import-c-again"),
        ];
        let (polls, heavy) = scenario(&mixed_pipe, 1, combined);
        lines.push(summarize(
            run,
            "export + re-import (media already present)",
            2,
            &polls,
            &heavy,
        ));
        // Six clients polling at once (the pipe has four instances) while the
        // slow noise export runs: do the extra clients wait or fail?
        let crowded = vec![export(
            &large_pipe,
            "lib-noise",
            &format!("pipe-crowd-{run}"),
        )];
        let (polls, heavy) = scenario(&large_pipe, 6, crowded);
        lines.push(summarize(
            run,
            "export-noise with 6 pollers",
            1,
            &polls,
            &heavy,
        ));
        // The serve threads hold their databases open until the process ends,
        // so the fixtures under target/ cannot be removed here.
    }
    if let Ok(path) = std::env::var("AUDIOROUTER_TIMING_CSV") {
        std::fs::write(path, format!("{header}\n{}\n", lines.join("\n"))).unwrap();
    }
    eprintln!("{header}");
    for line in &lines {
        eprintln!("{line}");
    }
}

fn summarize(
    run: usize,
    name: &str,
    heavy_count: usize,
    polls: &[Poll],
    heavy: &[Heavy],
) -> String {
    let (window_start, window_end) = if heavy.is_empty() {
        (500.0, 1_500.0)
    } else {
        (
            heavy.iter().map(|h| h.start).fold(f64::MAX, f64::min),
            heavy.iter().map(|h| h.end).fold(0.0, f64::max),
        )
    };
    // A poll that was *sent* during the heavy window is the one that waits.
    let mut during: Vec<f64> = polls
        .iter()
        .filter(|(sent, _, _)| *sent >= window_start && *sent <= window_end)
        .map(|(_, latency, _)| *latency)
        .collect();
    let mut idle: Vec<f64> = polls
        .iter()
        .filter(|(sent, _, _)| *sent < window_start - 100.0)
        .map(|(_, latency, _)| *latency)
        .collect();
    during.sort_by(f64::total_cmp);
    idle.sort_by(f64::total_cmp);
    let failed = polls.iter().filter(|(_, _, error)| error.is_some()).count();
    for (sent, latency, error) in polls {
        if let Some(error) = error {
            eprintln!(
                "run {run} {name}: poll sent at {sent:.0} ms failed after {latency:.0} ms: {error}"
            );
        }
    }
    let over = during.iter().filter(|latency| **latency > 5_000.0).count();
    for item in heavy {
        eprintln!(
            "run {run} {name}: {} {:.0}..{:.0} ms",
            item.name, item.start, item.end
        );
    }
    format!(
        "{run},{name},{heavy_count},{:.1},{},{:.1},{:.1},{:.1},{over},{:.1},{:.1},{failed}",
        window_end - window_start,
        during.len(),
        if during.is_empty() {
            0.0
        } else {
            percentile(&during, 0.5)
        },
        if during.is_empty() {
            0.0
        } else {
            percentile(&during, 0.99)
        },
        during.last().copied().unwrap_or(0.0),
        if idle.is_empty() {
            0.0
        } else {
            percentile(&idle, 0.5)
        },
        idle.last().copied().unwrap_or(0.0),
    )
}
