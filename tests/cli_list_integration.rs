use anyhow::Result;
use cons::{Database, ListNotesOptions, NoteService};
use rusqlite::OptionalExtension;

/// Helper function that mimics the core logic of the list command.
///
/// This is used for integration testing without invoking the full CLI.
fn list_notes(
    limit: Option<usize>,
    tags: Option<&str>,
    service: &NoteService,
) -> Result<Vec<(i64, String, Vec<String>)>> {
    // Apply default limit of 10 when not specified
    let limit = limit.unwrap_or(10);

    // Parse tags if provided
    let parsed_tags = tags.map(|t| {
        t.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect::<Vec<String>>()
    });

    // Build options for list_notes - use Descending to get newest N notes
    use cons::SortOrder;
    let options = ListNotesOptions {
        limit: Some(limit),
        tags: parsed_tags,
        order: SortOrder::Descending,
    };

    // Retrieve notes (newest first from DB)
    let mut notes = service.list_notes(options)?;

    // Reverse to display oldest-first (newest last) - matches CLI behavior
    notes.reverse();

    // Convert notes to a simplified format for testing
    let mut results = Vec::new();
    for note in notes {
        let note_id = note.id().get();
        let content = note.content().to_string();

        // Get tag names
        let mut tag_names = Vec::new();
        for tag_assignment in note.tags() {
            // Query tag name directly from database
            let conn = service.database().connection();
            let tag_name: Option<String> = conn
                .query_row(
                    "SELECT name FROM tags WHERE id = ?1",
                    [tag_assignment.tag_id().get()],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(name) = tag_name {
                tag_names.push(name);
            }
        }

        results.push((note_id, content, tag_names));
    }

    Ok(results)
}

#[test]
fn test_list_full_workflow_with_multiple_notes() -> Result<()> {
    // Arrange: Create in-memory database and add multiple notes
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create notes with various tag combinations
    service.create_note("First note about Rust", Some(&["rust", "programming"]))?;
    service.create_note("Second note about learning", Some(&["learning"]))?;
    service.create_note("Third note with no tags", None)?;
    service.create_note(
        "Fourth note about Rust tutorials",
        Some(&["rust", "tutorial"]),
    )?;

    // Act: List all notes with default limit
    let results = list_notes(None, None, &service)?;

    // Assert: All 4 notes are returned (default limit is 10)
    assert_eq!(results.len(), 4, "should return 4 notes");

    // Verify notes are ordered oldest-first (newest last) for chronological display
    assert_eq!(results[0].1, "First note about Rust");
    assert_eq!(results[1].1, "Second note about learning");
    assert_eq!(results[2].1, "Third note with no tags");
    assert_eq!(results[3].1, "Fourth note about Rust tutorials");

    // Verify tag associations
    assert_eq!(results[0].2, vec!["rust", "programming"]);
    assert_eq!(results[1].2, vec!["learning"]);
    assert!(results[2].2.is_empty());
    assert_eq!(results[3].2, vec!["rust", "tutorial"]);

    Ok(())
}

#[test]
fn test_list_respects_limit_flag() -> Result<()> {
    // Arrange: Create in-memory database with 5 notes
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    for i in 1..=5 {
        service.create_note(&format!("Note number {}", i), None)?;
    }

    // Act: List with limit of 3
    let results = list_notes(Some(3), None, &service)?;

    // Assert: Only 3 notes returned
    assert_eq!(results.len(), 3, "should return exactly 3 notes");

    // Verify the 3 newest notes are returned, displayed oldest-first (newest last)
    assert_eq!(results[0].1, "Note number 3"); // oldest of the 3 newest
    assert_eq!(results[1].1, "Note number 4");
    assert_eq!(results[2].1, "Note number 5"); // newest

    Ok(())
}

#[test]
fn test_list_tags_filter_applies_correctly() -> Result<()> {
    // Arrange: Create in-memory database with notes having different tags
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create notes with various tag combinations
    service.create_note("Rust only", Some(&["rust"]))?;
    service.create_note("Programming only", Some(&["programming"]))?;
    service.create_note("Rust and programming", Some(&["rust", "programming"]))?;
    service.create_note(
        "Rust, programming, and tutorial",
        Some(&["rust", "programming", "tutorial"]),
    )?;
    service.create_note("No tags", None)?;

    // Act: Filter by tags "rust,programming" (AND logic - both tags required)
    let results = list_notes(Some(10), Some("rust,programming"), &service)?;

    // Assert: Only notes with BOTH rust AND programming tags are returned
    assert_eq!(results.len(), 2, "should return 2 notes with both tags");

    // Verify the correct notes are returned (oldest first, newest last)
    assert_eq!(results[0].1, "Rust and programming"); // older
    assert_eq!(results[1].1, "Rust, programming, and tutorial"); // newer

    // Verify tag associations
    assert!(results[0].2.contains(&"rust".to_string()));
    assert!(results[0].2.contains(&"programming".to_string()));
    assert!(results[1].2.contains(&"rust".to_string()));
    assert!(results[1].2.contains(&"programming".to_string()));

    Ok(())
}

#[test]
fn test_list_with_combined_short_flags() -> Result<()> {
    // Arrange: Create in-memory database with tagged notes
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create 5 notes, some with "rust" tag
    service.create_note("Rust note 1", Some(&["rust"]))?;
    service.create_note("Python note", Some(&["python"]))?;
    service.create_note("Rust note 2", Some(&["rust", "programming"]))?;
    service.create_note("Rust note 3", Some(&["rust"]))?;
    service.create_note("Rust note 4", Some(&["rust", "tutorial"]))?;

    // Act: List with limit=3 and tags=rust (simulating `cons list -l 3 -t rust`)
    let results = list_notes(Some(3), Some("rust"), &service)?;

    // Assert: Maximum 3 notes with "rust" tag are returned
    assert_eq!(results.len(), 3, "should return at most 3 notes");

    // Verify all returned notes have the "rust" tag
    for (_, _, tags) in &results {
        assert!(
            tags.contains(&"rust".to_string()),
            "all notes should have rust tag"
        );
    }

    // Verify oldest-first ordering (newest last) - the 3 newest rust notes displayed chronologically
    assert_eq!(results[0].1, "Rust note 2"); // oldest of the 3 newest
    assert_eq!(results[1].1, "Rust note 3");
    assert_eq!(results[2].1, "Rust note 4"); // newest

    Ok(())
}

#[test]
fn test_list_with_empty_database() -> Result<()> {
    // Arrange: Create in-memory database with no notes
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Act: List notes
    let results = list_notes(None, None, &service)?;

    // Assert: Empty results
    assert!(results.is_empty(), "should return no notes");

    Ok(())
}

#[test]
fn test_list_default_limit_is_ten() -> Result<()> {
    // Arrange: Create in-memory database with 15 notes
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    for i in 1..=15 {
        service.create_note(&format!("Note {}", i), None)?;
    }

    // Act: List without specifying limit (should default to 10)
    let results = list_notes(None, None, &service)?;

    // Assert: Exactly 10 notes returned
    assert_eq!(results.len(), 10, "default limit should be 10");

    Ok(())
}

#[test]
fn test_list_tags_filter_with_no_matching_notes() -> Result<()> {
    // Arrange: Create in-memory database with notes but none matching the filter
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    service.create_note("Rust note", Some(&["rust"]))?;
    service.create_note("Python note", Some(&["python"]))?;

    // Act: Filter by non-existent tag
    let results = list_notes(Some(10), Some("javascript"), &service)?;

    // Assert: No notes returned
    assert!(
        results.is_empty(),
        "should return no notes for non-matching tag"
    );

    Ok(())
}
