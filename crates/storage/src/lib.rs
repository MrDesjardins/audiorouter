//! SQLite persistence boundary for M01.

use audiorouter_domain::{validate_session, EntityId, Session};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    InvalidSession(String),
    DocumentTooLarge { bytes: usize, maximum: usize },
    InvalidBackupPath(String),
}

pub const MAX_SESSION_DOCUMENT_BYTES: usize = 1024 * 1024;

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

    pub fn load_history(&self, id: &EntityId, limit: usize) -> Result<Vec<Session>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT document FROM session_history WHERE session_id = ?1
             ORDER BY revision DESC LIMIT ?2",
        )?;
        let rows = statement.query_map(params![id.as_str(), limit as i64], |row| {
            row.get::<_, String>(0)
        })?;
        rows.map(|row| {
            let document = row?;
            serde_json::from_str(&document).map_err(StorageError::Json)
        })
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

    pub fn export_session(&self, id: &EntityId) -> Result<Option<String>, StorageError> {
        self.load_session(id)?
            .map(|session| serde_json::to_string(&session).map_err(StorageError::Json))
            .transpose()
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

    pub fn journal_commit(
        &self,
        key: &str,
        operation: &str,
        result: &str,
        revision: u64,
    ) -> Result<bool, StorageError> {
        let inserted = self.connection.execute(
            "INSERT OR IGNORE INTO operation_journal(idempotency_key, operation, result, committed_revision) VALUES (?1, ?2, ?3, ?4)",
            params![key, operation, result, revision as i64],
        )?;
        Ok(inserted == 1)
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
        let document = serde_json::to_string(session)?;
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
            "INSERT OR IGNORE INTO operation_journal(idempotency_key, operation, result, committed_revision) VALUES (?1, ?2, ?3, ?4)",
            params![key, operation, result, session.revision as i64],
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
