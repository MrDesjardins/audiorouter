//! SQLite persistence boundary for M01.

use audiorouter_domain::{node_registry, validate_session, EntityId, Session};
use audiorouter_recording::RecorderCheckpoint;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::io::Write;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use zip::ZipArchive;

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    InvalidSession(String),
    InvalidBundle(String),
    InvalidRecording(String),
    InvalidPluginState(String),
    CorruptDatabase(String),
    IdempotencyConflict,
    DocumentTooLarge { bytes: usize, maximum: usize },
    InvalidBackupPath(String),
}

pub const MAX_SESSION_DOCUMENT_BYTES: usize = 1024 * 1024;
pub const MAX_BACKUP_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_BUNDLE_COMPRESSED_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_BUNDLE_EXPANDED_BYTES: u64 = 250 * 1024 * 1024;
pub const MAX_BUNDLE_ENTRIES: usize = 1_000;
pub const MAX_BUNDLE_ASSET_BYTES: u64 = 16 * 1024 * 1024;
pub const IDEMPOTENCY_RETENTION_SECONDS: i64 = 24 * 60 * 60;

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
pub const GRAPH_PLAN_RETENTION_SECONDS: i64 = 5 * 60;
pub const DAILY_RECOVERY_BACKUP_LIMIT: usize = 10;

fn is_executable_name(name: &str) -> bool {
    matches!(
        std::path::Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("exe")
            | Some("dll")
            | Some("sys")
            | Some("com")
            | Some("bat")
            | Some("cmd")
            | Some("ps1")
    )
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Deserialize)]
struct BundleManifest {
    #[serde(rename = "format")]
    format: String,
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "graphPath")]
    graph_path: String,
    #[serde(default)]
    assets: Vec<BundleAsset>,
    #[serde(rename = "requiredNodeTypes", default)]
    required_node_types: Vec<RequiredNodeType>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
struct RequiredNodeType {
    #[serde(rename = "type")]
    type_name: String,
    version: u32,
}

#[derive(serde::Serialize)]
struct ExportBundleManifest<'a> {
    format: &'a str,
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "createdWith")]
    created_with: &'a str,
    #[serde(rename = "graphPath")]
    graph_path: &'a str,
    assets: Vec<String>,
    #[serde(rename = "requiredNodeTypes")]
    required_node_types: Vec<RequiredNodeType>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BundleAsset {
    Path(String),
    Metadata {
        path: String,
        #[serde(default)]
        sha256: Option<String>,
        #[serde(default)]
        size: Option<u64>,
    },
}

impl BundleAsset {
    fn path(&self) -> &str {
        match self {
            Self::Path(path) | Self::Metadata { path, .. } => path,
        }
    }

