//! SQLite persistence boundary for M01.

use audiorouter_domain::{node_registry, validate_session, EntityId, Session};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use zip::ZipArchive;

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    InvalidSession(String),
    InvalidBundle(String),
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
        storage.migrate()?;
        Ok(storage)
    }

    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let storage = Self {
            connection: Connection::open(path)?,
            database_path: Some(path.to_path_buf()),
        };
        storage.migrate()?;
        Ok(storage)
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
        if destination.exists()
            && std::fs::symlink_metadata(destination)?
                .file_type()
                .is_symlink()
        {
            return Err(StorageError::InvalidBackupPath(
                "backup destination cannot be a symbolic link".into(),
            ));
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
        if !source.is_file() || std::fs::symlink_metadata(source)?.file_type().is_symlink() {
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
        if destination.exists() {
            return Err(StorageError::InvalidBackupPath(
                "restore destination must not already exist".into(),
            ));
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
             CREATE TABLE IF NOT EXISTS client_enrollments (
                 client_id TEXT PRIMARY KEY,
                 role TEXT NOT NULL CHECK(role IN ('observer', 'editor', 'operator')),
                 revoked INTEGER NOT NULL DEFAULT 0 CHECK(revoked IN (0, 1)),
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 revoked_at TEXT
             );
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
        transaction.commit()?;
        Ok(changed != 0)
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
        if !bundle.is_file() || std::fs::symlink_metadata(bundle)?.file_type().is_symlink() {
            return Err(StorageError::InvalidBundle(
                "bundle must be a regular non-symlink file".into(),
            ));
        }
        if !staging_root.is_dir()
            || std::fs::symlink_metadata(staging_root)?
                .file_type()
                .is_symlink()
        {
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
    use std::io::Write;

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
    fn deleting_session_removes_current_and_history_rows() {
        let storage = Storage::open_memory().unwrap();
        let original = session();
        storage.save_session(&original).unwrap();
        assert!(storage.delete_session(&original.id).unwrap());
        assert!(!storage.delete_session(&original.id).unwrap());
        assert!(storage.load_session(&original.id).unwrap().is_none());
        assert!(storage.load_history(&original.id, 10).unwrap().is_empty());
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
}
