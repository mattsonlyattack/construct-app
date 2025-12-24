//! Architecture Validation Integration Tests
//!
//! This test file validates the layered architecture by confirming that NoteService
//! and all library types can be used independently of CLI dependencies (clap, dirs),
//! proving reusability for future TUI/GUI interfaces.
//!
//! **Critical Architecture Invariant:**
//! This file must NOT import anything from main.rs or CLI modules.
//! It must only use types exported from the `cons::` crate root.
//!
//! **CLI types that should NOT be exported from crate root:**
//! - Cli (clap command parser)
//! - Commands (clap subcommands enum)
//! - AddCommand (add subcommand)
//! - ListCommand (list subcommand)
//!
//! These types are intentionally kept in main.rs to maintain clean library boundaries.

use anyhow::Result;
use cons::{
    Database, ListNotesOptions, Note, NoteBuilder, NoteId, NoteService, Tag, TagAssignment, TagId,
    TagSource,
};

// =============================================================================
// Task Group 1: Integration Test File Setup
// =============================================================================

/// Helper function for NoteService instantiation.
///
/// Uses `Database::in_memory()` for test isolation.
/// Returns a `NoteService` instance ready for testing with no CLI context dependencies.
fn create_test_service() -> NoteService {
    let db = Database::in_memory().expect("failed to create in-memory database");
    NoteService::new(db)
}

// =============================================================================
// Task Group 2: NoteService Isolation Verification
// =============================================================================

#[test]
fn test_noteservice_instantiates_without_cli_context() {
    // Arrange: Create in-memory database directly (no CLI setup)
    let db = Database::in_memory().expect("failed to create in-memory database");

    // Act: Instantiate NoteService without any CLI context
    let service = NoteService::new(db);

    // Assert: Service is created and database accessor works
    let db_ref = service.database();
    let conn = db_ref.connection();

    // Verify schema is initialized (proves database is functional)
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        )
        .expect("failed to query schema");

    assert!(
        count >= 3,
        "expected at least 3 tables (notes, tags, note_tags), found {}",
        count
    );
}

#[test]
fn test_noteservice_database_accessor_returns_valid_reference() {
    // Arrange: Create service using helper
    let service = create_test_service();

    // Act: Access underlying database via service.database()
    let db_ref = service.database();
    let conn = db_ref.connection();

    // Assert: Verify connection can execute simple query
    let result: i64 = conn
        .query_row("SELECT 1", [], |row| row.get(0))
        .expect("failed to execute simple query");

    assert_eq!(result, 1, "simple query should return 1");
}

// =============================================================================
// Task Group 3: Core CRUD Operations Validation
// =============================================================================

#[test]
fn test_create_note_content_only() -> Result<()> {
    // Arrange
    let service = create_test_service();

    // Act: Create note with content, no tags
    let note = service.create_note("A thought captured without tags", None)?;

    // Assert: Note ID is positive
    assert!(
        note.id().get() > 0,
        "note ID should be positive, got {}",
        note.id().get()
    );

    // Assert: Content matches input
    assert_eq!(note.content(), "A thought captured without tags");

    // Assert: No tags
    assert!(note.tags().is_empty(), "note should have no tags");

    Ok(())
}

#[test]
fn test_create_note_with_tags() -> Result<()> {
    // Arrange
    let service = create_test_service();

    // Act: Create note with content and tags
    let note = service.create_note(
        "Learning about Rust ownership",
        Some(&["rust", "ownership"]),
    )?;

    // Assert: Note has expected tag count
    assert_eq!(note.tags().len(), 2, "note should have 2 tags");

    // Assert: Tags are user-sourced with 100% confidence
    for tag_assignment in note.tags() {
        assert!(
            tag_assignment.source().is_user(),
            "tags should be user-sourced"
        );
        assert_eq!(
            tag_assignment.confidence(),
            100,
            "user tags should have 100% confidence"
        );
    }

    Ok(())
}

#[test]
fn test_get_note_existing() -> Result<()> {
    // Arrange: Create service and a note
    let service = create_test_service();
    let created = service.create_note("Note to retrieve", Some(&["test"]))?;

    // Act: Retrieve note by ID
    let retrieved = service.get_note(created.id())?.expect("note should exist");

    // Assert: All fields match
    assert_eq!(retrieved.id(), created.id(), "IDs should match");
    assert_eq!(
        retrieved.content(),
        created.content(),
        "content should match"
    );
    assert_eq!(
        retrieved.created_at(),
        created.created_at(),
        "created_at should match"
    );
    assert_eq!(
        retrieved.updated_at(),
        created.updated_at(),
        "updated_at should match"
    );
    assert_eq!(
        retrieved.tags().len(),
        created.tags().len(),
        "tag count should match"
    );

    Ok(())
}

