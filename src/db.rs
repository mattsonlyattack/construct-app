mod schema;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use schema::{FTS_TABLE_CREATION, FTS_TRIGGERS, INITIAL_SCHEMA, MIGRATIONS};

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
    /// Runs migrations for column additions, ignoring "duplicate column" errors.
    /// Creates FTS5 virtual table and triggers if they don't exist.
    /// Populates FTS index from existing notes.
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute("PRAGMA foreign_keys = ON", [])?;
        self.conn.execute_batch(INITIAL_SCHEMA)?;

        // Execute migrations line by line, ignoring "duplicate column" errors
        // This allows idempotent execution on both fresh and existing databases
        for statement in MIGRATIONS.lines() {
            let trimmed = statement.trim();
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }

            // Execute ALTER TABLE statements, ignoring duplicate column errors
            match self.conn.execute(trimmed, []) {
                Ok(_) => {}
                Err(rusqlite::Error::SqliteFailure(err, msg)) => {
                    // Check if this is a "duplicate column name" error
                    // SQLite returns SQLITE_ERROR (code 1) for duplicate columns
                    let is_duplicate_column = msg
                        .as_ref()
                        .map(|s| s.contains("duplicate column"))
                        .unwrap_or(false);

                    if !is_duplicate_column {
                        // Not a duplicate column error, propagate it
                        return Err(rusqlite::Error::SqliteFailure(err, msg).into());
                    }
                    // Otherwise, silently ignore duplicate column errors
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Initialize FTS5 virtual table and triggers
        self.initialize_fts()?;

        Ok(())
    }

    /// Initializes FTS5 virtual table, triggers, and populates the index.
    ///
    /// FTS5 does NOT support IF NOT EXISTS, so we check sqlite_master first.
    /// After creating the table and triggers, we populate the index from existing notes.
    fn initialize_fts(&self) -> Result<()> {
        // Check if FTS table already exists
        let fts_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes_fts')",
            [],
            |row| row.get(0),
        )?;

        if !fts_exists {
            // Create FTS virtual table
            self.conn.execute_batch(FTS_TABLE_CREATION)?;
        }

        // Create triggers (idempotent with IF NOT EXISTS)
        self.conn.execute_batch(FTS_TRIGGERS)?;

        // Populate/rebuild FTS index from existing notes
        // This handles both fresh creation and existing databases where FTS might be stale
        self.populate_fts_index()?;

        Ok(())
    }

    /// Populates the FTS index from existing notes and tags.
    ///
    /// Clears the existing FTS index and rebuilds it from the notes table.
    /// This operation is idempotent and safe to run on every database open.
    fn populate_fts_index(&self) -> Result<()> {
        // Clear existing FTS index
        self.conn.execute("DELETE FROM notes_fts", [])?;

        // Rebuild from notes and tags
        self.conn.execute(
            "INSERT INTO notes_fts (note_id, content, content_enhanced, tags)
             SELECT
                 n.id,
                 n.content,
                 n.content_enhanced,
                 (SELECT GROUP_CONCAT(t.name, ' ')
                  FROM note_tags nt
                  JOIN tags t ON nt.tag_id = t.id
                  WHERE nt.note_id = n.id)
             FROM notes n",
            [],
        )?;

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
    use rusqlite::OptionalExtension;
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

    #[test]
    fn notes_has_content_enhanced_column() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check content_enhanced column exists
        let mut stmt = db.connection().prepare("PRAGMA table_info(notes)").unwrap();

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

        // Check content_enhanced column exists and is TEXT (nullable)
        let content_enhanced_column = columns
            .iter()
            .find(|(name, _)| name == "content_enhanced")
            .expect("content_enhanced column should exist");

        assert_eq!(content_enhanced_column.1, "TEXT");

        // Verify NULL is allowed by inserting without content_enhanced
        db.connection()
            .execute(
                "INSERT INTO notes (id, content) VALUES (1, 'test note')",
                [],
            )
            .unwrap();

        let content_enhanced: Option<String> = db
            .connection()
            .query_row(
                "SELECT content_enhanced FROM notes WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(content_enhanced, None);
    }

    #[test]
    fn notes_has_enhanced_at_column() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check enhanced_at column exists
        let mut stmt = db.connection().prepare("PRAGMA table_info(notes)").unwrap();

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

        // Check enhanced_at column exists and is INTEGER (nullable)
        let enhanced_at_column = columns
            .iter()
            .find(|(name, _)| name == "enhanced_at")
            .expect("enhanced_at column should exist");

        assert_eq!(enhanced_at_column.1, "INTEGER");

        // Verify NULL is allowed by inserting without enhanced_at
        db.connection()
            .execute(
                "INSERT INTO notes (id, content) VALUES (1, 'test note')",
                [],
            )
            .unwrap();

        let enhanced_at: Option<i64> = db
            .connection()
            .query_row("SELECT enhanced_at FROM notes WHERE id = 1", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(enhanced_at, None);
    }

    #[test]
    fn notes_has_enhancement_model_column() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check enhancement_model column exists
        let mut stmt = db.connection().prepare("PRAGMA table_info(notes)").unwrap();

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

        // Check enhancement_model column exists and is TEXT (nullable)
        let enhancement_model_column = columns
            .iter()
            .find(|(name, _)| name == "enhancement_model")
            .expect("enhancement_model column should exist");

        assert_eq!(enhancement_model_column.1, "TEXT");

        // Verify NULL is allowed by inserting without enhancement_model
        db.connection()
            .execute(
                "INSERT INTO notes (id, content) VALUES (1, 'test note')",
                [],
            )
            .unwrap();

        let enhancement_model: Option<String> = db
            .connection()
            .query_row(
                "SELECT enhancement_model FROM notes WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(enhancement_model, None);
    }

    #[test]
    fn notes_has_enhancement_confidence_column() {
        let db = Database::in_memory().unwrap();

        // Query table schema to check enhancement_confidence column exists
        let mut stmt = db.connection().prepare("PRAGMA table_info(notes)").unwrap();

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

        // Check enhancement_confidence column exists and is REAL (nullable)
        let enhancement_confidence_column = columns
            .iter()
            .find(|(name, _)| name == "enhancement_confidence")
            .expect("enhancement_confidence column should exist");

        assert_eq!(enhancement_confidence_column.1, "REAL");

        // Verify NULL is allowed by inserting without enhancement_confidence
        db.connection()
            .execute(
                "INSERT INTO notes (id, content) VALUES (1, 'test note')",
                [],
            )
            .unwrap();

        let enhancement_confidence: Option<f64> = db
            .connection()
            .query_row(
                "SELECT enhancement_confidence FROM notes WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(enhancement_confidence, None);
    }

    #[test]
    fn schema_migration_idempotent_on_existing_database() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create database with initial note
        {
            let db = Database::open(&db_path).unwrap();
            db.connection()
                .execute("INSERT INTO notes (content) VALUES ('existing note')", [])
                .unwrap();
        }

        // Reopen database - migrations should run without error
        let db2 = Database::open(&db_path).unwrap();

        // Verify enhancement columns exist
        let mut stmt = db2
            .connection()
            .prepare("PRAGMA table_info(notes)")
            .unwrap();

        let column_names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(column_names.contains(&"content_enhanced".to_string()));
        assert!(column_names.contains(&"enhanced_at".to_string()));
        assert!(column_names.contains(&"enhancement_model".to_string()));
        assert!(column_names.contains(&"enhancement_confidence".to_string()));

        // Verify existing data is preserved
        let content: String = db2
            .connection()
            .query_row("SELECT content FROM notes LIMIT 1", [], |row| row.get(0))
            .unwrap();

        assert_eq!(content, "existing note");

        // Reopen again - should be idempotent (no errors)
        let db3 = Database::open(&db_path);
        assert!(db3.is_ok(), "Schema migration should be idempotent");
    }

    #[test]
    fn fts5_virtual_table_created() {
        let db = Database::in_memory().unwrap();

        // Check that notes_fts virtual table exists
        let table_exists: bool = db
            .connection()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(table_exists, "notes_fts virtual table should exist");
    }

    #[test]
    fn fts5_virtual_table_creation_is_idempotent() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create database first time
        {
            let db = Database::open(&db_path).unwrap();
            let table_exists: bool = db
                .connection()
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes_fts')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(table_exists);
        }

        // Reopen - FTS table creation should be idempotent
        let db2 = Database::open(&db_path);
        assert!(db2.is_ok(), "FTS table creation should be idempotent");

        let table_exists: bool = db2
            .unwrap()
            .connection()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes_fts')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(table_exists);
    }

    #[test]
    fn fts_index_populated_on_database_open() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create database and insert notes before FTS exists
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
            conn.execute_batch(INITIAL_SCHEMA).unwrap();

            // Insert test notes
            conn.execute(
                "INSERT INTO notes (id, content) VALUES (1, 'rust programming')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO notes (id, content) VALUES (2, 'python scripting')",
                [],
            )
            .unwrap();

            // Insert tag
            conn.execute("INSERT INTO tags (id, name) VALUES (1, 'coding')", [])
                .unwrap();
            conn.execute("INSERT INTO note_tags (note_id, tag_id) VALUES (1, 1)", [])
                .unwrap();
        }

        // Reopen with Database wrapper - should populate FTS index
        let db = Database::open(&db_path).unwrap();

        // Verify FTS index contains the notes
        let count: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM notes_fts", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 2, "FTS index should contain both notes");

        // Verify search works
        let rust_note_id: i64 = db
            .connection()
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'rust'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(rust_note_id, 1);
    }

    #[test]
    fn fts_triggers_sync_on_note_insert() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();

        // Insert a new note
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (1, 'trigger test insert')",
            [],
        )
        .unwrap();

        // Verify FTS index was updated by trigger
        let note_id: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'trigger'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        assert_eq!(note_id, Some(1));
    }

    #[test]
    fn fts_triggers_sync_on_note_update() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();

        // Insert and then update a note
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (1, 'original content')",
            [],
        )
        .unwrap();

        conn.execute(
            "UPDATE notes SET content = 'updated content' WHERE id = 1",
            [],
        )
        .unwrap();

        // Verify FTS index reflects the update
        let matches_updated: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'updated'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        assert_eq!(matches_updated, Some(1));

        // Verify old content is not in FTS
        let matches_original: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'original'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        assert_eq!(matches_original, None);
    }

    #[test]
    fn fts_triggers_sync_on_note_delete() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();

        // Insert and then delete a note
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (1, 'to be deleted')",
            [],
        )
        .unwrap();

        // Verify it's in FTS
        let before_delete: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'deleted'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(before_delete, Some(1));

        // Delete the note
        conn.execute("DELETE FROM notes WHERE id = 1", []).unwrap();

        // Verify it's removed from FTS
        let after_delete: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'deleted'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();

        assert_eq!(after_delete, None);
    }

    #[test]
    fn fts_triggers_sync_on_note_tags_change() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();

        // Insert note and tags
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (1, 'test note')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO tags (id, name) VALUES (1, 'rust')", [])
            .unwrap();
        conn.execute("INSERT INTO tags (id, name) VALUES (2, 'python')", [])
            .unwrap();

        // Add first tag
        conn.execute("INSERT INTO note_tags (note_id, tag_id) VALUES (1, 1)", [])
            .unwrap();

        // Verify FTS contains the tag
        let with_rust: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'rust'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(with_rust, Some(1));

        // Add second tag
        conn.execute("INSERT INTO note_tags (note_id, tag_id) VALUES (1, 2)", [])
            .unwrap();

        // Verify both tags are in FTS
        let with_python: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'python'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(with_python, Some(1));

        // Remove first tag
        conn.execute("DELETE FROM note_tags WHERE note_id = 1 AND tag_id = 1", [])
            .unwrap();

        // Verify first tag is removed from FTS
        let rust_after_delete: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'rust'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(rust_after_delete, None);

        // Verify second tag still exists in FTS
        let python_after_delete: Option<i64> = conn
            .query_row(
                "SELECT note_id FROM notes_fts WHERE notes_fts MATCH 'python'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(python_after_delete, Some(1));
    }

    #[test]
    fn fts_bm25_ranking_orders_by_relevance() {
        let db = Database::in_memory().unwrap();
        let conn = db.connection();

        // Insert notes with different relevance for "rust"
        // Note 1: "rust" appears once
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (1, 'learning rust programming')",
            [],
        )
        .unwrap();

        // Note 2: "rust" appears three times
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (2, 'rust rust rust is great')",
            [],
        )
        .unwrap();

        // Note 3: "rust" appears twice
        conn.execute(
            "INSERT INTO notes (id, content) VALUES (3, 'rust and more rust')",
            [],
        )
        .unwrap();

        // Query with BM25 ordering
        let mut stmt = conn
            .prepare(
                "SELECT note_id, bm25(notes_fts) as score
                 FROM notes_fts
                 WHERE notes_fts MATCH 'rust'
                 ORDER BY score",
            )
            .unwrap();

        let note_ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // BM25 orders by ascending score (lower is better)
        // Note 2 should be most relevant (first), then Note 3, then Note 1
        assert_eq!(note_ids, vec![2, 3, 1]);
    }
}
