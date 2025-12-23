use crate::{Database, Note, NoteBuilder, NoteId, TagAssignment, TagId, TagSource};
use anyhow::Result;
use rusqlite::OptionalExtension;
use time::OffsetDateTime;

/// Service layer providing note management operations.
///
/// NoteService owns a Database instance and provides high-level business logic
/// for working with notes, tags, and their relationships. This service is
/// UI-independent and can be used by CLI, TUI, or future GUI interfaces.
///
/// # Examples
///
/// ```
/// use cons::{Database, NoteService};
///
/// # fn main() -> anyhow::Result<()> {
/// let db = Database::in_memory()?;
/// let service = NoteService::new(db);
/// # Ok(())
/// # }
/// ```
pub struct NoteService {
    db: Database,
}

impl NoteService {
    /// Creates a new NoteService with the given database.
    ///
    /// Takes ownership of the database instance. The service becomes the sole
    /// owner and manages all database operations through its methods.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Returns a reference to the underlying database.
    ///
    /// Useful for testing or advanced operations that need direct database access.
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Creates a new note with the given content and optional tags.
    ///
    /// Inserts the note into the database with current Unix timestamps
    /// for both `created_at` and `updated_at`. Returns the fully populated
    /// `Note` with its assigned `NoteId`.
    ///
    /// # Arguments
    ///
    /// * `content` - The note's text content
    /// * `tags` - Optional tag names to associate with the note
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// let note = service.create_note("My first note", None)?;
    /// assert!(note.id().get() > 0);
    /// assert_eq!(note.content(), "My first note");
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_note(&self, content: &str, tags: Option<&[&str]>) -> Result<Note> {
        use std::collections::HashSet;

        let conn = self.db.connection();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        // Use a transaction for atomicity
        conn.execute("BEGIN TRANSACTION", [])?;

        let result: Result<Note> = (|| {
            // Insert note with current timestamp
            conn.execute(
                "INSERT INTO notes (content, created_at, updated_at) VALUES (?1, ?2, ?3)",
                (content, now, now),
            )?;

            // Get the ID of the just-inserted note
            let note_id = conn.last_insert_rowid();

            // Handle tags if provided
            let mut tag_assignments = Vec::new();
            if let Some(tag_names) = tags {
                // Deduplicate tag names (case-insensitive)
                let mut seen_tags = HashSet::new();

                for tag_name in tag_names {
                    // Normalize to lowercase for deduplication
                    let normalized = tag_name.to_lowercase();

                    // Skip if we've already processed this tag
                    if !seen_tags.insert(normalized) {
                        continue;
                    }

                    // Get or create the tag
                    let tag_id = self.get_or_create_tag(tag_name)?;

                    // Insert note_tags entry with user source
                    conn.execute(
                        "INSERT INTO note_tags (note_id, tag_id, confidence, source, created_at, verified, model_version)
                         VALUES (?1, ?2, 1.0, 'user', ?3, 0, NULL)",
                        (note_id, tag_id.get(), now),
                    )?;

                    // Build TagAssignment for the returned Note
                    tag_assignments.push(TagAssignment::user(
                        tag_id,
                        OffsetDateTime::from_unix_timestamp(now)?,
                    ));
                }
            }

            // Build and return the Note
            let note = NoteBuilder::new()
                .id(NoteId::new(note_id))
                .content(content)
                .created_at(OffsetDateTime::from_unix_timestamp(now)?)
                .updated_at(OffsetDateTime::from_unix_timestamp(now)?)
                .tags(tag_assignments)
                .build();

            Ok(note)
        })();

        match result {
            Ok(note) => {
                conn.execute("COMMIT", [])?;
                Ok(note)
            }
            Err(e) => {
                conn.execute("ROLLBACK", []).ok();
                Err(e)
            }
        }
    }