    fn metadata(&self) -> (Option<&str>, Option<u64>) {
        match self {
            Self::Path(_) => (None, None),
            Self::Metadata { sha256, size, .. } => (sha256.as_deref(), *size),
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct Storage {
    connection: Connection,
    database_path: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingRecord {
    pub id: String,
    pub session_id: String,
    pub recorder_id: String,
    pub path: String,
    pub format: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: u64,
    pub file_bytes: u64,
    pub start_time: String,
    pub state: String,
    pub missing: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphPlanRecord {
    pub id: String,
    pub session_id: String,
    pub base_revision: u64,
    pub candidate: Session,
    pub expires_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginStateRecord {
    pub id: String,
    pub plugin_id: String,
    pub plugin_sha256: String,
    pub version: u32,
    pub path: String,
    pub state_sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalFailureStage {
    BeforeHistory,
    AfterHistory,
    AfterCurrent,
    AfterJournal,
}

impl Storage {
    pub fn open_memory() -> Result<Self, StorageError> {
        let storage = Self {
            connection: Connection::open_in_memory()?,
            database_path: None,
        };
        storage.check_integrity()?;
        storage.migrate()?;
        storage.prune_expired_recovery()?;
        Ok(storage)
    }

    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let storage = Self {
            connection: Connection::open(path)?,
            database_path: Some(path.to_path_buf()),
        };
        storage.check_integrity()?;
        storage.migrate()?;
        storage.prune_expired_recovery()?;
        Ok(storage)
    }

    /// Reject malformed SQLite before migrations can make any changes. This
    /// is intentionally a read-only check; recovery uses an explicit backup
    /// or restore destination instead of modifying the damaged source.
    fn check_integrity(&self) -> Result<(), StorageError> {
        let result: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|error| StorageError::CorruptDatabase(error.to_string()))?;
        if result == "ok" {
            Ok(())
        } else {
            Err(StorageError::CorruptDatabase(result))
        }
    }

    /// Create a consistent SQLite backup while the source connection remains open.
    pub fn backup_to(&self, destination: impl AsRef<std::path::Path>) -> Result<(), StorageError> {
        let destination = destination.as_ref();
        if !destination.is_absolute() {
            return Err(StorageError::InvalidBackupPath(
                "backup destination must be absolute".into(),
            ));
        }
        let parent = destination.parent().ok_or_else(|| {
            StorageError::InvalidBackupPath("backup destination must have a parent".into())
        })?;
        if !parent.is_dir() {
            return Err(StorageError::InvalidBackupPath(
                "backup destination parent must already exist".into(),
            ));
        }
        match std::fs::symlink_metadata(destination) {
            Ok(metadata) => {
                if is_reparse_point(&metadata) {
                    return Err(StorageError::InvalidBackupPath(
                        "backup destination cannot be a symbolic link or reparse point".into(),
                    ));
                }
                return Err(StorageError::InvalidBackupPath(
                    "backup destination must not already exist".into(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::Io(error)),
        }
        if let Some(source) = &self.database_path {
            let source = std::fs::canonicalize(source)?;
            let existing = destination
                .exists()
                .then(|| std::fs::canonicalize(destination))
                .transpose()?;
            if existing.as_ref() == Some(&source) {
                return Err(StorageError::InvalidBackupPath(
                    "backup destination cannot be the live database".into(),
                ));
            }
        }
        self.connection
            .backup(rusqlite::DatabaseName::Main, destination, None)
            .map_err(StorageError::Sql)
    }

    /// Retain the newest ten explicitly named daily recovery backups.
    ///
    /// Only direct regular files named audiorouter-backup-*.sqlite are
    /// eligible. Files named audiorouter-pre-migration-*.sqlite are always
    /// preserved, as are all unrelated files. The caller owns the selected
    /// directory and invokes this operation as an explicit maintenance action.
    pub fn prune_recovery_backups(
        directory: impl AsRef<std::path::Path>,
    ) -> Result<Vec<std::path::PathBuf>, StorageError> {
        let directory = directory.as_ref();
        if !directory.is_absolute() {
            return Err(StorageError::InvalidBackupPath(
                "backup retention directory must be absolute".into(),
            ));
        }
        let metadata = std::fs::symlink_metadata(directory)?;
        if !metadata.is_dir() || is_reparse_point(&metadata) {
            return Err(StorageError::InvalidBackupPath(
                "backup retention directory must be a regular non-symlink directory".into(),
            ));
        }

        let mut daily = Vec::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || is_reparse_point(&metadata) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with("audiorouter-backup-")
                && name.ends_with(".sqlite")
                && !name.starts_with("audiorouter-backup-pre-migration-")
            {
                daily.push((name.to_owned(), path));
            }
        }
        daily.sort_by(|left, right| right.0.cmp(&left.0));

        let mut removed = Vec::new();
        for (_, path) in daily.into_iter().skip(DAILY_RECOVERY_BACKUP_LIMIT) {
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
        Ok(removed)
    }

    /// Restore a validated SQLite backup into a new file. Existing files and
    /// symbolic links are never overwritten; callers must explicitly move a
    /// validated result into place using their own deployment policy.
    pub fn restore_backup(
        source: impl AsRef<std::path::Path>,
        destination: impl AsRef<std::path::Path>,
    ) -> Result<(), StorageError> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        if !source.is_absolute() || !destination.is_absolute() {
            return Err(StorageError::InvalidBackupPath(
                "backup paths must be absolute".into(),
            ));
        }
        if !source.is_file() || is_reparse_point(&std::fs::symlink_metadata(source)?) {
            return Err(StorageError::InvalidBackupPath(
                "backup source must be a regular non-symlink file".into(),
            ));
        }
        let size = std::fs::metadata(source)?.len();
        if size > MAX_BACKUP_BYTES {
            return Err(StorageError::DocumentTooLarge {
                bytes: size as usize,
                maximum: MAX_BACKUP_BYTES as usize,
            });
        }
        let parent = destination.parent().ok_or_else(|| {
            StorageError::InvalidBackupPath("restore destination must have a parent".into())
        })?;
        if !parent.is_dir() {
            return Err(StorageError::InvalidBackupPath(
                "restore destination parent must already exist".into(),
            ));
        }
        match std::fs::symlink_metadata(destination) {
            Ok(_) => {
                return Err(StorageError::InvalidBackupPath(
                    "restore destination must not already exist".into(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(StorageError::Io(error)),
        }
        if std::fs::canonicalize(source).ok()
            == std::fs::canonicalize(parent.join(destination.file_name().unwrap_or_default())).ok()
        {
            return Err(StorageError::InvalidBackupPath(
                "restore destination cannot be the backup source".into(),
            ));
        }
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
        let source_connection = Connection::open_with_flags(source, flags)?;
        let integrity: String =
            source_connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(StorageError::InvalidBackupPath(format!(
                "backup integrity check failed: {integrity}"
            )));
        }
        match source_connection.backup(rusqlite::DatabaseName::Main, destination, None) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(destination);
                Err(StorageError::Sql(error))
            }
        }
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 id TEXT PRIMARY KEY,
                 revision INTEGER NOT NULL,
                 document TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS session_history (
                 session_id TEXT NOT NULL,
                 revision INTEGER NOT NULL,
                 document TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 PRIMARY KEY(session_id, revision)
             );
             CREATE TABLE IF NOT EXISTS operation_journal (
                 idempotency_key TEXT PRIMARY KEY,
                 operation TEXT NOT NULL,
                 result TEXT NOT NULL,
                 committed_revision INTEGER NOT NULL,
                 request_hash TEXT NOT NULL DEFAULT '',
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS graph_plans (
                 id TEXT PRIMARY KEY,
                 session_id TEXT NOT NULL,
                 base_revision INTEGER NOT NULL,
                 candidate TEXT NOT NULL,
                 expires_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS graph_plans_expires_at ON graph_plans(expires_at);
             CREATE TABLE IF NOT EXISTS control_settings (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS client_enrollments (
                 client_id TEXT PRIMARY KEY,
                 role TEXT NOT NULL CHECK(role IN ('observer', 'editor', 'operator')),
                 revoked INTEGER NOT NULL DEFAULT 0 CHECK(revoked IN (0, 1)),
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 revoked_at TEXT
             );
             CREATE TABLE IF NOT EXISTS recordings (
                 id TEXT PRIMARY KEY,
                 session_id TEXT NOT NULL,
                 recorder_id TEXT NOT NULL,
                 path TEXT NOT NULL,
                 format TEXT NOT NULL,
                 channels INTEGER NOT NULL,
                 sample_rate INTEGER NOT NULL,
                 frames INTEGER NOT NULL,
                 file_bytes INTEGER NOT NULL,
                 start_time TEXT NOT NULL,
                 state TEXT NOT NULL,
                 missing INTEGER NOT NULL DEFAULT 0 CHECK(missing IN (0, 1)),
                 title TEXT,
                 artist TEXT,
                 comment TEXT
             );
             CREATE INDEX IF NOT EXISTS recordings_session_id ON recordings(session_id);
             CREATE TABLE IF NOT EXISTS recording_checkpoints (
                 recording_id TEXT PRIMARY KEY,
                 checkpoint TEXT NOT NULL,
                 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             CREATE TABLE IF NOT EXISTS plugin_states (
                 id TEXT PRIMARY KEY,
                 plugin_id TEXT NOT NULL,
                 plugin_sha256 TEXT NOT NULL,
                 version INTEGER NOT NULL,
                 path TEXT NOT NULL,
                 state_sha256 TEXT NOT NULL,
                 size_bytes INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS plugin_states_plugin_id ON plugin_states(plugin_id);
             INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);",
        )?;
        let has_request_hash = self
            .connection
            .prepare("PRAGMA table_info(operation_journal)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .any(|column| column.as_deref().ok() == Some("request_hash"));
        if !has_request_hash {
            self.connection.execute(
                "ALTER TABLE operation_journal ADD COLUMN request_hash TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        self.connection.execute(
            "CREATE INDEX IF NOT EXISTS operation_journal_created_at
             ON operation_journal(created_at)",
            [],
        )?;
        Ok(())
    }

    pub fn save_session(&self, session: &Session) -> Result<(), StorageError> {
        let document = serde_json::to_string(session)?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO session_history(session_id, revision, document) VALUES (?1, ?2, ?3)",
            params![session.id.as_str(), session.revision as i64, &document],
        )?;
        transaction.execute(
            "INSERT INTO sessions(id, revision, document) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET revision=excluded.revision, document=excluded.document",
            params![session.id.as_str(), session.revision as i64, &document],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_session(&self, id: &EntityId) -> Result<bool, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let changed =
            transaction.execute("DELETE FROM sessions WHERE id = ?1", params![id.as_str()])?;
        transaction.execute(
            "DELETE FROM session_history WHERE session_id = ?1",
            params![id.as_str()],
        )?;
        transaction.execute(
            "DELETE FROM graph_plans WHERE session_id = ?1",
            params![id.as_str()],
        )?;
        transaction.commit()?;
        Ok(changed != 0)
    }

    pub fn save_recording(&self, recording: &RecordingRecord) -> Result<(), StorageError> {
        if recording.id.is_empty()
            || recording.session_id.is_empty()
            || recording.recorder_id.is_empty()
            || recording.path.is_empty()
            || recording.format.is_empty()
            || recording.start_time.is_empty()
            || recording.state.is_empty()
            || !matches!(recording.channels, 1 | 2)
            || !matches!(recording.sample_rate, 44_100 | 48_000)
        {
            return Err(StorageError::InvalidRecording(
                "invalid recording fields".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO recordings
             (id, session_id, recorder_id, path, format, channels, sample_rate, frames,
              file_bytes, start_time, state, missing, title, artist, comment)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
               session_id=excluded.session_id, recorder_id=excluded.recorder_id,
               path=excluded.path, format=excluded.format, channels=excluded.channels,
               sample_rate=excluded.sample_rate, frames=excluded.frames,
               file_bytes=excluded.file_bytes, start_time=excluded.start_time,
               state=excluded.state, missing=excluded.missing, title=excluded.title,
               artist=excluded.artist, comment=excluded.comment",
            params![
                recording.id,
                recording.session_id,
                recording.recorder_id,
                recording.path,
                recording.format,
                i64::from(recording.channels),
                i64::from(recording.sample_rate),
                recording.frames as i64,
                recording.file_bytes as i64,
                recording.start_time,
                recording.state,
                recording.missing as i64,
                recording.title,
                recording.artist,
                recording.comment,
            ],
        )?;
        Ok(())
    }

    /// Persist only a validated recorder state checkpoint. Audio samples,
    /// queues, and file handles remain outside SQLite.
    pub fn save_recording_checkpoint(
        &self,
        recording_id: &str,
        checkpoint: &RecorderCheckpoint,
    ) -> Result<(), StorageError> {
        if recording_id.is_empty() {
            return Err(StorageError::InvalidRecording(
                "recording checkpoint ID is empty".into(),
            ));
        }
        let checkpoint = audiorouter_recording::RecorderController::restore(checkpoint.clone())
            .map_err(|_| StorageError::InvalidRecording("invalid recording checkpoint".into()))?;
        let document = checkpoint
            .checkpoint_json()
            .map_err(|error| StorageError::InvalidRecording(error.to_string()))?;
        self.connection.execute(
            "INSERT INTO recording_checkpoints(recording_id, checkpoint)
             VALUES (?1, ?2)
             ON CONFLICT(recording_id) DO UPDATE SET
               checkpoint=excluded.checkpoint, updated_at=CURRENT_TIMESTAMP",
            params![recording_id, document],
        )?;
        Ok(())
    }

    pub fn load_recording_checkpoint(
        &self,
        recording_id: &str,
    ) -> Result<Option<RecorderCheckpoint>, StorageError> {
        let document = self
            .connection
            .query_row(
                "SELECT checkpoint FROM recording_checkpoints WHERE recording_id = ?1",
                params![recording_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        document
            .map(|document| {
                audiorouter_recording::RecorderController::restore_json(&document)
                    .map(|controller| controller.checkpoint())
                    .map_err(|_| {
                        StorageError::InvalidRecording(
                            "persisted recording checkpoint is invalid".into(),
                        )
                    })
            })
            .transpose()
    }

    pub fn clear_recording_checkpoint(&self, recording_id: &str) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "DELETE FROM recording_checkpoints WHERE recording_id = ?1",
            params![recording_id],
        )? == 1)
    }

    pub fn list_recordings(
        &self,
        session_id: Option<&str>,
    ) -> Result<Vec<RecordingRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, session_id, recorder_id, path, format, channels, sample_rate,
                    frames, file_bytes, start_time, state, missing, title, artist, comment
             FROM recordings
             WHERE (?1 IS NULL OR session_id = ?1)
             ORDER BY start_time ASC, id ASC",
        )?;
        let records = statement
            .query_map(params![session_id], |row| {
                Ok(RecordingRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    recorder_id: row.get(2)?,
                    path: row.get(3)?,
                    format: row.get(4)?,
                    channels: row.get::<_, i64>(5)? as u16,
                    sample_rate: row.get::<_, i64>(6)? as u32,
                    frames: row.get::<_, i64>(7)? as u64,
                    file_bytes: row.get::<_, i64>(8)? as u64,
                    start_time: row.get(9)?,
                    state: row.get(10)?,
                    missing: row.get::<_, i64>(11)? != 0,
                    title: row.get(12)?,
                    artist: row.get(13)?,
                    comment: row.get(14)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn get_recording(&self, id: &str) -> Result<Option<RecordingRecord>, StorageError> {
        Ok(self
            .list_recordings(None)?
            .into_iter()
            .find(|recording| recording.id == id))
    }

    /// Removes only the durable library row; it never touches the recording path.
    pub fn remove_recording_entry(&self, id: &str) -> Result<bool, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let removed =
            transaction.execute("DELETE FROM recordings WHERE id = ?1", params![id])? == 1;
        if removed {
            transaction.execute(
                "DELETE FROM recording_checkpoints WHERE recording_id = ?1",
                params![id],
            )?;
        }
        transaction.commit()?;
        Ok(removed)
    }

    pub fn set_recording_missing(&self, id: &str, missing: bool) -> Result<bool, StorageError> {
        let changed = self.connection.execute(
            "UPDATE recordings SET missing = ?2 WHERE id = ?1",
            params![id, missing as i64],
        )?;
        Ok(changed == 1)
    }

    /// Rename a recording within its existing canonical directory and update
    /// only the durable library path. The destination must not already exist.
    pub fn rename_recording(&self, id: &str, new_path: &str) -> Result<bool, StorageError> {
        let Some(recording) = self.get_recording(id)? else {
            return Ok(false);
        };
        let source = std::path::Path::new(&recording.path);
        let destination = std::path::Path::new(new_path);
        if !destination.is_absolute()
            || !matches!(
                destination
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.to_ascii_lowercase())
                    .as_deref(),
                Some("wav") | Some("flac")
            )
        {
            return Err(StorageError::InvalidRecording(
                "rename destination must be an absolute wav/flac path".into(),
            ));
        }
        let source_metadata = std::fs::symlink_metadata(source)?;
        if !source_metadata.file_type().is_file() {
            return Err(StorageError::InvalidRecording(
                "recording source must be a regular file".into(),
            ));
        }
        if destination.exists() || std::fs::symlink_metadata(destination).is_ok() {
            return Err(StorageError::InvalidRecording(
                "rename destination must not already exist".into(),
            ));
        }
        let source_parent = std::fs::canonicalize(source.parent().ok_or_else(|| {
            StorageError::InvalidRecording("recording source has no parent".into())
        })?)?;
        let destination_parent = std::fs::canonicalize(destination.parent().ok_or_else(|| {
            StorageError::InvalidRecording("rename destination has no parent".into())
        })?)?;
        if source_parent != destination_parent {
            return Err(StorageError::InvalidRecording(
                "recordings may only be renamed within their existing directory".into(),
            ));
        }
        std::fs::rename(source, destination)?;
        let updated = self.connection.execute(
            "UPDATE recordings SET path = ?2 WHERE id = ?1",
            params![id, new_path],
        );
        match updated {
            Ok(1) => Ok(true),
            Ok(_) => {
                let _ = std::fs::rename(destination, source);
                Ok(false)
            }
            Err(error) => {
                let _ = std::fs::rename(destination, source);
                Err(StorageError::Sql(error))
            }
        }
    }

    pub fn update_recording_metadata(
        &self,
        id: &str,
        title: Option<&str>,
        artist: Option<&str>,
        comment: Option<&str>,
    ) -> Result<bool, StorageError> {
        for value in [title, artist, comment].into_iter().flatten() {
            if value.chars().count() > 256 || value.chars().any(|character| character.is_control())
            {
                return Err(StorageError::InvalidRecording("invalid metadata".into()));
            }
        }
        Ok(self.connection.execute(
            "UPDATE recordings SET title = ?2, artist = ?3, comment = ?4 WHERE id = ?1",
            params![id, title, artist, comment],
        )? == 1)
    }

    pub fn save_plugin_state(&self, state: &PluginStateRecord) -> Result<(), StorageError> {
        if state.id.is_empty()
            || state.plugin_id.is_empty()
            || !is_sha256(&state.plugin_sha256)
            || state.version == 0
            || state.path.is_empty()
            || !std::path::Path::new(&state.path).is_absolute()
            || !is_sha256(&state.state_sha256)
            || state.size_bytes == 0
            || state.size_bytes > MAX_BUNDLE_ASSET_BYTES
        {
            return Err(StorageError::InvalidPluginState(
                "invalid plugin state fields".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO plugin_states
             (id, plugin_id, plugin_sha256, version, path, state_sha256, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET plugin_id=excluded.plugin_id,
               plugin_sha256=excluded.plugin_sha256, version=excluded.version,
               path=excluded.path, state_sha256=excluded.state_sha256,
               size_bytes=excluded.size_bytes",
            params![
                state.id,
                state.plugin_id,
                state.plugin_sha256,
                i64::from(state.version),
                state.path,
                state.state_sha256,
                state.size_bytes as i64,
            ],
        )?;
        Ok(())
    }

    pub fn list_plugin_states(
        &self,
        plugin_id: Option<&str>,
    ) -> Result<Vec<PluginStateRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, plugin_id, plugin_sha256, version, path, state_sha256, size_bytes
             FROM plugin_states
             WHERE (?1 IS NULL OR plugin_id = ?1)
             ORDER BY id ASC",
        )?;
        let records = statement
            .query_map(params![plugin_id], |row| {
                Ok(PluginStateRecord {
                    id: row.get(0)?,
                    plugin_id: row.get(1)?,
                    plugin_sha256: row.get(2)?,
                    version: row.get::<_, i64>(3)? as u32,
                    path: row.get(4)?,
                    state_sha256: row.get(5)?,
                    size_bytes: row.get::<_, i64>(6)? as u64,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    /// Removes only state metadata; the asset file remains untouched.
    pub fn remove_plugin_state(&self, id: &str) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .execute("DELETE FROM plugin_states WHERE id = ?1", params![id])?
            == 1)
    }

    pub fn load_history(&self, id: &EntityId, limit: usize) -> Result<Vec<Session>, StorageError> {
        self.load_history_before(id, None, limit)
    }

    pub fn load_history_before(
        &self,
        id: &EntityId,
        before_revision: Option<u64>,
        limit: usize,
    ) -> Result<Vec<Session>, StorageError> {
        let mut statement = if before_revision.is_some() {
            self.connection.prepare(
                "SELECT document FROM session_history WHERE session_id = ?1 AND revision < ?2
                 ORDER BY revision DESC LIMIT ?3",
            )?
        } else {
            self.connection.prepare(
                "SELECT document FROM session_history WHERE session_id = ?1
                 ORDER BY revision DESC LIMIT ?2",
            )?
        };
        let documents: Vec<String> = if let Some(before_revision) = before_revision {
            statement
                .query_map(
                    params![id.as_str(), before_revision as i64, limit as i64],
                    |row| row.get::<_, String>(0),
                )?
                .collect::<Result<_, _>>()?
        } else {
            statement
                .query_map(params![id.as_str(), limit as i64], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<_, _>>()?
        };
        documents
            .into_iter()
            .map(|document| serde_json::from_str(&document).map_err(StorageError::Json))
            .collect()
    }

    pub fn load_session(&self, id: &EntityId) -> Result<Option<Session>, StorageError> {
        let document: Option<String> = self
            .connection
            .query_row(
                "SELECT document FROM sessions WHERE id = ?1",
                params![id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        document
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(Into::into)
    }

    pub fn list_sessions(&self, limit: usize) -> Result<Vec<Session>, StorageError> {
        self.list_sessions_after(None, limit)
    }

    pub fn count_sessions(&self) -> Result<usize, StorageError> {
        self.connection
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|count| count as usize)
            .map_err(Into::into)
    }

    pub fn list_sessions_after(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Session>, StorageError> {
        let mut statement = if cursor.is_some() {
            self.connection
                .prepare("SELECT document FROM sessions WHERE id > ?1 ORDER BY id ASC LIMIT ?2")?
        } else {
            self.connection
                .prepare("SELECT document FROM sessions ORDER BY id ASC LIMIT ?1")?
        };
        let documents: Vec<String> = if let Some(cursor) = cursor {
            statement
                .query_map(params![cursor, limit as i64], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()?
        } else {
            statement
                .query_map(params![limit as i64], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()?
        };
        documents
            .into_iter()
            .map(|document| serde_json::from_str(&document).map_err(StorageError::Json))
            .collect()
    }

    pub fn export_session(&self, id: &EntityId) -> Result<Option<String>, StorageError> {
        self.load_session(id)?
            .map(|session| serde_json::to_string(&session).map_err(StorageError::Json))
            .transpose()
    }

    /// Export a stopped session document as a v1 `.audiorouter` ZIP bundle.
    /// The destination must be an absolute, new regular file; this method
    /// never overwrites a file or follows a destination symbolic link.
    pub fn export_bundle(
        &self,
        id: &EntityId,
        destination: impl AsRef<std::path::Path>,
    ) -> Result<(), StorageError> {
        let destination = destination.as_ref();
        if !destination.is_absolute() {
            return Err(StorageError::InvalidBackupPath(
                "bundle destination must be absolute".into(),
            ));
        }
        let parent = destination.parent().ok_or_else(|| {
            StorageError::InvalidBackupPath("bundle destination must have a parent".into())
        })?;
        if !parent.is_dir() || destination.exists() {
            return Err(StorageError::InvalidBackupPath(
                "bundle destination parent must exist and destination must be new".into(),
            ));
        }
        let document = self
            .export_session(id)?
            .ok_or_else(|| StorageError::InvalidBundle("session not found".into()))?;
        let exported_session: Session = serde_json::from_str(&document)?;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        let mut archive = zip::ZipWriter::new(file);
        let manifest = ExportBundleManifest {
            format: "audiorouter.session",
            schema_version: 1,
            created_with: "0.1.0",
            graph_path: "session.json",
            assets: Vec::new(),
            required_node_types: node_registry()
                .iter()
                .filter(|spec| {
                    exported_session
                        .nodes
                        .iter()
                        .any(|node| node.kind.type_name() == spec.kind.type_name())
                })
                .map(|spec| RequiredNodeType {
                    type_name: spec.kind.type_name().into(),
                    version: spec.version,
                })
                .collect(),
        };
        let result = (|| -> Result<(), StorageError> {
            archive
                .start_file("manifest.json", zip::write::SimpleFileOptions::default())
                .map_err(|error| StorageError::InvalidBundle(error.to_string()))?;
            archive.write_all(&serde_json::to_vec(&manifest)?)?;
            archive
                .start_file("session.json", zip::write::SimpleFileOptions::default())
                .map_err(|error| StorageError::InvalidBundle(error.to_string()))?;
            archive.write_all(document.as_bytes())?;
            archive
                .finish()
                .map_err(|error| StorageError::InvalidBundle(error.to_string()))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(destination);
        }
        result
    }

    /// Validate and persist an imported session document. Validation happens
    /// before `save_session`, so rejected documents cannot create a row/history.
    pub fn import_session(&self, document: &str) -> Result<Session, StorageError> {
        if document.len() > MAX_SESSION_DOCUMENT_BYTES {
            return Err(StorageError::DocumentTooLarge {
                bytes: document.len(),
                maximum: MAX_SESSION_DOCUMENT_BYTES,
            });
        }
        let session: Session = serde_json::from_str(document)?;
        validate_session(&session).map_err(|errors| {
            StorageError::InvalidSession(
                errors
                    .into_iter()
                    .map(|error| format!("{error:?}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        })?;
        self.save_session(&session)?;
        Ok(session)
    }

    /// Safely stage and import a `.audiorouter` ZIP bundle. The staging root
    /// is caller-owned and must already exist; this method never writes
    /// anywhere else and never overwrites an existing staged file.
    pub fn import_bundle(
        &self,
        bundle: impl AsRef<std::path::Path>,
        staging_root: impl AsRef<std::path::Path>,
    ) -> Result<Session, StorageError> {
        let bundle = bundle.as_ref();
        let staging_root = staging_root.as_ref();
        if !bundle.is_absolute() || !staging_root.is_absolute() {
            return Err(StorageError::InvalidBundle(
                "bundle and staging paths must be absolute".into(),
            ));
        }
        if !bundle.is_file() || is_reparse_point(&std::fs::symlink_metadata(bundle)?) {
            return Err(StorageError::InvalidBundle(
                "bundle must be a regular non-symlink file".into(),
            ));
        }
        if !staging_root.is_dir() || is_reparse_point(&std::fs::symlink_metadata(staging_root)?) {
            return Err(StorageError::InvalidBundle(
                "staging root must be an existing non-symlink directory".into(),
            ));
        }
        let compressed = std::fs::metadata(bundle)?.len();
        if compressed > MAX_BUNDLE_COMPRESSED_BYTES {
            return Err(StorageError::DocumentTooLarge {
                bytes: compressed as usize,
                maximum: MAX_BUNDLE_COMPRESSED_BYTES as usize,
            });
        }
        let file = File::open(bundle)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|error| StorageError::InvalidBundle(format!("invalid ZIP: {error}")))?;
        if archive.len() > MAX_BUNDLE_ENTRIES {
            return Err(StorageError::InvalidBundle(
                "too many bundle entries".into(),
            ));
        }
        let staging = staging_root.join(format!(
            "audiorouter-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| StorageError::InvalidBundle(error.to_string()))?
                .as_nanos()
        ));
        std::fs::create_dir(&staging)?;
        let result = Self::stage_bundle(&mut archive, &staging).and_then(|manifest| {
            let graph = std::fs::read_to_string(staging.join(&manifest.graph_path))?;
            self.import_session(&graph)
        });
        if result.is_err() {
            let _ = std::fs::remove_dir_all(&staging);
        }
        result
    }

    fn stage_bundle(
        archive: &mut ZipArchive<File>,
        staging: &std::path::Path,
    ) -> Result<BundleManifest, StorageError> {
        let mut paths = HashSet::new();
        let mut hashes = std::collections::HashMap::new();
        let mut expanded = 0u64;
        let mut manifest = None;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| {
                StorageError::InvalidBundle(format!("cannot read ZIP entry: {error}"))
            })?;
            let name = entry.name().replace('\\', "/");
            let path = std::path::Path::new(&name);
            if name.is_empty()
                || name.starts_with('/')
                || name.contains(':')
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err(StorageError::InvalidBundle(format!(
                    "unsafe archive path: {name}"
                )));
            }
            if !paths.insert(name.clone()) {
                return Err(StorageError::InvalidBundle(format!(
                    "duplicate archive path: {name}"
                )));
            }
            if entry.is_dir() {
                continue;
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(StorageError::InvalidBundle(format!(
                    "symbolic links are not allowed: {name}"
                )));
            }
            if is_executable_name(&name) {
                return Err(StorageError::InvalidBundle(format!(
                    "executable content is not allowed: {name}"
                )));
            }
            let declared_size = entry.size();
            let entry_limit = if name == "manifest.json" {
                MAX_SESSION_DOCUMENT_BYTES as u64
            } else {
                MAX_BUNDLE_ASSET_BYTES
            };
            if declared_size > entry_limit {
                return Err(StorageError::DocumentTooLarge {
                    bytes: declared_size as usize,
                    maximum: entry_limit as usize,
                });
            }
            let mut bytes = Vec::with_capacity(declared_size.min(1024 * 1024) as usize);
            let actual_size = (&mut entry).take(entry_limit + 1).read_to_end(&mut bytes)? as u64;
            if actual_size > entry_limit {
                return Err(StorageError::DocumentTooLarge {
                    bytes: actual_size as usize,
                    maximum: entry_limit as usize,
                });
            }
            expanded = expanded.checked_add(actual_size).ok_or_else(|| {
                StorageError::InvalidBundle("bundle expanded size overflow".into())
            })?;
            if expanded > MAX_BUNDLE_EXPANDED_BYTES {
                return Err(StorageError::DocumentTooLarge {
                    bytes: expanded as usize,
                    maximum: MAX_BUNDLE_EXPANDED_BYTES as usize,
                });
            }
            let output = staging.join(path);
            if !output.starts_with(staging) {
                return Err(StorageError::InvalidBundle("staging escape".into()));
            }
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut target = std::fs::OpenOptions::new();
            target.write(true).create_new(true);
            use std::io::Write;
            let mut target = target.open(&output)?;
            target.write_all(&bytes)?;
            let hash = Sha256::digest(&bytes);
            hashes.insert(name.clone(), (actual_size, format!("{hash:x}")));
            if name == "manifest.json" {
                manifest = Some(serde_json::from_slice::<BundleManifest>(&bytes)?);
            }
        }
        let manifest = manifest
            .ok_or_else(|| StorageError::InvalidBundle("manifest.json is required".into()))?;
        if manifest.format != "audiorouter.session" || manifest.schema_version != 1 {
            return Err(StorageError::InvalidBundle(
                "unsupported bundle manifest".into(),
            ));
        }
        let registry = node_registry();
        for required in &manifest.required_node_types {
            let supported = registry
                .iter()
                .find(|spec| spec.kind.type_name() == required.type_name);
            if supported.map_or(true, |spec| spec.version != required.version) {
                return Err(StorageError::InvalidBundle(format!(
                    "unsupported required node type: {} v{}",
                    required.type_name, required.version
                )));
            }
        }
        for asset in &manifest.assets {
            let path = asset.path();
            if !paths.contains(path)
                || path.starts_with('/')
                || path.contains(':')
                || path.contains("..")
            {
                return Err(StorageError::InvalidBundle(format!(
                    "manifest references unsafe or missing path: {path}"
                )));
            }
            let (expected_hash, expected_size) = asset.metadata();
            let (actual_size, actual_hash) = hashes.get(path).ok_or_else(|| {
                StorageError::InvalidBundle(format!("manifest asset is not a file: {path}"))
            })?;
            if expected_size.is_some_and(|size| size != *actual_size) {
                return Err(StorageError::InvalidBundle(format!(
                    "asset size mismatch: {path}"
                )));
            }
            if let Some(expected_hash) = expected_hash {
                if expected_hash.len() != 64
                    || !expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || !expected_hash.eq_ignore_ascii_case(actual_hash)
                {
                    return Err(StorageError::InvalidBundle(format!(
                        "asset hash mismatch: {path}"
                    )));
                }
            }
        }
        if !paths.contains(&manifest.graph_path)
            || manifest.graph_path.starts_with('/')
            || manifest.graph_path.contains(':')
            || manifest.graph_path.contains("..")
        {
            return Err(StorageError::InvalidBundle(format!(
                "manifest references unsafe or missing path: {}",
                manifest.graph_path
            )));
        }
        Ok(manifest)
    }

    pub fn journal_result(&self, key: &str) -> Result<Option<String>, StorageError> {
        self.connection
            .query_row(
                "SELECT result FROM operation_journal WHERE idempotency_key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn operation_status(
        &self,
        operation_id: &str,
    ) -> Result<Option<(String, String, u64, String)>, StorageError> {
        self.connection
            .query_row(
                "SELECT operation, result, committed_revision, created_at
                 FROM operation_journal WHERE idempotency_key = ?1",
                params![operation_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get::<_, i64>(2)? as u64,
                        row.get(3)?,
                    ))
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn save_client_enrollment(&self, client_id: &str, role: &str) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO client_enrollments(client_id, role, revoked, revoked_at)
             VALUES (?1, ?2, 0, NULL)
             ON CONFLICT(client_id) DO UPDATE SET role=excluded.role, revoked=0, revoked_at=NULL",
            params![client_id, role],
        )?;
        Ok(())
    }

    pub fn revoke_client_enrollment(&self, client_id: &str) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "UPDATE client_enrollments SET revoked=1, revoked_at=CURRENT_TIMESTAMP
             WHERE client_id = ?1 AND revoked = 0",
            params![client_id],
        )? == 1)
    }

    pub fn load_client_enrollment(
        &self,
        client_id: &str,
    ) -> Result<Option<(String, bool)>, StorageError> {
        self.connection
            .query_row(
                "SELECT role, revoked FROM client_enrollments WHERE client_id = ?1",
                params![client_id],
                |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_client_enrollments(&self) -> Result<Vec<(String, String, bool)>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT client_id, role, revoked FROM client_enrollments ORDER BY client_id ASC",
        )?;
        let records = statement
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? != 0))
            })?
            .collect::<Result<_, _>>()
            .map_err(Into::into);
        records
    }

    pub fn journal_commit(
        &self,
        key: &str,
        operation: &str,
        result: &str,
        revision: u64,
    ) -> Result<bool, StorageError> {
        self.journal_commit_with_hash(key, operation, result, revision, "")
    }

    pub fn journal_commit_with_hash(
        &self,
        key: &str,
        operation: &str,
        result: &str,
        revision: u64,
        request_hash: &str,
    ) -> Result<bool, StorageError> {
        self.prune_expired_journal()?;
        let inserted = self.connection.execute(
            "INSERT OR IGNORE INTO operation_journal(idempotency_key, operation, result, committed_revision, request_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![key, operation, result, revision as i64, request_hash],
        )?;
        Ok(inserted == 1)
    }

    pub fn journal_result_checked(
        &self,
        key: &str,
        request_hash: &str,
    ) -> Result<Option<String>, StorageError> {
        self.prune_expired_journal()?;
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT result, request_hash FROM operation_journal WHERE idempotency_key = ?1",
                params![key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match row {
            Some((result, stored_hash)) if stored_hash == request_hash => Ok(Some(result)),
            Some(_) => Err(StorageError::IdempotencyConflict),
            None => Ok(None),
        }
    }

    fn prune_expired_journal(&self) -> Result<(), StorageError> {
        self.connection.execute(
            "DELETE FROM operation_journal
             WHERE created_at < datetime('now', ?1)",
            params![format!("-{} seconds", IDEMPOTENCY_RETENTION_SECONDS)],
        )?;
        Ok(())
    }

    pub fn save_graph_plan(&self, plan: &GraphPlanRecord) -> Result<(), StorageError> {
        let candidate = serde_json::to_string(&plan.candidate)?;
        self.connection.execute(
            "INSERT OR REPLACE INTO graph_plans(id, session_id, base_revision, candidate, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                plan.id,
                plan.session_id,
                plan.base_revision as i64,
                candidate,
                plan.expires_at
            ],
        )?;
        Ok(())
    }

    pub fn load_graph_plan(&self, id: &str) -> Result<Option<GraphPlanRecord>, StorageError> {
        self.prune_expired_graph_plans()?;
        self.connection
            .query_row(
                "SELECT id, session_id, base_revision, candidate, expires_at
                 FROM graph_plans WHERE id = ?1",
                params![id],
                |row| {
                    let candidate: String = row.get(3)?;
                    Ok(GraphPlanRecord {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        base_revision: row.get::<_, i64>(2)? as u64,
                        candidate: serde_json::from_str(&candidate).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                3,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                        expires_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::Sql)
    }

    pub fn delete_graph_plan(&self, id: &str) -> Result<(), StorageError> {
        self.connection
            .execute("DELETE FROM graph_plans WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn save_privacy_mute(&self, muted: bool) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO control_settings(key, value) VALUES ('privacyMute', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![if muted { "true" } else { "false" }],
        )?;
        Ok(())
    }

    pub fn load_privacy_mute(&self) -> Result<bool, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT value FROM control_settings WHERE key = 'privacyMute'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .as_deref()
            == Some("true"))
    }

    fn prune_expired_graph_plans(&self) -> Result<(), StorageError> {
        self.connection.execute(
            "DELETE FROM graph_plans WHERE expires_at <= unixepoch('now')",
            [],
        )?;
        Ok(())
    }

    /// Remove bounded recovery records that have passed their retention
    /// windows. This is safe maintenance only; it never touches sessions,
    /// recordings, plugin assets, or files on disk.
    pub fn prune_expired_recovery(&self) -> Result<(), StorageError> {
        self.prune_expired_journal()?;
        self.prune_expired_graph_plans()?;
        Ok(())
    }

    /// Persist the current session and its idempotency record as one SQLite
    /// transaction. The failure stage is test-only infrastructure used to
    /// prove that partial journal writes cannot survive a restart.
    pub fn save_session_with_journal(
        &self,
        session: &Session,
        key: &str,
        operation: &str,
        result: &str,
        failure: Option<JournalFailureStage>,
    ) -> Result<(), StorageError> {
        self.save_session_with_journal_with_hash(session, key, operation, result, "", failure)
    }

    pub fn save_session_with_journal_with_hash(
        &self,
        session: &Session,
        key: &str,
        operation: &str,
        result: &str,
        request_hash: &str,
        failure: Option<JournalFailureStage>,
    ) -> Result<(), StorageError> {
        let document = serde_json::to_string(session)?;
        self.prune_expired_journal()?;
        let transaction = self.connection.unchecked_transaction()?;
        if failure == Some(JournalFailureStage::BeforeHistory) {
            return Err(StorageError::InvalidSession(
                "injected journal failure".into(),
            ));
        }
        transaction.execute(
            "INSERT OR REPLACE INTO session_history(session_id, revision, document) VALUES (?1, ?2, ?3)",
            params![session.id.as_str(), session.revision as i64, &document],
        )?;
        if failure == Some(JournalFailureStage::AfterHistory) {
            return Err(StorageError::InvalidSession(
                "injected journal failure".into(),
            ));
        }
        transaction.execute(
            "INSERT INTO sessions(id, revision, document) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET revision=excluded.revision, document=excluded.document",
            params![session.id.as_str(), session.revision as i64, &document],
        )?;
        if failure == Some(JournalFailureStage::AfterCurrent) {
            return Err(StorageError::InvalidSession(
                "injected journal failure".into(),
            ));
        }
        transaction.execute(
            "INSERT OR IGNORE INTO operation_journal(idempotency_key, operation, result, committed_revision, request_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![key, operation, result, session.revision as i64, request_hash],
        )?;
        if failure == Some(JournalFailureStage::AfterJournal) {
            return Err(StorageError::InvalidSession(
                "injected journal failure".into(),
            ));
        }
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_domain::{Edge, Node, NodeKind, Port, PortDirection};
    use audiorouter_recording::{
        RecordingChunk, RecordingError, RecordingQueue, WavFormat, WavRecorder, WavWriter,
    };
    use std::io::{Cursor, Write};

    fn session() -> Session {
        Session {
            id: EntityId::new("session"),
            name: "persisted".into(),
            schema_version: 1,
            revision: 3,
            nodes: vec![
                Node {
                    id: EntityId::new("in"),
                    kind: NodeKind::PhysicalInput,
                    name: "Input".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    }],
                },
                Node {
                    id: EntityId::new("out"),
                    kind: NodeKind::PhysicalOutput,
                    name: "Output".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    }],
                },
            ],
            edges: vec![Edge {
                id: EntityId::new("edge"),
                source_node: EntityId::new("in"),
                source_port: "main".into(),
                destination_node: EntityId::new("out"),
                destination_port: "main".into(),
                matrix: vec![1.0],
                enabled: true,
            }],
        }
    }

    fn write_bundle(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }

    fn bundle_manifest() -> Vec<u8> {
        br#"{"format":"audiorouter.session","schemaVersion":1,"graphPath":"session.json","assets":[]}"#.to_vec()
    }

    #[test]
    fn migration_and_session_round_trip_are_durable_in_connection() {
        let storage = Storage::open_memory().unwrap();
        let original = session();
        storage.save_session(&original).unwrap();
        assert_eq!(storage.load_session(&original.id).unwrap(), Some(original));
        assert!(storage
            .load_session(&EntityId::new("missing"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn opening_corrupt_database_returns_explicit_read_only_error() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-corrupt-storage-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, b"not a sqlite database").unwrap();
        let result = Storage::open(&path);
        assert!(matches!(result, Err(StorageError::CorruptDatabase(_))));
        assert_eq!(std::fs::read(&path).unwrap(), b"not a sqlite database");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deleting_session_removes_current_and_history_rows() {
        let storage = Storage::open_memory().unwrap();
        let original = session();
        storage.save_session(&original).unwrap();
        storage
            .save_graph_plan(&GraphPlanRecord {
                id: "plan-delete".into(),
                session_id: original.id.as_str().into(),
                base_revision: original.revision,
                candidate: original.clone(),
                expires_at: i64::MAX,
            })
            .unwrap();
        assert!(storage.delete_session(&original.id).unwrap());
        assert!(!storage.delete_session(&original.id).unwrap());
        assert!(storage.load_session(&original.id).unwrap().is_none());
        assert!(storage.load_history(&original.id, 10).unwrap().is_empty());
        assert!(storage.load_graph_plan("plan-delete").unwrap().is_none());
    }

    #[test]
    fn session_listing_uses_stable_id_cursors() {
        let storage = Storage::open_memory().unwrap();
        for id in ["a", "b", "c"] {
            let mut value = session();
            value.id = EntityId::new(id);
            storage.save_session(&value).unwrap();
        }
        let first = storage.list_sessions_after(None, 2).unwrap();
        assert_eq!(
            first
                .iter()
                .map(|value| value.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        let second = storage.list_sessions_after(Some("b"), 2).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id.as_str(), "c");
    }

    #[test]
    fn journal_insert_is_idempotent() {
        let storage = Storage::open_memory().unwrap();
        assert!(storage
            .journal_commit("op", "graph.commit", "{\"revision\":1}", 1)
            .unwrap());
        assert!(!storage
            .journal_commit("op", "graph.commit", "{\"revision\":2}", 2)
            .unwrap());
        assert_eq!(
            storage.journal_result("op").unwrap().as_deref(),
            Some("{\"revision\":1}")
        );
    }

    #[test]
    fn journal_request_hash_replays_only_matching_requests() {
        let storage = Storage::open_memory().unwrap();
        assert!(storage
            .journal_commit_with_hash("hashed", "graph.commit", "result", 1, "hash-a")
            .unwrap());
        assert_eq!(
            storage
                .journal_result_checked("hashed", "hash-a")
                .unwrap()
                .as_deref(),
            Some("result")
        );
        assert!(matches!(
            storage.journal_result_checked("hashed", "hash-b"),
            Err(StorageError::IdempotencyConflict)
        ));
    }

    #[test]
    fn legacy_journal_schema_is_upgraded_with_a_conservative_empty_hash() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-legacy-journal-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE operation_journal (
                        idempotency_key TEXT PRIMARY KEY,
                        operation TEXT NOT NULL,
                        result TEXT NOT NULL,
                        committed_revision INTEGER NOT NULL,
                        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                    );
                    INSERT INTO operation_journal
                        (idempotency_key, operation, result, committed_revision)
                    VALUES ('legacy', 'graph.commit', 'old-result', 1);",
                )
                .unwrap();
        }
        let storage = Storage::open(&path).unwrap();
        assert_eq!(
            storage.journal_result_checked("legacy", "").unwrap(),
            Some("old-result".into())
        );
        assert!(matches!(
            storage.journal_result_checked("legacy", "new-hash"),
            Err(StorageError::IdempotencyConflict)
        ));
        drop(storage);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn expired_journal_entries_are_pruned_before_replay() {
        let storage = Storage::open_memory().unwrap();
        assert!(storage
            .journal_commit_with_hash("expired", "graph.commit", "old", 1, "hash")
            .unwrap());
        storage
            .connection
            .execute(
                "UPDATE operation_journal SET created_at = datetime('now', '-2 days') WHERE idempotency_key = 'expired'",
                [],
            )
            .unwrap();
        assert_eq!(
            storage.journal_result_checked("expired", "hash").unwrap(),
            None
        );
        assert!(storage
            .journal_commit_with_hash("expired", "graph.commit", "new", 2, "new-hash")
            .unwrap());
    }

    #[test]
    fn reopening_storage_prunes_idle_recovery_rows() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-retention-open-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let storage = Storage::open(&path).unwrap();
            storage
                .journal_commit("idle-journal", "graph.commit", "{}", 1)
                .unwrap();
            storage
                .connection
                .execute(
                    "UPDATE operation_journal SET created_at = datetime('now', '-2 days')",
                    [],
                )
                .unwrap();
            let candidate = session();
            storage
                .save_graph_plan(&GraphPlanRecord {
                    id: "idle-plan".into(),
                    session_id: candidate.id.as_str().into(),
                    base_revision: 0,
                    candidate,
                    expires_at: 0,
                })
                .unwrap();
        }
        let storage = Storage::open(&path).unwrap();
        let journal_count: i64 = storage
            .connection
            .query_row("SELECT COUNT(*) FROM operation_journal", [], |row| {
                row.get(0)
            })
            .unwrap();
        let plan_count: i64 = storage
            .connection
            .query_row("SELECT COUNT(*) FROM graph_plans", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_count, 0);
        assert_eq!(plan_count, 0);
        drop(storage);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn session_history_keeps_prior_revisions_and_honors_limit() {
        let storage = Storage::open_memory().unwrap();
        let original = session();
        storage.save_session(&original).unwrap();
        let mut newer = original.clone();
        newer.revision = 4;
        newer.name = "newer".into();
        storage.save_session(&newer).unwrap();
        let history = storage.load_history(&original.id, 2).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], newer);
        assert_eq!(history[1], original);
        assert_eq!(storage.load_history(&original.id, 1).unwrap(), vec![newer]);
        assert_eq!(
            storage
                .load_history_before(&original.id, Some(4), 2)
                .unwrap(),
            vec![original]
        );
    }

    #[test]
    fn import_validates_before_writing_and_export_round_trips() {
        let storage = Storage::open_memory().unwrap();
        let original = session();
        let document = serde_json::to_string(&original).unwrap();
        assert_eq!(storage.import_session(&document).unwrap(), original);
        assert_eq!(
            storage.export_session(&original.id).unwrap(),
            Some(document.clone())
        );

        let invalid = document.replace("\"nodes\":[", "\"nodes\":[{");
        assert!(matches!(
            storage.import_session(&invalid),
            Err(StorageError::Json(_)) | Err(StorageError::InvalidSession(_))
        ));
        assert_eq!(
            storage.load_session(&EntityId::new("missing")).unwrap(),
            None
        );
    }

    #[test]
    fn bundle_import_stages_a_valid_session_before_commit() {
        let suffix = format!("audiorouter-bundle-valid-{}", std::process::id());
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        let document = serde_json::to_vec(&session()).unwrap();
        write_bundle(
            &bundle,
            &[
                ("manifest.json", &bundle_manifest()),
                ("session.json", &document),
            ],
        );
        let storage = Storage::open_memory().unwrap();
        let imported = storage.import_bundle(&bundle, &staging).unwrap();
        assert_eq!(imported, session());
        assert_eq!(
            storage.load_session(&EntityId::new("session")).unwrap(),
            Some(session())
        );
        drop(storage);
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn bundle_import_rejects_unknown_required_node_type_before_commit() {
        let suffix = format!("audiorouter-bundle-node-type-{}", std::process::id());
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        let manifest = serde_json::to_vec(&serde_json::json!({
            "format": "audiorouter.session",
            "schemaVersion": 1,
            "graphPath": "session.json",
            "assets": [],
            "requiredNodeTypes": [{"type": "future-node", "version": 1}]
        }))
        .unwrap();
        let document = serde_json::to_vec(&session()).unwrap();
        write_bundle(
            &bundle,
            &[("manifest.json", &manifest), ("session.json", &document)],
        );
        let storage = Storage::open_memory().unwrap();
        let error = storage.import_bundle(&bundle, &staging);
        assert!(
            matches!(error, Err(StorageError::InvalidBundle(message)) if message.contains("required node type"))
        );
        assert!(storage
            .load_session(&EntityId::new("session"))
            .unwrap()
            .is_none());
        assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 0);
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn bundle_export_round_trips_and_never_overwrites() {
        let suffix = format!("audiorouter-bundle-export-{}", std::process::id());
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        let original = session();
        let storage = Storage::open_memory().unwrap();
        storage.save_session(&original).unwrap();
        storage.export_bundle(&original.id, &bundle).unwrap();
        assert!(matches!(
            storage.export_bundle(&original.id, &bundle),
            Err(StorageError::InvalidBackupPath(_))
        ));
        let imported_storage = Storage::open_memory().unwrap();
        assert_eq!(
            imported_storage.import_bundle(&bundle, &staging).unwrap(),
            original
        );
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn bundle_import_rejects_traversal_without_writing_outside_staging() {
        let suffix = format!("audiorouter-bundle-traversal-{}", std::process::id());
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let outside = std::env::temp_dir().join(format!("{suffix}-outside.txt"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_file(&outside);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        write_bundle(&bundle, &[("../outside.txt", b"must not escape")]);
        let error = Storage::open_memory()
            .unwrap()
            .import_bundle(&bundle, &staging);
        assert!(matches!(error, Err(StorageError::InvalidBundle(_))));
        assert!(!outside.exists());
        assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 0);
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn bundle_import_rejects_oversized_assets_before_extraction() {
        let suffix = format!("audiorouter-bundle-size-{}", std::process::id());
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        let oversized = vec![0u8; MAX_BUNDLE_ASSET_BYTES as usize + 1];
        write_bundle(&bundle, &[("asset.bin", &oversized)]);
        let error = Storage::open_memory()
            .unwrap()
            .import_bundle(&bundle, &staging);
        assert!(matches!(error, Err(StorageError::DocumentTooLarge { .. })));
        assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 0);
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn bundle_import_rejects_asset_hash_mismatch_before_commit() {
        let suffix = format!("audiorouter-bundle-hash-{}", std::process::id());
        let bundle = std::env::temp_dir().join(format!("{suffix}.audiorouter"));
        let staging = std::env::temp_dir().join(format!("{suffix}-staging"));
        let _ = std::fs::remove_file(&bundle);
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir(&staging).unwrap();
        let manifest = serde_json::to_vec(&serde_json::json!({
            "format": "audiorouter.session",
            "schemaVersion": 1,
            "graphPath": "session.json",
            "assets": [{"path": "state.bin", "size": 5, "sha256": "00".repeat(32)}]
        }))
        .unwrap();
        let document = serde_json::to_vec(&session()).unwrap();
        write_bundle(
            &bundle,
            &[
                ("manifest.json", &manifest),
                ("session.json", &document),
                ("state.bin", b"state"),
            ],
        );
        let storage = Storage::open_memory().unwrap();
        let error = storage.import_bundle(&bundle, &staging);
        assert!(
            matches!(error, Err(StorageError::InvalidBundle(message)) if message.contains("hash mismatch"))
        );
        assert!(storage
            .load_session(&EntityId::new("session"))
            .unwrap()
            .is_none());
        let _ = std::fs::remove_file(bundle);
        let _ = std::fs::remove_dir_all(staging);
    }

    #[test]
    fn online_backup_round_trips_a_live_database() {
        let suffix = format!("audiorouter-storage-backup-{}", std::process::id());
        let source = std::env::temp_dir().join(format!("{suffix}-source.sqlite"));
        let destination = std::env::temp_dir().join(format!("{suffix}-destination.sqlite"));
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&destination);
        let storage = Storage::open(&source).unwrap();
        let original = session();
        storage.save_session(&original).unwrap();
        storage.backup_to(&destination).unwrap();
        let backup = Storage::open(&destination).unwrap();
        assert_eq!(backup.load_session(&original.id).unwrap(), Some(original));
        drop(backup);
        drop(storage);
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(destination);
    }

    #[test]
    fn restore_backup_requires_new_destination_and_round_trips_data() {
        let suffix = format!("audiorouter-storage-restore-{}", std::process::id());
        let source = std::env::temp_dir().join(format!("{suffix}-source.sqlite"));
        let backup = std::env::temp_dir().join(format!("{suffix}-backup.sqlite"));
        let restored = std::env::temp_dir().join(format!("{suffix}-restored.sqlite"));
        for path in [&source, &backup, &restored] {
            let _ = std::fs::remove_file(path);
        }
        let storage = Storage::open(&source).unwrap();
        let original = session();
        storage.save_session(&original).unwrap();
        storage.backup_to(&backup).unwrap();
        Storage::restore_backup(&backup, &restored).unwrap();
        let restored_storage = Storage::open(&restored).unwrap();
        assert_eq!(
            restored_storage.load_session(&original.id).unwrap(),
            Some(original)
        );
        assert!(matches!(
            Storage::restore_backup(&backup, &restored),
            Err(StorageError::InvalidBackupPath(_))
        ));
        drop(restored_storage);
        drop(storage);
        for path in [&source, &backup, &restored] {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn backup_rejects_relative_missing_parent_and_live_database_targets() {
        let suffix = format!("audiorouter-storage-policy-{}", std::process::id());
        let source = std::env::temp_dir().join(format!("{suffix}-source.sqlite"));
        let missing = std::env::temp_dir()
            .join(format!("{suffix}-missing"))
            .join("backup.sqlite");
        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_dir_all(missing.parent().unwrap());
        let storage = Storage::open(&source).unwrap();
        assert!(matches!(
            storage.backup_to("relative.sqlite"),
            Err(StorageError::InvalidBackupPath(_))
        ));
        assert!(matches!(
            storage.backup_to(&missing),
            Err(StorageError::InvalidBackupPath(_))
        ));
        assert!(matches!(
            storage.backup_to(&source),
            Err(StorageError::InvalidBackupPath(_))
        ));
        drop(storage);
        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn backup_never_overwrites_an_existing_recovery_copy() {
        let suffix = format!("audiorouter-storage-backup-existing-{}", std::process::id());
        let source = std::env::temp_dir().join(format!("{suffix}-source.sqlite"));
        let destination = std::env::temp_dir().join(format!("{suffix}-destination.sqlite"));
        for path in [&source, &destination] {
            let _ = std::fs::remove_file(path);
        }
        let storage = Storage::open(&source).unwrap();
        storage.save_session(&session()).unwrap();
        std::fs::write(&destination, b"preserve this recovery copy").unwrap();
        assert!(matches!(
            storage.backup_to(&destination),
            Err(StorageError::InvalidBackupPath(message)) if message.contains("must not already exist")
        ));
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"preserve this recovery copy"
        );
        std::fs::remove_file(&destination).unwrap();
        let dangling_target = std::env::temp_dir().join(format!("{suffix}-missing.sqlite"));
        let _ = std::fs::remove_file(&dangling_target);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_file(&dangling_target, &destination);
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&dangling_target, &destination);
        if link_result.is_ok() {
            assert!(matches!(
                storage.backup_to(&destination),
                Err(StorageError::InvalidBackupPath(message))
                    if message.contains("symbolic link")
            ));
            std::fs::remove_file(&destination).unwrap();
        }
        drop(storage);
        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(dangling_target);
        let _ = std::fs::remove_file(destination);
    }

    #[test]
    fn recovery_retention_keeps_newest_daily_and_all_pre_migration_backups() {
        let directory = std::env::temp_dir().join(format!(
            "audiorouter-storage-retention-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory).unwrap();
        for index in 1..=12 {
            std::fs::write(
                directory.join(format!("audiorouter-backup-202609{:02}.sqlite", index)),
                [index as u8],
            )
            .unwrap();
        }
        let pre_migration = directory.join("audiorouter-pre-migration-20260901.sqlite");
        let unrelated = directory.join("notes.txt");
        std::fs::write(&pre_migration, b"keep").unwrap();
        std::fs::write(&unrelated, b"keep").unwrap();

        let removed = Storage::prune_recovery_backups(&directory).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(!directory
            .join("audiorouter-backup-20260901.sqlite")
            .exists());
        assert!(!directory
            .join("audiorouter-backup-20260902.sqlite")
            .exists());
        assert!(directory
            .join("audiorouter-backup-20260903.sqlite")
            .exists());
        assert!(directory
            .join("audiorouter-backup-20260912.sqlite")
            .exists());
        assert!(pre_migration.exists());
        assert!(unrelated.exists());
        assert!(matches!(
            Storage::prune_recovery_backups("relative"),
            Err(StorageError::InvalidBackupPath(message))
                if message.contains("must be absolute")
        ));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn live_wav_worker_persists_each_committed_boundary_to_storage() {
        let storage = Storage::open_memory().unwrap();
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        let mut recorder = WavRecorder::new(writer);
        recorder.arm().unwrap();
        recorder.start(100).unwrap();
        let queue = RecordingQueue::new(2).unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 100,
                samples: vec![0.25, -0.25],
            })
            .unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 102,
                samples: vec![0.5],
            })
            .unwrap();

        let drained = recorder
            .drain_queue_with_checkpoint(&queue, 2, |checkpoint| {
                storage
                    .save_recording_checkpoint("live-worker", checkpoint)
                    .map_err(|_| RecordingError::InvalidWav)
            })
            .unwrap();
        assert_eq!(drained, 2);
        assert_eq!(
            storage
                .load_recording_checkpoint("live-worker")
                .unwrap()
                .unwrap()
                .last_frame,
            Some(103)
        );
    }

    #[test]
    fn journal_failure_stages_leave_no_partial_snapshot() {
        for stage in [
            JournalFailureStage::BeforeHistory,
            JournalFailureStage::AfterHistory,
            JournalFailureStage::AfterCurrent,
            JournalFailureStage::AfterJournal,
        ] {
            let storage = Storage::open_memory().unwrap();
            let original = session();
            storage.save_session(&original).unwrap();
            let mut changed = original.clone();
            changed.revision = 4;
            changed.name = "partial-must-not-survive".into();
            assert!(storage
                .save_session_with_journal(
                    &changed,
                    "crash-op",
                    "graph.commit",
                    "{\"revision\":4}",
                    Some(stage),
                )
                .is_err());
            assert_eq!(
                storage.load_session(&original.id).unwrap(),
                Some(original.clone())
            );
            assert_eq!(
                storage.load_history(&original.id, 10).unwrap(),
                vec![original]
            );
            assert_eq!(storage.journal_result("crash-op").unwrap(), None);
        }
    }

    #[test]
    fn recording_checkpoint_persists_and_surfaces_corruption() {
        let storage = Storage::open_memory().unwrap();
        let mut recorder = audiorouter_recording::RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(10).unwrap();
        recorder.pause(20).unwrap();
        let checkpoint = recorder.checkpoint();
        storage
            .save_recording(&RecordingRecord {
                id: "recording".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: "C:\\recordings\\recovery.wav".into(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 48_000,
                frames: 0,
                file_bytes: 0,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "recording".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            })
            .unwrap();
        storage
            .save_recording_checkpoint("recording", &checkpoint)
            .unwrap();
        assert_eq!(
            storage.load_recording_checkpoint("recording").unwrap(),
            Some(checkpoint.clone())
        );
        storage
            .connection
            .execute(
                "UPDATE recording_checkpoints SET checkpoint = ?1 WHERE recording_id = 'recording'",
                params![r#"{"version":1,"state":"Paused"}"#],
            )
            .unwrap();
        assert!(matches!(
            storage.load_recording_checkpoint("recording"),
            Err(StorageError::InvalidRecording(_))
        ));
        assert!(storage.remove_recording_entry("recording").unwrap());
        assert_eq!(
            storage.load_recording_checkpoint("recording").unwrap(),
            None
        );
        assert!(!storage.remove_recording_entry("recording").unwrap());
    }

    #[test]
    fn client_enrollment_is_explicit_and_revocation_is_auditable() {
        let storage = Storage::open_memory().unwrap();
        assert_eq!(storage.load_client_enrollment("client").unwrap(), None);
        storage.save_client_enrollment("client", "editor").unwrap();
        assert_eq!(
            storage.load_client_enrollment("client").unwrap(),
            Some(("editor".into(), false))
        );
        assert!(storage.revoke_client_enrollment("client").unwrap());
        assert!(!storage.revoke_client_enrollment("client").unwrap());
        assert_eq!(
            storage.load_client_enrollment("client").unwrap(),
            Some(("editor".into(), true))
        );
        storage
            .save_client_enrollment("client", "observer")
            .unwrap();
        assert_eq!(
            storage.load_client_enrollment("client").unwrap(),
            Some(("observer".into(), false))
        );
    }

    #[test]
    fn recording_library_rows_persist_and_remove_without_file_action() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-recording-storage-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let storage = Storage::open(&path).unwrap();
        let recording = RecordingRecord {
            id: "rec-1".into(),
            session_id: "session".into(),
            recorder_id: "voice".into(),
            path: "C:\\Recordings\\voice.wav".into(),
            format: "wav".into(),
            channels: 2,
            sample_rate: 48_000,
            frames: 480,
            file_bytes: 1_964,
            start_time: "2026-09-05T12:00:00Z".into(),
            state: "completed".into(),
            missing: false,
            title: None,
            artist: None,
            comment: None,
        };
        storage.save_recording(&recording).unwrap();
        let mut second = recording.clone();
        second.id = "rec-2".into();
        second.start_time = "2026-09-05T12:01:00Z".into();
        second.missing = true;
        storage.save_recording(&second).unwrap();
        assert_eq!(storage.list_recordings(Some("session")).unwrap().len(), 2);
        assert!(storage
            .update_recording_metadata("rec-1", Some("Take 1"), Some("User"), None)
            .unwrap());
        assert!(matches!(
            storage.update_recording_metadata("rec-1", Some("bad\nvalue"), None, None),
            Err(StorageError::InvalidRecording(_))
        ));
        drop(storage);
        let reopened = Storage::open(&path).unwrap();
        assert_eq!(
            reopened.list_recordings(Some("session")).unwrap()[0].title,
            Some("Take 1".into())
        );
        assert!(reopened.remove_recording_entry("rec-1").unwrap());
        assert!(!reopened.remove_recording_entry("rec-1").unwrap());
        assert_eq!(reopened.list_recordings(None).unwrap().len(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recording_rename_updates_path_only_after_safe_same_directory_move() {
        let root = std::env::temp_dir();
        let suffix = format!("audiorouter-recording-rename-{}", std::process::id());
        let database = root.join(format!("{suffix}.sqlite"));
        let source = root.join(format!("{suffix}.wav"));
        let destination = root.join(format!("{suffix}-renamed.wav"));
        for path in [&database, &source, &destination] {
            let _ = std::fs::remove_file(path);
        }
        std::fs::write(&source, b"recording placeholder").unwrap();
        let storage = Storage::open(&database).unwrap();
        storage
            .save_recording(&RecordingRecord {
                id: "rename-1".into(),
                session_id: "session".into(),
                recorder_id: "voice".into(),
                path: source.to_string_lossy().into_owned(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 48_000,
                frames: 0,
                file_bytes: 20,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "complete".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            })
            .unwrap();
        assert!(storage
            .rename_recording("rename-1", &destination.to_string_lossy())
            .unwrap());
        assert!(!source.exists());
        assert!(destination.exists());
        assert_eq!(
            storage.get_recording("rename-1").unwrap().unwrap().path,
            destination.to_string_lossy()
        );
        let _ = std::fs::remove_file(database);
        let _ = std::fs::remove_file(destination);
    }

    #[test]
    fn plugin_state_metadata_persists_and_removal_keeps_asset_path_untouched() {
        let storage = Storage::open_memory().unwrap();
        let state = PluginStateRecord {
            id: "state-1".into(),
            plugin_id: "plugin-1".into(),
            plugin_sha256: "a".repeat(64),
            version: 1,
            path: "C:\\AudioRouter\\state\\state-1.bin".into(),
            state_sha256: "b".repeat(64),
            size_bytes: 128,
        };
        storage.save_plugin_state(&state).unwrap();
        assert_eq!(
            storage.list_plugin_states(Some("plugin-1")).unwrap(),
            vec![state]
        );
        assert!(storage.remove_plugin_state("state-1").unwrap());
        assert!(storage.list_plugin_states(None).unwrap().is_empty());
    }

    #[test]
    fn plugin_state_rejects_relative_asset_paths() {
        let storage = Storage::open_memory().unwrap();
        let state = PluginStateRecord {
            id: "state-relative".into(),
            plugin_id: "plugin-1".into(),
            plugin_sha256: "a".repeat(64),
            version: 1,
            path: "relative/state.bin".into(),
            state_sha256: "b".repeat(64),
            size_bytes: 1,
        };
        assert!(matches!(
            storage.save_plugin_state(&state),
            Err(StorageError::InvalidPluginState(_))
        ));
    }
}
