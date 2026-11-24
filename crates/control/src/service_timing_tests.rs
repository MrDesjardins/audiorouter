//! Opt-in synthetic service-timing harness (all tests `#[ignore]`).
//!
//! Answers two questions of the 2026-11-02 code-review plan and of resilience
//! finding RES-6, on a disposable synthetic library and never on user data:
//!
//! * `res6_slow_method_timings`: how long do the slow, device-free methods
//!   take through `ControlPlane::dispatch` for bundles up to the importer's
//!   limits, compared with their `METHOD_DEADLINES` class (quick 5 s,
//!   long 60 s)?
//! * `service_cadence_while_bundle_work_runs`: while a bundle export or the
//!   import stages run on the helper thread, how regularly does the control
//!   thread's service closure run (the cadence of `service_running_native_audio`
//!   passes), against an idle baseline of the same length? It also times the
//!   inline (not serviced) steps of an import, and how import time scales with
//!   earlier imports of the same session ID.
//!
//! * `play_path_media_load_and_compile`: the device-free steps of Play (decode
//!   every Audio File, compile the graph) for eight 100 s stereo files (a Mixer takes at most 8 sources).
//! * `export_of_an_asset_over_the_import_limit`: a small probe that a 20 MiB
//!   audio asset exports and imports (media entries may be 64 MiB).
//!
//! A passing run deletes its `target/service-timing-*` directory; a failing
//! one leaves it behind for inspection, so remove stale ones by hand.
//!
//! The pipe-level question (how long a quick request waits behind a running
//! export) is `crates/transport/tests/service_timing_pipe.rs`.
//!
//! Run from the repository root (release mode; one test at a time, because
//! each builds a ~400 MiB database under a unique directory):
//!
//! ```text
//! $env:CARGO_BUILD_JOBS = 6
//! cargo test -p audiorouter-control --release --lib --locked service_timing_tests::res6 -- --ignored --exact --nocapture
//! cargo test -p audiorouter-control --release --lib --locked service_timing_tests::service_cadence -- --ignored --exact --nocapture
//! ```
//!
//! Environment (all optional):
//!
//! * `AUDIOROUTER_TIMING_CSV`: file that receives the raw CSV rows (the rows
//!   are always printed to stderr).
//! * `AUDIOROUTER_TIMING_RUNS`: repetitions, default 3. Odd runs build and
//!   measure the fixtures in the opposite order, so a warm cache does not
//!   always favour the same one.
//! * `AUDIOROUTER_TIMING_SCALE`: divides every blob size and count, default 1.
//!   Use 8 for a quick smoke run; recorded evidence uses 1.
//!
//! `sessions.importFile` stages every bundle under `%TEMP%\audiorouter-session-import`
//! (a fixed name shared with the ordinary test suite), so each test points
//! `TEMP`/`TMP` at its own run directory first.

use super::*;
use crate::test_support::{Edge, Node, NodeKind, Port, PortDirection};
use std::time::{Duration, Instant};

/// Per-asset bundle limit; each fixture blob stays just below it.
const BLOB_BYTES: usize = 16 * 1024 * 1024 - 1024;
/// Quick requests use the UI's ordinary 5 s deadline; others get 60 s.
const QUICK_DEADLINE_MS: f64 = 5_000.0;
const LONG_DEADLINE_MS: f64 = 60_000.0;

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

/// Seeded xorshift64: incompressible payload without a dependency.
struct Xorshift(u64);

impl Xorshift {
    fn fill(&mut self, buffer: &mut [u8]) {
        for chunk in buffer.chunks_mut(8) {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            let bytes = self.0.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }
}

/// A valid mono 16-bit 48 kHz WAV of `total` bytes. Compressible blobs repeat
/// a 4 KiB sine period (deflate shrinks them to a few percent); the others are
/// noise, which is what real recordings behave like under deflate.
fn wav(total: usize, compressible: bool, seed: u64) -> Vec<u8> {
    wav_with_format(total, compressible, seed, 48_000, 1)
}

/// Like [`wav`], with the sample rate and channel count in the header (16-bit PCM).
fn wav_with_format(
    total: usize,
    compressible: bool,
    seed: u64,
    sample_rate: u32,
    channels: u16,
) -> Vec<u8> {
    let data = total - 44;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
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
        let mut noise = vec![0u8; data];
        Xorshift(seed | 1).fill(&mut noise);
        out.extend_from_slice(&noise);
    }
    out
}

