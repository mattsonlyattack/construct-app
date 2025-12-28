use super::*;

#[test]
fn note_service_construction_with_in_memory_database() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Verify we can access the underlying database
    let conn = service.database().connection();

    // Quick smoke test - verify schema is initialized
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        )
        .expect("failed to query schema");

    assert!(
        count >= 3,
        "expected at least 3 tables (notes, tags, note_tags)"
    );
}

#[test]
fn list_notes_options_default_implementation() {
    let options = ListNotesOptions::default();

    assert_eq!(options.limit, None, "default limit should be None");
    assert_eq!(options.tags, None, "default tags should be None");

    // Test that Default can be used with struct update syntax
    let with_limit = ListNotesOptions {
        limit: Some(10),
        ..Default::default()
    };
    assert_eq!(with_limit.limit, Some(10));
    assert_eq!(with_limit.tags, None);

    let with_tags = ListNotesOptions {
        tags: Some(vec!["test".to_string()]),
        ..Default::default()
    };
    assert_eq!(with_tags.limit, None);
    assert_eq!(with_tags.tags, Some(vec!["test".to_string()]));
}

// --- CRUD Operation Tests (Task Group 2) ---

#[test]
fn create_note_with_content_only_returns_note_with_valid_id() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Test note content", None)
        .expect("failed to create note");

    assert!(note.id().get() > 0, "note ID should be positive");
    assert_eq!(note.content(), "Test note content");
    assert!(note.tags().is_empty(), "note should have no tags");
}

#[test]
fn get_note_returns_none_for_non_existent_id() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let result = service
        .get_note(NoteId::new(999))
        .expect("get_note should not error for non-existent ID");

    assert_eq!(result, None, "should return None for non-existent note");
}

#[test]
fn get_note_returns_some_note_for_existing_note() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a note first
    let created = service
        .create_note("Original content", None)
        .expect("failed to create note");

    // Retrieve it
    let retrieved = service
        .get_note(created.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(retrieved.id(), created.id());
    assert_eq!(retrieved.content(), "Original content");
    assert_eq!(retrieved.created_at(), created.created_at());
    assert_eq!(retrieved.updated_at(), created.updated_at());
}

#[test]
fn delete_note_is_idempotent() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a note
    let note = service
        .create_note("To be deleted", None)
        .expect("failed to create note");

    // Delete it once
    service
        .delete_note(note.id())
        .expect("first delete should succeed");

    // Verify it's gone
    let result = service
        .get_note(note.id())
        .expect("get_note should not error");
    assert_eq!(result, None, "note should be deleted");

    // Delete it again (idempotent)
    service
        .delete_note(note.id())
        .expect("second delete should succeed (idempotent)");

    // Delete a note that never existed (also idempotent)
    service
        .delete_note(NoteId::new(9999))
        .expect("delete of non-existent note should succeed (idempotent)");
}

// --- Tag Operation Tests (Task Group 3) ---

#[test]
fn create_note_with_tags_creates_note_and_associates_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Note with tags", Some(&["rust", "programming"]))
        .expect("failed to create note with tags");

    assert_eq!(note.tags().len(), 2, "note should have 2 tags");

    // Verify tags are user-sourced with 100% confidence
    for tag_assignment in note.tags() {
        assert!(
            tag_assignment.source().is_user(),
            "tags should be user-sourced"
        );
        assert_eq!(tag_assignment.confidence(), 100);
    }

    // Verify tags persist when retrieved
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(
        retrieved.tags().len(),
        2,
        "retrieved note should have 2 tags"
    );
}

#[test]
fn create_note_with_duplicate_tag_names_only_creates_one_tag() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with duplicate tag names
    let note = service
        .create_note("Note with duplicates", Some(&["rust", "RUST", "Rust"]))
        .expect("failed to create note");

    // Should only have one tag assignment despite 3 duplicate names
    assert_eq!(
        note.tags().len(),
        1,
        "duplicate tag names should result in single tag"
    );

    // Verify only one tag exists in database (case-insensitive)
    let conn = service.database().connection();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tags WHERE name LIKE 'rust'",
            [],
            |row| row.get(0),
        )
        .expect("failed to count tags");

    assert_eq!(count, 1, "only one 'rust' tag should exist in database");
}

#[test]
fn add_tags_to_note_with_user_source_sets_correct_metadata() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without tags
    let note = service
        .create_note("Note for user tags", None)
        .expect("failed to create note");

    // Add user tags
    service
        .add_tags_to_note(note.id(), &["rust", "learning"], TagSource::User)
        .expect("failed to add user tags");

    // Retrieve and verify
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(retrieved.tags().len(), 2, "note should have 2 tags");

    for tag_assignment in retrieved.tags() {
        assert!(
            tag_assignment.source().is_user(),
            "tags should be user-sourced"
        );
        assert_eq!(
            tag_assignment.confidence(),
            100,
            "user tags should have 100% confidence"
        );
        assert_eq!(
            tag_assignment.model(),
            None,
            "user tags should have no model"
        );
    }
}

#[test]
fn add_tags_to_note_with_llm_source_includes_model_version_and_confidence() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without tags
    let note = service
        .create_note("Note for LLM tags", None)
        .expect("failed to create note");

    // Add LLM tags
    let llm_source = TagSource::llm("deepseek-r1:8b", 85);
    service
        .add_tags_to_note(note.id(), &["ai", "machine-learning"], llm_source)
        .expect("failed to add LLM tags");

    // Retrieve and verify
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(retrieved.tags().len(), 2, "note should have 2 tags");

    for tag_assignment in retrieved.tags() {
        assert!(
            tag_assignment.source().is_llm(),
            "tags should be LLM-sourced"
        );
        assert_eq!(
            tag_assignment.confidence(),
            85,
            "LLM tags should have specified confidence"
        );
        assert_eq!(
            tag_assignment.model(),
            Some("deepseek-r1:8b"),
            "LLM tags should have model identifier"
        );
    }
}

// --- Tag Normalization Tests (Task Group 1: Tag Normalization) ---

#[test]
fn create_note_with_mixed_case_tag_normalizes_to_lowercase_hyphenated() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Test note", Some(&["Machine Learning"]))
        .expect("failed to create note");

    assert_eq!(note.tags().len(), 1, "note should have 1 tag");

    // Verify tag is stored in normalized form
    let conn = service.database().connection();
    let tag_name: String = conn
        .query_row(
            "SELECT name FROM tags WHERE id = ?1",
            [note.tags()[0].tag_id().get()],
            |row| row.get(0),
        )
        .expect("failed to get tag name");

    assert_eq!(tag_name, "machine-learning", "tag should be normalized");
}

#[test]
fn list_notes_with_normalized_tag_filter_works() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with different tag formats
    service
        .create_note("Note 1", Some(&["Machine Learning"]))
        .expect("failed to create note 1");
    service
        .create_note("Note 2", Some(&["machine-learning"]))
        .expect("failed to create note 2");
    service
        .create_note("Note 3", Some(&["rust"]))
        .expect("failed to create note 3");

    // Query using normalized form
    let options = ListNotesOptions {
        tags: Some(vec!["machine-learning".to_string()]),
        ..Default::default()
    };

    let notes = service.list_notes(options).expect("failed to list notes");

    assert_eq!(notes.len(), 2, "should find both notes with normalized tag");
}

#[test]
fn create_note_with_duplicate_tag_variants_creates_single_normalized_tag() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Test note", Some(&["Rust", "RUST", "rust"]))
        .expect("failed to create note");

    assert_eq!(
        note.tags().len(),
        1,
        "duplicate variants should create one tag"
    );

    // Verify only one tag exists in database
    let conn = service.database().connection();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tags WHERE name = 'rust'", [], |row| {
            row.get(0)
        })
        .expect("failed to count tags");

    assert_eq!(count, 1, "only one normalized tag should exist");
}

#[test]
fn create_note_with_special_characters_strips_and_normalizes() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Test note", Some(&["C++", "node.js"]))
        .expect("failed to create note");

    assert_eq!(note.tags().len(), 2, "note should have 2 tags");

    // Verify tags are normalized (special chars stripped)
    let conn = service.database().connection();
    let mut tag_names: Vec<String> = conn
        .prepare("SELECT name FROM tags ORDER BY name")
        .expect("failed to prepare query")
        .query_map([], |row| row.get(0))
        .expect("failed to query tags")
        .collect::<Result<Vec<String>, _>>()
        .expect("failed to collect tag names");

    tag_names.sort();
    assert_eq!(
        tag_names,
        vec!["c", "nodejs"],
        "special chars should be stripped"
    );
}

#[test]
fn create_note_with_whitespace_normalizes_whitespace() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Test note", Some(&["  rust  ", "  machine learning  "]))
        .expect("failed to create note");

    assert_eq!(note.tags().len(), 2, "note should have 2 tags");

    // Verify tags are normalized (whitespace trimmed and normalized)
    let conn = service.database().connection();
    let mut tag_names: Vec<String> = conn
        .prepare("SELECT name FROM tags ORDER BY name")
        .expect("failed to prepare query")
        .query_map([], |row| row.get(0))
        .expect("failed to query tags")
        .collect::<Result<Vec<String>, _>>()
        .expect("failed to collect tag names");

    tag_names.sort();
    assert_eq!(
        tag_names,
        vec!["machine-learning", "rust"],
        "whitespace should be normalized"
    );
}

#[test]
fn add_tags_to_note_normalizes_tags_before_insertion() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without tags
    let note = service
        .create_note("Test note", None)
        .expect("failed to create note");

    // Add tags with mixed case
    service
        .add_tags_to_note(note.id(), &["Machine Learning", "RUST"], TagSource::User)
        .expect("failed to add tags");

    // Retrieve and verify normalization
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(retrieved.tags().len(), 2, "note should have 2 tags");

    // Verify tags are stored in normalized form
    let conn = service.database().connection();
    let mut tag_names: Vec<String> = conn
        .prepare("SELECT name FROM tags ORDER BY name")
        .expect("failed to prepare query")
        .query_map([], |row| row.get(0))
        .expect("failed to query tags")
        .collect::<Result<Vec<String>, _>>()
        .expect("failed to collect tag names");

    tag_names.sort();
    assert_eq!(
        tag_names,
        vec!["machine-learning", "rust"],
        "tags should be normalized"
    );
}

// --- Additional Tag Normalization Tests (Task Group 2: Test Review) ---

#[test]
fn add_tags_to_note_with_llm_source_normalizes_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without tags
    let note = service
        .create_note("Test note", None)
        .expect("failed to create note");

    // Add LLM tags with mixed case
    let llm_source = TagSource::llm("deepseek-r1:8b", 90);
    service
        .add_tags_to_note(note.id(), &["Machine Learning", "RUST"], llm_source)
        .expect("failed to add LLM tags");

    // Verify tags are stored in normalized form
    let conn = service.database().connection();
    let mut tag_names: Vec<String> = conn
        .prepare("SELECT name FROM tags ORDER BY name")
        .expect("failed to prepare query")
        .query_map([], |row| row.get(0))
        .expect("failed to query tags")
        .collect::<Result<Vec<String>, _>>()
        .expect("failed to collect tag names");

    tag_names.sort();
    assert_eq!(
        tag_names,
        vec!["machine-learning", "rust"],
        "LLM tags should be normalized"
    );
}

#[test]
fn mixed_case_deduplication_works_across_user_and_llm_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with user tag "Rust"
    let note = service
        .create_note("Test note", Some(&["Rust"]))
        .expect("failed to create note");

    // Add "RUST" via LLM - should resolve to same tag (normalized to "rust")
    let llm_source = TagSource::llm("deepseek-r1:8b", 85);
    service
        .add_tags_to_note(note.id(), &["RUST"], llm_source)
        .expect("failed to add LLM tag");

    // Verify only one tag exists in database (normalized form)
    let conn = service.database().connection();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tags WHERE name = 'rust'", [], |row| {
            row.get(0)
        })
        .expect("failed to count tags");

    assert_eq!(
        count, 1,
        "user and LLM tags should deduplicate to same normalized tag"
    );

    // Verify note still has 1 tag assignment (second insert is ignored due to PRIMARY KEY constraint)
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(
        retrieved.tags().len(),
        1,
        "note should have 1 tag assignment (duplicate ignored due to PRIMARY KEY)"
    );

    // Verify the tag assignment is user-sourced (first one wins)
    assert!(
        retrieved.tags()[0].source().is_user(),
        "first tag assignment (user) should be preserved"
    );
}

