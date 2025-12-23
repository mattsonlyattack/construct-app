# Specification: NoteService Implementation

## Goal

Build the core business logic layer (NoteService) that provides UI-independent note CRUD operations and tag management, serving as the reusable interface between presentation layers (CLI/TUI/GUI) and the SQLite database.

## User Stories

- As a developer, I want a single NoteService struct that handles all note operations so that CLI, TUI, and GUI can share the same business logic
- As a user, I want to create notes with optional tags (by name) so that I can capture thoughts without worrying about tag IDs or management

## Specific Requirements

**NoteService struct with Database ownership**
- Create `NoteService` struct that owns a `Database` instance
- Provide `new(db: Database) -> Self` constructor
- Follow the same struct wrapper pattern used in `src/db.rs`
- No lifetime parameters; service takes full ownership of database

**Create note with optional tags**
- Signature: `create_note(&self, content: &str, tags: Option<&[&str]>) -> Result<Note>`
- Accept tag names as strings; service handles tag creation/lookup internally
- Insert note into `notes` table with current timestamp for `created_at` and `updated_at`
- For each tag name: look up existing tag by name (case-insensitive) or create new tag
- Create `note_tags` junction entries with `source='user'`, `confidence=1.0`, `verified=0`
- Return fully populated `Note` with `NoteId` and `TagAssignment` list
- Use transactions to ensure atomicity of note + tag operations

**Get note by ID**
- Signature: `get_note(&self, id: NoteId) -> Result<Option<Note>>`
- Return `None` if note does not exist (do not error)
- Include tag assignments in returned Note (join with `note_tags` and `tags` tables)
- Convert Unix timestamps to `OffsetDateTime` using `time` crate

**List notes with options**
- Signature: `list_notes(&self, options: ListNotesOptions) -> Result<Vec<Note>>`
- Create `ListNotesOptions` struct with `limit: Option<usize>` and `tags: Option<Vec<String>>`
- Default ordering: `created_at DESC` (newest first)
- Tag filtering: notes must have ALL specified tags (AND logic)
- Include tag assignments for each returned note
- Implement `Default` for `ListNotesOptions` for ergonomic usage

**Delete note by ID**
- Signature: `delete_note(&self, id: NoteId) -> Result<()>`
- Delete from `notes` table; cascade deletes handle `note_tags` cleanup
- Return `Ok(())` even if note does not exist (idempotent delete)
- Do not return whether the note existed

**Add tags to existing note**
- Signature: `add_tags_to_note(&self, note_id: NoteId, tags: &[&str], source: TagSource) -> Result<()>`
- Look up or create each tag by name (case-insensitive)
- Insert `note_tags` entries with appropriate `source`, `confidence`, `model_version` based on `TagSource`
- For `TagSource::User`: `source='user'`, `confidence=1.0`, `model_version=NULL`
- For `TagSource::Llm`: `source='llm'`, `confidence` from variant, `model_version` from variant
- Skip duplicate tag assignments (ON CONFLICT DO NOTHING or check first)
- Return error if note does not exist

**Module organization**
- Create `src/service.rs` file (modern Rust convention, not `src/service/mod.rs`)
- Export `NoteService` and `ListNotesOptions` from `src/lib.rs`
- Add `pub mod service;` to `lib.rs` module declarations

**Error handling**
- Use `anyhow::Result` for all fallible operations
- Propagate database errors with `?` operator
- No custom error enum for this iteration

## Existing Code to Leverage

**Database struct in `src/db.rs`**
- Follow the same ownership pattern: struct wrapping inner resource
- Use `db.connection()` method to access underlying `rusqlite::Connection`
- Pattern for schema initialization not needed (already done by Database)

**Domain types in `src/models/`**
- Use `Note`, `NoteBuilder`, `NoteId` for note representation
- Use `Tag`, `TagId` for tag representation
- Use `TagAssignment`, `TagSource` for tag-note relationships
- `NoteBuilder` provides ergonomic construction with defaults for timestamps

**Re-export pattern in `src/lib.rs`**
- Follow existing `pub use` pattern for new service types
- Group service exports with domain types

**Database schema in `src/db/schema.rs`**
- `notes.created_at` and `notes.updated_at` are INTEGER (Unix timestamps)
- `tags.name` uses `COLLATE NOCASE` for case-insensitive lookup
- `note_tags` has `confidence`, `source`, `verified`, `model_version` columns
- Foreign keys with `ON DELETE CASCADE` handle cleanup automatically

## Out of Scope

- CLI commands and argument parsing (roadmap items #4, #5)
- Ollama HTTP client integration (roadmap item #7)
- Auto-tagging via LLM (roadmap items #8, #9, #10)
- Tag normalization and alias resolution (roadmap item #11)
- Full-text search with FTS5 (roadmap item #12)
- Note content editing/updating (future roadmap item)
- Tag alias management via NoteService
- Async operations (SQLite is local, sync is appropriate)
- Custom error types (using anyhow for simplicity)
- Pagination with cursors (simple limit is sufficient for MVP)
