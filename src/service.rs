use crate::{
    AliasInfo, Database, Note, NoteBuilder, NoteId, TagAssignment, TagId, TagSource,
    autotagger::TagNormalizer,
};
use anyhow::Result;
use rusqlite::OptionalExtension;
use time::OffsetDateTime;

/// Search result with relevance score for dual-channel retrieval.
///
/// Contains a note and its normalized relevance score (0.0-1.0) from BM25 ranking.
/// The score enables combining FTS results with graph-based retrieval scores
/// in dual-channel search (see KNOWLEDGE.md).
///
/// # Examples
///
/// ```
/// use cons::{Database, NoteService};
///
/// # fn main() -> anyhow::Result<()> {
/// let db = Database::in_memory()?;
/// let service = NoteService::new(db);
/// service.create_note("Learning Rust programming", Some(&["rust"]))?;
///
/// let results = service.search_notes("rust", None)?;
/// for result in &results {
///     println!("Score: {:.2}, Note: {}", result.relevance_score, result.note.content());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matched note with full content and tags.
    pub note: Note,
    /// Normalized relevance score (0.0-1.0, higher = more relevant).
    /// Derived from BM25: `1.0 / (1.0 + raw_score.abs())`
    pub relevance_score: f64,
}

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
                // Deduplicate tag names using full normalization
                let mut seen_tags = HashSet::new();

                for tag_name in tag_names {
                    // Normalize using TagNormalizer for deduplication
                    let normalized = TagNormalizer::normalize_tag(tag_name);

                    // Skip if we've already processed this tag
                    if !seen_tags.insert(normalized) {
                        continue;
                    }

                    // Get or create the tag (get_or_create_tag will normalize again, but that's idempotent)
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

        let mut stmt = conn.prepare(
            "SELECT id, content, created_at, updated_at, content_enhanced, enhanced_at, enhancement_model, enhancement_confidence
             FROM notes WHERE id = ?1"
        )?;

        let result = stmt.query_row([id.get()], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(1)?;
            let created_at: i64 = row.get(2)?;
            let updated_at: i64 = row.get(3)?;
            let content_enhanced: Option<String> = row.get(4)?;
            let enhanced_at: Option<i64> = row.get(5)?;
            let enhancement_model: Option<String> = row.get(6)?;
            let enhancement_confidence: Option<f64> = row.get(7)?;

            Ok((
                id,
                content,
                created_at,
                updated_at,
                content_enhanced,
                enhanced_at,
                enhancement_model,
                enhancement_confidence,
            ))
        });

        match result {
            Ok((
                id,
                content,
                created_at,
                updated_at,
                content_enhanced,
                enhanced_at,
                enhancement_model,
                enhancement_confidence,
            )) => {
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

                // Build Note with enhancement fields
                let mut builder = NoteBuilder::new()
                    .id(NoteId::new(id))
                    .content(content)
                    .created_at(OffsetDateTime::from_unix_timestamp(created_at)?)
                    .updated_at(OffsetDateTime::from_unix_timestamp(updated_at)?)
                    .tags(tag_assignments);

                // Add enhancement fields if present
                if let Some(enhanced_content) = content_enhanced {
                    builder = builder.content_enhanced(enhanced_content);
                }
                if let Some(enhanced_timestamp) = enhanced_at {
                    builder = builder
                        .enhanced_at(OffsetDateTime::from_unix_timestamp(enhanced_timestamp)?);
                }
                if let Some(model) = enhancement_model {
                    builder = builder.enhancement_model(model);
                }
                if let Some(confidence) = enhancement_confidence {
                    builder = builder.enhancement_confidence(confidence);
                }

                let note = builder.build();

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

    /// Gets or creates a tag by name.
    ///
    /// Queries the tags table by name (case-insensitive via COLLATE NOCASE).
    /// If an alias exists for the normalized name, returns the canonical tag ID.
    /// If the tag exists, returns its TagId. If not found, creates a new tag
    /// and returns its TagId.
    ///
    /// # Arguments
    ///
    /// * `name` - The tag name to get or create
    pub fn get_or_create_tag(&self, name: &str) -> Result<TagId> {
        // Normalize tag name before database operations
        let normalized = TagNormalizer::normalize_tag(name);
        let conn = self.db.connection();

        // Check if this name is an alias first
        if let Some(canonical_tag_id) = self.resolve_alias(&normalized)? {
            return Ok(canonical_tag_id);
        }

        // Try to find existing tag (case-insensitive)
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                [&normalized],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing {
            return Ok(TagId::new(id));
        }

        // Tag doesn't exist, create it with normalized name
        conn.execute("INSERT INTO tags (name) VALUES (?1)", [&normalized])?;

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
    /// Returns notes ordered by creation time (order controlled by `ListNotesOptions::order`)
    /// with optional filtering by tags and limiting of results.
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
                // Resolve aliases for each tag filter independently
                let mut resolved_tag_names = Vec::new();
                for tag_name in &tag_names {
                    // Normalize the tag name
                    let normalized = TagNormalizer::normalize_tag(tag_name);

                    // Check if it's an alias
                    if let Some(canonical_tag_id) = self.resolve_alias(&normalized)? {
                        // It's an alias - get the canonical tag name
                        let canonical_name: String = conn.query_row(
                            "SELECT name FROM tags WHERE id = ?1",
                            [canonical_tag_id.get()],
                            |row| row.get(0),
                        )?;
                        resolved_tag_names.push(canonical_name);
                    } else {
                        // Not an alias - use the normalized name
                        resolved_tag_names.push(normalized);
                    }
                }

                // Query for notes that have ALL specified tags (AND logic)
                // We use HAVING COUNT to ensure the note has all tags
                let tag_count = resolved_tag_names.len();

                // Build placeholders for the IN clause
                let placeholders: Vec<&str> = resolved_tag_names.iter().map(|_| "?").collect();
                let in_clause = placeholders.join(", ");

                let order_clause = match options.order {
                    SortOrder::Ascending => "ASC",
                    SortOrder::Descending => "DESC",
                };
                let limit_clause = if let Some(limit) = options.limit {
                    format!(" LIMIT {}", limit)
                } else {
                    String::new()
                };
                let query = format!(
                    "SELECT DISTINCT n.id
                     FROM notes n
                     JOIN note_tags nt ON n.id = nt.note_id
                     JOIN tags t ON nt.tag_id = t.id
                     WHERE t.name IN ({}) COLLATE NOCASE
                     GROUP BY n.id
                     HAVING COUNT(DISTINCT t.id) = ?
                     ORDER BY n.created_at {}{}",
                    in_clause, order_clause, limit_clause
                );

                let mut stmt = conn.prepare(&query)?;

                // Bind tag names and then the count
                let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
                for tag_name in &resolved_tag_names {
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

                ids
            }
        } else {
            // No tag filtering - get all notes
            let order_clause = match options.order {
                SortOrder::Ascending => "ASC",
                SortOrder::Descending => "DESC",
            };
            let query = if let Some(limit) = options.limit {
                format!(
                    "SELECT id FROM notes ORDER BY created_at {} LIMIT {}",
                    order_clause, limit
                )
            } else {
                format!("SELECT id FROM notes ORDER BY created_at {}", order_clause)
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

    /// Resolves an alias to its canonical tag ID.
    ///
    /// Normalizes the input alias name before lookup using COLLATE NOCASE matching.
    /// Returns `None` if no alias exists with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The alias name to resolve
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
    /// // Create a canonical tag and alias
    /// let canonical_tag_id = service.get_or_create_tag("machine-learning")?;
    /// service.create_alias("ml", canonical_tag_id, "user", 1.0, None)?;
    ///
    /// // Resolve the alias
    /// let resolved = service.resolve_alias("ml")?;
    /// assert_eq!(resolved, Some(canonical_tag_id));
    ///
    /// // Non-existent alias returns None
    /// assert_eq!(service.resolve_alias("non-existent")?, None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolve_alias(&self, name: &str) -> Result<Option<TagId>> {
        // Normalize input before lookup
        let normalized = TagNormalizer::normalize_tag(name);
        let conn = self.db.connection();

        // Query tag_aliases with COLLATE NOCASE matching
        let result: Option<i64> = conn
            .query_row(
                "SELECT canonical_tag_id FROM tag_aliases WHERE alias = ?1 COLLATE NOCASE",
                [&normalized],
                |row| row.get(0),
            )
            .optional()?;

        Ok(result.map(TagId::new))
    }

    /// Creates an alias mapping an alternate name to a canonical tag.
    ///
    /// Normalizes the alias before storage and verifies that:
    /// - The canonical tag exists
    /// - The canonical tag is not itself an alias (prevents chains)
    ///
    /// Uses INSERT OR REPLACE for idempotent updates.
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias name to create
    /// * `canonical_tag_id` - The canonical tag this alias resolves to
    /// * `source` - The source of the alias ("user" or "llm")
    /// * `confidence` - Confidence score (0.0-1.0)
    /// * `model_version` - Optional model version for LLM-created aliases
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
    /// // Create a canonical tag
    /// let canonical_tag_id = service.get_or_create_tag("machine-learning")?;
    ///
    /// // Create a user alias
    /// service.create_alias("ml", canonical_tag_id, "user", 1.0, None)?;
    ///
    /// // Create an LLM alias
    /// service.create_alias("ML", canonical_tag_id, "llm", 0.85, Some("deepseek-r1:8b"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_alias(
        &self,
        alias: &str,
        canonical_tag_id: TagId,
        source: &str,
        confidence: f64,
        model_version: Option<&str>,
    ) -> Result<()> {
        // Normalize alias before storage
        let normalized_alias = TagNormalizer::normalize_tag(alias);
        let conn = self.db.connection();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        // Verify canonical_tag_id exists in tags table
        let tag_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
            [canonical_tag_id.get()],
            |row| row.get(0),
        )?;

        if !tag_exists {
            anyhow::bail!("Canonical tag with id {} does not exist", canonical_tag_id);
        }

        // Verify the tag name isn't already used as an alias (prevent chains)
        // Get the tag name for the canonical_tag_id
        let tag_name: String = conn.query_row(
            "SELECT name FROM tags WHERE id = ?1",
            [canonical_tag_id.get()],
            |row| row.get(0),
        )?;

        // Check if this tag name is already an alias
        let is_alias: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tag_aliases WHERE alias = ?1 COLLATE NOCASE)",
            [&tag_name],
            |row| row.get(0),
        )?;

        if is_alias {
            anyhow::bail!(
                "Cannot create alias: tag '{}' (id {}) is itself an alias",
                tag_name,
                canonical_tag_id
            );
        }

        // Insert with INSERT OR REPLACE for idempotent updates
        conn.execute(
            "INSERT OR REPLACE INTO tag_aliases (alias, canonical_tag_id, source, confidence, created_at, model_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                normalized_alias,
                canonical_tag_id.get(),
                source,
                confidence,
                now,
                model_version,
            ],
        )?;

        Ok(())
    }

    /// Lists all tag aliases.
    ///
    /// Returns aliases with their metadata, ordered by canonical tag name
    /// then by alias name.
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
    /// // Create canonical tags and aliases
    /// let ml_tag = service.get_or_create_tag("machine-learning")?;
    /// service.create_alias("ml", ml_tag, "user", 1.0, None)?;
    ///
    /// let ai_tag = service.get_or_create_tag("artificial-intelligence")?;
    /// service.create_alias("ai", ai_tag, "user", 1.0, None)?;
    ///
    /// // List all aliases
    /// let aliases = service.list_aliases()?;
    /// assert_eq!(aliases.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_aliases(&self) -> Result<Vec<AliasInfo>> {
        let conn = self.db.connection();

        let mut stmt = conn.prepare(
            "SELECT ta.alias, ta.canonical_tag_id, ta.source, ta.confidence, ta.created_at, ta.model_version, t.name
             FROM tag_aliases ta
             JOIN tags t ON ta.canonical_tag_id = t.id
             ORDER BY t.name, ta.alias",
        )?;

        let rows = stmt.query_map([], |row| {
            let alias: String = row.get(0)?;
            let canonical_tag_id: i64 = row.get(1)?;
            let source: String = row.get(2)?;
            let confidence: f64 = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let model_version: Option<String> = row.get(5)?;

            Ok((
                alias,
                canonical_tag_id,
                source,
                confidence,
                created_at,
                model_version,
            ))
        })?;

        let mut aliases = Vec::new();
        for row_result in rows {
            let (alias, canonical_tag_id, source, confidence, created_at, model_version) =
                row_result?;

            let alias_info = AliasInfo::new(
                alias,
                TagId::new(canonical_tag_id),
                source,
                confidence,
                OffsetDateTime::from_unix_timestamp(created_at)?,
                model_version,
            );

            aliases.push(alias_info);
        }

        Ok(aliases)
    }

    /// Removes an alias mapping.
    ///
    /// Normalizes the alias before deletion using COLLATE NOCASE matching.
    /// This operation is idempotent: removing a non-existent alias succeeds
    /// without error.
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias name to remove
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
    /// // Create a canonical tag and alias
    /// let canonical_tag_id = service.get_or_create_tag("machine-learning")?;
    /// service.create_alias("ml", canonical_tag_id, "user", 1.0, None)?;
    ///
    /// // Remove the alias
    /// service.remove_alias("ml")?;
    ///
    /// // Verify it's gone
    /// assert_eq!(service.resolve_alias("ml")?, None);
    ///
    /// // Removing again is idempotent
    /// service.remove_alias("ml")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn remove_alias(&self, alias: &str) -> Result<()> {
        // Normalize alias before deletion
        let normalized = TagNormalizer::normalize_tag(alias);
        let conn = self.db.connection();

        // Delete with COLLATE NOCASE matching (idempotent)
        conn.execute(
            "DELETE FROM tag_aliases WHERE alias = ?1 COLLATE NOCASE",
            [&normalized],
        )?;

        Ok(())
    }

    /// Expands a search term to include all related aliases and canonical forms.
    ///
    /// Performs bi-directional alias expansion:
    /// - If the term is an alias, includes the canonical tag name
    /// - If the term matches a canonical tag, includes all its aliases
    ///
    /// Applies confidence-based filtering:
    /// - User-created aliases (source = 'user') are always included
    /// - LLM-suggested aliases (source = 'llm') are only included if confidence >= 0.8
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to expand
    ///
    /// # Returns
    ///
    /// A vector of unique expansion terms including the original term.
    /// Returns only the original term if no aliases exist.
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
    /// // Create canonical tag and aliases
    /// let ml_tag = service.get_or_create_tag("machine-learning")?;
    /// service.create_alias("ml", ml_tag, "user", 1.0, None)?;
    ///
    /// // Expand alias -> includes canonical
    /// let expanded = service.expand_search_term("ml")?;
    /// assert!(expanded.contains(&"ml".to_string()));
    /// assert!(expanded.contains(&"machine-learning".to_string()));
    ///
    /// // Expand canonical -> includes aliases
    /// let expanded = service.expand_search_term("machine-learning")?;
    /// assert!(expanded.contains(&"machine-learning".to_string()));
    /// assert!(expanded.contains(&"ml".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn expand_search_term(&self, term: &str) -> Result<Vec<String>> {
        use std::collections::HashSet;

        // Normalize the input term
        let normalized = TagNormalizer::normalize_tag(term);
        let conn = self.db.connection();

        let mut expansions = HashSet::new();
        // Always include the original normalized term
        expansions.insert(normalized.clone());

        // Check if term is an alias -> get canonical_tag_id
        let alias_canonical_id: Option<i64> = conn
            .query_row(
                "SELECT canonical_tag_id FROM tag_aliases WHERE alias = ?1 COLLATE NOCASE",
                [&normalized],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(canonical_id) = alias_canonical_id {
            // Term is an alias - get the canonical tag name
            let canonical_name: Option<String> = conn
                .query_row(
                    "SELECT name FROM tags WHERE id = ?1",
                    [canonical_id],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(name) = canonical_name {
                expansions.insert(name);
            }

            // Also get all other aliases for this canonical tag (with confidence filtering)
            let mut stmt = conn.prepare(
                "SELECT alias FROM tag_aliases
                 WHERE canonical_tag_id = ?1
                   AND (source = 'user' OR (source = 'llm' AND confidence >= 0.8))",
            )?;

            let alias_rows = stmt.query_map([canonical_id], |row| row.get::<_, String>(0))?;

            for alias_result in alias_rows {
                expansions.insert(alias_result?);
            }
        }

        // Check if term matches a canonical tag name
        let canonical_tag_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                [&normalized],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(tag_id) = canonical_tag_id {
            // Term is a canonical tag - get all its aliases (with confidence filtering)
            let mut stmt = conn.prepare(
                "SELECT alias FROM tag_aliases
                 WHERE canonical_tag_id = ?1
                   AND (source = 'user' OR (source = 'llm' AND confidence >= 0.8))",
            )?;

            let alias_rows = stmt.query_map([tag_id], |row| row.get::<_, String>(0))?;

            for alias_result in alias_rows {
                expansions.insert(alias_result?);
            }
        }

        Ok(expansions.into_iter().collect())
    }

    /// Builds an FTS5 query fragment with alias expansion for a single term.
    ///
    /// Expands the term using `expand_search_term()` and formats the result
    /// as an FTS5 OR expression with proper quoting:
    /// - All terms are quoted for exact matching
    /// - Multi-word aliases use phrase matching
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to expand and format
    ///
    /// # Returns
    ///
    /// An FTS5 query fragment. For single term: `"term"`.
    /// For multiple expansions with OR: `("ml" OR "machine-learning")`.
    fn build_expanded_fts_term(&self, term: &str) -> Result<String> {
        let expansions = self.expand_search_term(term)?;

        if expansions.len() == 1 {
            // Single term - just escape and quote it
            let escaped = expansions[0].replace('"', "\"\"");
            return Ok(format!("\"{}\"", escaped));
        }

        // Multiple expansions - build OR group with proper FTS5 syntax
        // FTS5 requires: ("term1" OR "term2" OR "term3")
        let formatted_terms: Vec<String> = expansions
            .iter()
            .map(|expansion| {
                // Escape quotes and wrap in quotes
                let escaped = expansion.replace('"', "\"\"");
                format!("\"{}\"", escaped)
            })
            .collect();

        // Use parentheses for grouping OR expressions in FTS5
        Ok(format!("({})", formatted_terms.join(" OR ")))
    }

    /// Searches for notes using full-text search across content, enhanced content, and tags.
    ///
    /// Uses SQLite FTS5 with BM25 relevance ranking to find notes matching the search query.
    /// All search terms must match (AND logic). Porter stemming automatically handles word
    /// variations (e.g., "running" matches "run").
    ///
    /// **Alias Expansion**: Before executing the search, each term is expanded using
    /// the `tag_aliases` table. For example, searching for "ML" will also match notes
    /// tagged with "machine-learning" if an alias relationship exists.
    ///
    /// Returns `SearchResult` objects containing the note and a normalized relevance score
    /// (0.0-1.0, higher = more relevant). The score enables dual-channel retrieval where
    /// FTS scores can be combined with graph-based scores (see KNOWLEDGE.md).
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (cannot be empty or whitespace-only)
    /// * `limit` - Optional maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a vector of `SearchResult` objects ordered by relevance (most relevant first).
    /// Each result contains the full Note (including tags) and a normalized relevance score.
    ///
    /// # Errors
    ///
    /// Returns an error if the query is empty or contains only whitespace.
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
    /// // Create some notes
    /// service.create_note("Learning Rust programming", Some(&["rust"]))?;
    /// service.create_note("Python tutorial", Some(&["python"]))?;
    ///
    /// // Search for notes about Rust - returns SearchResult with score
    /// let results = service.search_notes("rust", None)?;
    /// assert_eq!(results.len(), 1);
    /// assert!(results[0].relevance_score > 0.0 && results[0].relevance_score <= 1.0);
    ///
    /// // Access the note from the result
    /// let note = &results[0].note;
    /// assert!(note.content().contains("Rust"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn search_notes(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>> {
        // Validate query is not empty or whitespace-only
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            anyhow::bail!("Search query cannot be empty");
        }

        // Split query into terms and expand each with alias expansion
        let terms: Vec<&str> = trimmed_query.split_whitespace().collect();

        // Build FTS5 query with alias expansion for each term
        // AND logic between original query terms, OR within expansions
        let expanded_terms: Result<Vec<String>> = terms
            .iter()
            .map(|term| self.build_expanded_fts_term(term))
            .collect();

        // Join with explicit AND for FTS5 when using parenthesized OR groups
        // FTS5 syntax requires explicit AND between parenthesized groups
        let fts_query = expanded_terms?.join(" AND ");

        let conn = self.db.connection();

        // Query FTS5 table with BM25 ranking, also selecting the score
        // ORDER BY bm25() ascending (lower/more negative scores are more relevant in FTS5)
        let query_sql = if let Some(limit_val) = limit {
            format!(
                "SELECT note_id, bm25(notes_fts) as score FROM notes_fts
                 WHERE notes_fts MATCH ?
                 ORDER BY score
                 LIMIT {}",
                limit_val
            )
        } else {
            "SELECT note_id, bm25(notes_fts) as score FROM notes_fts
             WHERE notes_fts MATCH ?
             ORDER BY score"
                .to_string()
        };

        let mut stmt = conn.prepare(&query_sql)?;
        let rows: Vec<(i64, f64)> = stmt
            .query_map([&fts_query], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(i64, f64)>, _>>()?;

        // Load full Note objects and construct SearchResults with normalized scores
        let mut results = Vec::new();
        for (id, raw_score) in rows {
            if let Some(note) = self.get_note(NoteId::new(id))? {
                // Normalize BM25 score to 0.0-1.0 range (higher = more relevant)
                // BM25 returns negative values where more negative = more relevant
                let relevance_score = 1.0 / (1.0 + raw_score.abs());
                results.push(SearchResult {
                    note,
                    relevance_score,
                });
            }
        }

        Ok(results)
    }

    /// Updates the enhancement fields for an existing note.
    ///
    /// This method is designed for the enhancement workflow where:
    /// 1. Note is saved first (original content preserved)
    /// 2. Enhancement is attempted
    /// 3. If successful, this method updates the note with enhancement data
    ///
    /// # Arguments
    ///
    /// * `note_id` - The ID of the note to update
    /// * `content_enhanced` - The AI-enhanced version of the note content
    /// * `model` - The model identifier used for enhancement
    /// * `confidence` - Enhancement confidence score (0.0-1.0)
    /// * `enhanced_at` - Timestamp when enhancement occurred
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService};
    /// use time::OffsetDateTime;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// // Create note
    /// let note = service.create_note("Quick thought", None)?;
    ///
    /// // Later, after LLM enhancement succeeds
    /// let now = OffsetDateTime::now_utc();
    /// service.update_note_enhancement(
    ///     note.id(),
    ///     "This is a quick thought about something important.",
    ///     "deepseek-r1:8b",
    ///     0.85,
    ///     now,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_note_enhancement(
        &self,
        note_id: NoteId,
        content_enhanced: &str,
        model: &str,
        confidence: f64,
        enhanced_at: OffsetDateTime,
    ) -> Result<()> {
        let conn = self.db.connection();
        let enhanced_timestamp = enhanced_at.unix_timestamp();

        // Update only the enhancement fields, leaving original content unchanged
        conn.execute(
            "UPDATE notes
             SET content_enhanced = ?1,
                 enhanced_at = ?2,
                 enhancement_model = ?3,
                 enhancement_confidence = ?4
             WHERE id = ?5",
            (
                content_enhanced,
                enhanced_timestamp,
                model,
                confidence,
                note_id.get(),
            ),
        )?;

        Ok(())
    }

    /// Gets all tags that have at least one associated note.
    ///
    /// Queries the tags table using JOIN with note_tags to filter for tags
    /// that are actually used. Orphan tags (tags with no notes) are excluded.
    ///
    /// # Returns
    ///
    /// Returns a vector of tuples containing (TagId, tag name) for each tag
    /// with associated notes, ordered by tag name.
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
    /// // Create notes with tags
    /// service.create_note("Rust note", Some(&["rust"]))?;
    /// service.create_note("Python note", Some(&["python"]))?;
    ///
    /// // Get tags with notes
    /// let tags = service.get_tags_with_notes()?;
    /// assert_eq!(tags.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_tags_with_notes(&self) -> Result<Vec<(TagId, String)>> {
        let conn = self.db.connection();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT t.id, t.name
             FROM tags t
             JOIN note_tags nt ON t.id = nt.tag_id
             ORDER BY t.name",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((TagId::new(id), name))
        })?;

        let mut tags = Vec::new();
        for row_result in rows {
            tags.push(row_result?);
        }

        Ok(tags)
    }

    /// Creates an edge between two tags in the hierarchy.
    ///
    /// Inserts a directed edge from source_tag_id (narrower/child concept) to
    /// target_tag_id (broader/parent concept). Uses INSERT OR IGNORE for
    /// idempotent operation - duplicate edges are silently ignored.
    ///
    /// # Arguments
    ///
    /// * `source_tag_id` - The narrower/child tag (more specific concept)
    /// * `target_tag_id` - The broader/parent tag (more general concept)
    /// * `confidence` - Confidence score (0.0-1.0)
    /// * `hierarchy_type` - "generic" for is-a relationships, "partitive" for part-of
    /// * `model_version` - Optional LLM model identifier (e.g., "deepseek-r1:8b")
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
    /// // Create tags
    /// let transformer = service.get_or_create_tag("transformer")?;
    /// let neural_network = service.get_or_create_tag("neural-network")?;
    ///
    /// // Create generic edge: transformer specializes neural-network
    /// service.create_edge(
    ///     transformer,
    ///     neural_network,
    ///     0.9,
    ///     "generic",
    ///     Some("deepseek-r1:8b"),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_edge(
        &self,
        source_tag_id: TagId,
        target_tag_id: TagId,
        confidence: f64,
        hierarchy_type: &str,
        model_version: Option<&str>,
    ) -> Result<()> {
        let conn = self.db.connection();
        let now = OffsetDateTime::now_utc().unix_timestamp();

        // Validate both tag IDs exist
        let source_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
            [source_tag_id.get()],
            |row| row.get(0),
        )?;

        let target_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
            [target_tag_id.get()],
            |row| row.get(0),
        )?;

        if !source_exists {
            anyhow::bail!("Source tag with id {} does not exist", source_tag_id);
        }

        if !target_exists {
            anyhow::bail!("Target tag with id {} does not exist", target_tag_id);
        }

        // Check if edge already exists (for idempotent operation)
        // We only check for edges with NULL temporal validity (hierarchy edges)
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM edges
             WHERE source_tag_id = ?1 AND target_tag_id = ?2
               AND valid_from IS NULL AND valid_until IS NULL)",
            [source_tag_id.get(), target_tag_id.get()],
            |row| row.get(0),
        )?;

        if exists {
            // Edge already exists, skip insert (idempotent)
            return Ok(());
        }

        // Insert edge
        conn.execute(
            "INSERT INTO edges
             (source_tag_id, target_tag_id, confidence, hierarchy_type, source, model_version, verified, created_at, updated_at, valid_from, valid_until)
             VALUES (?1, ?2, ?3, ?4, 'llm', ?5, 0, ?6, ?6, NULL, NULL)",
            rusqlite::params![
                source_tag_id.get(),
                target_tag_id.get(),
                confidence,
                hierarchy_type,
                model_version,
                now,
            ],
        )?;

        Ok(())
    }

    /// Creates multiple edges atomically in a single transaction.
    ///
    /// Wraps multiple create_edge calls in a transaction for atomicity.
    /// If any edge creation fails, all changes are rolled back.
    ///
    /// # Arguments
    ///
    /// * `edges` - Slice of tuples containing (source_tag_id, target_tag_id, confidence, hierarchy_type, model_version)
    ///
    /// # Returns
    ///
    /// Returns the count of edges created (for CLI output).
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
    /// // Create tags
    /// let tag1 = service.get_or_create_tag("tag1")?;
    /// let tag2 = service.get_or_create_tag("tag2")?;
    /// let tag3 = service.get_or_create_tag("tag3")?;
    ///
    /// // Create edges in batch
    /// let edges = vec![
    ///     (tag1, tag2, 0.9, "generic", Some("deepseek-r1:8b")),
    ///     (tag2, tag3, 0.85, "partitive", Some("deepseek-r1:8b")),
    /// ];
    ///
    /// let count = service.create_edges_batch(&edges)?;
    /// assert_eq!(count, 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_edges_batch(
        &self,
        edges: &[(TagId, TagId, f64, &str, Option<&str>)],
    ) -> Result<usize> {
        let conn = self.db.connection();

        // Use a transaction for atomicity
        conn.execute("BEGIN TRANSACTION", [])?;

        let result: Result<usize> = (|| {
            let mut count = 0;

            for (source_tag_id, target_tag_id, confidence, hierarchy_type, model_version) in edges {
                self.create_edge(
                    *source_tag_id,
                    *target_tag_id,
                    *confidence,
                    hierarchy_type,
                    *model_version,
                )?;
                count += 1;
            }

            Ok(count)
        })();

        match result {
            Ok(count) => {
                conn.execute("COMMIT", [])?;
                Ok(count)
            }
            Err(e) => {
                conn.execute("ROLLBACK", []).ok();
                Err(e)
            }
        }
    }
}

/// Sort order for listing notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Oldest notes first (ascending by creation time)
    Ascending,
    /// Newest notes first (descending by creation time)
    #[default]
    Descending,
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
/// // Use defaults (no limit, no tag filtering, newest first)
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
#[derive(Debug, Clone, PartialEq)]
pub struct ListNotesOptions {
    /// Maximum number of notes to return. None means no limit.
    pub limit: Option<usize>,

    /// Filter notes by these tags. None means no tag filtering.
    /// When specified, returns notes that have ALL of the given tags.
    pub tags: Option<Vec<String>>,

    /// Sort order for notes. Defaults to Descending (newest first).
    pub order: SortOrder,
}

impl Default for ListNotesOptions {
    fn default() -> Self {
        Self {
            limit: None,
            tags: None,
            order: SortOrder::Descending,
        }
    }
}

#[cfg(test)]
#[path = "service/tests.rs"]
mod tests;