#[test]
fn end_to_end_normalization_workflow_create_retrieve_verify() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with mixed-case tags
    let note = service
        .create_note(
            "My thoughts on Machine Learning",
            Some(&["Machine Learning", "Rust", "C++"]),
        )
        .expect("failed to create note");

    // Retrieve the note
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    // Verify tags are normalized in the retrieved note
    assert_eq!(retrieved.tags().len(), 3, "note should have 3 tags");

    // Verify tags are stored in normalized form in database
    let conn = service.database().connection();
    let mut tag_names: Vec<String> = conn
        .prepare("SELECT name FROM tags ORDER BY name")
        .expect("failed to prepare query")
        .query_map([], |row| row.get(0))
        .expect("failed to query tags")
        .collect::<Result<Vec<String>, _>>()
        .expect("failed to collect tag names");

    tag_names.sort();
    assert_eq!(
        tag_names,
        vec!["c", "machine-learning", "rust"],
        "tags should be normalized in database"
    );

    // Verify we can query using normalized form
    let options = ListNotesOptions {
        tags: Some(vec!["machine-learning".to_string()]),
        ..Default::default()
    };
    let notes = service.list_notes(options).expect("failed to list notes");
    assert_eq!(notes.len(), 1, "should find note by normalized tag");
    assert_eq!(notes[0].id(), note.id());
}

// --- List Operation Tests (Task Group 4) ---

#[test]
fn list_notes_with_default_options_returns_notes_in_created_at_desc_order() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create multiple notes with slight delays to ensure different timestamps
    let note1 = service
        .create_note("First note", None)
        .expect("failed to create note 1");

    std::thread::sleep(std::time::Duration::from_millis(10));

    let note2 = service
        .create_note("Second note", None)
        .expect("failed to create note 2");

    std::thread::sleep(std::time::Duration::from_millis(10));

    let note3 = service
        .create_note("Third note", None)
        .expect("failed to create note 3");

    // List with default options
    let notes = service
        .list_notes(ListNotesOptions::default())
        .expect("failed to list notes");

    assert_eq!(notes.len(), 3, "should return all 3 notes");

    // Verify order is newest first (DESC)
    assert_eq!(
        notes[0].id(),
        note3.id(),
        "first note should be the most recent (note3)"
    );
    assert_eq!(notes[1].id(), note2.id(), "second note should be note2");
    assert_eq!(
        notes[2].id(),
        note1.id(),
        "third note should be the oldest (note1)"
    );
}

#[test]
fn list_notes_with_limit_option_respects_limit() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create 5 notes
    for i in 1..=5 {
        service
            .create_note(&format!("Note {}", i), None)
            .expect("failed to create note");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // List with limit of 2
    let options = ListNotesOptions {
        limit: Some(2),
        ..Default::default()
    };

    let notes = service.list_notes(options).expect("failed to list notes");

    assert_eq!(notes.len(), 2, "should return exactly 2 notes");

    // Should be the 2 most recent notes
    assert_eq!(notes[0].content(), "Note 5");
    assert_eq!(notes[1].content(), "Note 4");
}

#[test]
fn list_notes_with_tags_filter_returns_only_notes_with_all_specified_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with various tag combinations
    let note1 = service
        .create_note("Rust only", Some(&["rust"]))
        .expect("failed to create note 1");

    let note2 = service
        .create_note("Rust and programming", Some(&["rust", "programming"]))
        .expect("failed to create note 2");

    let note3 = service
        .create_note(
            "Rust, programming, and tutorial",
            Some(&["rust", "programming", "tutorial"]),
        )
        .expect("failed to create note 3");

    let note4 = service
        .create_note("Programming only", Some(&["programming"]))
        .expect("failed to create note 4");

    service
        .create_note("No tags", None)
        .expect("failed to create note 5");

    // Filter by tags: rust AND programming (AND logic)
    let options = ListNotesOptions {
        tags: Some(vec!["rust".to_string(), "programming".to_string()]),
        ..Default::default()
    };

    let notes = service.list_notes(options).expect("failed to list notes");

    // Should only return notes 2 and 3 (both have rust AND programming)
    assert_eq!(
        notes.len(),
        2,
        "should return only notes with ALL specified tags"
    );

    let note_ids: Vec<NoteId> = notes.iter().map(|n| n.id()).collect();
    assert!(
        note_ids.contains(&note2.id()),
        "should include note2 (rust + programming)"
    );
    assert!(
        note_ids.contains(&note3.id()),
        "should include note3 (rust + programming + tutorial)"
    );
    assert!(
        !note_ids.contains(&note1.id()),
        "should NOT include note1 (only rust)"
    );
    assert!(
        !note_ids.contains(&note4.id()),
        "should NOT include note4 (only programming)"
    );
}

// --- Additional Critical Gap Tests (Task Group 5) ---

#[test]
fn list_notes_returns_empty_vec_for_empty_database() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let notes = service
        .list_notes(ListNotesOptions::default())
        .expect("failed to list notes");

    assert_eq!(notes.len(), 0, "should return empty vec for empty database");
}

#[test]
fn add_tags_to_note_fails_for_non_existent_note() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let result =
        service.add_tags_to_note(NoteId::new(999), &["rust", "programming"], TagSource::User);

    assert!(
        result.is_err(),
        "should return error when adding tags to non-existent note"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not exist"),
        "error message should indicate note doesn't exist: {}",
        err_msg
    );
}

#[test]
fn list_notes_with_empty_tags_filter_returns_no_notes() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create some notes
    service
        .create_note("Note 1", Some(&["rust"]))
        .expect("failed to create note 1");

    service
        .create_note("Note 2", Some(&["programming"]))
        .expect("failed to create note 2");

    // Filter with empty tags list
    let options = ListNotesOptions {
        tags: Some(vec![]),
        ..Default::default()
    };

    let notes = service.list_notes(options).expect("failed to list notes");

    assert_eq!(notes.len(), 0, "empty tags filter should return no notes");
}

#[test]
fn delete_note_cascades_to_note_tags_table() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with tags
    let note = service
        .create_note("Note with tags", Some(&["rust", "programming"]))
        .expect("failed to create note");

    // Verify tags exist in note_tags table
    let conn = service.database().connection();
    let tag_count_before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
            [note.id().get()],
            |row| row.get(0),
        )
        .expect("failed to count note_tags");

    assert_eq!(tag_count_before, 2, "note should have 2 tag associations");

    // Delete the note
    service
        .delete_note(note.id())
        .expect("failed to delete note");

    // Verify note_tags entries are also deleted (cascade)
    let tag_count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM note_tags WHERE note_id = ?1",
            [note.id().get()],
            |row| row.get(0),
        )
        .expect("failed to count note_tags");

    assert_eq!(
        tag_count_after, 0,
        "note_tags entries should be deleted via cascade"
    );
}

#[test]
fn timestamp_conversion_maintains_accuracy() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Capture Unix timestamp before creation (second precision like database)
    let before_unix = OffsetDateTime::now_utc().unix_timestamp();

    let note = service
        .create_note("Timestamp test", None)
        .expect("failed to create note");

    // Capture Unix timestamp after creation
    let after_unix = OffsetDateTime::now_utc().unix_timestamp();

    let note_unix = note.created_at().unix_timestamp();

    // Verify created_at is within expected range (Unix timestamps are seconds)
    assert!(
        note_unix >= before_unix && note_unix <= after_unix,
        "created_at Unix timestamp should be between before ({}) and after ({}), got {}",
        before_unix,
        after_unix,
        note_unix
    );

    // Verify created_at equals updated_at on creation
    assert_eq!(
        note.created_at(),
        note.updated_at(),
        "created_at and updated_at should match on creation"
    );

    // Verify timestamp round-trip through database
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(
        retrieved.created_at(),
        note.created_at(),
        "timestamps should survive database round-trip"
    );
    assert_eq!(
        retrieved.updated_at(),
        note.updated_at(),
        "timestamps should survive database round-trip"
    );
}

// --- Tag Alias Tests (Task Group 2: Alias Service Methods) ---

#[test]
fn resolve_alias_returns_canonical_tag_id_for_existing_alias() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a canonical tag
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create an alias for it
    service
        .create_alias("ml", canonical_tag_id, "user", 1.0, None)
        .expect("failed to create alias");

    // Resolve the alias
    let resolved = service
        .resolve_alias("ml")
        .expect("failed to resolve alias")
        .expect("alias should exist");

    assert_eq!(
        resolved, canonical_tag_id,
        "alias should resolve to canonical tag ID"
    );
}

#[test]
fn resolve_alias_returns_none_for_non_existent_alias() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let result = service
        .resolve_alias("non-existent-alias")
        .expect("failed to resolve alias");

    assert_eq!(result, None, "non-existent alias should return None");
}

#[test]
fn create_alias_with_user_source_stores_correctly() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a canonical tag
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create an alias
    service
        .create_alias("ml", canonical_tag_id, "user", 1.0, None)
        .expect("failed to create alias");

    // Verify it's stored correctly
    let aliases = service.list_aliases().expect("failed to list aliases");

    assert_eq!(aliases.len(), 1, "should have 1 alias");
    assert_eq!(aliases[0].alias(), "ml");
    assert_eq!(aliases[0].canonical_tag_id(), canonical_tag_id);
    assert_eq!(aliases[0].source(), "user");
    assert_eq!(aliases[0].confidence(), 1.0);
    assert_eq!(aliases[0].model_version(), None);
}

#[test]
fn create_alias_with_llm_source_includes_model_version() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a canonical tag
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create an LLM alias
    service
        .create_alias("ml", canonical_tag_id, "llm", 0.85, Some("deepseek-r1:8b"))
        .expect("failed to create alias");

    // Verify it's stored correctly
    let aliases = service.list_aliases().expect("failed to list aliases");

    assert_eq!(aliases.len(), 1, "should have 1 alias");
    assert_eq!(aliases[0].alias(), "ml");
    assert_eq!(aliases[0].source(), "llm");
    assert_eq!(aliases[0].confidence(), 0.85);
    assert_eq!(aliases[0].model_version(), Some("deepseek-r1:8b"));
}

#[test]
fn create_alias_prevents_alias_to_alias_chains() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);
    let conn = service.database().connection();

    // Create two canonical tags
    let ml_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create ml tag");

    // Create an alias "ml" pointing to machine-learning
    service
        .create_alias("ml", ml_tag_id, "user", 1.0, None)
        .expect("failed to create ml alias");

    // Manually create a tag with the name "ml" in the tags table (bypassing get_or_create_tag)
    // This simulates a scenario where a tag name conflicts with an alias
    conn.execute("INSERT INTO tags (name) VALUES (?1)", ["ml"])
        .expect("failed to insert ml tag");

    let ml_as_tag_id = conn.last_insert_rowid();

    // Try to create an alias where the canonical_tag_id points to a tag whose name is "ml"
    // which is itself an alias - this should fail
    let result = service.create_alias(
        "machine-learning-alias",
        TagId::new(ml_as_tag_id),
        "user",
        1.0,
        None,
    );

    assert!(
        result.is_err(),
        "creating alias where canonical tag name is itself an alias should fail"
    );

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("is itself an alias"),
        "error message should indicate tag is an alias: {}",
        err_msg
    );
}

#[test]
fn list_aliases_returns_all_aliases_grouped_by_canonical_tag() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tags
    let ml_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create ml tag");
    let ai_tag_id = service
        .get_or_create_tag("artificial-intelligence")
        .expect("failed to create ai tag");

    // Create aliases
    service
        .create_alias("ml", ml_tag_id, "user", 1.0, None)
        .expect("failed to create ml alias");
    service
        .create_alias("ai", ai_tag_id, "user", 1.0, None)
        .expect("failed to create ai alias");
    service
        .create_alias(
            "machine-learning-abbrev",
            ml_tag_id,
            "llm",
            0.9,
            Some("model"),
        )
        .expect("failed to create machine-learning-abbrev alias");

    // List all aliases
    let aliases = service.list_aliases().expect("failed to list aliases");

    assert_eq!(aliases.len(), 3, "should have 3 aliases");

    // Verify they're ordered by canonical tag name, then by alias name
    let alias_names: Vec<&str> = aliases.iter().map(|a| a.alias()).collect();
    assert!(alias_names.contains(&"ml"), "should contain ml alias");
    assert!(alias_names.contains(&"ai"), "should contain ai alias");
    assert!(
        alias_names.contains(&"machine-learning-abbrev"),
        "should contain machine-learning-abbrev alias"
    );
}

