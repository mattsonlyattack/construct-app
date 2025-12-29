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
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name",
        )
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

// ========== Edges Table Tests ==========

#[test]
fn edges_table_exists() {
    let db = Database::in_memory().unwrap();

    // Check that edges table exists
    let table_exists: bool = db
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='edges')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(
        table_exists,
        "edges table should exist after Database::in_memory()"
    );
}

#[test]
fn edges_table_has_all_required_columns() {
    let db = Database::in_memory().unwrap();

    // Query table schema for edges table
    let mut stmt = db.connection().prepare("PRAGMA table_info(edges)").unwrap();

    let columns: Vec<(String, String, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?, // name
                row.get::<_, String>(2)?, // type
                row.get::<_, i32>(3)?,    // notnull
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Expected columns with their types and NOT NULL constraints
    let expected = vec![
        ("id", "INTEGER", false),
        ("source_tag_id", "INTEGER", true),
        ("target_tag_id", "INTEGER", true),
        ("confidence", "REAL", false),
        ("hierarchy_type", "TEXT", false),
        ("valid_from", "INTEGER", false),
        ("valid_until", "INTEGER", false),
        ("source", "TEXT", false),
        ("model_version", "TEXT", false),
        ("verified", "INTEGER", false),
        ("created_at", "INTEGER", false),
        ("updated_at", "INTEGER", false),
    ];

    for (name, expected_type, _expected_notnull) in expected {
        let col = columns
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("Column '{}' should exist in edges table", name));

        assert_eq!(
            col.1, expected_type,
            "Column '{}' should have type '{}'",
            name, expected_type
        );
    }

    // Verify source_tag_id and target_tag_id are NOT NULL
    let source_col = columns
        .iter()
        .find(|(n, _, _)| n == "source_tag_id")
        .unwrap();
    assert_eq!(source_col.2, 1, "source_tag_id should be NOT NULL");

    let target_col = columns
        .iter()
        .find(|(n, _, _)| n == "target_tag_id")
        .unwrap();
    assert_eq!(target_col.2, 1, "target_tag_id should be NOT NULL");
}

#[test]
fn edges_hierarchy_type_check_constraint() {
    let db = Database::in_memory().unwrap();

    // Insert valid tags for foreign key constraints
    db.connection()
        .execute(
            "INSERT INTO tags (id, name) VALUES (1, 'neural-networks')",
            [],
        )
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'transformers')", [])
        .unwrap();

    // Test valid 'generic' hierarchy_type
    let result_generic = db.connection().execute(
        "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (1, 2, 'generic')",
        [],
    );
    assert!(
        result_generic.is_ok(),
        "Should allow 'generic' hierarchy_type"
    );

    // Test valid 'partitive' hierarchy_type
    let result_partitive = db.connection().execute(
        "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (2, 1, 'partitive')",
        [],
    );
    assert!(
        result_partitive.is_ok(),
        "Should allow 'partitive' hierarchy_type"
    );

    // Test NULL hierarchy_type is allowed
    let result_null = db.connection().execute(
        "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (1, 2, NULL)",
        [],
    );
    assert!(result_null.is_ok(), "Should allow NULL hierarchy_type");

    // Test invalid hierarchy_type is rejected
    let result_invalid = db.connection().execute(
        "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (1, 2, 'invalid')",
        [],
    );
    assert!(
        result_invalid.is_err(),
        "Should reject invalid hierarchy_type values"
    );

    let error = result_invalid.unwrap_err();
    assert!(
        error.to_string().contains("CHECK") || error.to_string().contains("constraint"),
        "Error should be about CHECK constraint: {}",
        error
    );
}

#[test]
fn edges_foreign_key_cascade_delete_source_tag() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'source-tag')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'target-tag')", [])
        .unwrap();

    // Insert edge
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (1, 2, 'generic')",
            [],
        )
        .unwrap();

    // Verify edge exists
    let count_before: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count_before, 1);

    // Delete source tag - should CASCADE delete the edge
    db.connection()
        .execute("DELETE FROM tags WHERE id = 1", [])
        .unwrap();

    // Verify edge was CASCADE deleted
    let count_after: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        count_after, 0,
        "Edge should be CASCADE deleted when source tag is removed"
    );
}

