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

    #[test]
    fn note_tags_has_verified_column() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check verified column exists and is INTEGER
        let mut stmt = db
            .connection()
            .prepare("PRAGMA table_info(note_tags)")
            .unwrap();

        let columns: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // name
                    row.get::<_, String>(2)?, // type
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // Check verified column exists and is INTEGER
        let verified_column = columns
            .iter()
            .find(|(name, _)| name == "verified")
            .expect("verified column should exist");

        assert_eq!(verified_column.1, "INTEGER");

        // Verify default value by inserting a row without specifying verified
        db.connection()
            .execute(
                "INSERT INTO notes (id, content) VALUES (1, 'test note')",
                [],
            )
            .unwrap();
        db.connection()
            .execute("INSERT INTO tags (id, name) VALUES (1, 'test tag')", [])
            .unwrap();
        db.connection()
            .execute("INSERT INTO note_tags (note_id, tag_id) VALUES (1, 1)", [])
            .unwrap();

        let verified: i32 = db
            .connection()
            .query_row(
                "SELECT verified FROM note_tags WHERE note_id = 1 AND tag_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(verified, 0);
    }

    #[test]
    fn note_tags_has_model_version_column() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check model_version column exists
        let mut stmt = db
            .connection()
            .prepare("PRAGMA table_info(note_tags)")
            .unwrap();

        let columns: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // name
                    row.get::<_, String>(2)?, // type
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // Check model_version column exists and is TEXT (nullable)
        let model_version_column = columns
            .iter()
            .find(|(name, _)| name == "model_version")
            .expect("model_version column should exist");

        assert_eq!(model_version_column.1, "TEXT");

        // Verify NULL is allowed by inserting without model_version
        db.connection()
            .execute(
                "INSERT INTO notes (id, content) VALUES (1, 'test note')",
                [],
            )
            .unwrap();
        db.connection()
            .execute("INSERT INTO tags (id, name) VALUES (1, 'test tag')", [])
            .unwrap();
        db.connection()
            .execute("INSERT INTO note_tags (note_id, tag_id) VALUES (1, 1)", [])
            .unwrap();

        let model_version: Option<String> = db
            .connection()
            .query_row(
                "SELECT model_version FROM note_tags WHERE note_id = 1 AND tag_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(model_version, None);
    }

    #[test]
    fn note_tags_created_at_is_integer() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check created_at column type
        let mut stmt = db
            .connection()
            .prepare("PRAGMA table_info(note_tags)")
            .unwrap();

        let columns: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // name
                    row.get::<_, String>(2)?, // type
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // Check created_at column is INTEGER
        let created_at_column = columns
            .iter()
            .find(|(name, _)| name == "created_at")
            .expect("created_at column should exist");

        assert_eq!(created_at_column.1, "INTEGER");
    }

    #[test]
    fn idx_tag_aliases_canonical_exists() {
        let db = Database::in_memory().unwrap();

        let indexes: Vec<String> = db
            .connection()
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name = 'idx_tag_aliases_canonical'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            indexes.contains(&"idx_tag_aliases_canonical".to_string()),
            "idx_tag_aliases_canonical index should exist"
        );
    }

    #[test]
    fn tag_aliases_insert_with_all_metadata() {
        let db = Database::in_memory().unwrap();

        // Insert a canonical tag first
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (1, 'machine-learning')",
                [],
            )
            .unwrap();

        // Insert alias with all metadata columns
        let timestamp = 1735142400; // 2024-12-25 12:00:00 UTC
        db.connection()
            .execute(
                "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at, model_version)
                 VALUES ('ml', 1, 'llm', 0.85, ?1, 'deepseek-r1:8b')",
                [timestamp],
            )
            .unwrap();

        // Verify all columns are stored correctly
        let (alias, tag_id, source, confidence, created, model): (
            String,
            i64,
            String,
            f64,
            i64,
            String,
        ) = db
            .connection()
            .query_row(
                "SELECT alias, canonical_tag_id, source, confidence, created_at, model_version
                 FROM tag_aliases WHERE alias = 'ml'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(alias, "ml");
        assert_eq!(tag_id, 1);
        assert_eq!(source, "llm");
        assert_eq!(confidence, 0.85);
        assert_eq!(created, timestamp);
        assert_eq!(model, "deepseek-r1:8b");
    }

    #[test]
    fn tag_aliases_case_insensitive_lookup() {
        let db = Database::in_memory().unwrap();

        // Insert canonical tag
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (1, 'machine-learning')",
                [],
            )
            .unwrap();

        // Insert lowercase alias
        let timestamp = 1735142400;
        db.connection()
            .execute(
                "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at)
                 VALUES ('ml', 1, 'user', 1.0, ?1)",
                [timestamp],
            )
            .unwrap();

        // Lookup with different case variations
        for variant in ["ml", "ML", "Ml", "mL"] {
            let tag_id: i64 = db
                .connection()
                .query_row(
                    "SELECT canonical_tag_id FROM tag_aliases WHERE alias = ?1",
                    [variant],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(tag_id, 1, "Failed for variant: {}", variant);
        }
    }

    #[test]
    fn tag_aliases_cascade_delete_on_canonical_tag_removal() {
        let db = Database::in_memory().unwrap();

        // Insert canonical tag
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (1, 'machine-learning')",
                [],
            )
            .unwrap();

        // Insert multiple aliases pointing to the same canonical tag
        let timestamp = 1735142400;
        for alias in ["ml", "ML-tag", "machine_learning"] {
            db.connection()
                .execute(
                    "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at)
                     VALUES (?1, 1, 'user', 1.0, ?2)",
                    rusqlite::params![alias, timestamp],
                )
                .unwrap();
        }

        // Verify aliases exist
        let count: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM tag_aliases", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);

        // Delete canonical tag
        db.connection()
            .execute("DELETE FROM tags WHERE id = 1", [])
            .unwrap();

        // Verify all aliases were CASCADE deleted
        let count_after: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM tag_aliases", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count_after, 0, "Aliases should be CASCADE deleted");
    }

    #[test]
    fn tag_aliases_index_reverse_lookup_performance() {
        let db = Database::in_memory().unwrap();

        // Insert canonical tags
        for i in 1..=5 {
            db.connection()
                .execute(
                    "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                    rusqlite::params![i, format!("tag-{}", i)],
                )
                .unwrap();
        }

        // Insert aliases for multiple tags
        let timestamp = 1735142400;
        for tag_id in 1..=5 {
            for j in 0..3 {
                db.connection()
                    .execute(
                        "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at)
                         VALUES (?1, ?2, 'user', 1.0, ?3)",
                        rusqlite::params![format!("alias-{}-{}", tag_id, j), tag_id, timestamp],
                    )
                    .unwrap();
            }
        }

        // Query aliases by canonical_tag_id (uses index)
        let aliases: Vec<String> = db
            .connection()
            .prepare("SELECT alias FROM tag_aliases WHERE canonical_tag_id = 3 ORDER BY alias")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(aliases.len(), 3);
        assert_eq!(aliases[0], "alias-3-0");
        assert_eq!(aliases[1], "alias-3-1");
        assert_eq!(aliases[2], "alias-3-2");

        // Verify index is being used (query plan should contain idx_tag_aliases_canonical)
        let query_plan: String = db
            .connection()
            .query_row(
                "EXPLAIN QUERY PLAN SELECT alias FROM tag_aliases WHERE canonical_tag_id = 3",
                [],
                |row| row.get::<_, String>(3), // detail column
            )
            .unwrap();

        assert!(
            query_plan.contains("idx_tag_aliases_canonical"),
            "Index should be used for canonical_tag_id lookup. Query plan: {}",
            query_plan
        );
    }

    #[test]
    fn tag_aliases_unique_alias_constraint() {
        let db = Database::in_memory().unwrap();

        // Insert canonical tags
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (1, 'machine-learning')",
                [],
            )
            .unwrap();
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (2, 'artificial-intelligence')",
                [],
            )
            .unwrap();

        // Insert alias pointing to first tag
        let timestamp = 1735142400;
        db.connection()
            .execute(
                "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at)
                 VALUES ('ml', 1, 'user', 1.0, ?1)",
                [timestamp],
            )
            .unwrap();

        // Attempt to insert duplicate alias (even with different canonical_tag_id)
        let result = db.connection().execute(
            "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at)
             VALUES ('ml', 2, 'user', 1.0, ?1)",
            [timestamp],
        );

        assert!(
            result.is_err(),
            "Duplicate alias should violate PRIMARY KEY constraint"
        );

        // Verify error is about uniqueness
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("UNIQUE") || error.to_string().contains("PRIMARY KEY"),
            "Error should be about uniqueness: {}",
            error
        );
    }

    #[test]
    fn tag_aliases_user_source_with_no_model_version() {
        let db = Database::in_memory().unwrap();

        // Insert canonical tag
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (1, 'machine-learning')",
                [],
            )
            .unwrap();

        // Insert user alias without model_version (NULL)
        let timestamp = 1735142400;
        db.connection()
            .execute(
                "INSERT INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at, model_version)
                 VALUES ('ml', 1, 'user', 1.0, ?1, NULL)",
                [timestamp],
            )
            .unwrap();

        // Verify model_version is NULL
        let model_version: Option<String> = db
            .connection()
            .query_row(
                "SELECT model_version FROM tag_aliases WHERE alias = 'ml'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(model_version, None);
    }
}
