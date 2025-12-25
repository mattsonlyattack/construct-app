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

    let notes = service
        .list_notes(options)
        .expect("failed to list notes");

    assert_eq!(notes.len(), 2, "should find both notes with normalized tag");
}

#[test]
fn create_note_with_duplicate_tag_variants_creates_single_normalized_tag() {
    let db = Database::in_memory().expect("failed to create in-memory database");
    let service = NoteService::new(db);

    let note = service
        .create_note("Test note", Some(&["Rust", "RUST", "rust"]))
        .expect("failed to create note");

    assert_eq!(note.tags().len(), 1, "duplicate variants should create one tag");

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
    assert_eq!(tag_names, vec!["c", "nodejs"], "special chars should be stripped");
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

    assert_eq!(count, 1, "user and LLM tags should deduplicate to same normalized tag");

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
    let notes = service
        .list_notes(options)
        .expect("failed to list notes");
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