#[test]
fn test_get_note_nonexistent_returns_none() -> Result<()> {
    // Arrange
    let service = create_test_service();

    // Act: Query for non-existent NoteId
    let result = service.get_note(NoteId::new(99999))?;

    // Assert: Returns None (not error)
    assert_eq!(result, None, "non-existent note should return None");

    Ok(())
}

#[test]
fn test_delete_note_removes_note() -> Result<()> {
    // Arrange: Create service and a note
    let service = create_test_service();
    let note = service.create_note("Note to be deleted", None)?;
    let note_id = note.id();

    // Verify note exists
    let exists_before = service.get_note(note_id)?.is_some();
    assert!(exists_before, "note should exist before deletion");

    // Act: Delete the note
    service.delete_note(note_id)?;

    // Assert: get_note returns None after deletion
    let exists_after = service.get_note(note_id)?;
    assert_eq!(exists_after, None, "note should not exist after deletion");

    Ok(())
}

// =============================================================================
// Task Group 4: Tag Operations Validation
// =============================================================================

#[test]
fn test_add_tags_with_user_source() -> Result<()> {
    // Arrange: Create note without tags
    let service = create_test_service();
    let note = service.create_note("Note for user tagging", None)?;

    // Act: Add tags with TagSource::User
    service.add_tags_to_note(note.id(), &["rust", "learning"], TagSource::User)?;

    // Assert: Retrieve note and verify tags present
    let retrieved = service.get_note(note.id())?.expect("note should exist");

    assert_eq!(retrieved.tags().len(), 2, "note should have 2 tags");

    // Assert: Source is user, confidence is 100%
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

    Ok(())
}

#[test]
fn test_add_tags_with_llm_source() -> Result<()> {
    // Arrange: Create note without tags
    let service = create_test_service();
    let note = service.create_note("Note for LLM tagging", None)?;

    // Act: Add tags with TagSource::llm() including model name and confidence value
    let llm_source = TagSource::llm("deepseek-r1:8b", 85);
    service.add_tags_to_note(note.id(), &["ai", "machine-learning"], llm_source)?;

    // Assert: Retrieve note and verify tag metadata persists correctly
    let retrieved = service.get_note(note.id())?.expect("note should exist");

    assert_eq!(retrieved.tags().len(), 2, "note should have 2 tags");

    // Assert: Tags have correct LLM metadata
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

    Ok(())
}

// =============================================================================
// Task Group 5: List Operations Validation
// =============================================================================

#[test]
fn test_list_notes_default_options() -> Result<()> {
    // Arrange: Create multiple notes
    let service = create_test_service();
    service.create_note("First note", None)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    service.create_note("Second note", None)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    service.create_note("Third note", None)?;

    // Act: List with ListNotesOptions::default()
    let notes = service.list_notes(ListNotesOptions::default())?;

    // Assert: All notes returned in correct order (newest first)
    assert_eq!(notes.len(), 3, "should return all 3 notes");
    assert_eq!(
        notes[0].content(),
        "Third note",
        "newest note should be first"
    );
    assert_eq!(notes[1].content(), "Second note", "second note in middle");
    assert_eq!(
        notes[2].content(),
        "First note",
        "oldest note should be last"
    );

    Ok(())
}