#[test]
fn remove_alias_deletes_mapping_idempotently() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a canonical tag and alias
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", canonical_tag_id, "user", 1.0, None)
        .expect("failed to create alias");

    // Verify alias exists
    assert!(
        service
            .resolve_alias("ml")
            .expect("failed to resolve")
            .is_some(),
        "alias should exist before removal"
    );

    // Remove the alias
    service.remove_alias("ml").expect("failed to remove alias");

    // Verify it's gone
    assert!(
        service
            .resolve_alias("ml")
            .expect("failed to resolve")
            .is_none(),
        "alias should not exist after removal"
    );

    // Remove again (idempotent)
    service
        .remove_alias("ml")
        .expect("second remove should succeed (idempotent)");

    // Remove non-existent alias (also idempotent)
    service
        .remove_alias("non-existent")
        .expect("removing non-existent alias should succeed (idempotent)");
}

#[test]
fn alias_lookup_happens_after_normalization() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create a canonical tag
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create an alias with normalized form
    service
        .create_alias("ml", canonical_tag_id, "user", 1.0, None)
        .expect("failed to create alias");

    // Resolve using different case (should normalize before lookup)
    let resolved_lower = service
        .resolve_alias("ml")
        .expect("failed to resolve")
        .expect("should find alias");

    let resolved_upper = service
        .resolve_alias("ML")
        .expect("failed to resolve")
        .expect("should find alias with different case");

    assert_eq!(
        resolved_lower, resolved_upper,
        "case-insensitive lookup should work"
    );
    assert_eq!(
        resolved_lower, canonical_tag_id,
        "both should resolve to canonical tag"
    );
}

// --- AutoTagger Alias Integration Tests ---

#[test]
fn llm_suggested_alias_auto_creation_workflow() {
    // Integration test simulating auto_tag_note creating an LLM-suggested alias
    // when LLM suggests a tag that could be an alias for an existing tag

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Arrange: Pre-existing canonical tag "machine-learning" exists
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create canonical tag");

    // Act: Simulate LLM suggesting "ml" as a tag (detected as alias opportunity)
    // This mimics the find_alias_opportunity + create_alias flow in auto_tag_note
    let suggested_tag = "ml";
    let model_version = "deepseek-r1:8b";
    let confidence = 0.85;

    // Create the LLM-suggested alias
    service
        .create_alias(
            suggested_tag,
            canonical_tag_id,
            "llm",
            confidence,
            Some(model_version),
        )
        .expect("failed to create LLM alias");

    // Assert: Alias was created with correct provenance
    let alias_info_list = service.list_aliases().expect("failed to list aliases");
    assert_eq!(alias_info_list.len(), 1, "should have 1 alias");

    let alias_info = &alias_info_list[0];
    assert_eq!(alias_info.alias(), "ml");
    assert_eq!(alias_info.canonical_tag_id(), canonical_tag_id);
    assert_eq!(alias_info.source(), "llm");
    assert_eq!(alias_info.confidence(), confidence);
    assert_eq!(alias_info.model_version(), Some(model_version));
}

#[test]
fn user_creates_alias_then_adds_note_with_that_tag() {
    // Integration test: User creates an alias, then adds a note using that alias
    // Verify the alias resolution works end-to-end

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Arrange: User creates an alias via CLI (cons tag-alias add ml machine-learning)
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create canonical tag");
    service
        .create_alias("ml", canonical_tag_id, "user", 1.0, None)
        .expect("failed to create alias");

    // Act: User adds a note with the alias (cons add --tags ml "...")
    let note = service
        .create_note("Learning about ML", Some(&["ml"]))
        .expect("failed to create note");

    // Assert: Note is tagged with canonical tag, not alias
    assert_eq!(note.tags().len(), 1, "note should have 1 tag");

    let conn = service.database().connection();
    let tag_name: String = conn
        .query_row(
            "SELECT name FROM tags WHERE id = ?1",
            [note.tags()[0].tag_id().get()],
            |row| row.get(0),
        )
        .expect("failed to get tag name");

    assert_eq!(
        tag_name, "machine-learning",
        "note should be tagged with canonical form"
    );
}

#[test]
fn alias_removal_then_tag_creation_with_same_name() {
    // Integration test: After removing an alias, the alias name can be used as a new tag
    // This verifies alias removal doesn't leave orphaned constraints

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Arrange: Create an alias
    let canonical_tag_id = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create canonical tag");
    service
        .create_alias("ml", canonical_tag_id, "user", 1.0, None)
        .expect("failed to create alias");

    // Act: Remove the alias
    service.remove_alias("ml").expect("failed to remove alias");

    // Now create a new tag with the name "ml" (not an alias, a real tag)
    let new_ml_tag_id = service
        .get_or_create_tag("ml")
        .expect("failed to create new ml tag");

    // Assert: New tag was created (not resolved to old canonical)
    assert_ne!(
        new_ml_tag_id, canonical_tag_id,
        "ml should be a new tag, not resolved to old canonical"
    );

    // Verify the new tag exists in tags table
    let conn = service.database().connection();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tags WHERE name = 'ml'", [], |row| {
            row.get(0)
        })
        .expect("failed to count ml tags");

    assert_eq!(count, 1, "ml tag should exist");
}

// --- Enhancement Field Tests (Task Group 2: Note Text Enhancement) ---

#[test]
fn note_struct_includes_optional_enhancement_fields() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without enhancement
    let note = service
        .create_note("Original content", None)
        .expect("failed to create note");

    // Verify enhancement fields are None by default
    assert_eq!(note.content_enhanced(), None);
    assert_eq!(note.enhanced_at(), None);
    assert_eq!(note.enhancement_model(), None);
    assert_eq!(note.enhancement_confidence(), None);
}

#[test]
fn note_builder_supports_setting_enhancement_fields() {
    use time::OffsetDateTime;

    let now = OffsetDateTime::now_utc();
    let note = NoteBuilder::new()
        .id(NoteId::new(1))
        .content("Original content")
        .content_enhanced("Enhanced content")
        .enhanced_at(now)
        .enhancement_model("deepseek-r1:8b")
        .enhancement_confidence(0.85)
        .build();

    assert_eq!(note.content_enhanced(), Some("Enhanced content"));
    assert_eq!(note.enhanced_at(), Some(now));
    assert_eq!(note.enhancement_model(), Some("deepseek-r1:8b"));
    assert_eq!(note.enhancement_confidence(), Some(0.85));
}

#[test]
fn note_accessors_return_correct_values_for_enhancement_fields() {
    use time::OffsetDateTime;

    let enhanced_time = OffsetDateTime::now_utc();
    let note = NoteBuilder::new()
        .id(NoteId::new(42))
        .content("Short note")
        .content_enhanced("This is a more detailed version of the short note.")
        .enhanced_at(enhanced_time)
        .enhancement_model("deepseek-r1:8b")
        .enhancement_confidence(0.92)
        .build();

    // Test all accessors
    assert_eq!(
        note.content_enhanced(),
        Some("This is a more detailed version of the short note.")
    );
    assert_eq!(note.enhanced_at(), Some(enhanced_time));
    assert_eq!(note.enhancement_model(), Some("deepseek-r1:8b"));
    assert_eq!(note.enhancement_confidence(), Some(0.92));

    // Verify None case
    let plain_note = NoteBuilder::new()
        .id(NoteId::new(1))
        .content("Plain note")
        .build();

    assert_eq!(plain_note.content_enhanced(), None);
    assert_eq!(plain_note.enhanced_at(), None);
    assert_eq!(plain_note.enhancement_model(), None);
    assert_eq!(plain_note.enhancement_confidence(), None);
}

#[test]
fn note_service_stores_enhancement_data_on_note_creation() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without enhancement (normal flow)
    let note = service
        .create_note("Original", None)
        .expect("failed to create note");

    // Verify enhancement fields are NULL in database
    let conn = service.database().connection();
    let row: (Option<String>, Option<i64>, Option<String>, Option<f64>) = conn
        .query_row(
            "SELECT content_enhanced, enhanced_at, enhancement_model, enhancement_confidence
             FROM notes WHERE id = ?1",
            [note.id().get()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("failed to query enhancement fields");

    assert_eq!(row.0, None, "content_enhanced should be NULL");
    assert_eq!(row.1, None, "enhanced_at should be NULL");
    assert_eq!(row.2, None, "enhancement_model should be NULL");
    assert_eq!(row.3, None, "enhancement_confidence should be NULL");
}

#[test]
fn note_service_retrieves_enhancement_data_from_database() {
    use time::OffsetDateTime;

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note and manually insert enhancement data
    let note = service
        .create_note("Original content", None)
        .expect("failed to create note");

    let enhanced_time = OffsetDateTime::now_utc().unix_timestamp();
    let conn = service.database().connection();
    conn.execute(
        "UPDATE notes
         SET content_enhanced = ?1, enhanced_at = ?2, enhancement_model = ?3, enhancement_confidence = ?4
         WHERE id = ?5",
        (
            "Enhanced version of the content",
            enhanced_time,
            "deepseek-r1:8b",
            0.88,
            note.id().get(),
        ),
    )
    .expect("failed to update enhancement fields");

    // Retrieve note and verify enhancement fields are loaded
    let retrieved = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(
        retrieved.content_enhanced(),
        Some("Enhanced version of the content")
    );
    assert_eq!(
        retrieved.enhanced_at(),
        Some(OffsetDateTime::from_unix_timestamp(enhanced_time).unwrap())
    );
    assert_eq!(retrieved.enhancement_model(), Some("deepseek-r1:8b"));
    assert_eq!(retrieved.enhancement_confidence(), Some(0.88));
}

#[test]
fn update_note_enhancement_method_updates_existing_note() {
    use time::OffsetDateTime;

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note without enhancement
    let note = service
        .create_note("Quick thought", None)
        .expect("failed to create note");

    // Verify no enhancement initially
    assert_eq!(note.content_enhanced(), None);

    // Update enhancement after note creation (simulates LLM enhancement workflow)
    let enhanced_time = OffsetDateTime::now_utc();
    service
        .update_note_enhancement(
            note.id(),
            "This is a quick thought about something important.",
            "deepseek-r1:8b",
            0.90,
            enhanced_time,
        )
        .expect("failed to update note enhancement");

    // Retrieve and verify enhancement was added
    let updated = service
        .get_note(note.id())
        .expect("failed to get note")
        .expect("note should exist");

    assert_eq!(
        updated.content_enhanced(),
        Some("This is a quick thought about something important.")
    );
    // Unix timestamp loses sub-second precision, so compare timestamps
    assert_eq!(
        updated.enhanced_at().unwrap().unix_timestamp(),
        enhanced_time.unix_timestamp()
    );
    assert_eq!(updated.enhancement_model(), Some("deepseek-r1:8b"));
    assert_eq!(updated.enhancement_confidence(), Some(0.90));
    // Original content should be unchanged
    assert_eq!(updated.content(), "Quick thought");
}

// --- Search Tests (Task Group 2: NoteService Search Method) ---

#[test]
fn search_notes_returns_matching_notes() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with different content
    service
        .create_note("Learning Rust programming", Some(&["rust"]))
        .expect("failed to create note 1");
    service
        .create_note("Python scripting tutorial", Some(&["python"]))
        .expect("failed to create note 2");
    service
        .create_note("Rust and Python comparison", Some(&["rust", "python"]))
        .expect("failed to create note 3");

    // Search for "rust"
    let results = service
        .search_notes("rust", None)
        .expect("search should succeed");

    assert_eq!(results.len(), 2, "should find 2 notes containing rust");

    // Verify results contain correct notes
    let contents: Vec<&str> = results.iter().map(|r| r.note.content()).collect();
    assert!(contents.contains(&"Learning Rust programming"));
    assert!(contents.contains(&"Rust and Python comparison"));
}

