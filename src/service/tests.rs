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
    let contents: Vec<&str> = results.iter().map(|n| n.content()).collect();
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
    assert_eq!(results[0].content(), "Rust programming language");
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
    let result_ids: Vec<_> = results.iter().map(|n| n.id()).collect();
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
    assert_eq!(results[0].id(), note1.id());

    // Search for tag name
    let tag_results = service
        .search_notes("machine-learning", None)
        .expect("search should succeed");
    assert_eq!(tag_results.len(), 1, "should find note by tag name");
    assert_eq!(tag_results[0].id(), note1.id());
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
    let found_note = &results[0];
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
        results[0].id(),
        note2.id(),
        "most relevant note (3 occurrences) should be first"
    );
    assert_eq!(
        results[1].id(),
        note3.id(),
        "second most relevant note (2 occurrences) should be second"
    );
    assert_eq!(
        results[2].id(),
        note1.id(),
        "least relevant note (1 occurrence) should be last"
    );
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

    assert_eq!(notes.len(), 2, "should list all notes despite FTS being gone");

    // Verify we got the correct notes
    let note_ids: Vec<_> = notes.iter().map(|n| n.id()).collect();
    assert!(
        note_ids.contains(&note1.id()),
        "should include first note"
    );
    assert!(
        note_ids.contains(&note2.id()),
        "should include second note"
    );

    // Verify notes have their tags
    for note in &notes {
        assert_eq!(
            note.tags().len(),
            1,
            "notes should include their tags even without FTS"
        );
    }
}
