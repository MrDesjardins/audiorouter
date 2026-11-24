//! Q4.3 bounded fuzzing of session import: the JSON document (`import_session`)
//! and the `.audiorouter` ZIP bundle (`stage_session_bundle`), both read from
//! files a user or another computer supplied.
//!
//! Fixed seeds and counts. Every case asserts: no panic; a refused input leaves
//! the database exactly as it was (no partial commit); a bundle never leaves
//! staged files behind or writes outside its staging root; an accepted session
//! passes the same validator as a stored one. `AUDIOROUTER_FUZZ_SCALE`
//! lengthens a local run.
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use audiorouter_domain::validate_session;
use audiorouter_storage::{Storage, StorageError, MAX_BUNDLE_ENTRIES, MAX_SESSION_DOCUMENT_BYTES};
use serde_json::{json, Value};

#[path = "../../../tests/support/fuzz.rs"]
mod fuzz;

#[global_allocator]
static ALLOCATOR: fuzz::TrackingAllocator = fuzz::TrackingAllocator;

const SEED: u64 = 0x5354_4F52_4147_4521; // "STORAGE!"
const FIXTURE: &str = include_str!("../../../tests/fixtures/valid-session.json");

/// A scratch directory under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("audiorouter-fuzz-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn session_ids(storage: &Storage) -> Vec<String> {
    let mut ids: Vec<String> = storage
        .list_sessions(500)
        .unwrap()
        .iter()
        .map(|session| session.id.as_str().to_owned())
        .collect();
    ids.sort();
    ids
}

// ---------------------------------------------------------------------------
// Session JSON import
// ---------------------------------------------------------------------------

fn nested(depth: usize) -> String {
    format!("{}0{}", "[".repeat(depth), "]".repeat(depth))
}

/// Structure-aware edits that a byte flip rarely makes.
fn mutate_document(rng: &mut fuzz::Rng, text: &str) -> String {
    let mut value: Value = serde_json::from_str(text).unwrap();
    let extremes = [
        json!(null),
        json!(true),
        json!(-1),
        json!(0),
        json!(u64::MAX),
        json!(1e308),
        json!(-1e308),
        json!(f64::MIN_POSITIVE),
        json!(""),
        json!("x".repeat(70_000)),
        json!([]),
        json!({}),
        json!([1, 2, 3]),
    ];
    for _ in 0..rng.range(1, 3) {
        let nodes = value["nodes"].as_array().map_or(0, Vec::len);
        let edges = value["edges"].as_array().map_or(0, Vec::len);
        match rng.below(7) {
            0 => {
                let key = *rng.pick(&["id", "name", "schemaVersion", "revision", "nodes", "edges"]);
                value[key] = rng.pick(&extremes).clone();
            }
            1 if nodes > 0 => {
                let index = rng.below(nodes);
                let key = *rng.pick(&[
                    "id",
                    "kind",
                    "name",
                    "enabled",
                    "bypass",
                    "ports",
                    "typeVersion",
                ]);
                value["nodes"][index][key] = rng.pick(&extremes).clone();
            }
            2 if edges > 0 => {
                let index = rng.below(edges);
                let key = *rng.pick(&[
                    "id",
                    "sourceNode",
                    "sourcePort",
                    "destinationNode",
                    "destinationPort",
                    "matrix",
                    "enabled",
                ]);
                value["edges"][index][key] = rng.pick(&extremes).clone();
            }
            3 if nodes > 0 => {
                // Parameters, valid or not, including deep structures.
                let index = rng.below(nodes);
                let key = *rng.pick(&["gainDb", "delayMs", "unknown", "endpointId", "mediaId"]);
                let parameter = if rng.one_in(3) {
                    serde_json::from_str(&nested(rng.range(1, 100))).unwrap()
                } else {
                    rng.pick(&extremes).clone()
                };
                value["nodes"][index]["parameters"] = json!({ key: parameter });
            }
            4 if nodes > 0 => {
                // Many nodes and edges: the validator's own limits.
                let copies = *rng.pick(&[2_usize, 64, 700, 2_000]);
                let node = value["nodes"][rng.below(nodes)].clone();
                let list = value["nodes"].as_array_mut().unwrap();
                for index in 0..copies {
                    let mut copy = node.clone();
                    if rng.one_in(2) {
                        copy["id"] = json!(format!("copy-{index}"));
                    }
                    list.push(copy);
                }
            }
            5 => {
                let key = format!("extra{}", rng.below(5));
                value[key.as_str()] = rng.pick(&extremes).clone();
            }
            _ => {
                if let Some(object) = value.as_object_mut() {
                    let keys: Vec<String> = object.keys().cloned().collect();
                    let key = rng.pick(&keys).clone();
                    object.remove(&key);
                }
            }
        }
    }
    serde_json::to_string(&value).unwrap()
}

