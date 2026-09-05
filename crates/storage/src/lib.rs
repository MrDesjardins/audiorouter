//! SQLite persistence boundary for M01.

use audiorouter_domain::{EntityId, Session};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug)]
pub enum StorageError {
    Sql(rusqlite::Error),
    Json(serde_json::Error),
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

pub struct Storage {
    connection: Connection,
}

impl Storage {
    pub fn open_memory() -> Result<Self, StorageError> {
        let storage = Self {
            connection: Connection::open_in_memory()?,
        };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, StorageError> {
        let storage = Self {
            connection: Connection::open(path)?,
        };
        storage.migrate()?;
        Ok(storage)
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
             CREATE TABLE IF NOT EXISTS operation_journal (
                 idempotency_key TEXT PRIMARY KEY,
                 operation TEXT NOT NULL,
                 result TEXT NOT NULL,
                 committed_revision INTEGER NOT NULL,
                 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );
             INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);",
        )?;
        Ok(())
    }

    pub fn save_session(&self, session: &Session) -> Result<(), StorageError> {
        let document = serde_json::to_string(session)?;
        self.connection.execute(
            "INSERT INTO sessions(id, revision, document) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET revision=excluded.revision, document=excluded.document",
            params![session.id.as_str(), session.revision as i64, document],
        )?;
        Ok(())
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
}
