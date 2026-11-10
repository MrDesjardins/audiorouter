//! Unsaved route edits, kept in a file beside the database so they survive any
//! way out of the app: the tray's Quit, the window's Quit, or a crash. The page
//! owns the meaning of the contents and checks them when it reads them back;
//! this module only bounds the size and never leaves half a file.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

static SAVE_LOCK: Mutex<()> = Mutex::new(());
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// Far above a real route. A larger write is refused rather than truncated.
const MAX_DRAFT_BYTES: usize = 8 * 1024 * 1024;

pub fn path_beside(database_path: &Path) -> PathBuf {
    database_path.with_file_name("unsaved-route.json")
}

/// The saved edits, or nothing when the file is missing, oversized or damaged.
pub fn load(path: &Path) -> Option<serde_json::Value> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_DRAFT_BYTES as u64 {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.is_object().then_some(value)
}

/// Replace the saved edits through a temporary file, or remove them for `None`.
pub fn save(path: &Path, draft: Option<&serde_json::Value>) -> Result<(), String> {
    // Includes clearing: a concurrent remove must not race a file replacement.
    let _guard = SAVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let Some(draft) = draft else {
        return match std::fs::remove_file(path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("unsaved edits could not be removed: {error}"))
            }
            _ => Ok(()),
        };
    };
    if !draft.is_object() {
        return Err("unsaved edits must be a JSON object".into());
    }
    let text = serde_json::to_string(draft).map_err(|error| error.to_string())?;
    if text.len() > MAX_DRAFT_BYTES {
        return Err("unsaved edits are too large to keep".into());
    }
    let temporary = path.with_extension(format!(
        "json.{}.{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("unsaved edits could not be written: {error}"))?;
    let result = file.write_all(text.as_bytes());
    drop(file);
    let result = result.and_then(|()| std::fs::rename(&temporary, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("unsaved edits could not be saved: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "audiorouter-ui-draft-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn concurrent_saves_publish_whole_drafts_without_temporary_file_collisions() {
        let folder = folder("concurrent");
        let path = path_beside(&folder.join("state.sqlite"));
        std::thread::scope(|scope| {
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
            let workers: Vec<_> = (0..8)
                .map(|worker| {
                    let barrier = barrier.clone();
                    let path = &path;
                    scope.spawn(move || {
                        barrier.wait();
                        for index in 0..25 {
                            save(path, Some(&json!({"worker": worker, "index": index}))).unwrap();
                        }
                    })
                })
                .collect();
            for worker in workers {
                worker.join().unwrap();
            }
        });
        assert!(load(&path).is_some());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn edits_round_trip_beside_the_database_and_clear() {
        let folder = folder("round-trip");
        let path = path_beside(&folder.join("state.sqlite"));
        assert_eq!(path.parent(), Some(folder.as_path()));
        assert_eq!(load(&path), None, "nothing saved yet");

        let first = json!({"version": 1, "sessionId": "a", "draft": {"name": "One"}});
        save(&path, Some(&first)).unwrap();
        assert_eq!(load(&path), Some(first));

        let second = json!({"version": 1, "sessionId": "a", "draft": {"name": "Two"}});
        save(&path, Some(&second)).unwrap();
        assert_eq!(load(&path), Some(second), "a later edit replaces the first");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "no temporary file stays behind"
        );

        save(&path, None).unwrap();
        assert_eq!(load(&path), None);
        save(&path, None).expect("clearing twice is not an error");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn damaged_oversized_or_wrong_shaped_files_read_as_nothing() {
        let folder = folder("damaged");
        let path = path_beside(&folder.join("state.sqlite"));
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), None);
        std::fs::write(&path, "[1, 2]").unwrap();
        assert_eq!(load(&path), None);
        std::fs::write(&path, "x".repeat(MAX_DRAFT_BYTES + 1)).unwrap();
        assert_eq!(load(&path), None);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_refused_write_keeps_the_edits_already_saved() {
        let folder = folder("refused");
        let path = path_beside(&folder.join("state.sqlite"));
        let kept = json!({"version": 1, "draft": {"name": "Kept"}});
        save(&path, Some(&kept)).unwrap();

        let huge = json!({"pad": "x".repeat(MAX_DRAFT_BYTES + 1)});
        assert!(save(&path, Some(&huge)).is_err());
        assert!(save(&path, Some(&json!([1]))).is_err());
        assert_eq!(load(&path), Some(kept));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
