//! Integration tests for tag-alias CLI commands.
//!
//! These tests verify the end-to-end workflows for managing tag aliases
//! through the CLI interface.

use anyhow::Result;
use cons::{Database, NoteService};

#[test]
fn test_tag_alias_add_creates_alias_successfully() -> Result<()> {
    use cons::autotagger::TagNormalizer;

    // Arrange: Create in-memory database and service
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Simulate CLI command: cons tag-alias add ml machine-learning
    let alias = "ml";
    let canonical = "machine-learning";

    // Act: Execute tag-alias add command logic
    let normalized_alias = TagNormalizer::normalize_tag(alias);
    let normalized_canonical = TagNormalizer::normalize_tag(canonical);

    // Get or create canonical tag
    let canonical_tag_id = service.get_or_create_tag(&normalized_canonical)?;

    // Create the alias with user source
    service.create_alias(&normalized_alias, canonical_tag_id, "user", 1.0, None)?;

    // Assert: Alias was created and can be resolved
    let resolved = service.resolve_alias(&normalized_alias)?;
    assert!(resolved.is_some(), "alias should be created");
    assert_eq!(
        resolved.unwrap(),
        canonical_tag_id,
        "alias should resolve to canonical tag"
    );

    Ok(())
}

#[test]
fn test_tag_alias_add_with_non_existent_canonical_creates_tag_first() -> Result<()> {
    use cons::autotagger::TagNormalizer;

    // Arrange: Create in-memory database and service
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Verify canonical tag doesn't exist yet
    let conn = service.database().connection();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tags WHERE name = 'machine-learning'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(count, 0, "canonical tag should not exist before add");

    // Act: Execute tag-alias add (should create canonical tag)
    let alias = "ml";
    let canonical = "machine-learning";

    let normalized_alias = TagNormalizer::normalize_tag(alias);
    let normalized_canonical = TagNormalizer::normalize_tag(canonical);

    let canonical_tag_id = service.get_or_create_tag(&normalized_canonical)?;
    service.create_alias(&normalized_alias, canonical_tag_id, "user", 1.0, None)?;

    // Assert: Canonical tag now exists
    let count_after: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tags WHERE name = 'machine-learning'",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        count_after, 1,
        "canonical tag should exist after add command"
    );

    Ok(())
}

#[test]
fn test_tag_alias_list_displays_aliases_correctly() -> Result<()> {
    // Arrange: Create database with multiple aliases
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create canonical tags
    let ml_tag = service.get_or_create_tag("machine-learning")?;
    let ai_tag = service.get_or_create_tag("artificial-intelligence")?;

    // Create aliases (simulating multiple tag-alias add commands)
    service.create_alias("ml", ml_tag, "user", 1.0, None)?;
    service.create_alias("ai", ai_tag, "user", 1.0, None)?;
    service.create_alias(
        "machine-learning-abbrev",
        ml_tag,
        "llm",
        0.85,
        Some("deepseek-r1:8b"),
    )?;

    // Act: Execute tag-alias list command
    let aliases = service.list_aliases()?;

    // Assert: All 3 aliases returned
    assert_eq!(aliases.len(), 3, "should return 3 aliases");

    // Verify aliases are present
    let alias_names: Vec<&str> = aliases.iter().map(|a| a.alias()).collect();
    assert!(alias_names.contains(&"ml"), "should contain ml alias");
    assert!(alias_names.contains(&"ai"), "should contain ai alias");
    assert!(
        alias_names.contains(&"machine-learning-abbrev"),
        "should contain machine-learning-abbrev alias"
    );

    // Verify sources and confidence
    let ml_alias = aliases.iter().find(|a| a.alias() == "ml").unwrap();
    assert_eq!(ml_alias.source(), "user");
    assert_eq!(ml_alias.confidence(), 1.0);

    let llm_alias = aliases
        .iter()
        .find(|a| a.alias() == "machine-learning-abbrev")
        .unwrap();
    assert_eq!(llm_alias.source(), "llm");
    assert_eq!(llm_alias.confidence(), 0.85);
    assert_eq!(llm_alias.model_version(), Some("deepseek-r1:8b"));

    Ok(())
}

#[test]
fn test_tag_alias_remove_deletes_alias() -> Result<()> {
    use cons::autotagger::TagNormalizer;

    // Arrange: Create database with an alias
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    let canonical_tag_id = service.get_or_create_tag("machine-learning")?;
    service.create_alias("ml", canonical_tag_id, "user", 1.0, None)?;

    // Verify alias exists
    assert!(
        service.resolve_alias("ml")?.is_some(),
        "alias should exist before removal"
    );

    // Act: Execute tag-alias remove command
    let alias = "ml";
    let normalized_alias = TagNormalizer::normalize_tag(alias);
    service.remove_alias(&normalized_alias)?;

    // Assert: Alias no longer exists
    assert!(
        service.resolve_alias("ml")?.is_none(),
        "alias should not exist after removal"
    );

    Ok(())
}

#[test]
fn test_cons_add_with_alias_resolves_to_canonical() -> Result<()> {
    // This is an E2E test simulating: cons add --tags ml "note content"
    // where "ml" is an alias for "machine-learning"

    // Arrange: Create database with an alias
    let db = Database::in_memory()?;
    let service = NoteService::new(db);

    // Create canonical tag and alias (simulating prior tag-alias add)
    let canonical_tag_id = service.get_or_create_tag("machine-learning")?;
    service.create_alias("ml", canonical_tag_id, "user", 1.0, None)?;

    // Act: User adds a note with tag "ml" (which is an alias)
    // Simulates: cons add --tags ml "Learning about ML algorithms"
    let note = service.create_note("Learning about ML algorithms", Some(&["ml"]))?;

    // Assert: Note is tagged with the canonical tag "machine-learning", not "ml"
    assert_eq!(note.tags().len(), 1, "note should have 1 tag");

    // Verify the tag is the canonical one
    let conn = service.database().connection();
    let tag_name: String = conn.query_row(
        "SELECT name FROM tags WHERE id = ?1",
        [note.tags()[0].tag_id().get()],
        |row| row.get(0),
    )?;

    assert_eq!(
        tag_name, "machine-learning",
        "tag should be canonical form, not alias"
    );

    Ok(())
}