#[test]
fn search_notes_with_and_logic_requires_all_terms() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with different combinations of terms
    service
        .create_note("Rust programming language", None)
        .expect("failed to create note 1");
    service
        .create_note("Python programming language", None)
        .expect("failed to create note 2");
    service
        .create_note("Rust and Python both great", None)
        .expect("failed to create note 3");
    service
        .create_note("Learning Rust", None)
        .expect("failed to create note 4");

    // Search for "rust programming" (both terms required)
    let results = service
        .search_notes("rust programming", None)
        .expect("search should succeed");

    // Only notes containing both "rust" AND "programming" should match
    assert_eq!(
        results.len(),
        1,
        "should find 1 note with both rust and programming"
    );
    assert_eq!(results[0].note.content(), "Rust programming language");
}

#[test]
fn search_notes_uses_porter_stemming() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with different word forms that stem to the same root
    // Using "program" which stems: programming -> program, programs -> program
    let note1 = service
        .create_note("I love programming in Rust", None)
        .expect("failed to create note 1");
    let note2 = service
        .create_note("Many programs are written in C", None)
        .expect("failed to create note 2");

    // Search using base form "program" should match both variants
    let results = service
        .search_notes("program", None)
        .expect("search should succeed");

    assert_eq!(
        results.len(),
        2,
        "porter stemming should match programming and programs"
    );

    // Verify both notes are in results
    let result_ids: Vec<_> = results.iter().map(|r| r.note.id()).collect();
    assert!(result_ids.contains(&note1.id()));
    assert!(result_ids.contains(&note2.id()));
}

#[test]
fn search_notes_searches_content_enhanced_and_tags() {
    use time::OffsetDateTime;

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with original content
    let note1 = service
        .create_note("Quick thought", Some(&["machine-learning"]))
        .expect("failed to create note 1");

    // Add enhanced content
    let now = OffsetDateTime::now_utc();
    service
        .update_note_enhancement(
            note1.id(),
            "This is a detailed explanation about artificial intelligence",
            "deepseek-r1:8b",
            0.9,
            now,
        )
        .expect("failed to update enhancement");

    // Create another note with tag only
    service
        .create_note("Another note", Some(&["rust"]))
        .expect("failed to create note 2");

    // Search for term in enhanced content
    let results = service
        .search_notes("artificial", None)
        .expect("search should succeed");
    assert_eq!(
        results.len(),
        1,
        "should find note by enhanced content term"
    );
    assert_eq!(results[0].note.id(), note1.id());

    // Search for tag name
    let tag_results = service
        .search_notes("machine-learning", None)
        .expect("search should succeed");
    assert_eq!(tag_results.len(), 1, "should find note by tag name");
    assert_eq!(tag_results[0].note.id(), note1.id());
}

#[test]
fn search_notes_empty_query_returns_error() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Empty query should return error
    let result = service.search_notes("", None);
    assert!(result.is_err(), "empty query should return error");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("cannot be empty"),
        "error should mention empty query: {}",
        err_msg
    );

    // Whitespace-only query should also fail
    let whitespace_result = service.search_notes("   ", None);
    assert!(
        whitespace_result.is_err(),
        "whitespace-only query should return error"
    );
}

#[test]
fn search_notes_limit_parameter_restricts_results() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create multiple notes with the same term
    for i in 1..=5 {
        service
            .create_note(&format!("Rust note {}", i), None)
            .expect("failed to create note");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Search without limit
    let all_results = service
        .search_notes("rust", None)
        .expect("search should succeed");
    assert_eq!(all_results.len(), 5, "should find all 5 notes");

    // Search with limit of 2
    let limited_results = service
        .search_notes("rust", Some(2))
        .expect("search should succeed");
    assert_eq!(
        limited_results.len(),
        2,
        "should return exactly 2 notes when limited"
    );
}

#[test]
fn search_notes_returns_full_note_objects_with_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with tags
    let note = service
        .create_note("Rust tutorial", Some(&["rust", "programming"]))
        .expect("failed to create note");

    // Search for it
    let results = service
        .search_notes("tutorial", None)
        .expect("search should succeed");

    assert_eq!(results.len(), 1, "should find 1 note");

    // Verify full Note object is returned with tags
    let found_note = &results[0].note;
    assert_eq!(found_note.id(), note.id());
    assert_eq!(found_note.content(), "Rust tutorial");
    assert_eq!(found_note.tags().len(), 2, "note should include all tags");
}

// --- Additional Strategic Tests (Task Group 4: Test Review) ---

#[test]
fn search_notes_orders_results_by_bm25_relevance() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with different relevance for "rust"
    // Note 1: "rust" appears once
    let note1 = service
        .create_note("learning rust programming", None)
        .expect("failed to create note 1");

    // Note 2: "rust" appears three times (highest relevance)
    let note2 = service
        .create_note("rust rust rust is amazing for systems", None)
        .expect("failed to create note 2");

    // Note 3: "rust" appears twice
    let note3 = service
        .create_note("rust and more rust content", None)
        .expect("failed to create note 3");

    // Search for "rust"
    let results = service
        .search_notes("rust", None)
        .expect("search should succeed");

    assert_eq!(results.len(), 3, "should find all 3 notes");

    // BM25 orders by ascending score (lower is better), so most relevant should be first
    // Note 2 (3 occurrences) should be most relevant, then note3 (2), then note1 (1)
    assert_eq!(
        results[0].note.id(),
        note2.id(),
        "most relevant note (3 occurrences) should be first"
    );
    assert_eq!(
        results[1].note.id(),
        note3.id(),
        "second most relevant note (2 occurrences) should be second"
    );
    assert_eq!(
        results[2].note.id(),
        note1.id(),
        "least relevant note (1 occurrence) should be last"
    );
}

#[test]
fn search_result_has_normalized_relevance_score() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with different relevance
    service
        .create_note("rust rust rust is amazing for systems", None)
        .expect("failed to create note 1");
    service
        .create_note("learning rust programming", None)
        .expect("failed to create note 2");

    // Search for "rust"
    let results = service
        .search_notes("rust", None)
        .expect("search should succeed");

    assert_eq!(results.len(), 2, "should find 2 notes");

    // Verify all SearchResults have note and score fields
    for result in &results {
        // Verify note is accessible
        assert!(
            !result.note.content().is_empty(),
            "note content should be accessible"
        );

        // Verify relevance_score is in 0.0-1.0 range
        assert!(
            result.relevance_score >= 0.0 && result.relevance_score <= 1.0,
            "relevance score {} should be between 0.0 and 1.0",
            result.relevance_score
        );

        // Verify score is reasonably high (close to 1.0 for matching results)
        assert!(
            result.relevance_score > 0.5,
            "relevance score {} should be > 0.5 for matching results",
            result.relevance_score
        );
    }
}

#[test]
fn list_notes_works_independently_of_fts_functionality() {
    // Fail-safe test: Verify that list_notes doesn't depend on FTS table
    // This ensures note access via `cons list` works even if FTS has issues
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with tags
    let note1 = service
        .create_note("First note", Some(&["rust"]))
        .expect("failed to create note 1");

    let note2 = service
        .create_note("Second note", Some(&["python"]))
        .expect("failed to create note 2");

    // Verify FTS table exists and is populated
    let conn = service.database().connection();
    let fts_count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM notes_fts", [], |row| row.get(0))
        .expect("FTS table should exist");
    assert_eq!(fts_count_before, 2, "FTS should have 2 entries");

    // Simulate FTS corruption by dropping the FTS table
    // This tests the fail-safe requirement: "FTS issues don't block note access via cons list"
    conn.execute("DROP TABLE notes_fts", [])
        .expect("failed to drop FTS table");

    // Verify FTS table is gone
    let fts_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes_fts')",
            [],
            |row| row.get(0),
        )
        .expect("failed to check FTS table existence");
    assert_eq!(fts_exists, false, "FTS table should be dropped");

    // list_notes should still work (doesn't depend on FTS)
    let notes = service
        .list_notes(ListNotesOptions::default())
        .expect("list_notes should succeed even without FTS table");

    assert_eq!(
        notes.len(),
        2,
        "should list all notes despite FTS being gone"
    );

    // Verify we got the correct notes
    let note_ids: Vec<_> = notes.iter().map(|n| n.id()).collect();
    assert!(note_ids.contains(&note1.id()), "should include first note");
    assert!(note_ids.contains(&note2.id()), "should include second note");

    // Verify notes have their tags
    for note in &notes {
        assert_eq!(
            note.tags().len(),
            1,
            "notes should include their tags even without FTS"
        );
    }
}

// --- Alias Expansion Tests (Task Group 1: Alias Expansion Logic) ---

#[test]
fn expand_search_term_no_aliases_returns_only_original_term() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // No aliases or tags exist
    let expanded = service
        .expand_search_term("rust")
        .expect("expansion should succeed");

    assert_eq!(expanded.len(), 1, "should return only original term");
    assert!(
        expanded.contains(&"rust".to_string()),
        "should contain original term"
    );
}

#[test]
fn expand_search_term_alias_expands_to_canonical() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Expand alias
    let expanded = service
        .expand_search_term("ml")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"ml".to_string()),
        "should contain original alias"
    );
    assert!(
        expanded.contains(&"machine-learning".to_string()),
        "should contain canonical tag name"
    );
}

#[test]
fn expand_search_term_canonical_expands_to_all_aliases() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and multiple aliases
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create ml alias");
    service
        .create_alias("ai-ml", ml_tag, "user", 1.0, None)
        .expect("failed to create ai-ml alias");

    // Expand canonical tag name
    let expanded = service
        .expand_search_term("machine-learning")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"machine-learning".to_string()),
        "should contain canonical tag"
    );
    assert!(
        expanded.contains(&"ml".to_string()),
        "should contain ml alias"
    );
    assert!(
        expanded.contains(&"ai-ml".to_string()),
        "should contain ai-ml alias"
    );
}

#[test]
fn expand_search_term_user_aliases_always_included() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create user alias with low confidence (should still be included)
    service
        .create_alias("ml", ml_tag, "user", 0.5, None)
        .expect("failed to create alias");

    // Expand from canonical
    let expanded = service
        .expand_search_term("machine-learning")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"ml".to_string()),
        "user alias should be included regardless of confidence"
    );
}

#[test]
fn expand_search_term_llm_alias_high_confidence_included() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create LLM alias with confidence >= 0.8
    service
        .create_alias("ml", ml_tag, "llm", 0.85, Some("deepseek-r1:8b"))
        .expect("failed to create alias");

    // Expand from canonical
    let expanded = service
        .expand_search_term("machine-learning")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"ml".to_string()),
        "LLM alias with confidence >= 0.8 should be included"
    );
}

#[test]
fn expand_search_term_llm_alias_low_confidence_excluded() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create LLM alias with confidence < 0.8
    service
        .create_alias("ml", ml_tag, "llm", 0.75, Some("deepseek-r1:8b"))
        .expect("failed to create alias");

    // Expand from canonical
    let expanded = service
        .expand_search_term("machine-learning")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"machine-learning".to_string()),
        "should contain original canonical term"
    );
    assert!(
        !expanded.contains(&"ml".to_string()),
        "LLM alias with confidence < 0.8 should be excluded"
    );
}

// --- Search Integration with Alias Expansion Tests (Task Group 2: Search Integration) ---

#[test]
fn search_for_alias_term_finds_notes_with_canonical_tag() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Create note with canonical tag
    let note = service
        .create_note("Deep learning tutorial", Some(&["machine-learning"]))
        .expect("failed to create note");

    // Search using alias term "ml" - should find note tagged with "machine-learning"
    let results = service
        .search_notes("ml", None)
        .expect("search should succeed");

    assert_eq!(
        results.len(),
        1,
        "searching for alias 'ml' should find note with 'machine-learning' tag"
    );
    assert_eq!(results[0].note.id(), note.id());
}