/// A stereo 32-bit IEEE-float WAV of `frames` noise frames in [-1, 1).
fn wav_float32_stereo(frames: usize, sample_rate: u32, seed: u64) -> Vec<u8> {
    let data = frames * 8;
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 8).to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    let mut state = seed | 1;
    for _ in 0..frames * 2 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let sample = (state >> 40) as f32 / 8_388_608.0 - 1.0;
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let value = (u32::from(group[0]) << 16)
            | (u32::from(*group.get(1).unwrap_or(&0)) << 8)
            | u32::from(*group.get(2).unwrap_or(&0));
        for shift in [18, 12, 6, 0] {
            out.push(ALPHABET[((value >> shift) & 63) as usize] as char);
        }
        let padding = 3 - group.len();
        for _ in 0..padding {
            out.pop();
        }
        for _ in 0..padding {
            out.push('=');
        }
    }
    out
}

fn audio_node(id: &str, media_id: &str) -> Node {
    Node {
        id: EntityId::new(id),
        kind: NodeKind::AudioFile,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: serde_json::from_value(json!({
            "mediaId": media_id, "fileName": "fixture.wav", "loop": false
        }))
        .unwrap(),
        ports: vec![Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    }
}

fn plain_node(id: &str, kind: NodeKind, direction: PortDirection) -> Node {
    Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports: vec![Port {
            name: "main".into(),
            direction,
            channels: 2,
        }],
    }
}

/// A session of `media` AudioFile nodes (one per blob ID) and no edges.
fn media_session(id: &str, media: &[String]) -> Session {
    let mut session = crate::test_support::session();
    session.id = EntityId::new(id);
    session.name = id.into();
    session.nodes = media
        .iter()
        .enumerate()
        .map(|(index, media_id)| audio_node(&format!("audio-{index}"), media_id))
        .collect();
    session.nodes.push(plain_node(
        "out",
        NodeKind::PhysicalOutput,
        PortDirection::Input,
    ));
    session.edges.clear();
    session
}

fn filler_session(index: usize) -> Session {
    let mut session = crate::test_support::session();
    session.id = EntityId::new(format!("filler-{index:03}"));
    session.name = format!("Filler {index}");
    session.nodes = vec![plain_node("only", NodeKind::Gain, PortDirection::Output)];
    session.edges = Vec::<Edge>::new();
    session
}

/// What the synthetic "large library" holds, and why that size.
///
/// The plan never quantifies "large", so this fills the stores toward their
/// own caps: media toward `MAX_AUDIO_MEDIA_TOTAL_BYTES` (512 MiB; 25 blobs of
/// ~16 MiB is ~400 MiB, the size of the 2026-11-01 open-timing fixture), the
/// session count toward `MAX_SESSIONS_GLOBAL` (128 sessions; the node cap of
/// 128 leaves room for 100 one-node fillers beside the two export sessions),
/// and 2,000 recording rows (listing is capped at 500 per page).
struct Library {
    root: std::path::PathBuf,
    recordings: usize,
}

impl Library {
    fn database(&self) -> std::path::PathBuf {
        self.root.join("library.sqlite")
    }
}

fn build_library(root: &std::path::Path, scale: usize) -> Library {
    std::fs::create_dir_all(root).unwrap();
    let blob = (BLOB_BYTES / scale).max(64 * 1024);
    let compressible_count = (15 / scale).max(2);
    let noise_count = (10 / scale).max(2);
    let storage = Storage::open(root.join("library.sqlite")).unwrap();
    let mut compressible_media = Vec::new();
    let mut noise_media = Vec::new();
    for index in 0..compressible_count {
        let id = format!("lib-c-{index:02}");
        storage
            .store_audio_media(&id, "fixture.wav", "wav", &wav(blob, true, 0), None)
            .unwrap();
        compressible_media.push(id);
    }
    for index in 0..noise_count {
        let id = format!("lib-r-{index:02}");
        storage
            .store_audio_media(
                &id,
                "fixture.wav",
                "wav",
                &wav(blob, false, 0x9e37_79b9 + index as u64),
                None,
            )
            .unwrap();
        noise_media.push(id);
    }
    storage
        .save_session(&media_session("export-compressible", &compressible_media))
        .unwrap();
    let noise_exported = &noise_media[..noise_media.len().min(6)];
    storage
        .save_session(&media_session("export-noise", noise_exported))
        .unwrap();
    for index in 0..(100 / scale).max(4) {
        storage.save_session(&filler_session(index)).unwrap();
    }
    let recordings = (2_000 / scale).max(50);
    for index in 0..recordings {
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: format!("rec-{index:05}"),
                session_id: "export-noise".into(),
                recorder_id: "recorder".into(),
                path: format!("C:\\synthetic\\take-{index:05}.wav"),
                format: "wav".into(),
                channels: 2,
                sample_rate: 48_000,
                frames: 48_000 * 30,
                file_bytes: 48_000 * 30 * 4,
                start_time: "2026-11-01T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: Some(format!("Take {index}")),
                artist: None,
                comment: None,
                dither: false,
                conversion: "none".into(),
            })
            .unwrap();
    }
    drop(storage);
    Library {
        root: root.to_path_buf(),
        recordings,
    }
}

