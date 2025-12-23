use anyhow::Result;
use cons::{Database, NoteService};

/// Helper function that mimics the core logic of the add command.
///
/// This is used for integration testing without invoking the full CLI.
fn add_note(content: &str, tags: Option<&str>, db: Database) -> Result<(i64, Vec<String>)> {
    let service = NoteService::new(db);

    // Parse tags if provided
    let parsed_tags = tags.map(|t| {
        t.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect::<Vec<String>>()
    });

    // Create note with optional tags
    let note = if let Some(ref tags) = parsed_tags {
        let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
        service.create_note(content, Some(&tag_refs))
    } else {
        service.create_note(content, None)
    }?;

    let note_id = note.id().get();
    let tags_created = parsed_tags.unwrap_or_default();

    Ok((note_id, tags_created))
}

#[test]
fn test_add_note_without_tags() -> Result<()> {
    // Arrange: Create in-memory database
    let db = Database::in_memory()?;

    // Act: Add a note without tags
    let (note_id, tags) = add_note("This is a test note", None, db)?;

    // Assert: Note created successfully
    assert_eq!(note_id, 1); // First note should have ID 1
    assert!(tags.is_empty());

    Ok(())
}

#[test]
fn test_add_note_with_tags() -> Result<()> {
    // Arrange: Create in-memory database
    let db = Database::in_memory()?;

    // Act: Add a note with tags
    let (note_id, tags) = add_note("Learning Rust", Some("rust,learning"), db)?;

    // Assert: Note created successfully with tags
    assert_eq!(note_id, 1);
    assert_eq!(tags, vec!["rust", "learning"]);

    Ok(())
}

#[test]
fn test_add_note_verifies_persistence() -> Result<()> {
    // Arrange: Create in-memory database
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Act: Add a note
    let note = service.create_note("Persistent note", Some(&["test"]))?;
    let note_id = note.id();

    // Retrieve the note to verify persistence
    let retrieved_note = service.get_note(note_id)?;

    // Assert: Note was persisted correctly
    assert!(retrieved_note.is_some());
    let retrieved_note = retrieved_note.unwrap();
    assert_eq!(retrieved_note.content(), "Persistent note");

    Ok(())
}

#[test]
fn test_add_multiple_notes() -> Result<()> {
    // Arrange: Create in-memory database and service
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Act: Add multiple notes using the service directly
    let note1 = service.create_note("First note", None)?;
    let note2 = service.create_note("Second note", Some(&["tag1"]))?;
    let note3 = service.create_note("Third note", Some(&["tag1", "tag2"]))?;

    // Assert: Each note gets a unique ID
    assert_eq!(note1.id().get(), 1);
    assert_eq!(note2.id().get(), 2);
    assert_eq!(note3.id().get(), 3);

    Ok(())
}