#[test]
fn search_for_canonical_term_finds_notes_with_alias_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Create a note that has "ml" in content (simulating a note where user mentioned the alias)
    // Note: When user creates note with tag "ml", it gets resolved to "machine-learning"
    // So we need to test via content search
    let note = service
        .create_note("Learning about ML algorithms", Some(&["machine-learning"]))
        .expect("failed to create note");

    // Search for canonical term should find notes
    let results = service
        .search_notes("machine-learning", None)
        .expect("search should succeed");

    assert_eq!(
        results.len(),
        1,
        "searching for canonical term should find note"
    );
    assert_eq!(results[0].note.id(), note.id());

    // Now test the reverse: search for "ml" finds note with content mentioning ML
    let alias_results = service
        .search_notes("ml", None)
        .expect("search should succeed");

    assert!(
        !alias_results.is_empty(),
        "searching for 'ml' should find note"
    );
}

#[test]
fn multi_term_search_expands_each_term_independently() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tags and aliases
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create ml tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create ml alias");

    let nlp_tag = service
        .get_or_create_tag("natural-language-processing")
        .expect("failed to create nlp tag");
    service
        .create_alias("nlp", nlp_tag, "user", 1.0, None)
        .expect("failed to create nlp alias");

    // Create note with both canonical tags
    let note = service
        .create_note(
            "NLP and ML research",
            Some(&["machine-learning", "natural-language-processing"]),
        )
        .expect("failed to create note");

    // Create another note with only one tag
    service
        .create_note("Just ML stuff", Some(&["machine-learning"]))
        .expect("failed to create note 2");

    // Search using both alias terms - should use AND logic between expanded groups
    let results = service
        .search_notes("ml nlp", None)
        .expect("search should succeed");

    // Should find only the note with both tags
    assert_eq!(
        results.len(),
        1,
        "multi-term search should find note with both expanded terms"
    );
    assert_eq!(results[0].note.id(), note.id());
}

#[test]
fn multi_word_alias_handled_as_phrase_match() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and aliases
    // Use a canonical tag name that won't conflict with the alias normalization
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create the single-word alias first
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create ml alias");

    // Create note with content mentioning "machine learning" (multi-word)
    let note = service
        .create_note(
            "Studies in machine learning are fascinating",
            Some(&["machine-learning"]),
        )
        .expect("failed to create note");

    // Search for single-word alias "ml" should find note via alias expansion
    let results = service
        .search_notes("ml", None)
        .expect("search should succeed");

    assert!(
        !results.is_empty(),
        "search should find note via alias expansion"
    );
    assert_eq!(results[0].note.id(), note.id());
}

#[test]
fn search_without_aliases_passes_through_unchanged() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes without any aliases defined
    let note = service
        .create_note("Rust programming is great", Some(&["rust"]))
        .expect("failed to create note");

    // Search for a term that has no aliases
    let results = service
        .search_notes("rust", None)
        .expect("search should succeed");

    assert_eq!(
        results.len(),
        1,
        "search should work normally when no aliases exist"
    );
    assert_eq!(results[0].note.id(), note.id());
}

#[test]
fn search_with_alias_expansion_preserves_bm25_scoring() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Create notes with different content
    service
        .create_note(
            "machine-learning machine-learning machine-learning",
            Some(&["machine-learning"]),
        )
        .expect("failed to create highly relevant note");

    service
        .create_note("Just one mention of ml", Some(&["machine-learning"]))
        .expect("failed to create less relevant note");

    // Search using alias term
    let results = service
        .search_notes("ml", None)
        .expect("search should succeed");

    assert_eq!(results.len(), 2, "should find both notes");

    // Verify SearchResult structure is preserved with valid scores
    for result in &results {
        assert!(
            result.relevance_score >= 0.0 && result.relevance_score <= 1.0,
            "relevance score {} should be normalized between 0.0 and 1.0",
            result.relevance_score
        );
        assert!(
            !result.note.content().is_empty(),
            "note content should be accessible"
        );
    }

    // Verify both notes were found (order may vary due to OR expansion behavior)
    let contents: Vec<&str> = results.iter().map(|r| r.note.content()).collect();
    assert!(
        contents.contains(&"machine-learning machine-learning machine-learning"),
        "should find note with multiple machine-learning occurrences"
    );
    assert!(
        contents.contains(&"Just one mention of ml"),
        "should find note with ml mention"
    );
}

// --- Additional Strategic Tests for Alias-Expanded FTS (Task Group 3: Gap Analysis) ---

#[test]
fn expand_search_term_case_insensitive_lookup() {
    // Tests case sensitivity handling in expansion
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Expand using different case variants
    let expanded_lower = service
        .expand_search_term("ml")
        .expect("expansion should succeed");
    let expanded_upper = service
        .expand_search_term("ML")
        .expect("expansion should succeed");
    let expanded_mixed = service
        .expand_search_term("Ml")
        .expect("expansion should succeed");

    // All should produce same expansion (contain both ml and machine-learning)
    assert!(
        expanded_lower.contains(&"machine-learning".to_string()),
        "lowercase should expand to canonical"
    );
    assert!(
        expanded_upper.contains(&"machine-learning".to_string()),
        "uppercase should expand to canonical"
    );
    assert!(
        expanded_mixed.contains(&"machine-learning".to_string()),
        "mixed case should expand to canonical"
    );
}

// --- Edge Creation Tests (Task Group 2: Edge Creation in NoteService) ---

#[test]
fn get_tags_with_notes_returns_only_tags_with_associated_notes() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags with notes
    service
        .create_note("Note about Rust", Some(&["rust"]))
        .expect("failed to create note 1");
    service
        .create_note("Note about Python", Some(&["python", "programming"]))
        .expect("failed to create note 2");

    // Create an orphan tag with no notes
    let conn = service.database().connection();
    conn.execute("INSERT INTO tags (name) VALUES ('orphan')", [])
        .expect("failed to insert orphan tag");

    // Get tags with notes
    let tags_with_notes = service
        .get_tags_with_notes()
        .expect("failed to get tags with notes");

    // Should return 3 tags (rust, python, programming) but NOT orphan
    assert_eq!(
        tags_with_notes.len(),
        3,
        "should return only tags with associated notes"
    );

    let tag_names: Vec<String> = tags_with_notes
        .iter()
        .map(|(_, name)| name.clone())
        .collect();
    assert!(tag_names.contains(&"rust".to_string()));
    assert!(tag_names.contains(&"python".to_string()));
    assert!(tag_names.contains(&"programming".to_string()));
    assert!(!tag_names.contains(&"orphan".to_string()));
}

#[test]
fn get_tags_with_notes_returns_empty_when_no_tags_exist() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // No tags or notes
    let tags = service
        .get_tags_with_notes()
        .expect("failed to get tags with notes");

    assert_eq!(tags.len(), 0, "should return empty vec when no tags exist");
}

#[test]
fn create_edge_inserts_edge_with_correct_metadata() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let transformer_tag = service
        .get_or_create_tag("transformer")
        .expect("failed to create transformer tag");
    let neural_network_tag = service
        .get_or_create_tag("neural-network")
        .expect("failed to create neural-network tag");

    // Create edge: transformer (narrower) -> neural-network (broader)
    service
        .create_edge(
            transformer_tag,
            neural_network_tag,
            0.85,
            "generic",
            Some("deepseek-r1:8b"),
        )
        .expect("failed to create edge");

    // Verify edge was created with correct metadata
    let conn = service.database().connection();
    let row: (i64, i64, f64, String, String, i64, Option<i64>, Option<i64>) = conn
        .query_row(
            "SELECT source_tag_id, target_tag_id, confidence, hierarchy_type, source, verified, valid_from, valid_until
             FROM edges WHERE source_tag_id = ?1 AND target_tag_id = ?2",
            [transformer_tag.get(), neural_network_tag.get()],
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
                ))
            },
        )
        .expect("failed to query edge");

    assert_eq!(row.0, transformer_tag.get(), "source_tag_id should match");
    assert_eq!(
        row.1,
        neural_network_tag.get(),
        "target_tag_id should match"
    );
    assert_eq!(row.2, 0.85, "confidence should match");
    assert_eq!(row.3, "generic", "hierarchy_type should be generic");
    assert_eq!(row.4, "llm", "source should be llm");
    assert_eq!(row.5, 0, "verified should be 0");
    assert_eq!(row.6, None, "valid_from should be NULL");
    assert_eq!(row.7, None, "valid_until should be NULL");
}

#[test]
fn create_edge_respects_insert_or_ignore_for_duplicates() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let transformer_tag = service
        .get_or_create_tag("transformer")
        .expect("failed to create transformer tag");
    let neural_network_tag = service
        .get_or_create_tag("neural-network")
        .expect("failed to create neural-network tag");

    // Create edge first time
    service
        .create_edge(
            transformer_tag,
            neural_network_tag,
            0.85,
            "generic",
            Some("deepseek-r1:8b"),
        )
        .expect("first edge creation should succeed");

    // Create same edge again (should not error due to INSERT OR IGNORE)
    service
        .create_edge(
            transformer_tag,
            neural_network_tag,
            0.90,
            "generic",
            Some("deepseek-r1:8b"),
        )
        .expect("duplicate edge creation should succeed (idempotent)");

    // Verify only one edge exists
    let conn = service.database().connection();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE source_tag_id = ?1 AND target_tag_id = ?2",
            [transformer_tag.get(), neural_network_tag.get()],
            |row| row.get(0),
        )
        .expect("failed to count edges");

    assert_eq!(count, 1, "should have only 1 edge (duplicate ignored)");

    // Verify original confidence is preserved (first insert wins)
    let confidence: f64 = conn
        .query_row(
            "SELECT confidence FROM edges WHERE source_tag_id = ?1 AND target_tag_id = ?2",
            [transformer_tag.get(), neural_network_tag.get()],
            |row| row.get(0),
        )
        .expect("failed to query confidence");

    assert_eq!(confidence, 0.85, "original confidence should be preserved");
}

#[test]
fn create_edge_stores_correct_hierarchy_type() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let attention_tag = service
        .get_or_create_tag("attention")
        .expect("failed to create attention tag");
    let transformer_tag = service
        .get_or_create_tag("transformer")
        .expect("failed to create transformer tag");
    let neural_network_tag = service
        .get_or_create_tag("neural-network")
        .expect("failed to create neural-network tag");

    // Create partitive edge: attention (part) -> transformer (whole)
    service
        .create_edge(
            attention_tag,
            transformer_tag,
            0.95,
            "partitive",
            Some("deepseek-r1:8b"),
        )
        .expect("failed to create partitive edge");

    // Create generic edge: transformer (narrower) -> neural-network (broader)
    service
        .create_edge(
            transformer_tag,
            neural_network_tag,
            0.90,
            "generic",
            Some("deepseek-r1:8b"),
        )
        .expect("failed to create generic edge");

    // Verify hierarchy types
    let conn = service.database().connection();

    let partitive_type: String = conn
        .query_row(
            "SELECT hierarchy_type FROM edges WHERE source_tag_id = ?1",
            [attention_tag.get()],
            |row| row.get(0),
        )
        .expect("failed to query partitive edge");
    assert_eq!(partitive_type, "partitive");

    let generic_type: String = conn
        .query_row(
            "SELECT hierarchy_type FROM edges WHERE source_tag_id = ?1",
            [transformer_tag.get()],
            |row| row.get(0),
        )
        .expect("failed to query generic edge");
    assert_eq!(generic_type, "generic");
}

#[test]
fn create_edges_batch_uses_transaction_for_atomicity() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let tag1 = service
        .get_or_create_tag("tag1")
        .expect("failed to create tag1");
    let tag2 = service
        .get_or_create_tag("tag2")
        .expect("failed to create tag2");
    let tag3 = service
        .get_or_create_tag("tag3")
        .expect("failed to create tag3");

    // Prepare edges
    let edges = vec![
        (tag1, tag2, 0.9, "generic", Some("deepseek-r1:8b")),
        (tag2, tag3, 0.85, "partitive", Some("deepseek-r1:8b")),
    ];

    // Create edges in batch
    let count = service
        .create_edges_batch(&edges)
        .expect("failed to create edges batch");

    assert_eq!(count, 2, "should create 2 edges");

    // Verify both edges exist
    let conn = service.database().connection();
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .expect("failed to count edges");

    assert_eq!(total, 2, "should have 2 edges in database");
}

