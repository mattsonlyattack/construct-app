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

/// Configuration for dual-channel search combining FTS and graph-based retrieval.
///
/// Parsed from environment variables at method call time with fallback defaults.
#[derive(Debug, Clone)]
pub struct DualSearchConfig {
    /// Weight applied to FTS channel scores (default 1.0).
    pub fts_weight: f64,
    /// Weight applied to graph channel scores (default 1.0).
    pub graph_weight: f64,
    /// Bonus score added when a note is found by both channels (default 0.5).
    pub intersection_bonus: f64,
    /// Minimum average activation threshold for graph channel (default 0.1).
    pub min_avg_activation: f64,
    /// Minimum number of activated tags required for graph channel (default 2).
    pub min_activated_tags: usize,
}

impl Default for DualSearchConfig {
    fn default() -> Self {
        Self {
            fts_weight: 1.0,
            graph_weight: 1.0,
            intersection_bonus: 0.5,
            min_avg_activation: 0.1,
            min_activated_tags: 2,
        }
    }
}

impl DualSearchConfig {
    /// Parses configuration from environment variables.
    ///
    /// Falls back to defaults when env vars not set or invalid.
    ///
    /// # Environment Variables
    ///
    /// - `CONS_FTS_WEIGHT` (f64, default 1.0): Weight for FTS channel scores
    /// - `CONS_GRAPH_WEIGHT` (f64, default 1.0): Weight for graph channel scores
    /// - `CONS_INTERSECTION_BONUS` (f64, default 0.5): Bonus when found by both channels
    /// - `CONS_MIN_AVG_ACTIVATION` (f64, default 0.1): Minimum average activation threshold
    /// - `CONS_MIN_ACTIVATED_TAGS` (usize, default 2): Minimum activated tags required
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::service::DualSearchConfig;
    ///
    /// let config = DualSearchConfig::from_env();
    /// assert_eq!(config.fts_weight, 1.0); // default when env var not set
    /// ```
    pub fn from_env() -> Self {
        let fts_weight = std::env::var("CONS_FTS_WEIGHT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);

        let graph_weight = std::env::var("CONS_GRAPH_WEIGHT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);

        let intersection_bonus = std::env::var("CONS_INTERSECTION_BONUS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.5);

        let min_avg_activation = std::env::var("CONS_MIN_AVG_ACTIVATION")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.1);

        let min_activated_tags = std::env::var("CONS_MIN_ACTIVATED_TAGS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2);

        Self {
            fts_weight,
            graph_weight,
            intersection_bonus,
            min_avg_activation,
            min_activated_tags,
        }
    }
}

/// Configuration for query expansion with broader concepts.
///
/// Parsed from environment variables at method call time with fallback defaults.
#[derive(Debug, Clone)]
pub struct QueryExpansionConfig {
    /// Maximum depth for broader concept traversal (default 1).
    pub expansion_depth: usize,
    /// Maximum number of expanded terms per original term (default 10).
    pub max_expansion_terms: usize,
    /// Minimum confidence threshold for including broader concepts (default 0.7).
    pub broader_min_confidence: f64,
}

impl Default for QueryExpansionConfig {
    fn default() -> Self {
        Self {
            expansion_depth: 1,
            max_expansion_terms: 10,
            broader_min_confidence: 0.7,
        }
    }
}

impl QueryExpansionConfig {
    /// Parses configuration from environment variables.
    ///
    /// Falls back to defaults when env vars not set or invalid.
    ///
    /// # Environment Variables
    ///
    /// - `CONS_EXPANSION_DEPTH` (usize, default 1): Maximum depth for broader concept traversal
    /// - `CONS_MAX_EXPANSION_TERMS` (usize, default 10): Maximum expanded terms per original term
    /// - `CONS_BROADER_MIN_CONFIDENCE` (f64, default 0.7): Minimum confidence for broader concepts
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::service::QueryExpansionConfig;
    ///
    /// let config = QueryExpansionConfig::from_env();
    /// assert_eq!(config.expansion_depth, 1); // default when env var not set
    /// ```
    pub fn from_env() -> Self {
        let expansion_depth = std::env::var("CONS_EXPANSION_DEPTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let max_expansion_terms = std::env::var("CONS_MAX_EXPANSION_TERMS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let broader_min_confidence = std::env::var("CONS_BROADER_MIN_CONFIDENCE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.7);

        Self {
            expansion_depth,
            max_expansion_terms,
            broader_min_confidence,
        }
    }
}

/// Search result for dual-channel retrieval combining FTS and graph scores.
///
/// Contains a note with scores from both search channels and a combined final score.
#[derive(Debug, Clone)]
pub struct DualSearchResult {
    /// The matched note with full content and tags.
    pub note: Note,
    /// Combined final score (0.0-1.0, higher = more relevant).
    pub final_score: f64,
    /// FTS channel score if found by FTS search (0.0-1.0).
    pub fts_score: Option<f64>,
    /// Graph channel score if found by graph search (0.0-1.0).
    pub graph_score: Option<f64>,
    /// True if the note was found by both FTS and graph channels.
    pub found_by_both: bool,
}

/// Metadata about dual-channel search execution.
///
/// Captures information about whether graph channel was used and result counts.
#[derive(Debug, Clone)]
pub struct DualSearchMetadata {
    /// True if graph channel was skipped due to sparse activation.
    pub graph_skipped: bool,
    /// Reason why graph channel was skipped (e.g., "sparse graph activation").
    pub skip_reason: Option<String>,
    /// Number of results returned by FTS channel.
    pub fts_result_count: usize,
    /// Number of results returned by graph channel.
    pub graph_result_count: usize,
    /// The expanded FTS query showing alias and broader concept expansion.
    pub expanded_fts_query: String,
}

/// Determines whether broader concept expansion should be applied for a query.
///
/// Returns `true` if the query has fewer than 3 whitespace-separated terms,
/// `false` otherwise. This prevents over-expansion for longer, more specific queries.
///
/// # Arguments
///
/// * `query` - The search query string to analyze
///
/// # Examples
///
/// ```
/// use cons::service::should_expand_broader;
///
/// assert!(should_expand_broader("rust"));
/// assert!(should_expand_broader("rust programming"));
/// assert!(!should_expand_broader("rust programming language"));
/// ```
pub fn should_expand_broader(query: &str) -> bool {
    let term_count = query.split_whitespace().count();
    term_count < 3
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

                    // Build TagAssignment for the returned Note (use normalized name)
                    let stored_name = TagNormalizer::normalize_tag(tag_name);
                    tag_assignments.push(TagAssignment::user(
                        tag_id,
                        stored_name,
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
                // Load tag assignments for this note (with tag names)
                let mut tag_stmt = conn.prepare(
                    "SELECT nt.tag_id, t.name, nt.confidence, nt.source, nt.created_at, nt.model_version
                     FROM note_tags nt
                     JOIN tags t ON nt.tag_id = t.id
                     WHERE nt.note_id = ?1
                     ORDER BY nt.created_at",
                )?;

                let tag_rows = tag_stmt.query_map([id], |row| {
                    let tag_id: i64 = row.get(0)?;
                    let tag_name: String = row.get(1)?;
                    let confidence: f64 = row.get(2)?;
                    let source: String = row.get(3)?;
                    let tag_created_at: i64 = row.get(4)?;
                    let model_version: Option<String> = row.get(5)?;

                    Ok((tag_id, tag_name, confidence, source, tag_created_at, model_version))
                })?;

                let mut tag_assignments = Vec::new();
                for row_result in tag_rows {
                    let (tag_id, tag_name, confidence, source, tag_created_at, model_version) = row_result?;

                    let tag_assignment = if source == "user" {
                        TagAssignment::user(
                            TagId::new(tag_id),
                            tag_name,
                            OffsetDateTime::from_unix_timestamp(tag_created_at)?,
                        )
                    } else {
                        // LLM source - convert confidence from f64 (0.0-1.0) to u8 (0-100)
                        let confidence_u8 = (confidence * 100.0).round() as u8;
                        let model = model_version.unwrap_or_else(|| "unknown".to_string());

                        TagAssignment::llm(
                            TagId::new(tag_id),
                            tag_name,
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

    /// Expands a search term with both alias expansion and broader concept expansion.
    ///
    /// Performs expansion in two stages:
    /// 1. Alias expansion (always applied) using `expand_search_term()`
    /// 2. Broader concept expansion (conditional) using tag hierarchy
    ///
    /// Broader concept expansion adds parent concepts from the tag hierarchy,
    /// filtered by confidence threshold and limited by max_expansion_terms.
    /// When the limit is exceeded, aliases are preferred over broader concepts.
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to expand
    /// * `config` - Query expansion configuration (thresholds and limits)
    ///
    /// # Returns
    ///
    /// A vector of unique expansion terms including the original term, aliases,
    /// and broader concepts (up to max_expansion_terms).
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::{Database, NoteService, service::QueryExpansionConfig};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let db = Database::in_memory()?;
    /// let service = NoteService::new(db);
    ///
    /// // Create tag hierarchy
    /// let rust = service.get_or_create_tag("rust")?;
    /// let programming = service.get_or_create_tag("programming")?;
    /// service.create_edge(rust, programming, 0.9, "generic", Some("test"))?;
    ///
    /// // Expand with broader concepts
    /// let config = QueryExpansionConfig::default();
    /// let expanded = service.expand_search_term_with_broader("rust", &config)?;
    /// assert!(expanded.contains(&"rust".to_string()));
    /// assert!(expanded.contains(&"programming".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn expand_search_term_with_broader(
        &self,
        term: &str,
        config: &QueryExpansionConfig,
    ) -> Result<Vec<String>> {
        use std::collections::HashSet;

        // Stage 1: Alias expansion (always applied)
        let alias_expansions = self.expand_search_term(term)?;

        // Convert to HashSet for deduplication
        let mut expansions: HashSet<String> = alias_expansions.into_iter().collect();

        // Stage 2: Broader concept expansion (conditional based on caller)
        // Look up TagId for the normalized term
        let normalized = TagNormalizer::normalize_tag(term);
        let conn = self.db.connection();

        // First check if term is an alias and get canonical tag
        let tag_id: Option<i64> = if let Some(canonical_id) = self.resolve_alias(&normalized)? {
            Some(canonical_id.get())
        } else {
            // Not an alias, try direct tag lookup
            conn.query_row(
                "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                [&normalized],
                |row| row.get(0),
            )
            .optional()?
        };

        // If we found a TagId, get broader concepts
        if let Some(id) = tag_id {
            let broader_concept_names =
                self.get_broader_concept_names(TagId::new(id), config.broader_min_confidence)?;

            // Add broader concept names to expansions
            for (_, name) in broader_concept_names {
                expansions.insert(name);
            }
        }

        // Stage 3: Apply term limit, preferring aliases over broader concepts
        let mut final_expansions: Vec<String> = expansions.into_iter().collect();

        if final_expansions.len() > config.max_expansion_terms {
            // We need to prioritize: original term + aliases > broader concepts
            // First, identify which terms are from alias expansion
            let original_alias_expansions = self.expand_search_term(term)?;
            let alias_set: HashSet<String> = original_alias_expansions.into_iter().collect();

            // Separate into aliases and broader concepts
            let mut aliases: Vec<String> = Vec::new();
            let mut broader: Vec<String> = Vec::new();

            for expansion in final_expansions {
                if alias_set.contains(&expansion) {
                    aliases.push(expansion);
                } else {
                    broader.push(expansion);
                }
            }

            // Take all aliases first, then fill remaining slots with broader concepts
            final_expansions = aliases;
            let remaining_slots = config
                .max_expansion_terms
                .saturating_sub(final_expansions.len());

            if remaining_slots > 0 {
                broader.truncate(remaining_slots);
                final_expansions.extend(broader);
            }
        }

        Ok(final_expansions)
    }

    /// Builds an FTS5 query fragment with alias and broader concept expansion.
    ///
    /// Expands the term using `expand_search_term_with_broader()` and formats
    /// the result as an FTS5 OR expression with proper quoting:
    /// - All terms are quoted for exact matching
    /// - Multi-word aliases use phrase matching
    /// - Broader concepts included when query length permits
    ///
    /// # Arguments
    ///
    /// * `term` - The search term to expand and format
    /// * `config` - Query expansion configuration for broader concepts
    ///
    /// # Returns
    ///
    /// An FTS5 query fragment. For single term: `"term"`.
    /// For multiple expansions with OR: `("rust" OR "rustlang" OR "programming")`.
    fn build_expanded_fts_term_with_config(
        &self,
        term: &str,
        config: &QueryExpansionConfig,
    ) -> Result<String> {
        let expansions = self.expand_search_term_with_broader(term, config)?;

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

    /// Builds an FTS5 query fragment with alias expansion for a single term.
    ///
    /// This is a convenience method that uses default configuration.
    /// For backward compatibility, this method only performs alias expansion
    /// without broader concept expansion.
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
        let fts_query = self.build_fts_query(query)?;
        self.execute_fts_search(&fts_query, limit)
    }

    /// Builds the expanded FTS query string for a search query.
    ///
    /// This method handles alias expansion (always) and broader concept expansion
    /// (for queries with fewer than 3 terms). The returned string can be used
    /// directly with FTS5 MATCH queries.
    ///
    /// # Returns
    ///
    /// The expanded FTS query string, e.g., `("rust" OR "rustlang" OR "programming")`.
    pub fn build_fts_query(&self, query: &str) -> Result<String> {
        // Validate query is not empty or whitespace-only
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            anyhow::bail!("Search query cannot be empty");
        }

        // Load query expansion configuration from environment
        let config = QueryExpansionConfig::from_env();

        // Split query into terms and expand each with alias expansion
        let terms: Vec<&str> = trimmed_query.split_whitespace().collect();

        // Check if we should apply broader concept expansion (< 3 terms)
        let should_expand = should_expand_broader(trimmed_query);

        // Build FTS5 query with expansion for each term
        // AND logic between original query terms, OR within expansions
        let expanded_terms: Result<Vec<String>> = terms
            .iter()
            .map(|term| {
                if should_expand {
                    // Apply broader concept expansion
                    self.build_expanded_fts_term_with_config(term, &config)
                } else {
                    // Only apply alias expansion for queries with 3+ terms
                    self.build_expanded_fts_term(term)
                }
            })
            .collect();

        // Join with explicit AND for FTS5 when using parenthesized OR groups
        // FTS5 syntax requires explicit AND between parenthesized groups
        Ok(expanded_terms?.join(" AND "))
    }

    /// Executes an FTS5 search with the given pre-built query string.
    fn execute_fts_search(
        &self,
        fts_query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
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
            .query_map([fts_query], |row| Ok((row.get(0)?, row.get(1)?)))?
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

    /// Gets all tags with their statistics including note count and degree centrality.
    ///
    /// Queries tags that have at least one associated note, returning the tag ID,
    /// name, count of associated notes, and degree centrality (number of edges).
    ///
    /// # Returns
    ///
    /// Returns a vector of tuples containing (TagId, tag name, note count, degree centrality)
    /// for each tag with associated notes, ordered by tag name.
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
    /// service.create_note("Another rust note", Some(&["rust"]))?;
    ///
    /// // Get tags with statistics
    /// let tags = service.get_tags_with_stats()?;
    /// assert_eq!(tags.len(), 1);
    /// let (tag_id, name, note_count, centrality) = &tags[0];
    /// assert_eq!(name, "rust");
    /// assert_eq!(*note_count, 2);
    /// assert_eq!(*centrality, 0); // No edges created yet
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_tags_with_stats(&self) -> Result<Vec<(TagId, String, i64, i64)>> {
        let conn = self.db.connection();

        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, COUNT(DISTINCT nt.note_id) as note_count, COALESCE(t.degree_centrality, 0) as centrality
             FROM tags t
             JOIN note_tags nt ON t.id = nt.tag_id
             GROUP BY t.id, t.name, t.degree_centrality
             ORDER BY t.name",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let note_count: i64 = row.get(2)?;
            let centrality: i64 = row.get(3)?;
            Ok((TagId::new(id), name, note_count, centrality))
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
    /// Internal helper to create an edge without managing transactions.
    ///
    /// This method is used internally by both create_edge() and create_edges_batch()
    /// to avoid nested transactions. The caller is responsible for transaction management.
    fn create_edge_internal(
        &self,
        source_tag_id: TagId,
        target_tag_id: TagId,
        confidence: f64,
        hierarchy_type: &str,
        model_version: Option<&str>,
        now: i64,
    ) -> Result<()> {
        let conn = self.db.connection();

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

        // Increment degree_centrality for both tags
        conn.execute(
            "UPDATE tags SET degree_centrality = degree_centrality + 1 WHERE id = ?",
            [source_tag_id.get()],
        )?;

        conn.execute(
            "UPDATE tags SET degree_centrality = degree_centrality + 1 WHERE id = ?",
            [target_tag_id.get()],
        )?;

        Ok(())
    }

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

        // Use a transaction for atomicity (edge insert + centrality updates)
        conn.execute("BEGIN TRANSACTION", [])?;

        let result = self.create_edge_internal(
            source_tag_id,
            target_tag_id,
            confidence,
            hierarchy_type,
            model_version,
            now,
        );

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                Ok(())
            }
            Err(e) => {
                conn.execute("ROLLBACK", []).ok();
                Err(e)
            }
        }
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
            let now = OffsetDateTime::now_utc().unix_timestamp();

            for (source_tag_id, target_tag_id, confidence, hierarchy_type, model_version) in edges {
                self.create_edge_internal(
                    *source_tag_id,
                    *target_tag_id,
                    *confidence,
                    hierarchy_type,
                    *model_version,
                    now,
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

    /// Retrieves broader concepts for a given tag by traversing generic hierarchy edges.
    ///
    /// Queries the edges table for targets of generic (is-a) edges originating from
    /// the specified tag, filtered by minimum confidence threshold. Only traverses
    /// generic hierarchy type edges (not partitive).
    ///
    /// Edge direction: source_tag_id (narrower) -> target_tag_id (broader)
    ///
    /// # Arguments
    ///
    /// * `tag_id` - The tag to find broader concepts for
    /// * `min_confidence` - Minimum confidence threshold (0.0-1.0) for edges
    ///
    /// # Returns
    ///
    /// Returns `Vec<TagId>` of broader concept tag IDs, ordered by confidence descending.
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
    /// // Create tag hierarchy
    /// let rust = service.get_or_create_tag("rust")?;
    /// let programming = service.get_or_create_tag("programming")?;
    /// service.create_edge(rust, programming, 0.9, "generic", Some("test"))?;
    ///
    /// // Get broader concepts
    /// let broader = service.get_broader_concepts(rust, 0.7)?;
    /// assert_eq!(broader.len(), 1);
    /// assert_eq!(broader[0], programming);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_broader_concepts(&self, tag_id: TagId, min_confidence: f64) -> Result<Vec<TagId>> {
        let conn = self.db.connection();

        let mut stmt = conn.prepare(
            "SELECT target_tag_id FROM edges
             WHERE source_tag_id = ?1
               AND hierarchy_type = 'generic'
               AND confidence >= ?2
             ORDER BY confidence DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![tag_id.get(), min_confidence], |row| {
            let target_id: i64 = row.get(0)?;
            Ok(TagId::new(target_id))
        })?;

        let mut broader_concepts = Vec::new();
        for row_result in rows {
            broader_concepts.push(row_result?);
        }

        Ok(broader_concepts)
    }

    /// Retrieves broader concepts with their tag names.
    ///
    /// Combines broader concept retrieval with tag name lookup, returning both
    /// TagId and tag name for each broader concept. This is useful for building
    /// FTS queries with expanded terms.
    ///
    /// # Arguments
    ///
    /// * `tag_id` - The tag to find broader concepts for
    /// * `min_confidence` - Minimum confidence threshold (0.0-1.0) for edges
    ///
    /// # Returns
    ///
    /// Returns `Vec<(TagId, String)>` of (tag_id, tag_name) pairs, ordered by confidence.
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
    /// // Create tag hierarchy
    /// let rust = service.get_or_create_tag("rust")?;
    /// let programming = service.get_or_create_tag("programming")?;
    /// service.create_edge(rust, programming, 0.9, "generic", Some("test"))?;
    ///
    /// // Get broader concept names
    /// let broader_names = service.get_broader_concept_names(rust, 0.7)?;
    /// assert_eq!(broader_names.len(), 1);
    /// assert_eq!(broader_names[0].1, "programming");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_broader_concept_names(
        &self,
        tag_id: TagId,
        min_confidence: f64,
    ) -> Result<Vec<(TagId, String)>> {
        let conn = self.db.connection();

        let mut stmt = conn.prepare(
            "SELECT e.target_tag_id, t.name
             FROM edges e
             JOIN tags t ON e.target_tag_id = t.id
             WHERE e.source_tag_id = ?1
               AND e.hierarchy_type = 'generic'
               AND e.confidence >= ?2
             ORDER BY e.confidence DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![tag_id.get(), min_confidence], |row| {
            let target_id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((TagId::new(target_id), name))
        })?;

        let mut broader_concepts = Vec::new();
        for row_result in rows {
            broader_concepts.push(row_result?);
        }

        Ok(broader_concepts)
    }

    /// Deletes an edge between two tags in the hierarchy.
    ///
    /// Removes the directed edge from source_tag_id to target_tag_id and decrements
    /// the degree_centrality for both tags. Uses a transaction to ensure atomicity.
    /// This operation is idempotent - deleting a non-existent edge succeeds without error.
    ///
    /// # Arguments
    ///
    /// * `source_tag_id` - The source tag of the edge to delete
    /// * `target_tag_id` - The target tag of the edge to delete
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
    /// // Create tags and edge
    /// let tag1 = service.get_or_create_tag("tag1")?;
    /// let tag2 = service.get_or_create_tag("tag2")?;
    /// service.create_edge(tag1, tag2, 0.9, "generic", Some("test-model"))?;
    ///
    /// // Delete the edge
    /// service.delete_edge(tag1, tag2)?;
    ///
    /// // Deleting again is idempotent (no error)
    /// service.delete_edge(tag1, tag2)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_edge(&self, source_tag_id: TagId, target_tag_id: TagId) -> Result<()> {
        let conn = self.db.connection();

        // Use a transaction for atomicity (edge delete + centrality updates)
        conn.execute("BEGIN TRANSACTION", [])?;

        let result: Result<()> = (|| {
            // Check if edge exists before attempting delete
            let exists: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM edges
                 WHERE source_tag_id = ?1 AND target_tag_id = ?2
                   AND valid_from IS NULL AND valid_until IS NULL)",
                [source_tag_id.get(), target_tag_id.get()],
                |row| row.get(0),
            )?;

            if !exists {
                // Edge doesn't exist, this is a no-op (idempotent)
                return Ok(());
            }

            // Delete the edge
            conn.execute(
                "DELETE FROM edges
                 WHERE source_tag_id = ?1 AND target_tag_id = ?2
                   AND valid_from IS NULL AND valid_until IS NULL",
                [source_tag_id.get(), target_tag_id.get()],
            )?;

            // Decrement degree_centrality for both tags, but ensure it never goes negative
            // Use MAX(0, degree_centrality - 1) to prevent negative values
            conn.execute(
                "UPDATE tags SET degree_centrality = MAX(0, degree_centrality - 1) WHERE id = ?",
                [source_tag_id.get()],
            )?;

            conn.execute(
                "UPDATE tags SET degree_centrality = MAX(0, degree_centrality - 1) WHERE id = ?",
                [target_tag_id.get()],
            )?;

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                Ok(())
            }
            Err(e) => {
                conn.execute("ROLLBACK", []).ok();
                Err(e)
            }
        }
    }

    /// Searches for notes using spreading activation through the tag hierarchy graph.
    ///
    /// Parses the query string into terms, expands each term using alias resolution,
    /// and uses the resulting tags as seeds for spreading activation. Returns notes
    /// scored by the sum of (tag_activation * note_tags.confidence) for each activated
    /// tag on the note.
    ///
    /// # Algorithm
    ///
    /// 1. Parse query into whitespace-separated terms
    /// 2. Expand each term using `expand_search_term()` to handle aliases
    /// 3. Look up TagIds for all expanded terms
    /// 4. Execute spreading activation with initial activation 1.0 for seed tags
    /// 5. Score notes: `SUM(tag_activation * note_tags.confidence)` for each activated tag
    /// 6. Normalize scores to 0.0-1.0 range using min-max normalization
    /// 7. Sort by score descending and apply limit
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (terms separated by whitespace)
    /// * `limit` - Optional maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns `Vec<SearchResult>` with notes and normalized relevance scores (0.0-1.0).
    /// Returns empty vector if no tags match the query terms (cold-start case).
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
    /// // Create tags and notes
    /// let rust_tag = service.get_or_create_tag("rust")?;
    /// service.create_note("Learning Rust", Some(&["rust"]))?;
    ///
    /// // Search using graph spreading
    /// let results = service.graph_search("rust", Some(10))?;
    /// for result in &results {
    ///     println!("Score: {:.2}, Note: {}", result.relevance_score, result.note.content());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn graph_search(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>> {
        use crate::spreading_activation::{SpreadingActivationConfig, spread_activation};
        use std::collections::HashMap;

        let conn = self.db.connection();

        // Parse query into terms
        let terms: Vec<&str> = query.split_whitespace().collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        // Expand each term and collect all tag names
        let mut all_tag_names = std::collections::HashSet::new();
        for term in terms {
            let expansions = self.expand_search_term(term)?;
            for expansion in expansions {
                all_tag_names.insert(expansion);
            }
        }

        // Look up TagIds for all expanded tag names
        let mut seed_tags = HashMap::new();
        for tag_name in &all_tag_names {
            // Try direct tag lookup
            let tag_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
                    [tag_name],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(id) = tag_id {
                seed_tags.insert(TagId::new(id), 1.0);
            }
        }

        // Cold-start case: no matching tags found
        if seed_tags.is_empty() {
            return Ok(Vec::new());
        }

        // Execute spreading activation
        let config = SpreadingActivationConfig::from_env();
        let activated_tags = spread_activation(conn, &seed_tags, &config)?;

        // Score notes using: SUM(tag_activation * note_tags.confidence)
        // Since we can't bind arrays, we'll execute multiple queries
        let mut note_scores: HashMap<i64, f64> = HashMap::new();

        for (tag_id, activation) in &activated_tags {
            let mut stmt =
                conn.prepare("SELECT note_id, confidence FROM note_tags WHERE tag_id = ?1")?;

            let rows = stmt.query_map([tag_id.get()], |row| {
                let note_id: i64 = row.get(0)?;
                let confidence: f64 = row.get(1)?;
                Ok((note_id, confidence))
            })?;

            for row_result in rows {
                let (note_id, confidence) = row_result?;
                let score_contribution = activation * confidence;
                *note_scores.entry(note_id).or_insert(0.0) += score_contribution;
            }
        }

        // Sort by score descending
        let mut scored_notes: Vec<(i64, f64)> = note_scores.into_iter().collect();
        scored_notes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        if let Some(lim) = limit {
            scored_notes.truncate(lim);
        }

        // Load notes and normalize scores
        let mut results = Vec::new();

        // Find max score for min-max normalization
        let max_score = scored_notes
            .iter()
            .map(|(_, score)| *score)
            .fold(0.0_f64, f64::max);

        for (note_id, raw_score) in scored_notes {
            if let Some(note) = self.get_note(NoteId::new(note_id))? {
                // Normalize score to 0.0-1.0 range using min-max normalization
                // Higher raw scores = higher normalized scores
                let relevance_score = if max_score > 0.0 {
                    raw_score / max_score
                } else {
                    0.0
                };
                results.push(SearchResult {
                    note,
                    relevance_score,
                });
            }
        }

        Ok(results)
    }

    /// Searches for notes related to a given note using spreading activation.
    ///
    /// Uses the tags of the seed note as the starting points for spreading activation,
    /// with initial activation values weighted by the tag confidence from note_tags.
    /// The seed note itself is excluded from results.
    ///
    /// # Algorithm
    ///
    /// 1. Query note_tags to get all tags associated with the seed note
    /// 2. Use note_tags.confidence as initial activation weight for each tag
    /// 3. Execute spreading activation with confidence-weighted seeds
    /// 4. Score notes: `SUM(tag_activation * note_tags.confidence)` for each activated tag
    /// 5. Exclude the seed note from results
    /// 6. Normalize scores to 0.0-1.0 range
    /// 7. Sort by score descending and apply limit
    ///
    /// # Arguments
    ///
    /// * `note_id` - The ID of the note to find related notes for
    /// * `limit` - Optional maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns `Vec<SearchResult>` with related notes and normalized relevance scores.
    /// The seed note is excluded from results.
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
    /// // Create notes
    /// let note1 = service.create_note("Rust note", Some(&["rust"]))?;
    /// let note2 = service.create_note("Programming note", Some(&["programming"]))?;
    ///
    /// // Find notes related to note1
    /// let results = service.graph_search_from_note(note1.id(), Some(10))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn graph_search_from_note(
        &self,
        note_id: NoteId,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        use crate::spreading_activation::{SpreadingActivationConfig, spread_activation};
        use std::collections::HashMap;

        let conn = self.db.connection();

        // Get all tags associated with the seed note
        let mut stmt =
            conn.prepare("SELECT tag_id, confidence FROM note_tags WHERE note_id = ?1")?;

        let rows = stmt.query_map([note_id.get()], |row| {
            let tag_id: i64 = row.get(0)?;
            let confidence: f64 = row.get(1)?;
            Ok((TagId::new(tag_id), confidence))
        })?;

        let mut seed_tags = HashMap::new();
        for row_result in rows {
            let (tag_id, confidence) = row_result?;
            // Use note_tags.confidence as initial activation weight
            seed_tags.insert(tag_id, confidence);
        }

        // Cold-start case: seed note has no tags
        if seed_tags.is_empty() {
            return Ok(Vec::new());
        }

        // Execute spreading activation
        let config = SpreadingActivationConfig::from_env();
        let activated_tags = spread_activation(conn, &seed_tags, &config)?;

        // Score notes using: SUM(tag_activation * note_tags.confidence)
        let mut note_scores: HashMap<i64, f64> = HashMap::new();

        for (tag_id, activation) in &activated_tags {
            let mut stmt =
                conn.prepare("SELECT note_id, confidence FROM note_tags WHERE tag_id = ?1")?;

            let rows = stmt.query_map([tag_id.get()], |row| {
                let note_id_val: i64 = row.get(0)?;
                let confidence: f64 = row.get(1)?;
                Ok((note_id_val, confidence))
            })?;

            for row_result in rows {
                let (note_id_val, confidence) = row_result?;
                // Exclude the seed note from results
                if note_id_val == note_id.get() {
                    continue;
                }
                let score_contribution = activation * confidence;
                *note_scores.entry(note_id_val).or_insert(0.0) += score_contribution;
            }
        }

        // Sort by score descending
        let mut scored_notes: Vec<(i64, f64)> = note_scores.into_iter().collect();
        scored_notes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply limit
        if let Some(lim) = limit {
            scored_notes.truncate(lim);
        }

        // Load notes and normalize scores
        let mut results = Vec::new();

        // Find max score for min-max normalization
        let max_score = scored_notes
            .iter()
            .map(|(_, score)| *score)
            .fold(0.0_f64, f64::max);

        for (note_id_val, raw_score) in scored_notes {
            if let Some(note) = self.get_note(NoteId::new(note_id_val))? {
                // Normalize score to 0.0-1.0 range using min-max normalization
                // Higher raw scores = higher normalized scores
                let relevance_score = if max_score > 0.0 {
                    raw_score / max_score
                } else {
                    0.0
                };
                results.push(SearchResult {
                    note,
                    relevance_score,
                });
            }
        }

        Ok(results)
    }

    /// Searches for notes using dual-channel retrieval combining FTS and graph search.
    ///
    /// Executes both FTS (via `search_notes`) and graph-based (via `graph_search`)
    /// retrieval in parallel, then merges results using additive RRF-style scoring
    /// with an intersection bonus for notes found by both channels.
    ///
    /// Implements graceful degradation: when graph activation is sparse (below
    /// `min_avg_activation` threshold or fewer than `min_activated_tags` tags activated),
    /// the method falls back to FTS-only results to avoid noisy graph scores.
    ///
    /// # Algorithm
    ///
    /// 1. Load configuration from environment (or use defaults)
    /// 2. Execute FTS search via `search_notes(query, None)` (unlimited)
    /// 3. Execute graph search via `graph_search(query, None)` (unlimited)
    /// 4. Check cold-start conditions on graph results:
    ///    - Average relevance score < `min_avg_activation`, OR
    ///    - Result count < `min_activated_tags`
    /// 5. If cold-start detected, skip graph channel and return FTS-only results
    /// 6. Otherwise, merge results using HashMap<NoteId, DualSearchResult>
    /// 7. Calculate final scores:
    ///    - `final_score = (fts_score * fts_weight) + (graph_score * graph_weight) + intersection_bonus`
    ///    - Intersection bonus only applied when `found_by_both == true`
    /// 8. Sort by `final_score` descending
    /// 9. Apply `limit` if specified
    /// 10. Return results with metadata
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (whitespace-separated terms)
    /// * `limit` - Optional maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns tuple of `(Vec<DualSearchResult>, DualSearchMetadata)`:
    /// - Results ordered by `final_score` descending
    /// - Metadata indicating whether graph was skipped and result counts
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
    /// // Create notes with tags and edges
    /// let rust_tag = service.get_or_create_tag("rust")?;
    /// let prog_tag = service.get_or_create_tag("programming")?;
    /// service.create_edge(rust_tag, prog_tag, 0.9, "generic", Some("test"))?;
    /// service.create_note("Learning Rust programming", Some(&["rust"]))?;
    ///
    /// // Dual-channel search combines FTS and graph results
    /// let (results, metadata) = service.dual_search("rust", Some(10))?;
    ///
    /// for result in &results {
    ///     println!("Final score: {:.2}, Note: {}", result.final_score, result.note.content());
    ///     if result.found_by_both {
    ///         println!("  Found by both FTS and graph (bonus applied)");
    ///     }
    /// }
    ///
    /// if metadata.graph_skipped {
    ///     println!("Note: Graph search was skipped ({})", metadata.skip_reason.unwrap());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn dual_search(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<(Vec<DualSearchResult>, DualSearchMetadata)> {
        use std::collections::HashMap;

        // Load configuration from environment
        let config = DualSearchConfig::from_env();

        // Build and capture the expanded FTS query for metadata
        let expanded_fts_query = self.build_fts_query(query)?;

        // Execute both search channels
        let fts_results = self.search_notes(query, None)?;
        let graph_results = self.graph_search(query, None)?;

        let fts_result_count = fts_results.len();
        let graph_result_count = graph_results.len();

        // Check cold-start conditions for graph channel
        let should_skip_graph = if graph_results.is_empty() {
            true
        } else {
            // Calculate average activation score from graph results
            let avg_activation: f64 = graph_results.iter().map(|r| r.relevance_score).sum::<f64>()
                / graph_results.len() as f64;

            // Check both conditions (OR relationship)
            avg_activation < config.min_avg_activation
                || graph_results.len() < config.min_activated_tags
        };

        // If cold-start detected, return FTS-only results
        if should_skip_graph {
            let mut fts_only_results: Vec<DualSearchResult> = fts_results
                .into_iter()
                .map(|r| DualSearchResult {
                    final_score: r.relevance_score * config.fts_weight,
                    note: r.note,
                    fts_score: Some(r.relevance_score),
                    graph_score: None,
                    found_by_both: false,
                })
                .collect();

            // Sort by final_score descending
            fts_only_results.sort_by(|a, b| {
                b.final_score
                    .partial_cmp(&a.final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply limit
            if let Some(lim) = limit {
                fts_only_results.truncate(lim);
            }

            let metadata = DualSearchMetadata {
                graph_skipped: true,
                skip_reason: Some("sparse graph activation".to_string()),
                fts_result_count,
                graph_result_count: 0,
                expanded_fts_query: expanded_fts_query.clone(),
            };

            return Ok((fts_only_results, metadata));
        }

        // Merge results using HashMap keyed by NoteId
        let mut merged: HashMap<i64, DualSearchResult> = HashMap::new();

        // Add FTS results
        for fts_result in fts_results {
            let note_id = fts_result.note.id().get();
            merged.insert(
                note_id,
                DualSearchResult {
                    note: fts_result.note,
                    final_score: fts_result.relevance_score * config.fts_weight,
                    fts_score: Some(fts_result.relevance_score),
                    graph_score: None,
                    found_by_both: false,
                },
            );
        }

        // Add or merge graph results
        for graph_result in graph_results {
            let note_id = graph_result.note.id().get();

            if let Some(existing) = merged.get_mut(&note_id) {
                // Note found by both channels - merge scores
                existing.graph_score = Some(graph_result.relevance_score);
                existing.found_by_both = true;

                // Recalculate final_score with both channels and intersection bonus
                let fts_contribution = existing.fts_score.unwrap() * config.fts_weight;
                let graph_contribution = graph_result.relevance_score * config.graph_weight;
                existing.final_score =
                    fts_contribution + graph_contribution + config.intersection_bonus;
            } else {
                // Note found only by graph channel
                merged.insert(
                    note_id,
                    DualSearchResult {
                        note: graph_result.note,
                        final_score: graph_result.relevance_score * config.graph_weight,
                        fts_score: None,
                        graph_score: Some(graph_result.relevance_score),
                        found_by_both: false,
                    },
                );
            }
        }

        // Convert HashMap to Vec and sort by final_score descending
        let mut results: Vec<DualSearchResult> = merged.into_values().collect();
        results.sort_by(|a, b| {
            b.final_score
                .partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if let Some(lim) = limit {
            results.truncate(lim);
        }

        // Build metadata
        let metadata = DualSearchMetadata {
            graph_skipped: false,
            skip_reason: None,
            fts_result_count,
            graph_result_count,
            expanded_fts_query,
        };

        Ok((results, metadata))
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