#[test]
fn edges_foreign_key_cascade_delete_target_tag() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'source-tag')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'target-tag')", [])
        .unwrap();

    // Insert edge
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (1, 2, 'partitive')",
            [],
        )
        .unwrap();

    // Verify edge exists
    let count_before: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count_before, 1);

    // Delete target tag - should CASCADE delete the edge
    db.connection()
        .execute("DELETE FROM tags WHERE id = 2", [])
        .unwrap();

    // Verify edge was CASCADE deleted
    let count_after: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        count_after, 0,
        "Edge should be CASCADE deleted when target tag is removed"
    );
}

#[test]
fn edges_allows_duplicate_source_target_with_different_validity() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'tag-a')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'tag-b')", [])
        .unwrap();

    // Insert first edge with validity window 2023
    let timestamp_2023_start = 1672531200; // 2023-01-01 00:00:00 UTC
    let timestamp_2023_end = 1704067199; // 2023-12-31 23:59:59 UTC
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, valid_from, valid_until) VALUES (1, 2, ?1, ?2)",
            rusqlite::params![timestamp_2023_start, timestamp_2023_end],
        )
        .unwrap();

    // Insert second edge with validity window 2024 (same source/target, different validity)
    let timestamp_2024_start = 1704067200; // 2024-01-01 00:00:00 UTC
    let timestamp_2024_end = 1735689599; // 2024-12-31 23:59:59 UTC
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, valid_from, valid_until) VALUES (1, 2, ?1, ?2)",
            rusqlite::params![timestamp_2024_start, timestamp_2024_end],
        )
        .unwrap();

    // Verify both edges exist
    let count: i64 = db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE source_tag_id = 1 AND target_tag_id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 2,
        "Should allow duplicate source/target pairs with different validity windows"
    );

    // Verify we can query each edge by its validity window
    let edge_2023_id: i64 = db
        .connection()
        .query_row(
            "SELECT id FROM edges WHERE source_tag_id = 1 AND target_tag_id = 2 AND valid_from = ?1",
            [timestamp_2023_start],
            |row| row.get(0),
        )
        .unwrap();

    let edge_2024_id: i64 = db
        .connection()
        .query_row(
            "SELECT id FROM edges WHERE source_tag_id = 1 AND target_tag_id = 2 AND valid_from = ?1",
            [timestamp_2024_start],
            |row| row.get(0),
        )
        .unwrap();

    assert_ne!(
        edge_2023_id, edge_2024_id,
        "Each edge should have a unique ID"
    );
}

// ========== Edges Table Index Tests ==========

#[test]
fn idx_edges_source_exists() {
    let db = Database::in_memory().unwrap();

    // Query sqlite_master for idx_edges_source index
    let index_exists: bool = db
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_edges_source')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists, "idx_edges_source index should exist");

    // Verify index is on the correct column by checking EXPLAIN QUERY PLAN
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges WHERE source_tag_id = 1",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_source"),
        "idx_edges_source should be used for source_tag_id queries. Query plan: {}",
        query_plan
    );
}

#[test]
fn idx_edges_target_exists() {
    let db = Database::in_memory().unwrap();

    // Query sqlite_master for idx_edges_target index
    let index_exists: bool = db
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_edges_target')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists, "idx_edges_target index should exist");

    // Verify index is on the correct column by checking EXPLAIN QUERY PLAN
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges WHERE target_tag_id = 1",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_target"),
        "idx_edges_target should be used for target_tag_id queries. Query plan: {}",
        query_plan
    );
}

#[test]
fn idx_edges_created_at_exists() {
    let db = Database::in_memory().unwrap();

    // Query sqlite_master for idx_edges_created_at index
    let index_exists: bool = db
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_edges_created_at')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists, "idx_edges_created_at index should exist");

    // Verify index is on the correct column by checking EXPLAIN QUERY PLAN
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges ORDER BY created_at",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_created_at"),
        "idx_edges_created_at should be used for created_at ordering. Query plan: {}",
        query_plan
    );
}