    /// Retrieves a note by its ID.
    ///
    /// Returns `None` if no note exists with the given ID. This is not
    /// considered an error condition.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the note to retrieve
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService, NoteId};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// // Non-existent note returns None
    /// assert_eq!(service.get_note(NoteId::new(999))?, None);
    ///
    /// // Create and retrieve a note
    /// let created = service.create_note("Test", None)?;
    /// let retrieved = service.get_note(created.id())?.expect("note should exist");
    /// assert_eq!(retrieved.content(), "Test");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_note(&self, id: NoteId) -> Result<Option<Note>> {
        let conn = self.db.connection();

        let mut stmt =
            conn.prepare("SELECT id, content, created_at, updated_at FROM notes WHERE id = ?1")?;

        let result = stmt.query_row([id.get()], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(1)?;
            let created_at: i64 = row.get(2)?;
            let updated_at: i64 = row.get(3)?;

            Ok((id, content, created_at, updated_at))
        });

        match result {
            Ok((id, content, created_at, updated_at)) => {
                // Load tag assignments for this note
                let mut tag_stmt = conn.prepare(
                    "SELECT nt.tag_id, nt.confidence, nt.source, nt.created_at, nt.model_version
                     FROM note_tags nt
                     WHERE nt.note_id = ?1
                     ORDER BY nt.created_at",
                )?;

                let tag_rows = tag_stmt.query_map([id], |row| {
                    let tag_id: i64 = row.get(0)?;
                    let confidence: f64 = row.get(1)?;
                    let source: String = row.get(2)?;
                    let tag_created_at: i64 = row.get(3)?;
                    let model_version: Option<String> = row.get(4)?;

                    Ok((tag_id, confidence, source, tag_created_at, model_version))
                })?;

                let mut tag_assignments = Vec::new();
                for row_result in tag_rows {
                    let (tag_id, confidence, source, tag_created_at, model_version) = row_result?;

                    let tag_assignment = if source == "user" {
                        TagAssignment::user(
                            TagId::new(tag_id),
                            OffsetDateTime::from_unix_timestamp(tag_created_at)?,
                        )
                    } else {
                        // LLM source - convert confidence from f64 (0.0-1.0) to u8 (0-100)
                        let confidence_u8 = (confidence * 100.0).round() as u8;
                        let model = model_version.unwrap_or_else(|| "unknown".to_string());

                        TagAssignment::llm(
                            TagId::new(tag_id),
                            model,
                            confidence_u8,
                            OffsetDateTime::from_unix_timestamp(tag_created_at)?,
                        )
                    };

                    tag_assignments.push(tag_assignment);
                }

                let note = NoteBuilder::new()
                    .id(NoteId::new(id))
                    .content(content)
                    .created_at(OffsetDateTime::from_unix_timestamp(created_at)?)
                    .updated_at(OffsetDateTime::from_unix_timestamp(updated_at)?)
                    .tags(tag_assignments)
                    .build();

                Ok(Some(note))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Deletes a note by its ID.
    ///
    /// This operation is idempotent: deleting a non-existent note returns
    /// `Ok(())` without error. Foreign key constraints ensure that related
    /// tag associations are automatically removed.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the note to delete
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService, NoteId};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// let note = service.create_note("To be deleted", None)?;
    ///
    /// // First delete succeeds
    /// service.delete_note(note.id())?;
    ///
    /// // Second delete also succeeds (idempotent)
    /// service.delete_note(note.id())?;
    ///
    /// // Verify note is gone
    /// assert_eq!(service.get_note(note.id())?, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_note(&self, id: NoteId) -> Result<()> {
        let conn = self.db.connection();

        conn.execute("DELETE FROM notes WHERE id = ?1", [id.get()])?;

        Ok(())
    }

    /// Helper method to get or create a tag by name.
    ///
    /// Queries the tags table by name (case-insensitive via COLLATE NOCASE).
    /// If found, returns the existing TagId. If not found, creates a new tag
    /// and returns its TagId.
    ///
    /// # Arguments
    ///
    /// * `name` - The tag name to get or create
    fn get_or_create_tag(&self, name: &str) -> Result<TagId> {
        let conn = self.db.connection();

        // Try to find existing tag (case-insensitive)
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                [name],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            return Ok(TagId::new(id));
        }

        // Tag doesn't exist, create it
        conn.execute("INSERT INTO tags (name) VALUES (?1)", [name])?;

        let tag_id = conn.last_insert_rowid();
        Ok(TagId::new(tag_id))
    }

    /// Adds tags to an existing note with the specified source.
    ///
    /// # Arguments
    ///
    /// * `note_id` - The ID of the note to add tags to
    /// * `tags` - Slice of tag names to add
    /// * `source` - The source of the tag assignment (User or Llm)
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService, TagSource};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// let note = service.create_note("My note", None)?;
    ///
    /// // Add user tags
    /// service.add_tags_to_note(note.id(), &["rust", "programming"], TagSource::User)?;
    ///
    /// // Add LLM tags
    /// let llm_source = TagSource::llm("deepseek-r1:8b", 85);
    /// service.add_tags_to_note(note.id(), &["ai"], llm_source)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_tags_to_note(
        &self,
        note_id: NoteId,
        tags: &[&str],
        source: TagSource,
    ) -> Result<()> {
        let conn = self.db.connection();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        // Verify note exists first
        let note_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM notes WHERE id = ?1)",
            [note_id.get()],
            |row| row.get(0),
        )?;

        if !note_exists {
            anyhow::bail!("Note with id {} does not exist", note_id);
        }

        // Process each tag
        for tag_name in tags {
            let tag_id = self.get_or_create_tag(tag_name)?;

            // Prepare metadata based on source
            let (source_str, confidence, model_version) = match &source {
                TagSource::User => ("user", 1.0, None),
                TagSource::Llm { model, confidence } => {
                    // Convert u8 (0-100) to f64 (0.0-1.0)
                    let confidence_f64 = f64::from(*confidence) / 100.0;
                    ("llm", confidence_f64, Some(model.as_str()))
                }
            };

            // Insert note_tag association (INSERT OR IGNORE for duplicates)
            conn.execute(
                "INSERT OR IGNORE INTO note_tags
                 (note_id, tag_id, confidence, source, created_at, verified, model_version)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
                rusqlite::params![
                    note_id.get(),
                    tag_id.get(),
                    confidence,
                    source_str,
                    now,
                    model_version,
                ],
            )?;
        }

        Ok(())
    }

    /// Lists notes with optional filtering and pagination.
    ///
    /// Returns notes ordered by creation time (newest first) with optional
    /// filtering by tags and limiting of results.
    ///
    /// # Arguments
    ///
    /// * `options` - Filtering and pagination options
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService, ListNotesOptions};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// // Create some notes
    /// service.create_note("First note", Some(&["rust"]))?;
    /// service.create_note("Second note", Some(&["rust", "programming"]))?;
    ///
    /// // List all notes
    /// let all_notes = service.list_notes(ListNotesOptions::default())?;
    ///
    /// // List with limit
    /// let recent_notes = service.list_notes(ListNotesOptions {
    ///     limit: Some(5),
    ///     ..Default::default()
    /// })?;
    ///
    /// // Filter by tags (AND logic)
    /// let filtered_notes = service.list_notes(ListNotesOptions {
    ///     tags: Some(vec!["rust".to_string(), "programming".to_string()]),
    ///     ..Default::default()
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_notes(&self, options: ListNotesOptions) -> Result<Vec<Note>> {
        let conn = self.db.connection();

        // Build the query based on whether we have tag filters
        let note_ids: Vec<i64> = if let Some(tag_names) = options.tags {
            if tag_names.is_empty() {
                // Empty tag filter means no notes match
                Vec::new()
            } else {
                // Query for notes that have ALL specified tags (AND logic)
                // We use HAVING COUNT to ensure the note has all tags
                let tag_count = tag_names.len();

                // Build placeholders for the IN clause
                let placeholders: Vec<&str> = tag_names.iter().map(|_| "?").collect();
                let in_clause = placeholders.join(", ");

                let query = format!(
                    "SELECT DISTINCT n.id
                     FROM notes n
                     JOIN note_tags nt ON n.id = nt.note_id
                     JOIN tags t ON nt.tag_id = t.id
                     WHERE t.name IN ({}) COLLATE NOCASE
                     GROUP BY n.id
                     HAVING COUNT(DISTINCT t.id) = ?
                     ORDER BY n.created_at DESC",
                    in_clause
                );

                let mut stmt = conn.prepare(&query)?;

                // Bind tag names and then the count
                let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
                for tag_name in &tag_names {
                    params.push(tag_name);
                }
                params.push(&tag_count);

                let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
                    row.get::<_, i64>(0)
                })?;

                let mut ids = Vec::new();
                for row_result in rows {
                    ids.push(row_result?);
                }

                // Apply limit if specified
                if let Some(limit) = options.limit {
                    ids.truncate(limit);
                }

                ids
            }
        } else {
            // No tag filtering - get all notes
            let query = if let Some(limit) = options.limit {
                format!(
                    "SELECT id FROM notes ORDER BY created_at DESC LIMIT {}",
                    limit
                )
            } else {
                "SELECT id FROM notes ORDER BY created_at DESC".to_string()
            };

            let mut stmt = conn.prepare(&query)?;
            let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;

            let mut ids = Vec::new();
            for row_result in rows {
                ids.push(row_result?);
            }

            ids
        };

        // Now load each note with its full data including tags
        let mut notes = Vec::new();
        for id in note_ids {
            if let Some(note) = self.get_note(NoteId::new(id))? {
                notes.push(note);
            }
        }

        Ok(notes)
    }
}

/// Options for listing notes.
///
/// Provides flexible filtering and pagination for note queries.
/// All fields are optional with sensible defaults.
///
/// # Examples
///
/// ```
/// use cons::ListNotesOptions;
///
/// // Use defaults (no limit, no tag filtering)
/// let options = ListNotesOptions::default();
///
/// // Limit to 10 most recent notes
/// let options = ListNotesOptions {
///     limit: Some(10),
///     ..Default::default()
/// };
///
/// // Filter by tags
/// let options = ListNotesOptions {
///     tags: Some(vec!["rust".to_string(), "project".to_string()]),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ListNotesOptions {
    /// Maximum number of notes to return. None means no limit.
    pub limit: Option<usize>,

    /// Filter notes by these tags. None means no tag filtering.
    /// When specified, returns notes that have ALL of the given tags.
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
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
}