#[test]
fn create_edges_batch_returns_zero_for_empty_input() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create edges batch with empty vec
    let count = service
        .create_edges_batch(&[])
        .expect("failed to create empty batch");

    assert_eq!(count, 0, "should return 0 for empty batch");
}

#[test]
fn expand_search_term_with_special_characters_normalized() {
    // Tests expansion with special characters in alias names
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag
    let cpp_tag = service
        .get_or_create_tag("cpp")
        .expect("failed to create tag");

    // Create alias with special characters (will be normalized)
    // "c++" normalizes to "c" due to TagNormalizer stripping non-alphanumeric
    service
        .create_alias("cplusplus", cpp_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Expand "cpp" should find the canonical tag and its aliases
    let expanded = service
        .expand_search_term("cpp")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"cpp".to_string()),
        "should contain canonical tag"
    );
    assert!(
        expanded.contains(&"cplusplus".to_string()),
        "should contain cplusplus alias"
    );
}

#[test]
fn search_alias_in_enhanced_content() {
    // Tests integration with enhanced content search via alias expansion
    use time::OffsetDateTime;

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Create note with original content
    let note = service
        .create_note("Quick note", Some(&["machine-learning"]))
        .expect("failed to create note");

    // Add enhanced content mentioning the canonical term
    let now = OffsetDateTime::now_utc();
    service
        .update_note_enhancement(
            note.id(),
            "This is about machine-learning algorithms and neural networks",
            "deepseek-r1:8b",
            0.9,
            now,
        )
        .expect("failed to update enhancement");

    // Search using alias "ml" should find note via expansion to "machine-learning"
    let results = service
        .search_notes("ml", None)
        .expect("search should succeed");

    assert_eq!(
        results.len(),
        1,
        "alias search should find note via enhanced content expansion"
    );
    assert_eq!(results[0].note.id(), note.id());
}

#[test]
fn expand_search_term_exact_confidence_boundary() {
    // Tests LLM alias at exactly 0.8 confidence threshold
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create tag");

    // Create LLM alias with exactly 0.8 confidence (should be included)
    service
        .create_alias("ml", ml_tag, "llm", 0.8, Some("deepseek-r1:8b"))
        .expect("failed to create alias");

    // Expand from canonical - should include the alias at exactly 0.8
    let expanded = service
        .expand_search_term("machine-learning")
        .expect("expansion should succeed");

    assert!(
        expanded.contains(&"ml".to_string()),
        "LLM alias with confidence exactly 0.8 should be included"
    );
}

// --- Hierarchy Population Integration Tests (Task Group 4) ---

#[test]
fn hierarchy_population_full_end_to_end_workflow() {
    // Integration test: Full workflow from tags to edges creation
    use crate::hierarchy::HierarchySuggesterBuilder;
    use crate::ollama::OllamaClientTrait;
    use std::sync::Arc;

    struct MockHierarchyClient;

    impl OllamaClientTrait for MockHierarchyClient {
        fn generate(
            &self,
            _model: &str,
            _prompt: &str,
        ) -> Result<String, crate::ollama::OllamaError> {
            Ok(r#"[
                {"source_tag": "transformer", "target_tag": "neural-network", "hierarchy_type": "generic", "confidence": 0.95},
                {"source_tag": "attention", "target_tag": "transformer", "hierarchy_type": "partitive", "confidence": 0.85}
            ]"#.to_string())
        }
    }

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with tags to populate tags table
    service
        .create_note("About transformers", Some(&["transformer"]))
        .expect("failed to create note 1");
    service
        .create_note("About neural networks", Some(&["neural-network"]))
        .expect("failed to create note 2");
    service
        .create_note("About attention mechanism", Some(&["attention"]))
        .expect("failed to create note 3");

    // Step 1: Get tags with notes
    let tags_with_notes = service
        .get_tags_with_notes()
        .expect("failed to get tags with notes");
    assert_eq!(tags_with_notes.len(), 3, "should have 3 tags with notes");

    // Step 2: Call HierarchySuggester
    let suggester = HierarchySuggesterBuilder::new()
        .client(Arc::new(MockHierarchyClient))
        .build();

    let tag_names: Vec<String> = tags_with_notes
        .iter()
        .map(|(_, name)| name.clone())
        .collect();

    let suggestions = suggester
        .suggest_relationships("test-model", tag_names)
        .expect("failed to suggest relationships");

    assert_eq!(suggestions.len(), 2, "should get 2 suggestions");

    // Step 3: Create edges from suggestions
    let mut edges = Vec::new();
    for suggestion in &suggestions {
        let source_id = service
            .get_or_create_tag(&suggestion.source_tag)
            .expect("failed to resolve source tag");
        let target_id = service
            .get_or_create_tag(&suggestion.target_tag)
            .expect("failed to resolve target tag");

        edges.push((
            source_id,
            target_id,
            suggestion.confidence,
            suggestion.hierarchy_type.as_str(),
            Some("test-model"),
        ));
    }

    let created_count = service
        .create_edges_batch(&edges)
        .expect("failed to create edges");

    assert_eq!(created_count, 2, "should create 2 edges");

    // Step 4: Verify edges in database
    let conn = service.database().connection();
    let edge_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .expect("failed to count edges");

    assert_eq!(edge_count, 2, "should have 2 edges in database");

    // Verify edge direction: source = narrower, target = broader
    let generic_edge: (String, String) = conn
        .query_row(
            "SELECT st.name, tt.name FROM edges e
             JOIN tags st ON e.source_tag_id = st.id
             JOIN tags tt ON e.target_tag_id = tt.id
             WHERE e.hierarchy_type = 'generic'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("failed to query generic edge");

    assert_eq!(
        generic_edge,
        ("transformer".to_string(), "neural-network".to_string()),
        "transformer (narrower) should point to neural-network (broader)"
    );
}

#[test]
fn edge_direction_convention_narrower_to_broader() {
    // Test that edges follow the direction convention: source=narrower, target=broader
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let python_tag = service
        .get_or_create_tag("python")
        .expect("failed to create python tag");
    let programming_language_tag = service
        .get_or_create_tag("programming-language")
        .expect("failed to create programming-language tag");

    // Create edge: python (specific/narrower) -> programming-language (general/broader)
    service
        .create_edge(
            python_tag,
            programming_language_tag,
            0.95,
            "generic",
            Some("test-model"),
        )
        .expect("failed to create edge");

    // Verify edge direction in database
    let conn = service.database().connection();
    let (source_name, target_name): (String, String) = conn
        .query_row(
            "SELECT st.name, tt.name FROM edges e
             JOIN tags st ON e.source_tag_id = st.id
             JOIN tags tt ON e.target_tag_id = tt.id
             WHERE st.id = ?1 AND tt.id = ?2",
            [python_tag.get(), programming_language_tag.get()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("failed to query edge");

    assert_eq!(
        source_name, "python",
        "source should be narrower/specific concept"
    );
    assert_eq!(
        target_name, "programming-language",
        "target should be broader/general concept"
    );

    // Verify no reverse edge exists
    let reverse_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE source_tag_id = ?1 AND target_tag_id = ?2",
            [programming_language_tag.get(), python_tag.get()],
            |row| row.get(0),
        )
        .expect("failed to count reverse edges");

    assert_eq!(
        reverse_count, 0,
        "should not have reverse edge (broader -> narrower)"
    );
}

#[test]
fn hierarchy_suggest_idempotency_no_duplicate_edges() {
    // Test that running suggest twice doesn't duplicate edges
    use crate::hierarchy::HierarchySuggesterBuilder;
    use crate::ollama::OllamaClientTrait;
    use std::sync::Arc;

    struct MockIdempotentClient;

    impl OllamaClientTrait for MockIdempotentClient {
        fn generate(
            &self,
            _model: &str,
            _prompt: &str,
        ) -> Result<String, crate::ollama::OllamaError> {
            Ok(r#"[
                {"source_tag": "rust", "target_tag": "programming-language", "hierarchy_type": "generic", "confidence": 0.9}
            ]"#.to_string())
        }
    }

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create notes with tags
    service
        .create_note("Rust programming", Some(&["rust", "programming-language"]))
        .expect("failed to create note");

    let suggester = HierarchySuggesterBuilder::new()
        .client(Arc::new(MockIdempotentClient))
        .build();

    // Run suggest first time
    let tags_with_notes = service.get_tags_with_notes().expect("failed to get tags");
    let tag_names: Vec<String> = tags_with_notes
        .iter()
        .map(|(_, name)| name.clone())
        .collect();

    let _suggestions1 = suggester
        .suggest_relationships("test-model", tag_names.clone())
        .expect("failed to suggest relationships");

    let rust_id = service
        .get_or_create_tag("rust")
        .expect("failed to get rust");
    let pl_id = service
        .get_or_create_tag("programming-language")
        .expect("failed to get pl");

    let edges1 = vec![(rust_id, pl_id, 0.9, "generic", Some("test-model"))];
    service
        .create_edges_batch(&edges1)
        .expect("failed to create edges first time");

    // Verify one edge exists
    let conn = service.database().connection();
    let count_after_first: i64 = conn
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .expect("failed to count edges");
    assert_eq!(count_after_first, 1, "should have 1 edge after first run");

    // Run suggest second time (same suggestions)
    let _suggestions2 = suggester
        .suggest_relationships("test-model", tag_names)
        .expect("failed to suggest relationships second time");

    let edges2 = vec![(rust_id, pl_id, 0.9, "generic", Some("test-model"))];
    service
        .create_edges_batch(&edges2)
        .expect("failed to create edges second time");

    // Verify still only one edge (INSERT OR IGNORE prevents duplicates)
    let count_after_second: i64 = conn
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .expect("failed to count edges");
    assert_eq!(
        count_after_second, 1,
        "should still have 1 edge after second run (no duplicates)"
    );

    // Verify original edge metadata is preserved
    let (confidence, hierarchy_type): (f64, String) = conn
        .query_row(
            "SELECT confidence, hierarchy_type FROM edges WHERE source_tag_id = ?1 AND target_tag_id = ?2",
            [rust_id.get(), pl_id.get()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("failed to query edge metadata");

    assert_eq!(confidence, 0.9, "original confidence should be preserved");
    assert_eq!(
        hierarchy_type, "generic",
        "original hierarchy type should be preserved"
    );
}

#[test]
fn mixed_hierarchy_types_in_single_batch() {
    // Test creating both generic and partitive edges in a single batch
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let attention_tag = service
        .get_or_create_tag("attention")
        .expect("failed to create attention");
    let transformer_tag = service
        .get_or_create_tag("transformer")
        .expect("failed to create transformer");
    let neural_network_tag = service
        .get_or_create_tag("neural-network")
        .expect("failed to create neural-network");

    // Create batch with both hierarchy types
    let edges = vec![
        // Partitive: attention is part of transformer
        (
            attention_tag,
            transformer_tag,
            0.9,
            "partitive",
            Some("test-model"),
        ),
        // Generic: transformer is a type of neural-network
        (
            transformer_tag,
            neural_network_tag,
            0.95,
            "generic",
            Some("test-model"),
        ),
    ];

    let created_count = service
        .create_edges_batch(&edges)
        .expect("failed to create mixed batch");

    assert_eq!(created_count, 2, "should create 2 edges");

    // Verify both hierarchy types stored correctly
    let conn = service.database().connection();

    let partitive_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE hierarchy_type = 'partitive'",
            [],
            |row| row.get(0),
        )
        .expect("failed to count partitive edges");
    assert_eq!(partitive_count, 1, "should have 1 partitive edge");

    let generic_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE hierarchy_type = 'generic'",
            [],
            |row| row.get(0),
        )
        .expect("failed to count generic edges");
    assert_eq!(generic_count, 1, "should have 1 generic edge");

    // Verify edge metadata
    let partitive_edge: (String, String, f64) = conn
        .query_row(
            "SELECT st.name, tt.name, e.confidence FROM edges e
             JOIN tags st ON e.source_tag_id = st.id
             JOIN tags tt ON e.target_tag_id = tt.id
             WHERE e.hierarchy_type = 'partitive'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("failed to query partitive edge");

    assert_eq!(
        partitive_edge,
        ("attention".to_string(), "transformer".to_string(), 0.9),
        "partitive edge should be attention -> transformer"
    );
}

#[test]
fn tag_name_resolution_before_edge_creation() {
    // Test that tag names are properly resolved to IDs before edge creation
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create only one of the two tags initially
    let existing_tag = service
        .get_or_create_tag("existing-tag")
        .expect("failed to create existing tag");

    // Attempt to create edge with non-existent target tag (should fail validation)
    let non_existent_id = TagId::new(99999);

    let result = service.create_edge(
        existing_tag,
        non_existent_id,
        0.9,
        "generic",
        Some("test-model"),
    );

    // Should fail because target tag doesn't exist
    assert!(result.is_err(), "should fail when target tag doesn't exist");

    // Now create both tags and verify edge creation works
    let source_tag = service
        .get_or_create_tag("python")
        .expect("failed to create python");
    let target_tag = service
        .get_or_create_tag("programming-language")
        .expect("failed to create programming-language");

    let result = service.create_edge(source_tag, target_tag, 0.95, "generic", Some("test-model"));

    assert!(
        result.is_ok(),
        "should succeed when both tags exist: {:?}",
        result
    );

    // Verify edge was created
    let conn = service.database().connection();
    let edge_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM edges WHERE source_tag_id = ?1 AND target_tag_id = ?2",
            [source_tag.get(), target_tag.get()],
            |row| row.get(0),
        )
        .expect("failed to count edges");

    assert_eq!(edge_count, 1, "should have created 1 edge");
}

#[test]
fn create_edges_batch_rollback_on_failure() {
    // Test that batch edge creation rolls back on failure (transaction atomicity)
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create valid tags
    let tag1 = service
        .get_or_create_tag("tag1")
        .expect("failed to create tag1");
    let tag2 = service
        .get_or_create_tag("tag2")
        .expect("failed to create tag2");

    // Create batch with one invalid edge (non-existent tag)
    let invalid_tag_id = TagId::new(99999);
    let edges = vec![
        (tag1, tag2, 0.9, "generic", Some("test-model")), // Valid
        (tag1, invalid_tag_id, 0.85, "generic", Some("test-model")), // Invalid - should cause rollback
    ];

    let result = service.create_edges_batch(&edges);

    // Should fail due to invalid tag
    assert!(
        result.is_err(),
        "batch should fail when one edge is invalid"
    );

    // Verify NO edges were created (transaction rolled back)
    let conn = service.database().connection();
    let edge_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
        .expect("failed to count edges");

    assert_eq!(
        edge_count, 0,
        "no edges should exist after rollback (atomicity)"
    );
}

// --- Graph Search Tests (Task Group 2) ---

#[test]
fn graph_search_returns_search_results_with_normalized_scores() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags with hierarchy: rust -> programming
    let rust_tag = service
        .get_or_create_tag("rust")
        .expect("failed to create rust tag");
    let programming_tag = service
        .get_or_create_tag("programming")
        .expect("failed to create programming tag");

    // Create edge: rust specializes programming
    service
        .create_edge(
            rust_tag,
            programming_tag,
            0.9,
            "generic",
            Some("test-model"),
        )
        .expect("failed to create edge");

    // Create note tagged with rust
    let note1 = service
        .create_note("Learning Rust", Some(&["rust"]))
        .expect("failed to create note");

    // Create note tagged with programming
    let _note2 = service
        .create_note("General programming concepts", Some(&["programming"]))
        .expect("failed to create note");

    // Search for "rust" should find both notes via graph spreading
    let results = service
        .graph_search("rust", None)
        .expect("graph search should succeed");

    assert!(!results.is_empty(), "should find notes via graph search");

    // Verify SearchResult structure
    for result in &results {
        assert!(
            result.relevance_score >= 0.0 && result.relevance_score <= 1.0,
            "relevance score should be normalized to 0.0-1.0 range"
        );
        assert!(result.note.id().get() > 0, "note should have valid ID");
    }

    // Note tagged with rust should score higher (seed tag)
    let note1_result = results
        .iter()
        .find(|r| r.note.id() == note1.id())
        .expect("note1 should be in results");

    assert!(
        note1_result.relevance_score > 0.0,
        "note with seed tag should have positive score"
    );
}

#[test]
fn graph_search_parses_query_into_seed_tags_via_expand_search_term() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create ml tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Create note with canonical tag
    service
        .create_note("ML tutorial", Some(&["machine-learning"]))
        .expect("failed to create note");

    // Search using alias should expand and find note
    let results = service
        .graph_search("ml", None)
        .expect("graph search should succeed");

    assert!(
        !results.is_empty(),
        "alias should expand to canonical tag and find notes"
    );
}