#[test]
fn idx_edges_updated_at_exists() {
    let db = Database::in_memory().unwrap();

    // Query sqlite_master for idx_edges_updated_at index
    let index_exists: bool = db
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_edges_updated_at')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists, "idx_edges_updated_at index should exist");

    // Verify index is on the correct column by checking EXPLAIN QUERY PLAN
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges ORDER BY updated_at",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_updated_at"),
        "idx_edges_updated_at should be used for updated_at ordering. Query plan: {}",
        query_plan
    );
}

#[test]
fn idx_edges_hierarchy_type_exists() {
    let db = Database::in_memory().unwrap();

    // Query sqlite_master for idx_edges_hierarchy_type index
    let index_exists: bool = db
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name='idx_edges_hierarchy_type')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert!(index_exists, "idx_edges_hierarchy_type index should exist");

    // Verify index is on the correct column by checking EXPLAIN QUERY PLAN
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges WHERE hierarchy_type = 'generic'",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_hierarchy_type"),
        "idx_edges_hierarchy_type should be used for hierarchy_type queries. Query plan: {}",
        query_plan
    );
}

// ========== Edges Integration Tests ==========

#[test]
fn edges_insert_with_all_columns_populated() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute(
            "INSERT INTO tags (id, name) VALUES (1, 'neural-networks')",
            [],
        )
        .unwrap();
    db.connection()
        .execute(
            "INSERT INTO tags (id, name) VALUES (2, 'deep-learning')",
            [],
        )
        .unwrap();

    // Insert edge with all columns populated
    let timestamp_created = 1735142400; // 2024-12-25 12:00:00 UTC
    let timestamp_updated = 1735228800; // 2024-12-26 12:00:00 UTC
    let timestamp_valid_from = 1704067200; // 2024-01-01 00:00:00 UTC
    let timestamp_valid_until = 1735689599; // 2024-12-31 23:59:59 UTC

    db.connection()
        .execute(
            "INSERT INTO edges (
                source_tag_id, target_tag_id, confidence, hierarchy_type,
                valid_from, valid_until, source, model_version, verified,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                1,                     // source_tag_id
                2,                     // target_tag_id
                0.92,                  // confidence
                "generic",             // hierarchy_type
                timestamp_valid_from,  // valid_from
                timestamp_valid_until, // valid_until
                "llm",                 // source
                "deepseek-r1:8b",      // model_version
                1,                     // verified
                timestamp_created,     // created_at
                timestamp_updated,     // updated_at
            ],
        )
        .unwrap();

    // Verify all columns are stored correctly
    let (
        source,
        target,
        confidence,
        hierarchy_type,
        valid_from,
        valid_until,
        source_field,
        model_version,
        verified,
        created_at,
        updated_at,
    ): (
        i64,
        i64,
        f64,
        String,
        i64,
        i64,
        String,
        String,
        i32,
        i64,
        i64,
    ) = db
        .connection()
        .query_row(
            "SELECT source_tag_id, target_tag_id, confidence, hierarchy_type,
                    valid_from, valid_until, source, model_version, verified,
                    created_at, updated_at
             FROM edges WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(source, 1);
    assert_eq!(target, 2);
    assert_eq!(confidence, 0.92);
    assert_eq!(hierarchy_type, "generic");
    assert_eq!(valid_from, timestamp_valid_from);
    assert_eq!(valid_until, timestamp_valid_until);
    assert_eq!(source_field, "llm");
    assert_eq!(model_version, "deepseek-r1:8b");
    assert_eq!(verified, 1);
    assert_eq!(created_at, timestamp_created);
    assert_eq!(updated_at, timestamp_updated);
}