#[test]
fn test_list_notes_with_limit() -> Result<()> {
    // Arrange: Create 5 notes
    let service = create_test_service();
    for i in 1..=5 {
        service.create_note(&format!("Note {}", i), None)?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Act: List with limit: Some(2)
    let options = ListNotesOptions {
        limit: Some(2),
        ..Default::default()
    };
    let notes = service.list_notes(options)?;

    // Assert: Exactly 2 notes returned (the most recent)
    assert_eq!(notes.len(), 2, "should return exactly 2 notes");
    assert_eq!(notes[0].content(), "Note 5", "should be most recent note");
    assert_eq!(notes[1].content(), "Note 4", "should be second most recent");

    Ok(())
}

#[test]
fn test_list_notes_with_tags_filter() -> Result<()> {
    // Arrange: Create notes with various tag combinations
    let service = create_test_service();

    // Note 1: only rust
    service.create_note("Rust only", Some(&["rust"]))?;

    // Note 2: rust AND programming
    let note2 = service.create_note("Rust and programming", Some(&["rust", "programming"]))?;

    // Note 3: rust AND programming AND tutorial
    let note3 = service.create_note(
        "Rust programming tutorial",
        Some(&["rust", "programming", "tutorial"]),
    )?;

    // Note 4: only programming
    service.create_note("Programming only", Some(&["programming"]))?;

    // Note 5: no tags
    service.create_note("No tags at all", None)?;

    // Act: Filter by tags (AND logic - must have BOTH rust AND programming)
    let options = ListNotesOptions {
        tags: Some(vec!["rust".to_string(), "programming".to_string()]),
        ..Default::default()
    };
    let notes = service.list_notes(options)?;

    // Assert: AND logic works correctly - only notes with BOTH tags returned
    assert_eq!(
        notes.len(),
        2,
        "should return only notes with ALL specified tags"
    );

    let note_ids: Vec<i64> = notes.iter().map(|n| n.id().get()).collect();
    assert!(
        note_ids.contains(&note2.id().get()),
        "should include note2 (rust + programming)"
    );
    assert!(
        note_ids.contains(&note3.id().get()),
        "should include note3 (rust + programming + tutorial)"
    );

    // Assert: Returned notes include their tag assignments
    for note in &notes {
        assert!(
            !note.tags().is_empty(),
            "returned notes should include tag assignments"
        );
    }

    Ok(())
}

// =============================================================================
// Task Group 6: Public API Cleanliness Check
// =============================================================================

#[test]
fn test_all_required_types_accessible_from_crate_root() {
    // This test verifies that all required types are accessible from `cons::` crate root.
    // If any of these types were not exported, this test would fail to compile.

    // --- Infrastructure types ---

    // Database: core storage layer
    let db = Database::in_memory().expect("Database::in_memory should work");

    // NoteService: business logic layer
    let service = NoteService::new(db);

    // ListNotesOptions: query options struct
    let _options = ListNotesOptions::default();
    let _options_with_limit = ListNotesOptions {
        limit: Some(10),
        ..Default::default()
    };
    let _options_with_tags = ListNotesOptions {
        tags: Some(vec!["rust".to_string()]),
        ..Default::default()
    };

    // --- Domain types ---

    // Note: core entity (via NoteBuilder)
    let note_via_builder: Note = NoteBuilder::new()
        .id(NoteId::new(1))
        .content("test content")
        .build();
    assert_eq!(note_via_builder.id(), NoteId::new(1));

    // NoteBuilder: builder pattern for Note
    let _builder = NoteBuilder::new();

    // NoteId: typed ID wrapper
    let note_id = NoteId::new(42);
    assert_eq!(note_id.get(), 42);

    // Tag: tag entity
    let tag = Tag::new(TagId::new(1), "test-tag");
    assert_eq!(tag.name(), "test-tag");

    // TagId: typed ID wrapper
    let tag_id = TagId::new(99);
    assert_eq!(tag_id.get(), 99);

    // TagSource: user vs LLM source
    let user_source = TagSource::User;
    assert!(user_source.is_user());

    let llm_source = TagSource::llm("model", 75);
    assert!(llm_source.is_llm());
    assert_eq!(llm_source.confidence(), 75);

    // TagAssignment: tag-note relationship with metadata
    let now = time::OffsetDateTime::now_utc();
    let user_assignment = TagAssignment::user(TagId::new(1), now);
    assert_eq!(user_assignment.confidence(), 100);

    let llm_assignment = TagAssignment::llm(TagId::new(2), "deepseek-r1:8b", 85, now);
    assert_eq!(llm_assignment.confidence(), 85);
    assert_eq!(llm_assignment.model(), Some("deepseek-r1:8b"));

    // --- Verify service operations work with these types ---

    // Create a note and verify types work together
    let created_note = service
        .create_note(
            "Integration test note",
            Some(&["architecture", "validation"]),
        )
        .expect("create_note should work");

    assert!(created_note.id().get() > 0);
    assert!(!created_note.tags().is_empty());

    // Verify list operations return proper types
    let notes: Vec<Note> = service
        .list_notes(ListNotesOptions::default())
        .expect("list_notes should work");
    assert!(!notes.is_empty());

    // Success: All required types are accessible and functional from crate root
}

// =============================================================================
// Documentation: CLI types that should NOT be exported
// =============================================================================
//
// The following types are intentionally NOT exported from the `cons::` crate root.
// They are CLI-specific and belong in main.rs only:
//
// - Cli: The clap command parser struct with #[derive(Parser)]
// - Commands: The clap subcommands enum with #[derive(Subcommand)]
// - AddCommand: The add subcommand struct with clap attributes
// - ListCommand: The list subcommand struct with clap attributes
//
// This separation ensures the library can be used by TUI, GUI, or other interfaces
// without pulling in CLI dependencies (clap, dirs).
//
// Verification: This test file compiles and runs successfully without importing
// any CLI types, proving the public API is clean.
// =============================================================================