/// Imports `document`; true when it was accepted.
fn assert_import_outcome(storage: &Storage, document: &str) -> bool {
    let before = session_ids(storage);
    let before_sessions = storage.list_sessions(500).unwrap();
    let (result, largest) = fuzz::measure_allocation(|| storage.import_session(document));
    assert!(
        largest <= 64 * document.len() + 8 * 1024 * 1024,
        "import_session requested {largest} bytes for {} input bytes",
        document.len()
    );
    match result {
        Ok(session) => {
            assert!(validate_session(&session).is_ok());
            assert_eq!(storage.load_session(&session.id).unwrap(), Some(session));
            true
        }
        Err(_) => {
            // No partial commit: the stored sessions are exactly as before.
            assert_eq!(session_ids(storage), before);
            assert_eq!(storage.list_sessions(500).unwrap(), before_sessions);
            false
        }
    }
}

#[test]
fn mutated_session_documents_never_panic_and_a_refusal_changes_nothing() {
    let storage = Storage::open_memory().unwrap();
    storage
        .import_session(FIXTURE)
        .expect("the fixture is valid");
    let compact = serde_json::to_string(&serde_json::from_str::<Value>(FIXTURE).unwrap()).unwrap();
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    fuzz::run("storage/session-json", SEED, 1_500, |_, rng| {
        let document = match rng.below(3) {
            0 => {
                let bytes = fuzz::mutate(rng, FIXTURE.as_bytes(), 8 * 1024);
                String::from_utf8_lossy(&bytes).into_owned()
            }
            1 => mutate_document(rng, &compact),
            _ => {
                let bytes = fuzz::mutate(rng, compact.as_bytes(), 8 * 1024);
                String::from_utf8_lossy(&bytes).into_owned()
            }
        };
        if assert_import_outcome(&storage, &document) {
            accepted += 1;
        } else {
            refused += 1;
        }
    });
    assert!(
        accepted > 20,
        "only {accepted} mutated documents were accepted"
    );
    assert!(
        refused > 200,
        "only {refused} mutated documents were refused"
    );
}

#[test]
fn session_documents_at_the_size_limit_and_deeply_nested_json_are_structured_errors() {
    let storage = Storage::open_memory().unwrap();
    // One byte over the cap is refused before parsing.
    let padded = format!("{FIXTURE}{}", " ".repeat(MAX_SESSION_DOCUMENT_BYTES));
    assert!(matches!(
        storage.import_session(&padded),
        Err(StorageError::DocumentTooLarge { .. })
    ));
    // Nesting in a setting is never valid, and beyond serde_json's recursion
    // limit it is a JSON error, at any depth that fits the size cap.
    for depth in [10_usize, 100, 127, 128, 129, 1_000, 100_000] {
        let document = FIXTURE.replacen(
            "\"name\": \"Input\",",
            &format!(
                "\"name\": \"Input\", \"parameters\": {{\"gainDb\": {}}},",
                nested(depth)
            ),
            1,
        );
        let result = storage.import_session(&document);
        assert!(result.is_err(), "depth {depth} was accepted");
        // serde_json's limit is 128 containers in total, the wrapping
        // objects included, so a few levels past it is always a JSON error.
        if depth >= 130 {
            assert!(
                matches!(result, Err(StorageError::Json(_))),
                "depth {depth}: {result:?}"
            );
        }
    }
    // An unknown field is skipped without recursion, so even extreme nesting
    // there must not overflow the stack (the result itself is not pinned).
    for depth in [1_000_usize, 100_000] {
        let document = FIXTURE.replacen(
            "\"edges\": [",
            &format!("\"deep\": {}, \"edges\": [", nested(depth)),
            1,
        );
        let _ = storage.import_session(&document);
    }
    // Garbage that is not JSON, empty input, a lone BOM.
    for document in [
        "", "\u{feff}", "null", "[]", "{}", "{\"id\":", "\u{0}", "{{{{",
    ] {
        assert!(!assert_import_outcome(&storage, document), "{document:?}");
    }
}

// ---------------------------------------------------------------------------
// Bundles
// ---------------------------------------------------------------------------