#[test]
fn edges_insert_with_minimal_columns_uses_defaults() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'tag-a')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'tag-b')", [])
        .unwrap();

    // Insert edge with only required columns (source_tag_id, target_tag_id)
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id) VALUES (1, 2)",
            [],
        )
        .unwrap();

    // Verify defaults are applied
    let (
        confidence,
        hierarchy_type,
        valid_from,
        valid_until,
        source,
        model_version,
        verified,
        created_at,
        updated_at,
    ): (
        Option<f64>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        String,
        Option<String>,
        i32,
        Option<i64>,
        Option<i64>,
    ) = db
        .connection()
        .query_row(
            "SELECT confidence, hierarchy_type, valid_from, valid_until,
                    source, model_version, verified, created_at, updated_at
             FROM edges WHERE source_tag_id = 1 AND target_tag_id = 2",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(confidence, None, "confidence should default to NULL");
    assert_eq!(
        hierarchy_type, None,
        "hierarchy_type should default to NULL"
    );
    assert_eq!(valid_from, None, "valid_from should default to NULL");
    assert_eq!(valid_until, None, "valid_until should default to NULL");
    assert_eq!(source, "user", "source should default to 'user'");
    assert_eq!(model_version, None, "model_version should default to NULL");
    assert_eq!(verified, 0, "verified should default to 0");
    assert_eq!(created_at, None, "created_at should default to NULL");
    assert_eq!(updated_at, None, "updated_at should default to NULL");
}

#[test]
fn edges_query_performance_uses_source_index() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    for i in 1..=10 {
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                rusqlite::params![i, format!("tag-{}", i)],
            )
            .unwrap();
    }

    // Insert edges for performance testing
    for i in 1..=9 {
        db.connection()
            .execute(
                "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (?1, ?2, 'generic')",
                rusqlite::params![i, i + 1],
            )
            .unwrap();
    }

    // Query by source_tag_id - should use idx_edges_source
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges WHERE source_tag_id = 5",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_source"),
        "Query by source_tag_id should use idx_edges_source index. Query plan: {}",
        query_plan
    );

    // Verify query returns correct results
    let target_id: i64 = db
        .connection()
        .query_row(
            "SELECT target_tag_id FROM edges WHERE source_tag_id = 5",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(target_id, 6);
}

#[test]
fn edges_query_performance_uses_target_index() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    for i in 1..=10 {
        db.connection()
            .execute(
                "INSERT INTO tags (id, name) VALUES (?1, ?2)",
                rusqlite::params![i, format!("tag-{}", i)],
            )
            .unwrap();
    }

    // Insert edges for performance testing
    for i in 1..=9 {
        db.connection()
            .execute(
                "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (?1, ?2, 'partitive')",
                rusqlite::params![i, i + 1],
            )
            .unwrap();
    }

    // Query by target_tag_id - should use idx_edges_target
    let query_plan: String = db
        .connection()
        .query_row(
            "EXPLAIN QUERY PLAN SELECT * FROM edges WHERE target_tag_id = 7",
            [],
            |row| row.get::<_, String>(3), // detail column
        )
        .unwrap();

    assert!(
        query_plan.contains("idx_edges_target"),
        "Query by target_tag_id should use idx_edges_target index. Query plan: {}",
        query_plan
    );

    // Verify query returns correct results
    let source_id: i64 = db
        .connection()
        .query_row(
            "SELECT source_tag_id FROM edges WHERE target_tag_id = 7",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(source_id, 6);
}