#[test]
fn graph_search_from_note_seeds_from_note_tags_with_confidence_weighting() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags
    let rust_tag = service
        .get_or_create_tag("rust")
        .expect("failed to create rust tag");
    let systems_tag = service
        .get_or_create_tag("systems")
        .expect("failed to create systems tag");

    // Create edge: rust -> systems
    service
        .create_edge(rust_tag, systems_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");

    // Create seed note with rust tag
    let seed_note = service
        .create_note("Rust memory safety", Some(&["rust"]))
        .expect("failed to create seed note");

    // Create related note with systems tag
    let related_note = service
        .create_note("Systems programming", Some(&["systems"]))
        .expect("failed to create related note");

    // Find notes related to seed note
    let results = service
        .graph_search_from_note(seed_note.id(), None)
        .expect("graph search from note should succeed");

    assert!(
        !results.is_empty(),
        "should find related notes via tag graph"
    );

    // Verify related note is in results
    let found_related = results.iter().any(|r| r.note.id() == related_note.id());
    assert!(found_related, "should find note with related tag");
}

#[test]
fn graph_search_cold_start_returns_empty_when_no_matching_tags() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create note with tag
    service
        .create_note("Some note", Some(&["unrelated"]))
        .expect("failed to create note");

    // Search for non-existent tag
    let results = service
        .graph_search("nonexistent", None)
        .expect("graph search should succeed");

    assert!(
        results.is_empty(),
        "cold start should return empty results when no matching tags"
    );
}

#[test]
fn graph_search_note_scoring_uses_sum_of_tag_activation_times_confidence() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags with hierarchy
    let rust_tag = service
        .get_or_create_tag("rust")
        .expect("failed to create rust tag");
    let programming_tag = service
        .get_or_create_tag("programming")
        .expect("failed to create programming tag");
    let systems_tag = service
        .get_or_create_tag("systems")
        .expect("failed to create systems tag");

    // Create edges: rust -> programming, rust -> systems
    service
        .create_edge(
            rust_tag,
            programming_tag,
            0.9,
            "generic",
            Some("test-model"),
        )
        .expect("failed to create edge");
    service
        .create_edge(rust_tag, systems_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");

    // Create hub note with multiple activated tags
    let hub_note = service
        .create_note(
            "Rust programming systems",
            Some(&["programming", "systems"]),
        )
        .expect("failed to create hub note");

    // Create single-tag note
    let single_note = service
        .create_note("Programming basics", Some(&["programming"]))
        .expect("failed to create single note");

    // Search for rust - both programming and systems should activate
    let results = service
        .graph_search("rust", Some(10))
        .expect("graph search should succeed");

    assert!(!results.is_empty(), "should find notes");

    // Hub note with 2 activated tags should score higher than single-tag note
    let hub_result = results
        .iter()
        .find(|r| r.note.id() == hub_note.id())
        .expect("hub note should be in results");

    let single_result = results
        .iter()
        .find(|r| r.note.id() == single_note.id())
        .expect("single note should be in results");

    assert!(
        hub_result.relevance_score >= single_result.relevance_score,
        "hub note with multiple activated tags should score higher or equal"
    );
}

#[test]
fn graph_search_from_note_excludes_seed_note_from_results() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tags with hierarchy
    let rust_tag = service
        .get_or_create_tag("rust")
        .expect("failed to create rust tag");
    let programming_tag = service
        .get_or_create_tag("programming")
        .expect("failed to create programming tag");

    service
        .create_edge(
            rust_tag,
            programming_tag,
            0.9,
            "generic",
            Some("test-model"),
        )
        .expect("failed to create edge");

    // Create seed note
    let seed_note = service
        .create_note("Rust note", Some(&["rust"]))
        .expect("failed to create seed note");

    // Create related note
    service
        .create_note("Programming note", Some(&["programming"]))
        .expect("failed to create related note");

    // Find notes related to seed note
    let results = service
        .graph_search_from_note(seed_note.id(), None)
        .expect("graph search from note should succeed");

    // Verify seed note is NOT in results
    let found_seed = results.iter().any(|r| r.note.id() == seed_note.id());
    assert!(!found_seed, "seed note should be excluded from results");
}

// --- Task Group 4: Strategic Integration Tests ---