fn plane_for(database: &std::path::Path, build: &str) -> ControlPlane {
    ControlPlane::try_with_storage(build, Storage::open(database).unwrap()).unwrap()
}

/// One timed `ControlPlane::dispatch`; a refused request still returns its
/// time but marks the row, so a failing method is never silently "fast".
fn timed(plane: &mut ControlPlane, method: &str, params: Value) -> (Duration, Option<Value>) {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!(1)),
        method: method.into(),
        params: Some(params),
    };
    let start = Instant::now();
    let response = plane.dispatch(request);
    let elapsed = start.elapsed();
    if let Some(error) = &response.error {
        eprintln!("ERROR {method}: {error:?}");
    }
    (elapsed, response.result)
}

struct Rows {
    header: &'static str,
    lines: Vec<String>,
}

impl Rows {
    fn new(header: &'static str) -> Self {
        Self {
            header,
            lines: Vec::new(),
        }
    }

    fn push(&mut self, line: String) {
        eprintln!("{line}");
        self.lines.push(line);
    }

    fn save(&self) {
        if let Ok(path) = std::env::var("AUDIOROUTER_TIMING_CSV") {
            let mut text = format!("{}\n", self.header);
            for line in &self.lines {
                text.push_str(line);
                text.push('\n');
            }
            std::fs::write(path, text).unwrap();
        }
    }
}

fn deadline_class(method: &str) -> (&'static str, f64) {
    // Mirrors `METHOD_DEADLINES` in contracts/src/index.ts for the methods
    // measured here; `deadline_classes_match_the_contract_catalog` checks it.
    match method {
        "sessions.get" | "sessions.list" | "recordings.list" => ("quick", QUICK_DEADLINE_MS),
        method if method.contains('.') => ("long", LONG_DEADLINE_MS),
        // Not an RPC method (fixture build, service pass): no deadline.
        _ => ("none", f64::INFINITY),
    }
}