#[test]
fn edges_temporal_validity_filtering() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute(
            "INSERT INTO tags (id, name) VALUES (1, 'machine-learning')",
            [],
        )
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'ai')", [])
        .unwrap();
    db.connection()
        .execute(
            "INSERT INTO tags (id, name) VALUES (3, 'deep-learning')",
            [],
        )
        .unwrap();

    // Insert edges with different temporal validity windows
    // Edge 1: Valid in 2023 only
    let ts_2023_start = 1672531200; // 2023-01-01 00:00:00 UTC
    let ts_2023_end = 1704067199; // 2023-12-31 23:59:59 UTC
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, valid_from, valid_until, hierarchy_type)
             VALUES (1, 2, ?1, ?2, 'generic')",
            rusqlite::params![ts_2023_start, ts_2023_end],
        )
        .unwrap();

    // Edge 2: Valid in 2024 only
    let ts_2024_start = 1704067200; // 2024-01-01 00:00:00 UTC
    let ts_2024_end = 1735689599; // 2024-12-31 23:59:59 UTC
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, valid_from, valid_until, hierarchy_type)
             VALUES (1, 3, ?1, ?2, 'partitive')",
            rusqlite::params![ts_2024_start, ts_2024_end],
        )
        .unwrap();

    // Edge 3: No temporal constraints (always valid)
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type)
             VALUES (2, 3, 'generic')",
            [],
        )
        .unwrap();

    // Query for edges valid at a specific timestamp in 2023
    let ts_query_2023 = 1688169600; // 2023-07-01 00:00:00 UTC (mid-2023)
    let edges_2023: Vec<(i64, i64)> = db
        .connection()
        .prepare(
            "SELECT source_tag_id, target_tag_id FROM edges
             WHERE (valid_from IS NULL OR valid_from <= ?1)
               AND (valid_until IS NULL OR valid_until >= ?1)
             ORDER BY id",
        )
        .unwrap()
        .query_map([ts_query_2023], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Should return Edge 1 (valid in 2023) and Edge 3 (no temporal constraints)
    assert_eq!(edges_2023.len(), 2);
    assert_eq!(edges_2023[0], (1, 2)); // Edge 1
    assert_eq!(edges_2023[1], (2, 3)); // Edge 3

    // Query for edges valid at a specific timestamp in 2024
    let ts_query_2024 = 1719792000; // 2024-07-01 00:00:00 UTC (mid-2024)
    let edges_2024: Vec<(i64, i64)> = db
        .connection()
        .prepare(
            "SELECT source_tag_id, target_tag_id FROM edges
             WHERE (valid_from IS NULL OR valid_from <= ?1)
               AND (valid_until IS NULL OR valid_until >= ?1)
             ORDER BY id",
        )
        .unwrap()
        .query_map([ts_query_2024], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Should return Edge 2 (valid in 2024) and Edge 3 (no temporal constraints)
    assert_eq!(edges_2024.len(), 2);
    assert_eq!(edges_2024[0], (1, 3)); // Edge 2
    assert_eq!(edges_2024[1], (2, 3)); // Edge 3

    // Query for edges valid at a timestamp outside both windows (2025)
    let ts_query_2025 = 1735689600; // 2025-01-01 00:00:00 UTC
    let edges_2025: Vec<(i64, i64)> = db
        .connection()
        .prepare(
            "SELECT source_tag_id, target_tag_id FROM edges
             WHERE (valid_from IS NULL OR valid_from <= ?1)
               AND (valid_until IS NULL OR valid_until >= ?1)
             ORDER BY id",
        )
        .unwrap()
        .query_map([ts_query_2025], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Should return only Edge 3 (no temporal constraints)
    assert_eq!(edges_2025.len(), 1);
    assert_eq!(edges_2025[0], (2, 3)); // Edge 3
}

#[test]
fn edges_schema_reopen_is_idempotent() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_edges.db");

    // First open: Create database and insert data
    {
        let db = Database::open(&db_path).unwrap();

        // Insert tags
        db.connection()
            .execute("INSERT INTO tags (id, name) VALUES (1, 'rust')", [])
            .unwrap();
        db.connection()
            .execute("INSERT INTO tags (id, name) VALUES (2, 'programming')", [])
            .unwrap();

        // Insert edge
        db.connection()
            .execute(
                "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type, confidence)
                 VALUES (1, 2, 'generic', 0.95)",
                [],
            )
            .unwrap();

        // Verify edge exists
        let count: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    // Second open: Reopen database - schema initialization should be idempotent
    let db2 = Database::open(&db_path);
    assert!(db2.is_ok(), "Schema initialization should be idempotent");

    let db2 = db2.unwrap();

    // Verify edges table still exists
    let table_exists: bool = db2
        .connection()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='edges')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(table_exists, "edges table should exist after reopen");

    // Verify existing data is preserved
    let (source_id, target_id, hierarchy_type, confidence): (i64, i64, String, f64) = db2
        .connection()
        .query_row(
            "SELECT source_tag_id, target_tag_id, hierarchy_type, confidence FROM edges WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(source_id, 1);
    assert_eq!(target_id, 2);
    assert_eq!(hierarchy_type, "generic");
    assert_eq!(confidence, 0.95);

    // Third open: Reopen again - should be completely idempotent
    let db3 = Database::open(&db_path);
    assert!(
        db3.is_ok(),
        "Schema initialization should be idempotent on multiple reopens"
    );

    // Verify all indexes exist after multiple reopens
    let indexes: Vec<String> = db3
        .unwrap()
        .connection()
        .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_edges_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(indexes.contains(&"idx_edges_source".to_string()));
    assert!(indexes.contains(&"idx_edges_target".to_string()));
    assert!(indexes.contains(&"idx_edges_created_at".to_string()));
    assert!(indexes.contains(&"idx_edges_updated_at".to_string()));
    assert!(indexes.contains(&"idx_edges_hierarchy_type".to_string()));
}

#[test]
fn edges_schema_integration_with_existing_tables() {
    let db = Database::in_memory().unwrap();

    // Verify all tables exist (including edges and existing tables)
    let tables: Vec<String> = db
        .connection()
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Check all expected tables exist
    assert!(
        tables.contains(&"notes".to_string()),
        "notes table should exist"
    );
    assert!(
        tables.contains(&"tags".to_string()),
        "tags table should exist"
    );
    assert!(
        tables.contains(&"note_tags".to_string()),
        "note_tags table should exist"
    );
    assert!(
        tables.contains(&"tag_aliases".to_string()),
        "tag_aliases table should exist"
    );
    assert!(
        tables.contains(&"edges".to_string()),
        "edges table should exist"
    );

    // Test foreign key from edges to tags works correctly
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'tag-one')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'tag-two')", [])
        .unwrap();

    // Insert edge referencing tags
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id, hierarchy_type) VALUES (1, 2, 'generic')",
            [],
        )
        .unwrap();

    // Verify edge references correct tags
    let (source_name, target_name): (String, String) = db
        .connection()
        .query_row(
            "SELECT ts.name, tt.name
             FROM edges e
             JOIN tags ts ON e.source_tag_id = ts.id
             JOIN tags tt ON e.target_tag_id = tt.id
             WHERE e.id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(source_name, "tag-one");
    assert_eq!(target_name, "tag-two");

    // Verify existing note_tags functionality still works
    db.connection()
        .execute(
            "INSERT INTO notes (id, content) VALUES (1, 'test note')",
            [],
        )
        .unwrap();
    db.connection()
        .execute("INSERT INTO note_tags (note_id, tag_id) VALUES (1, 1)", [])
        .unwrap();

    let note_tag_count: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM note_tags", [], |row| row.get(0))
        .unwrap();

    assert_eq!(
        note_tag_count, 1,
        "note_tags table should still function correctly"
    );
}