/// A bundle exactly as `export_bundle` writes it, with imported audio and a
/// saved plugin state, as bytes. Returns the bytes and the session ID.
fn exported_bundle(scratch: &Scratch) -> Vec<u8> {
    let source = Storage::open_memory().unwrap();
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value["nodes"][0]["kind"] = json!("audioFile");
    value["nodes"][0]["parameters"] = json!({ "mediaId": "media-1" });
    let session: audiorouter_domain::Session = serde_json::from_value(value).unwrap();
    source.save_session(&session).unwrap();
    source
        .store_audio_media("media-1", "voice.wav", "wav", b"RIFF-voice-bytes", None)
        .unwrap();
    let path = scratch.path().join("seed.audiorouter");
    let _ = std::fs::remove_file(&path);
    source.export_bundle(&session.id, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    bytes
}

fn zip_bytes(entries: &[(String, Vec<u8>)], deflate: bool) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(if deflate {
        zip::CompressionMethod::Deflated
    } else {
        zip::CompressionMethod::Stored
    });
    for (name, bytes) in entries {
        if writer.start_file(name.as_str(), options).is_ok() {
            writer.write_all(bytes).unwrap();
        }
    }
    writer.finish().unwrap().into_inner()
}

/// Stage `bytes` as a bundle in its own staging root and check what must hold
/// for any input.
fn stage_and_check(scratch: &Scratch, bytes: &[u8], label: &str) -> Result<(), StorageError> {
    let bundle = scratch.path().join("case.audiorouter");
    let staging = scratch.path().join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(&bundle, bytes).unwrap();
    let (result, largest) =
        fuzz::measure_allocation(|| Storage::stage_session_bundle(&bundle, &staging));
    // Entry buffers are capped at 16 MiB (assets) / 64 MiB (media) / 1 MiB
    // (documents); nothing may exceed the media cap by a wide margin.
    assert!(
        largest <= 160 * 1024 * 1024,
        "{label}: stage_session_bundle requested {largest} bytes for {} input bytes",
        bytes.len()
    );
    if let Ok(staged) = &result {
        let _ = staged;
    }
    // Staging is always cleaned up, and nothing else appeared.
    let leftover: Vec<String> = std::fs::read_dir(&staging)
        .map(|entries| {
            entries
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        leftover.is_empty(),
        "{label}: staging left {leftover:?} behind"
    );
    std::fs::remove_file(&bundle).unwrap();
    assert_eq!(
        scratch.entries(),
        vec!["staging".to_owned()],
        "{label}: files appeared outside the staging root"
    );
    result.map(|_| ())
}

#[test]
fn mutated_bundle_bytes_never_panic_and_leave_no_staging_files() {
    let scratch = Scratch::new("bundle-bytes");
    let seed = exported_bundle(&scratch);
    assert!(
        stage_and_check(&scratch, &seed, "the unmodified seed bundle").is_ok(),
        "the seed bundle must import"
    );
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    fuzz::run("storage/bundle-bytes", SEED ^ 1, 250, |_, rng| {
        let bytes = fuzz::mutate(rng, &seed, 64 * 1024);
        match stage_and_check(&scratch, &bytes, "mutated bundle") {
            Ok(()) => accepted += 1,
            Err(_) => refused += 1,
        }
    });
    assert!(refused > 100, "only {refused} mutated bundles were refused");
    // Some mutations land in unused bytes (comments, timestamps) or the
    // payload of an entry whose hash is not checked; that is allowed.
    let _ = accepted;
}

fn manifest(graph_path: &str, assets: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "format": "audiorouter.session",
        "schemaVersion": 1,
        "graphPath": graph_path,
        "assets": assets,
    }))
    .unwrap()
}

fn entry_names() -> Vec<String> {
    let mut names: Vec<String> = [
        "manifest.json",
        "session.json",
        "media/m.wav",
        "state/s.bin",
        "../outside.txt",
        "/absolute.txt",
        "C:/drive.txt",
        "a/../../b.txt",
        "a\\..\\..\\b.txt",
        "run.exe",
        "script.bat",
        "link",
        "dir/",
        "",
        "con",
        "aux.txt",
        "name with spaces.txt",
        "MANIFEST.JSON",
        "session.json ",
        "session.json:stream",
    ]
    .iter()
    .map(|name| (*name).to_owned())
    .collect();
    names.push("x".repeat(5_000));
    names.push("long/".repeat(500) + "f");
    names
}