fn unique_root(label: &str) -> std::path::PathBuf {
    let root = std::env::current_dir()
        .unwrap()
        .join("..")
        .join("..")
        .join("target")
        .join(format!("service-timing-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    // Export approval refuses the `\\?\` verbatim form `canonicalize` returns.
    let plain = root.canonicalize().unwrap();
    std::path::PathBuf::from(plain.to_string_lossy().trim_start_matches(r"\\?\"))
}

/// Point `TEMP`/`TMP` at `dir` so import staging does not share the fixed
/// `%TEMP%\audiorouter-session-import` with other processes. The harness is
/// the only thread running in its process when it calls this.
fn isolate_temp(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::env::set_var("TEMP", dir);
    std::env::set_var("TMP", dir);
}

#[test]
fn deadline_classes_match_the_contract_catalog() {
    let catalog = include_str!("../../../contracts/src/index.ts");
    for method in [
        "sessions.approveExport",
        "sessions.exportFile",
        "sessions.importFile",
        "sessions.importPlan",
        "sessions.importCommit",
        "audioMedia.beginUpload",
        "audioMedia.uploadChunk",
        "audioMedia.finishUpload",
        "audioMedia.delete",
        "sessions.get",
        "sessions.list",
        "recordings.list",
    ] {
        let (class, _) = deadline_class(method);
        assert!(
            catalog.contains(&format!("\"{method}\": \"{class}\",")),
            "{method} is not \"{class}\" in METHOD_DEADLINES"
        );
    }
}

#[test]
#[ignore = "opt-in timing harness; builds a ~400 MiB synthetic library"]
fn res6_slow_method_timings() {
    let scale = env_usize("AUDIOROUTER_TIMING_SCALE", 1);
    let runs = env_usize("AUDIOROUTER_TIMING_RUNS", 3);
    let root = unique_root("res6");
    isolate_temp(&root.join("temp"));
    let mut rows = Rows::new(
        "run,scenario,method,detail,millis,deadlineClass,deadlineMs,fractionOfDeadline,bytes",
    );
    let row = |rows: &mut Rows,
               run: usize,
               scenario: &str,
               method: &str,
               detail: &str,
               time: Duration,
               bytes: u64| {
        let (class, deadline) = deadline_class(method);
        let ms = millis(time);
        rows.push(format!(
            "{run},{scenario},{method},{detail},{ms:.3},{class},{deadline:.0},{:.4},{bytes}",
            ms / deadline
        ));
    };
    for run in 0..runs {
        let run_root = root.join(format!("run-{run}"));
        let build_start = Instant::now();
        let library = build_library(&run_root, scale);
        row(
            &mut rows,
            run,
            "fixture",
            "build",
            "library",
            build_start.elapsed(),
            0,
        );
        let mut order = ["export-compressible", "export-noise"];
        if run % 2 == 1 {
            order.reverse();
        }
        for session_id in order {
            let scenario = session_id.trim_start_matches("export-");
            let mut source = plane_for(&library.database(), "timing-source");
            let bundle = run_root.join(format!("{session_id}.audiorouter"));
            let (approve, _) = timed(
                &mut source,
                "sessions.approveExport",
                json!({ "path": bundle, "replace": false }),
            );
            row(
                &mut rows,
                run,
                scenario,
                "sessions.approveExport",
                "",
                approve,
                0,
            );
            let (export, result) = timed(
                &mut source,
                "sessions.exportFile",
                json!({ "sessionId": session_id, "path": bundle }),
            );
            let size = result
                .as_ref()
                .and_then(|value| value["bytes"].as_u64())
                .unwrap_or(0);
            row(
                &mut rows,
                run,
                scenario,
                "sessions.exportFile",
                "",
                export,
                size,
            );
            assert!(size > 0, "export of {session_id} failed");

            let target_db = run_root.join(format!("target-{session_id}.sqlite"));
            let mut target = plane_for(&target_db, "timing-target");
            let (import, result) = timed(
                &mut target,
                "sessions.importFile",
                json!({ "path": bundle }),
            );
            let result = result.expect("import failed");
            row(
                &mut rows,
                run,
                scenario,
                "sessions.importFile",
                "",
                import,
                size,
            );
            assert!(result["mediaRestored"].as_u64().unwrap_or(0) > 0);

            // The plan/commit pair carries the session document only.
            let (get, session) = timed(
                &mut source,
                "sessions.get",
                json!({ "sessionId": session_id }),
            );
            row(&mut rows, run, scenario, "sessions.get", "", get, 0);
            let mut copy = session.expect("session");
            copy["id"] = json!(format!("{session_id}-plan"));
            let mut fresh = plane_for(
                &run_root.join(format!("plan-{session_id}.sqlite")),
                "timing-plan",
            );
            let (plan, planned) = timed(
                &mut fresh,
                "sessions.importPlan",
                json!({ "session": copy }),
            );
            row(&mut rows, run, scenario, "sessions.importPlan", "", plan, 0);
            let planned = planned.expect("plan");
            let (commit, _) = timed(
                &mut fresh,
                "sessions.importCommit",
                json!({ "planId": planned["planId"], "idempotencyKey": format!("commit-{run}-{session_id}") }),
            );
            row(
                &mut rows,
                run,
                scenario,
                "sessions.importCommit",
                "",
                commit,
                0,
            );
            let (list, _) = timed(&mut source, "sessions.list", json!({}));
            row(
                &mut rows,
                run,
                scenario,
                "sessions.list",
                "100+ sessions",
                list,
                0,
            );
            drop((source, target, fresh));
        }

        // Uploads `finishUpload` accepts, chunk by chunk. The decoder takes
        // one or two channels, at most 120 s, and at most 16 Mi decoded
        // samples (64 MiB of f32). So the common heavy file is 119 s of
        // 44.1 kHz stereo 16-bit (20 MiB, resampled to the graph rate), and
        // the largest file the 64 MiB upload limit allows is high-rate
        // float32 stereo (83 s at 96 kHz is 61 MiB). 119 s of 96 kHz stereo
        // 16-bit was refused for exceeding the sample budget.
        let mut source = plane_for(&library.database(), "timing-upload");
        let seconds = (119 / scale).max(2);
        let float_frames = 8_000_000 / scale;
        let upload_cases = [
            (
                "upload-16bit-44k-119s",
                wav_with_format(
                    44 + seconds * 44_100 * 4,
                    false,
                    0xfeed + run as u64,
                    44_100,
                    2,
                ),
            ),
            (
                "upload-float32-96k-83s",
                wav_float32_stereo(float_frames, 96_000, 0xbeef + run as u64),
            ),
        ];
        for (label, upload_bytes) in upload_cases {
            let (begin, started) = timed(
                &mut source,
                "audioMedia.beginUpload",
                json!({ "fileName": "upload.wav", "sizeBytes": upload_bytes.len() }),
            );
            row(
                &mut rows,
                run,
                label,
                "audioMedia.beginUpload",
                "",
                begin,
                upload_bytes.len() as u64,
            );
            let started = started.expect("begin upload");
            let chunk_size = started["chunkBytes"].as_u64().unwrap() as usize;
            let mut slowest_chunk = Duration::ZERO;
            let mut chunk_total = Duration::ZERO;
            let mut chunks = 0usize;
            for (index, chunk) in upload_bytes.chunks(chunk_size).enumerate() {
                let encoded = base64(chunk);
                let (time, result) = timed(
                    &mut source,
                    "audioMedia.uploadChunk",
                    json!({ "uploadId": started["uploadId"], "chunkIndex": index, "dataBase64": encoded }),
                );
                assert!(result.is_some(), "chunk {index} refused");
                slowest_chunk = slowest_chunk.max(time);
                chunk_total += time;
                chunks += 1;
            }
            row(
                &mut rows,
                run,
                label,
                "audioMedia.uploadChunk",
                &format!("slowest of {chunks}"),
                slowest_chunk,
                chunk_size as u64,
            );
            row(
                &mut rows,
                run,
                label,
                "audioMedia.uploadChunk",
                &format!("sum of {chunks}"),
                chunk_total,
                upload_bytes.len() as u64,
            );
            let (finish, finished) = timed(
                &mut source,
                "audioMedia.finishUpload",
                json!({ "uploadId": started["uploadId"] }),
            );
            row(
                &mut rows,
                run,
                label,
                "audioMedia.finishUpload",
                "decode and store",
                finish,
                upload_bytes.len() as u64,
            );
            let finished = finished.expect("finish upload");
            let (delete, _) = timed(
                &mut source,
                "audioMedia.delete",
                json!({ "mediaId": finished["mediaId"] }),
            );
            row(&mut rows, run, label, "audioMedia.delete", "", delete, 0);
        }

        // Listing the recordings library: one page of 500, then every page.
        let (page, _) = timed(&mut source, "recordings.list", json!({ "limit": 500 }));
        row(
            &mut rows,
            run,
            "recordings",
            "recordings.list",
            "one page of 500",
            page,
            0,
        );
        let mut cursor = Value::Null;
        let mut all = Duration::ZERO;
        let mut pages = 0usize;
        loop {
            let (time, result) = timed(
                &mut source,
                "recordings.list",
                json!({ "limit": 500, "cursor": cursor }),
            );
            all += time;
            pages += 1;
            let result = result.expect("page");
            cursor = result["nextCursor"].clone();
            if cursor.is_null() {
                break;
            }
        }
        row(
            &mut rows,
            run,
            "recordings",
            "recordings.list",
            &format!("{} rows in {pages} pages", library.recordings),
            all,
            0,
        );

        // The idle service pass over this library: what each 1 ms pass costs
        // the control thread before any route runs.
        let mut passes = Vec::new();
        for _ in 0..2000 {
            let start = Instant::now();
            source.service_running_native_audio(start);
            passes.push(start.elapsed());
        }
        passes.sort();
        row(
            &mut rows,
            run,
            "service-pass",
            "service_running_native_audio",
            "median of 2000",
            passes[passes.len() / 2],
            0,
        );
        row(
            &mut rows,
            run,
            "service-pass",
            "service_running_native_audio",
            "max of 2000",
            *passes.last().unwrap(),
            0,
        );
        drop(source);
        // The harness owns this directory; remove it between runs to bound disk use.
        let _ = std::fs::remove_dir_all(&run_root);
    }
    rows.save();
    let _ = std::fs::remove_dir_all(&root);
}

#[derive(Default)]
struct Cadence {
    gaps: Vec<Duration>,
}

impl Cadence {
    fn summary(&mut self) -> (usize, f64, f64, f64, usize) {
        self.gaps.sort();
        let count = self.gaps.len();
        if count == 0 {
            return (0, 0.0, 0.0, 0.0, 0);
        }
        let late = self
            .gaps
            .iter()
            .filter(|gap| **gap > AUDIO_SERVICE_LATE_GAP)
            .count();
        (
            count,
            millis(self.gaps[count / 2]),
            millis(self.gaps[(count * 99 / 100).min(count - 1)]),
            millis(*self.gaps.last().unwrap()),
            late,
        )
    }
}

/// Run `work` under the production `run_while_servicing`, with a service
/// closure that calls the real service pass and records the time between
/// consecutive calls (what the audio service sees as its gap).
fn observed<T: Send>(
    plane: &mut ControlPlane,
    work: impl FnOnce() -> T + Send,
) -> (T, Duration, Cadence) {
    let mut cadence = Cadence::default();
    let mut last = None;
    let start = Instant::now();
    let value = audio_service::run_while_servicing(work, || {
        let now = Instant::now();
        if let Some(previous) = last {
            cadence.gaps.push(now.saturating_duration_since(previous));
        }
        plane.service_running_native_audio(now);
        last = Some(now);
    });
    (value, start.elapsed(), cadence)
}

#[test]
#[ignore = "opt-in timing harness; builds a ~400 MiB synthetic library"]
fn service_cadence_while_bundle_work_runs() {
    let scale = env_usize("AUDIOROUTER_TIMING_SCALE", 1);
    let runs = env_usize("AUDIOROUTER_TIMING_RUNS", 3);
    let root = unique_root("cadence");
    isolate_temp(&root.join("temp"));
    let mut rows = Rows::new(
        "run,phase,mode,work_ms,passes,median_gap_ms,p99_gap_ms,max_gap_ms,gaps_over_8ms,inline_ms",
    );
    let emit = |rows: &mut Rows,
                run: usize,
                phase: &str,
                mode: &str,
                work: Duration,
                mut cadence: Cadence,
                inline: f64| {
        let (passes, median, p99, max, late) = cadence.summary();
        rows.push(format!(
            "{run},{phase},{mode},{:.3},{passes},{median:.3},{p99:.3},{max:.3},{late},{inline:.3}",
            millis(work)
        ));
    };
    for run in 0..runs {
        let run_root = root.join(format!("run-{run}"));
        let library = build_library(&run_root, scale);
        let mut order = ["export-compressible", "export-noise"];
        if run % 2 == 1 {
            order.reverse();
        }
        for session_id in order {
            let mut plane = plane_for(&library.database(), "timing-cadence");
            let bundle = run_root.join(format!("{session_id}.audiorouter"));
            let id = EntityId::new(session_id);

            // Export: the database part on its own connection, then the
            // ZIP write on the helper, exactly as `dispatch_session_export_file`.
            let target = plane.storage.as_ref().unwrap().worker_target();
            let (prepared, prepare_time, cadence) = observed(&mut plane, || {
                let storage = target.open().unwrap();
                storage.prepare_bundle_export(&id).unwrap()
            });
            emit(
                &mut rows,
                run,
                &format!("{session_id}:prepare"),
                "loaded",
                prepare_time,
                cadence,
                0.0,
            );
            let (written, write_time, cadence) = observed(&mut plane, || prepared.write(&bundle));
            written.unwrap();
            emit(
                &mut rows,
                run,
                &format!("{session_id}:write"),
                "loaded",
                write_time,
                cadence,
                0.0,
            );
            // Same-length idle baseline: the closure's own scheduling cost.
            let ((), idle_time, cadence) = observed(&mut plane, || std::thread::sleep(write_time));
            emit(
                &mut rows,
                run,
                &format!("{session_id}:write"),
                "idle-baseline",
                idle_time,
                cadence,
                0.0,
            );

            // Import: stage (unzip, hash), restore into a second database.
            let staging = run_root.join(format!("staging-{session_id}"));
            let (staged, stage_time, cadence) = observed(&mut plane, || {
                Storage::stage_session_bundle(&bundle, &staging)
            });
            let staged = staged.unwrap();
            emit(
                &mut rows,
                run,
                &format!("{session_id}:stage"),
                "loaded",
                stage_time,
                cadence,
                0.0,
            );
            let mut destination = plane_for(
                &run_root.join(format!("dest-{session_id}.sqlite")),
                "timing-dest",
            );
            let dest_target = destination.storage.as_ref().unwrap().worker_target();
            let (restored, restore_time, cadence) = observed(&mut destination, || {
                let storage = dest_target.open().unwrap();
                storage.restore_session_bundle(staged).unwrap()
            });
            emit(
                &mut rows,
                run,
                &format!("{session_id}:restore"),
                "loaded",
                restore_time,
                cadence,
                0.0,
            );
            let (session, _report) = restored;

            // The part of an import that runs on the control thread with no
            // service passes: the occupancy check and the session insert.
            let inline_start = Instant::now();
            let _ = destination.ensure_session_loaded(&session.id);
            let taken = destination.store.session(&session.id).is_some();
            let mut fresh = session;
            fresh.revision = 0;
            destination.create_session(fresh).unwrap();
            let inline = millis(inline_start.elapsed());
            assert!(!taken);
            emit(
                &mut rows,
                run,
                &format!("{session_id}:inline-insert"),
                "unserviced",
                Duration::ZERO,
                Cadence::default(),
                inline,
            );

            // Whole-request view for the same export, with no helper-side load.
            let mut whole = plane_for(&library.database(), "timing-whole");
            let again = run_root.join(format!("{session_id}-again.audiorouter"));
            let (_, _) = timed(
                &mut whole,
                "sessions.approveExport",
                json!({ "path": again, "replace": false }),
            );
            let (request, _) = timed(
                &mut whole,
                "sessions.exportFile",
                json!({ "sessionId": session_id, "path": again }),
            );
            emit(
                &mut rows,
                run,
                &format!("{session_id}:exportFile-request"),
                "whole",
                request,
                Cadence::default(),
                millis(request),
            );
            drop((plane, destination, whole));
        }

        // Repeated imports of one small session: the "-imported-N" search.
        let small_root = run_root.join("small");
        std::fs::create_dir_all(&small_root).unwrap();
        let mut origin = plane_for(&small_root.join("origin.sqlite"), "timing-small");
        origin.create_session(filler_session(0)).unwrap();
        let small_bundle = small_root.join("small.audiorouter");
        timed(
            &mut origin,
            "sessions.approveExport",
            json!({ "path": small_bundle, "replace": false }),
        );
        timed(
            &mut origin,
            "sessions.exportFile",
            json!({ "sessionId": "filler-000", "path": small_bundle }),
        );
        let mut target = plane_for(&small_root.join("target.sqlite"), "timing-small-target");
        for copy in 0..100usize {
            let (time, result) = timed(
                &mut target,
                "sessions.importFile",
                json!({ "path": small_bundle }),
            );
            assert!(result.is_some(), "import {copy} failed");
            if matches!(copy, 0 | 1 | 9 | 24 | 49 | 74 | 99) {
                rows.push(format!(
                    "{run},small-import-copy-{},whole,{:.3},0,0,0,0,0,{:.3}",
                    copy + 1,
                    millis(time),
                    millis(time)
                ));
            }
        }
        drop((origin, target));
        let _ = std::fs::remove_dir_all(&run_root);
    }
    rows.save();
    let _ = std::fs::remove_dir_all(&root);
}

/// Do an export and an import of a 20 MiB audio asset both succeed? The
/// importer limits non-media entries to 16 MiB but media entries to 64 MiB, so
/// they should. Prints both outcomes; asserts nothing, so the result is
/// evidence, not a gate.
#[test]
#[ignore = "opt-in probe; stores a 20 MiB synthetic asset"]
fn export_of_an_asset_over_the_import_limit() {
    let root = unique_root("oversize");
    isolate_temp(&root.join("temp"));
    let database = root.join("source.sqlite");
    let storage = Storage::open(&database).unwrap();
    let blob = wav(20 * 1024 * 1024, false, 7);
    storage
        .store_audio_media("big-asset", "big.wav", "wav", &blob, None)
        .unwrap();
    storage
        .save_session(&media_session("oversize", &["big-asset".to_owned()]))
        .unwrap();
    drop(storage);
    let mut source = plane_for(&database, "timing-oversize-source");
    let bundle = root.join("oversize.audiorouter");
    timed(
        &mut source,
        "sessions.approveExport",
        json!({ "path": bundle, "replace": false }),
    );
    let (_, exported) = timed(
        &mut source,
        "sessions.exportFile",
        json!({ "sessionId": "oversize", "path": bundle }),
    );
    eprintln!("export of a 20 MiB asset: ok={}", exported.is_some());
    let mut target = plane_for(&root.join("target.sqlite"), "timing-oversize-target");
    let (_, imported) = timed(
        &mut target,
        "sessions.importFile",
        json!({ "path": bundle }),
    );
    eprintln!("import of that bundle: ok={}", imported.is_some());
    drop((source, target));
    let _ = std::fs::remove_dir_all(&root);
}

/// What Play does on the control thread before it touches a device: load and
/// decode every Audio File (`session_audio_media`, under
/// `while_servicing_audio`), then compile the graph with that media (inline).
/// Starting a route needs real endpoints (`nativePaths.prepare`), so this times
/// those two steps directly, plus a `session.start` on the one-shot "cli" plane
/// whose simulated lifecycle moves no audio. Eight 100 s stereo 16-bit
/// files, at the graph rate (48 kHz) and at 44.1 kHz (resampled).
#[test]
#[ignore = "opt-in timing harness; stores ~150 MiB of synthetic audio per run"]
fn play_path_media_load_and_compile() {
    let scale = env_usize("AUDIOROUTER_TIMING_SCALE", 1);
    let runs = env_usize("AUDIOROUTER_TIMING_RUNS", 3);
    let root = unique_root("play");
    isolate_temp(&root.join("temp"));
    let mut rows = Rows::new("run,scenario,step,millis,decodedMiB,note");
    let seconds = (100 / scale).max(2);
    // A Mixer takes at most `MAX_MIXER_INPUTS` (8) sources.
    let count = (8 / scale).max(2);
    for run in 0..runs {
        let mut cases = [("48k", 48_000u32), ("44.1k", 44_100u32)];
        if run % 2 == 1 {
            cases.reverse();
        }
        for (label, rate) in cases {
            let run_root = root.join(format!("run-{run}-{label}"));
            std::fs::create_dir_all(&run_root).unwrap();
            let database = run_root.join("play.sqlite");
            let storage = Storage::open(&database).unwrap();
            let mut ids = Vec::new();
            for index in 0..count {
                let id = format!("play-{index:02}");
                let frames = seconds * rate as usize;
                storage
                    .store_audio_media(
                        &id,
                        "fixture.wav",
                        "wav",
                        &wav_with_format(44 + frames * 4, false, index as u64 + 3, rate, 2),
                        None,
                    )
                    .unwrap();
                ids.push(id);
            }
            let session = play_session(&ids);
            storage.save_session(&session).unwrap();
            drop(storage);
            let mut plane = plane_for(&database, "cli");
            plane.audio_media_cache.clear();
            let start = Instant::now();
            let media = plane.session_audio_media(&session, 48_000).unwrap();
            let cold = start.elapsed();
            let decoded_mib = media
                .values()
                .map(|audio| audio.samples.len() * 4)
                .sum::<usize>() as f64
                / 1_048_576.0;
            let start = Instant::now();
            plane.session_audio_media(&session, 48_000).unwrap();
            let warm = start.elapsed();
            let start = Instant::now();
            // The multi-input compile `prepare_native_path_worker` runs
            // after `session_audio_media`.
            audiorouter_engine::compile_native_paths_with_plugins_and_audio(
                &session,
                audiorouter_engine::RuntimeGeneration::new(1),
                &HashMap::new(),
                &media,
            )
            .unwrap();
            let compile = start.elapsed();
            let (started, result) = timed(
                &mut plane,
                "session.start",
                json!({ "sessionId": "play", "idempotencyKey": format!("start-{run}-{label}") }),
            );
            assert!(result.is_some(), "session.start refused");
            for (step, time, note) in [
                ("load and decode (cold cache)", cold, "serviced"),
                ("load and decode (warm cache)", warm, "serviced"),
                (
                    "graph compile with media",
                    compile,
                    "control thread; no service passes",
                ),
                (
                    "session.start (simulated runtime)",
                    started,
                    "no media and no device",
                ),
            ] {
                rows.push(format!(
                    "{run},{label},{step},{:.3},{decoded_mib:.1},{note}",
                    millis(time)
                ));
            }
            drop((plane, media));
        }
    }
    rows.save();
    let _ = std::fs::remove_dir_all(&root);
}

/// Audio Files -> Mixer -> Output with real edges, so the graph compiles
/// (`media_session` has no edges, which is enough to export, not to play).
fn play_session(media: &[String]) -> Session {
    let stereo = |name: &str, direction| Port {
        name: name.into(),
        direction,
        channels: 2,
    };
    let mut session = media_session("play", media);
    session
        .nodes
        .retain(|node| node.kind == NodeKind::AudioFile);
    let kind_node = |id: &str, kind, ports| Node {
        id: EntityId::new(id),
        kind,
        type_version: 1,
        name: id.into(),
        enabled: true,
        bypass: false,
        parameters: Default::default(),
        ports,
    };
    session.nodes.push(kind_node(
        "mixer",
        NodeKind::Mixer,
        vec![
            stereo("in", PortDirection::Input),
            stereo("out", PortDirection::Output),
        ],
    ));
    session.nodes.push(kind_node(
        "out",
        NodeKind::PhysicalOutput,
        vec![stereo("in", PortDirection::Input)],
    ));
    let edge = |source: String, destination: &str| Edge {
        id: EntityId::new(format!("{source}->{destination}")),
        source_node: EntityId::new(source),
        source_port: "out".into(),
        destination_node: EntityId::new(destination),
        destination_port: "in".into(),
        matrix: vec![1.0, 0.0, 0.0, 1.0],
        enabled: true,
    };
    session.edges = media
        .iter()
        .enumerate()
        .map(|(index, _)| edge(format!("audio-{index}"), "mixer"))
        .collect();
    session.edges.push(edge("mixer".into(), "out"));
    session
}