// ========== Degree Centrality Tests ==========

#[test]
fn tags_has_degree_centrality_column() {
    let db = Database::in_memory().unwrap();

    // Query table schema to check degree_centrality column exists
    let mut stmt = db.connection().prepare("PRAGMA table_info(tags)").unwrap();

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

    // Check degree_centrality column exists and is INTEGER
    let degree_centrality_column = columns
        .iter()
        .find(|(name, _)| name == "degree_centrality")
        .expect("degree_centrality column should exist");

    assert_eq!(degree_centrality_column.1, "INTEGER");

    // Verify default value by inserting a tag without specifying degree_centrality
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'test-tag')", [])
        .unwrap();

    let degree_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        degree_centrality, 0,
        "Default degree_centrality should be 0"
    );
}

#[test]
fn degree_centrality_backfill_counts_existing_edges() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'rust')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'programming')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (3, 'systems')", [])
        .unwrap();

    // Insert edges
    // Tag 1 (rust) -> Tag 2 (programming): 1 edge for tag 1, 1 edge for tag 2
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id) VALUES (1, 2)",
            [],
        )
        .unwrap();

    // Tag 1 (rust) -> Tag 3 (systems): 2 edges for tag 1, 1 edge for tag 3
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id) VALUES (1, 3)",
            [],
        )
        .unwrap();

    // Tag 2 (programming) -> Tag 3 (systems): 2 edges for tag 2, 2 edges for tag 3
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id) VALUES (2, 3)",
            [],
        )
        .unwrap();

    // Manually trigger backfill (same query as in initialize_schema)
    db.connection()
        .execute(
            "UPDATE tags SET degree_centrality = (
                SELECT COUNT(*) FROM edges
                WHERE source_tag_id = tags.id OR target_tag_id = tags.id
            )",
            [],
        )
        .unwrap();

    // Verify counts
    let tag1_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tag1_centrality, 2, "Tag 1 (rust) should have 2 connections");

    let tag2_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        tag2_centrality, 2,
        "Tag 2 (programming) should have 2 connections"
    );

    let tag3_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 3",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        tag3_centrality, 2,
        "Tag 3 (systems) should have 2 connections"
    );
}

