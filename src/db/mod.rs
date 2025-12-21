mod schema;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use schema::INITIAL_SCHEMA;

/// Database wrapper providing connection management and schema initialization.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens an in-memory SQLite database.
    ///
    /// Automatically initializes the schema on connection open.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Opens a file-based SQLite database at the given path.
    ///
    /// Creates the database file if it does not exist.
    /// Automatically initializes the schema on connection open.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Initializes the database schema.
    ///
    /// Executes all schema statements in a single transaction.
    /// Uses IF NOT EXISTS for idempotent execution.
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute("PRAGMA foreign_keys = ON", [])?;
        self.conn.execute_batch(INITIAL_SCHEMA)?;
        Ok(())
    }

    /// Returns a reference to the underlying connection.
    ///
    /// Useful for executing custom queries in tests or future CRUD operations.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn in_memory_opens_successfully() {
        let result = Database::in_memory();
        assert!(result.is_ok());
    }

    #[test]
    fn schema_tables_exist() {
        let db = Database::in_memory().unwrap();

        let tables: Vec<String> = db
            .connection()
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"notes".to_string()));
        assert!(tables.contains(&"tags".to_string()));
        assert!(tables.contains(&"note_tags".to_string()));
    }

    #[test]
    fn schema_indexes_exist() {
        let db = Database::in_memory().unwrap();

        let indexes: Vec<String> = db
            .connection()
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(indexes.contains(&"idx_notes_created".to_string()));
        assert!(indexes.contains(&"idx_note_tags_note".to_string()));
        assert!(indexes.contains(&"idx_note_tags_tag".to_string()));
    }

    #[test]
    fn foreign_keys_enabled() {
        let db = Database::in_memory().unwrap();

        let fk_enabled: i32 = db
            .connection()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();

        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn open_creates_database_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let result = Database::open(&db_path);
        assert!(result.is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn reopen_is_idempotent() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Open and close first time
        {
            let db = Database::open(&db_path).unwrap();
            db.connection()
                .execute("INSERT INTO notes (content) VALUES ('test')", [])
                .unwrap();
        }

        // Reopen - schema initialization should not fail
        let db2 = Database::open(&db_path);
        assert!(db2.is_ok());

        // Verify data persisted
        let count: i32 = db2
            .unwrap()
            .connection()
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