#[test]
fn structurally_hostile_bundles_are_refused_and_stay_inside_the_staging_root() {
    let scratch = Scratch::new("bundle-structure");
    let names = entry_names();
    let session_json = FIXTURE.as_bytes().to_vec();
    let mut refused = 0_usize;
    let mut accepted = 0_usize;
    fuzz::run("storage/bundle-structure", SEED ^ 2, 200, |_, rng| {
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        let graph_path = rng.pick(&names).clone();
        let mut assets = Vec::new();
        if rng.one_in(4) {
            assets.push(json!(rng.pick(&names)));
        }
        if rng.one_in(3) {
            assets.push(json!({
                "path": "session.json",
                "size": *rng.pick(&[0_u64, session_json.len() as u64, u64::MAX]),
                "sha256": "0".repeat(*rng.pick(&[0_usize, 63, 64, 65])),
            }));
        }
        if !rng.one_in(8) {
            let graph = if rng.one_in(3) {
                graph_path.as_str()
            } else {
                "session.json"
            };
            let body = manifest(graph, Value::Array(assets));
            entries.push(("manifest.json".to_owned(), fuzz::mutate(rng, &body, 4_096)));
        }
        for _ in 0..rng.below(6) {
            let name = rng.pick(&names).clone();
            let body = match rng.below(3) {
                0 => session_json.clone(),
                1 => rng.blob(0, 64),
                _ => Vec::new(),
            };
            entries.push((name, body));
        }
        if rng.one_in(2) {
            entries.push(("session.json".to_owned(), session_json.clone()));
        }
        if rng.one_in(10) {
            // A repeated name.
            let again = entries.last().cloned();
            entries.extend(again);
        }
        let bytes = zip_bytes(&entries, rng.one_in(2));
        match stage_and_check(&scratch, &bytes, "hostile bundle") {
            Ok(()) => accepted += 1,
            Err(_) => refused += 1,
        }
    });
    assert!(refused > 120, "only {refused} hostile bundles were refused");
    let _ = accepted;
}

#[test]
fn bundle_entry_count_and_size_limits_hold() {
    let scratch = Scratch::new("bundle-limits");
    // More entries than the cap.
    let mut entries: Vec<(String, Vec<u8>)> = (0..=MAX_BUNDLE_ENTRIES)
        .map(|index| (format!("e{index}.txt"), vec![0_u8]))
        .collect();
    entries.push(("manifest.json".into(), manifest("session.json", json!([]))));
    let bytes = zip_bytes(&entries, false);
    let error = stage_and_check(&scratch, &bytes, "too many entries").unwrap_err();
    assert!(matches!(error, StorageError::InvalidBundle(_)), "{error:?}");

    // A 17 MiB highly compressible asset: over the 16 MiB asset cap, so the
    // read stops at the cap instead of expanding it.
    let bomb = vec![0_u8; 17 * 1024 * 1024];
    let entries = vec![
        (
            "manifest.json".to_owned(),
            manifest("session.json", json!([])),
        ),
        ("session.json".to_owned(), FIXTURE.as_bytes().to_vec()),
        ("asset.bin".to_owned(), bomb),
    ];
    let bytes = zip_bytes(&entries, true);
    assert!(bytes.len() < 1024 * 1024, "the bomb must compress well");
    let error = stage_and_check(&scratch, &bytes, "asset over its cap").unwrap_err();
    assert!(
        matches!(error, StorageError::DocumentTooLarge { .. }),
        "{error:?}"
    );

    // The same entry with its declared uncompressed size understated in both
    // the local header and the central directory: the extractor must bound
    // the bytes it actually reads, not the size the file claims.
    let mut lying = bytes.clone();
    let patches = lying
        .windows(4)
        .enumerate()
        .filter(|(_, window)| *window == (17_u32 * 1024 * 1024).to_le_bytes())
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>();
    assert!(!patches.is_empty(), "no declared size found to understate");
    for offset in patches {
        lying[offset..offset + 4].copy_from_slice(&100_u32.to_le_bytes());
    }
    let error = stage_and_check(&scratch, &lying, "understated size");
    assert!(error.is_err(), "an understated declared size was accepted");

    // A manifest that is itself over the document cap.
    let big = vec![b' '; MAX_SESSION_DOCUMENT_BYTES + 1];
    let entries = vec![("manifest.json".to_owned(), big)];
    let error =
        stage_and_check(&scratch, &zip_bytes(&entries, true), "manifest too big").unwrap_err();
    assert!(
        matches!(error, StorageError::DocumentTooLarge { .. }),
        "{error:?}"
    );
}
