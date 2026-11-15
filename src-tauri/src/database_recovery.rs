//! Explicit recovery while the owned backend is stopped. No arbitrary paths
//! cross the shell command boundary; choices are direct validated backup names.
use audiorouter_storage::Storage;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupChoice {
    pub name: String,
    pub bytes: u64,
}

fn candidate(directory: &Path, name: &str) -> Result<PathBuf, String> {
    if name.len() > 128
        || name.contains(['/', '\\', ':'])
        || !name.ends_with(".sqlite")
        || !(name.starts_with("audiorouter-backup-")
            || name.starts_with("audiorouter-pre-migration-"))
    {
        return Err("Choose a listed AudioRouter backup.".into());
    }
    Ok(directory.join(name))
}

pub fn choices(database: &Path) -> Result<Vec<BackupChoice>, String> {
    let directory = Storage::recovery_backup_directory(database);
    if !directory
        .try_exists()
        .map_err(|_| "Cannot inspect backup folder.")?
    {
        return Ok(Vec::new());
    }
    let mut choices = Vec::new();
    for (index, entry) in fs::read_dir(&directory)
        .map_err(|_| "Cannot read backup folder.")?
        .enumerate()
    {
        if index >= audiorouter_storage::MAX_RECOVERY_DIRECTORY_ENTRIES {
            return Err("Backup folder has too many entries.".into());
        }
        let entry = entry.map_err(|_| "Cannot inspect backup.")?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(path) = candidate(&directory, &name) else {
            continue;
        };
        if Storage::validate_recovery_backup(&path).is_err() {
            continue;
        }
        choices.push(BackupChoice {
            name,
            bytes: fs::metadata(path)
                .map_err(|_| "Cannot inspect backup size.")?
                .len(),
        });
    }
    choices.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(choices)
}

pub fn restore(database: &Path, name: &str) -> Result<(), String> {
    let backup = candidate(&Storage::recovery_backup_directory(database), name)?;
    Storage::validate_recovery_backup(&backup)
        .map_err(|_| "This backup failed validation. Your saved data has not been replaced.")?;
    // Reserve a new directory, never reusing a prior damaged database archive.
    let parent = database.parent().ok_or("Database folder is unavailable.")?;
    let archive = parent.join(format!(
        "damaged-{}-{}",
        std::process::id(),
        audiorouter_protocol::diagnostics::unix_time_ms()
    ));
    fs::create_dir(&archive).map_err(|_| "Cannot preserve the damaged database.")?;
    let staging = archive.join("restored.sqlite");
    Storage::restore_backup(&backup, &staging)
        .map_err(|_| "Cannot create a validated restore copy.")?;
    {
        let restored = Storage::open_checked(&staging)
            .map_err(|_| "Cannot open this backup with this version of AudioRouter.")?;
        restored
            .prepare_desktop_restore()
            .map_err(|_| "Cannot reset restored permissions safely.")?;
    }
    // Copy the sealed database through SQLite, so publication never depends on
    // a WAL sidecar left by migration or permission reset.
    let ready = archive.join("ready.sqlite");
    Storage::backup_database(&staging, &ready).map_err(|_| "Cannot finalize the restore copy.")?;
    let mut moved = Vec::new();
    for suffix in ["", "-wal", "-shm"] {
        let mut original = database.as_os_str().to_os_string();
        original.push(suffix);
        let original = PathBuf::from(original);
        if !original
            .try_exists()
            .map_err(|_| "Cannot inspect damaged database files.")?
        {
            continue;
        }
        let destination = archive.join(format!("original.sqlite{suffix}"));
        if fs::rename(&original, &destination).is_err() {
            for (from, to) in moved.iter().rev() {
                let _ = fs::rename(to, from);
            }
            return Err(
                "Cannot preserve all damaged database files. Recovery was not published.".into(),
            );
        }
        moved.push((original, destination));
    }
    if fs::rename(&ready, database).is_err() {
        for (from, to) in moved.iter().rev() {
            let _ = fs::rename(to, from);
        }
        return Err(
            "Cannot publish the restored database. Original files remain preserved.".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-restore-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn corrupt_database_is_preserved_and_restored_permissions_fail_closed() {
        let root = directory();
        let database = root.join("state.sqlite");
        let storage = Storage::open(&database).unwrap();
        let mut plane = audiorouter_control::ControlPlane::with_storage("restore-fixture", storage);
        let mut session = crate::default_desktop_session();
        session.name = "Preserved owner route".into();
        let session_id = session.id.clone();
        plane.insert_session(session).unwrap();
        drop(plane);
        let storage = Storage::open(&database).unwrap();
        storage
            .save_client_enrollment("old-client", "operator")
            .unwrap();
        storage.save_privacy_mute(false).unwrap();
        storage.save_device_access_allowed(true).unwrap();
        storage.save_startup_enabled(true).unwrap();
        Storage::ensure_daily_backup(&database, 86_400).unwrap();
        drop(storage);
        fs::write(&database, b"damaged original evidence").unwrap();
        let list = choices(&database).unwrap();
        assert_eq!(list.len(), 1);
        restore(&database, &list[0].name).unwrap();
        let restored = Storage::open_checked(&database).unwrap();
        assert!(restored.load_privacy_mute().unwrap());
        assert!(!restored.load_device_access_allowed().unwrap());
        assert!(!restored.load_startup_enabled().unwrap());
        assert!(restored.list_client_enrollments().unwrap().is_empty());
        let mut plane = audiorouter_control::ControlPlane::with_storage("restored", restored);
        assert_eq!(
            plane.get_session(&session_id).unwrap().name,
            "Preserved owner route"
        );
        let status = plane
            .dispatch(audiorouter_protocol::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "status.get".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(status["activeSessionCount"], 0);
        assert_eq!(status["privacyMute"]["muted"], true);
        let archive = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("damaged-")
            })
            .unwrap();
        assert_eq!(
            fs::read(archive.join("original.sqlite")).unwrap(),
            b"damaged original evidence"
        );
    }

    #[test]
    fn invalid_or_unlisted_backup_never_moves_original_data() {
        let root = directory();
        let database = root.join("state.sqlite");
        fs::write(&database, b"preserve me").unwrap();
        let backups = Storage::recovery_backup_directory(&database);
        fs::create_dir(&backups).unwrap();
        fs::write(backups.join("audiorouter-backup-bad.sqlite"), b"invalid").unwrap();
        assert!(choices(&database).unwrap().is_empty());
        for name in [
            "../outside.sqlite",
            "audiorouter-backup-bad.sqlite",
            "C:\\private.sqlite",
            "other.sqlite",
        ] {
            assert!(restore(&database, name).is_err());
            assert_eq!(fs::read(&database).unwrap(), b"preserve me");
        }
    }
}