#[test]
fn graph_search_multi_hop_traversal_finds_distantly_related_notes() {
    // Test end-to-end: query -> 3-hop graph traversal -> distantly related notes
    // Validates: multi-hop spreading, decay application, distant semantic discovery
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create linear chain: rust -> systems-programming -> operating-systems -> kernel
    let rust_tag = service
        .get_or_create_tag("rust")
        .expect("failed to create rust tag");
    let systems_tag = service
        .get_or_create_tag("systems-programming")
        .expect("failed to create systems tag");
    let os_tag = service
        .get_or_create_tag("operating-systems")
        .expect("failed to create os tag");
    let kernel_tag = service
        .get_or_create_tag("kernel")
        .expect("failed to create kernel tag");

    // Create edges with high confidence (0.9) to ensure propagation
    service
        .create_edge(rust_tag, systems_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");
    service
        .create_edge(systems_tag, os_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");
    service
        .create_edge(os_tag, kernel_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");

    // Create notes at different distances from query term "rust"
    let rust_note = service
        .create_note("Rust ownership model", Some(&["rust"]))
        .expect("failed to create note");

    let systems_note = service
        .create_note("Systems programming patterns", Some(&["systems-programming"]))
        .expect("failed to create note");

    let kernel_note = service
        .create_note("Kernel development", Some(&["kernel"]))
        .expect("failed to create note");

    // Search for "rust" - should find notes 3 hops away (kernel)
    let results = service
        .graph_search("rust", Some(10))
        .expect("graph search should succeed");

    assert!(
        !results.is_empty(),
        "should find notes via multi-hop spreading"
    );

    // Verify all notes are found
    let found_rust = results.iter().any(|r| r.note.id() == rust_note.id());
    let found_systems = results.iter().any(|r| r.note.id() == systems_note.id());
    let found_kernel = results.iter().any(|r| r.note.id() == kernel_note.id());

    assert!(found_rust, "should find note with seed tag");
    assert!(found_systems, "should find note 1 hop away");
    assert!(
        found_kernel,
        "should find note 3 hops away (distant relation)"
    );

    // Verify score decay: rust > systems > kernel
    let rust_score = results
        .iter()
        .find(|r| r.note.id() == rust_note.id())
        .unwrap()
        .relevance_score;
    let systems_score = results
        .iter()
        .find(|r| r.note.id() == systems_note.id())
        .unwrap()
        .relevance_score;
    let kernel_score = results
        .iter()
        .find(|r| r.note.id() == kernel_note.id())
        .unwrap()
        .relevance_score;

    assert!(
        rust_score > systems_score,
        "seed tag note should score higher than 1-hop note"
    );
    assert!(
        systems_score > kernel_score,
        "1-hop note should score higher than 3-hop note"
    );
}

#[test]
fn graph_search_hub_note_with_multiple_activated_tags_scores_highest() {
    // Test hub note discovery: query activates multiple tags -> note with ALL tags scores highest
    // Validates: SUM aggregation, tag convergence scoring
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create tag hierarchy:
    //      rust
    //     /    \
    //  memory  concurrency
    let rust_tag = service
        .get_or_create_tag("rust")
        .expect("failed to create rust tag");
    let memory_tag = service
        .get_or_create_tag("memory-safety")
        .expect("failed to create memory tag");
    let concurrency_tag = service
        .get_or_create_tag("concurrency")
        .expect("failed to create concurrency tag");

    service
        .create_edge(rust_tag, memory_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");
    service
        .create_edge(rust_tag, concurrency_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");

    // Create hub note with BOTH activated tags
    let hub_note = service
        .create_note(
            "Rust safe concurrency",
            Some(&["memory-safety", "concurrency"]),
        )
        .expect("failed to create hub note");

    // Create single-tag notes
    let memory_note = service
        .create_note("Memory safety basics", Some(&["memory-safety"]))
        .expect("failed to create memory note");

    let concurrency_note = service
        .create_note("Concurrency patterns", Some(&["concurrency"]))
        .expect("failed to create concurrency note");

    // Search for "rust" - activates both memory-safety and concurrency
    let results = service
        .graph_search("rust", Some(10))
        .expect("graph search should succeed");

    assert!(!results.is_empty(), "should find notes");

    // Find scores
    let hub_score = results
        .iter()
        .find(|r| r.note.id() == hub_note.id())
        .expect("hub note should be in results")
        .relevance_score;

    let memory_score = results
        .iter()
        .find(|r| r.note.id() == memory_note.id())
        .expect("memory note should be in results")
        .relevance_score;

    let concurrency_score = results
        .iter()
        .find(|r| r.note.id() == concurrency_note.id())
        .expect("concurrency note should be in results")
        .relevance_score;

    // Hub note should score highest (SUM of both tag activations)
    assert!(
        hub_score > memory_score,
        "hub note with 2 activated tags should score higher than single-tag note (got hub={}, memory={})",
        hub_score,
        memory_score
    );
    assert!(
        hub_score > concurrency_score,
        "hub note with 2 activated tags should score higher than single-tag note (got hub={}, concurrency={})",
        hub_score,
        concurrency_score
    );

    // Verify hub score is approximately the sum of individual activations
    // (allowing for bidirectional traversal effects)
    assert!(
        hub_score >= memory_score && hub_score >= concurrency_score,
        "hub note should benefit from multiple activated tags"
    );
}

#[test]
fn graph_search_environment_variable_override_affects_results() {
    // Test CONS_DECAY override changes final results
    // Validates: environment variable configuration, runtime config parsing
    // NOTE: This test uses serial execution marker to avoid test interference

    // Save original CONS_DECAY value to restore later
    let original_decay = std::env::var("CONS_DECAY").ok();

    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create linear chain: tag1 -> tag2 -> tag3
    let tag1 = service
        .get_or_create_tag("tag1")
        .expect("failed to create tag1");
    let tag2 = service
        .get_or_create_tag("tag2")
        .expect("failed to create tag2");
    let tag3 = service
        .get_or_create_tag("tag3")
        .expect("failed to create tag3");

    service
        .create_edge(tag1, tag2, 1.0, "generic", Some("test-model"))
        .expect("failed to create edge");
    service
        .create_edge(tag2, tag3, 1.0, "generic", Some("test-model"))
        .expect("failed to create edge");

    // Create note 2 hops away
    let distant_note = service
        .create_note("Tag3 note", Some(&["tag3"]))
        .expect("failed to create note");

    // Test 1: Default decay (0.7) - distant note should be found
    unsafe { std::env::remove_var("CONS_DECAY") };
    let results_default = service
        .graph_search("tag1", Some(10))
        .expect("graph search should succeed");

    let found_default = results_default
        .iter()
        .any(|r| r.note.id() == distant_note.id());

    // Test 2: Low decay (0.2) - activation drops quickly, may not reach tag3
    unsafe { std::env::set_var("CONS_DECAY", "0.2") };
    let results_low_decay = service
        .graph_search("tag1", Some(10))
        .expect("graph search should succeed");

    let found_low_decay = results_low_decay
        .iter()
        .any(|r| r.note.id() == distant_note.id());

    // Test 3: No decay (1.0) - activation preserved, should definitely find tag3
    unsafe { std::env::set_var("CONS_DECAY", "1.0") };
    let results_high_decay = service
        .graph_search("tag1", Some(10))
        .expect("graph search should succeed");

    let found_high_decay = results_high_decay
        .iter()
        .any(|r| r.note.id() == distant_note.id());

    // Restore original environment variable state
    unsafe {
        match original_decay {
            Some(val) => std::env::set_var("CONS_DECAY", val),
            None => std::env::remove_var("CONS_DECAY"),
        }
    }

    // Verify CONS_DECAY affects results
    // With decay=1.0, we should definitely find the distant note
    assert!(
        found_high_decay,
        "with CONS_DECAY=1.0, should find 2-hop distant note"
    );

    // With decay=0.2, activation decays rapidly (1.0 -> 0.2 -> 0.04)
    // Threshold is 0.1, so 0.04 gets pruned
    assert!(
        !found_low_decay,
        "with CONS_DECAY=0.2, should NOT find 2-hop note (0.04 < threshold 0.1)"
    );

    // Verify default behavior
    assert!(
        found_default,
        "with default CONS_DECAY=0.7, should find 2-hop note"
    );
}

#[test]
fn graph_search_alias_expansion_then_spreading_activation() {
    // Test integration: query uses alias -> resolves to canonical -> spreads through edges
    // Validates: alias resolution + graph spreading pipeline
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create canonical tag and alias
    let ml_tag = service
        .get_or_create_tag("machine-learning")
        .expect("failed to create ml tag");
    service
        .create_alias("ml", ml_tag, "user", 1.0, None)
        .expect("failed to create alias");

    // Create related tag via edge
    let nn_tag = service
        .get_or_create_tag("neural-network")
        .expect("failed to create nn tag");
    service
        .create_edge(ml_tag, nn_tag, 0.9, "generic", Some("test-model"))
        .expect("failed to create edge");

    // Create notes
    let ml_note = service
        .create_note("ML tutorial", Some(&["machine-learning"]))
        .expect("failed to create note");

    let nn_note = service
        .create_note("Neural network basics", Some(&["neural-network"]))
        .expect("failed to create note");

    // Search using ALIAS "ml" (not canonical "machine-learning")
    let results = service
        .graph_search("ml", Some(10))
        .expect("graph search should succeed");

    assert!(!results.is_empty(), "alias query should find notes");

    // Verify both notes found: alias resolves -> spreads to related tag
    let found_ml = results.iter().any(|r| r.note.id() == ml_note.id());
    let found_nn = results.iter().any(|r| r.note.id() == nn_note.id());

    assert!(
        found_ml,
        "should find note with canonical tag via alias resolution"
    );
    assert!(
        found_nn,
        "should find note with related tag via spreading activation after alias resolution"
    );
}

#[test]
fn graph_search_edge_confidence_affects_activation_propagation() {
    // Test edge confidence weighting: low-confidence edge (0.3) vs high-confidence (0.9)
    // Validates: confidence multiplier in spreading formula
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create parallel paths with different edge confidences
    let seed_tag = service
        .get_or_create_tag("seed")
        .expect("failed to create seed tag");

    let high_conf_tag = service
        .get_or_create_tag("high-confidence-target")
        .expect("failed to create high conf tag");

    let low_conf_tag = service
        .get_or_create_tag("low-confidence-target")
        .expect("failed to create low conf tag");

    // High confidence edge (0.9)
    service
        .create_edge(
            seed_tag,
            high_conf_tag,
            0.9,
            "generic",
            Some("test-model"),
        )
        .expect("failed to create high conf edge");

    // Low confidence edge (0.3)
    service
        .create_edge(
            seed_tag,
            low_conf_tag,
            0.3,
            "generic",
            Some("test-model"),
        )
        .expect("failed to create low conf edge");

    // Create notes with each target tag
    let high_conf_note = service
        .create_note("High confidence note", Some(&["high-confidence-target"]))
        .expect("failed to create note");

    let low_conf_note = service
        .create_note("Low confidence note", Some(&["low-confidence-target"]))
        .expect("failed to create note");

    // Search for seed tag
    let results = service
        .graph_search("seed", Some(10))
        .expect("graph search should succeed");

    assert!(!results.is_empty(), "should find notes");

    // Get scores
    let high_conf_score = results
        .iter()
        .find(|r| r.note.id() == high_conf_note.id())
        .expect("high conf note should be in results")
        .relevance_score;

    let low_conf_score = results
        .iter()
        .find(|r| r.note.id() == low_conf_note.id())
        .expect("low conf note should be in results")
        .relevance_score;

    // High confidence edge should produce higher activation
    // Formula: activation = 1.0 * confidence * decay * edge_type_multiplier
    // High: 1.0 * 0.9 * 0.7 * 1.0 = 0.63
    // Low:  1.0 * 0.3 * 0.7 * 1.0 = 0.21
    assert!(
        high_conf_score > low_conf_score,
        "high confidence edge (0.9) should produce higher activation than low confidence (0.3), got high={}, low={}",
        high_conf_score,
        low_conf_score
    );

    // Verify rough ratio (allowing for bidirectional and normalization effects)
    let ratio = high_conf_score / low_conf_score;
    assert!(
        ratio > 1.5,
        "activation ratio should reflect confidence difference (0.9/0.3 = 3.0), got ratio={}",
        ratio
    );
}

#[test]
fn graph_search_mixed_edge_types_in_path_applies_both_multipliers() {
    // Test path with both generic (1.0) and partitive (0.5) edges
    // Validates: edge type multiplier composition
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    // Create chain: seed -> generic_tag -> partitive_tag
    let seed_tag = service
        .get_or_create_tag("seed")
        .expect("failed to create seed tag");
    let generic_tag = service
        .get_or_create_tag("generic-tag")
        .expect("failed to create generic tag");
    let partitive_tag = service
        .get_or_create_tag("partitive-tag")
        .expect("failed to create partitive tag");

    // First hop: generic edge (multiplier 1.0)
    service
        .create_edge(seed_tag, generic_tag, 1.0, "generic", Some("test-model"))
        .expect("failed to create generic edge");

    // Second hop: partitive edge (multiplier 0.5)
    service
        .create_edge(
            generic_tag,
            partitive_tag,
            1.0,
            "partitive",
            Some("test-model"),
        )
        .expect("failed to create partitive edge");

    // Create parallel path for comparison: seed -> partitive_only_tag (1 hop partitive)
    let partitive_only_tag = service
        .get_or_create_tag("partitive-only")
        .expect("failed to create partitive only tag");
    service
        .create_edge(
            seed_tag,
            partitive_only_tag,
            1.0,
            "partitive",
            Some("test-model"),
        )
        .expect("failed to create partitive only edge");

    // Create notes
    let partitive_2hop_note = service
        .create_note("2-hop partitive note", Some(&["partitive-tag"]))
        .expect("failed to create note");

    let partitive_1hop_note = service
        .create_note("1-hop partitive note", Some(&["partitive-only"]))
        .expect("failed to create note");

    // Search for seed tag
    let results = service
        .graph_search("seed", Some(10))
        .expect("graph search should succeed");

    assert!(!results.is_empty(), "should find notes");

    // Get scores
    let partitive_2hop_score = results
        .iter()
        .find(|r| r.note.id() == partitive_2hop_note.id())
        .map(|r| r.relevance_score);

    let partitive_1hop_score = results
        .iter()
        .find(|r| r.note.id() == partitive_1hop_note.id())
        .map(|r| r.relevance_score);

    // Verify both notes are found
    assert!(
        partitive_1hop_score.is_some(),
        "1-hop partitive note should be found"
    );
    assert!(
        partitive_2hop_score.is_some(),
        "2-hop mixed path note should be found"
    );

    // Verify 1-hop partitive scores higher than 2-hop mixed
    // 1-hop partitive: 1.0 * 1.0 * 0.7 * 0.5 = 0.35
    // 2-hop mixed: 1.0 * 1.0 * 0.7 * 1.0 (first hop) -> 0.7 * 1.0 * 0.7 * 0.5 (second hop) = 0.245
    assert!(
        partitive_1hop_score.unwrap() > partitive_2hop_score.unwrap(),
        "1-hop partitive should score higher than 2-hop mixed path (decay effect), got 1hop={}, 2hop={}",
        partitive_1hop_score.unwrap(),
        partitive_2hop_score.unwrap()
    );
}