#[test]
fn degree_centrality_zero_for_tags_without_edges() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute(
            "INSERT INTO tags (id, name) VALUES (1, 'connected-tag')",
            [],
        )
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'isolated-tag')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (3, 'another-tag')", [])
        .unwrap();

    // Insert one edge connecting tags 1 and 3 (tag 2 remains isolated)
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id) VALUES (1, 3)",
            [],
        )
        .unwrap();

    // Manually trigger backfill
    db.connection()
        .execute(
            "UPDATE tags SET degree_centrality = (
                SELECT COUNT(*) FROM edges
                WHERE source_tag_id = tags.id OR target_tag_id = tags.id
            )",
            [],
        )
        .unwrap();

    // Verify isolated tag has zero centrality
    let isolated_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        isolated_centrality, 0,
        "Isolated tag should have 0 connections"
    );

    // Verify connected tags have non-zero centrality
    let tag1_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tag1_centrality, 1, "Tag 1 should have 1 connection");

    let tag3_centrality: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 3",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tag3_centrality, 1, "Tag 3 should have 1 connection");
}

#[test]
fn degree_centrality_backfill_idempotent() {
    let db = Database::in_memory().unwrap();

    // Insert tags
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (1, 'tag-a')", [])
        .unwrap();
    db.connection()
        .execute("INSERT INTO tags (id, name) VALUES (2, 'tag-b')", [])
        .unwrap();

    // Insert edge
    db.connection()
        .execute(
            "INSERT INTO edges (source_tag_id, target_tag_id) VALUES (1, 2)",
            [],
        )
        .unwrap();

    // Run backfill first time
    db.connection()
        .execute(
            "UPDATE tags SET degree_centrality = (
                SELECT COUNT(*) FROM edges
                WHERE source_tag_id = tags.id OR target_tag_id = tags.id
            )",
            [],
        )
        .unwrap();

    // Get centrality values after first backfill
    let tag1_first: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let tag2_first: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Run backfill second time (idempotent test)
    db.connection()
        .execute(
            "UPDATE tags SET degree_centrality = (
                SELECT COUNT(*) FROM edges
                WHERE source_tag_id = tags.id OR target_tag_id = tags.id
            )",
            [],
        )
        .unwrap();

    // Get centrality values after second backfill
    let tag1_second: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let tag2_second: i32 = db
        .connection()
        .query_row(
            "SELECT degree_centrality FROM tags WHERE id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // Verify values remain the same after re-running backfill
    assert_eq!(
        tag1_first, tag1_second,
        "Tag 1 centrality should remain {} after re-run",
        tag1_first
    );
    assert_eq!(
        tag2_first, tag2_second,
        "Tag 2 centrality should remain {} after re-run",
        tag2_first
    );

    // Verify values are correct (both should be 1)
    assert_eq!(tag1_first, 1, "Tag 1 should have 1 connection");
    assert_eq!(tag2_first, 1, "Tag 2 should have 1 connection");
}
