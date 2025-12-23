# Spec Requirements: NoteService Implementation

## Initial Description

Build the core business logic layer (NoteService) independent of any UI, handling note CRUD operations and tag management. This is roadmap item #3, building on the completed SQLite schema (#1) and core domain types (#2).

Context:
- Rust CLI app "cons" - structure-last personal knowledge management
- NoteService is the core business logic layer between UI (CLI/TUI/GUI) and database
- Must be UI-independent and reusable
- Domain types exist: Note, Tag, NoteId, TagId, TagSource, TagAssignment (in src/models/)
- Database layer exists with SQLite schema (in src/db/)
- Future AI integration via OllamaClient for auto-tagging (out of scope for this spec)

## Requirements Discussion

### First Round Questions

**Q1:** I assume NoteService will be a struct that takes ownership of or borrows a `Database` instance, similar to how the existing `Database` struct wraps `Connection`. Should NoteService own the Database (`NoteService { db: Database }`) or take a reference (`NoteService<'a> { db: &'a Database }`)?
**Answer:** Ownership (NoteService owns Database)

**Q2:** I assume NoteService will live in a new `src/service.rs` or `src/service/mod.rs` module and be re-exported from `lib.rs` alongside `Database`. Is that correct, or do you prefer a different module organization?
**Answer:** Use modern Rust convention - `src/service.rs` NOT `src/service/mod.rs` (mod.rs is older style)

**Q3:** For note creation, I assume the signature should be something like `create_note(&self, content: &str, tags: Option<Vec<&str>>) -> Result<Note>` where tags are optional string names (not TagIds) since users provide tag names. The service would handle tag creation/lookup internally. Is that the right approach?
**Answer:** Yes, deep abstractions - service handles tag creation/lookup internally

**Q4:** For listing notes, I assume we need list methods for recent notes and filtered listing. Should these be separate methods, or combined with an options struct?
**Answer:** Combine with options struct

**Q5:** I assume we need `get_note(&self, id: NoteId) -> Result<Option<Note>>` for single note retrieval. For delete, should `delete_note(&self, id: NoteId) -> Result<bool>` return whether the note existed, or just `Result<()>` ignoring not-found cases?
**Answer:** Return `Result<()>`, ignore not-found

**Q6:** The schema includes `tag_aliases` for SKOS-style synonym mapping. I assume for MVP we should implement basic tag normalization (lowercase, consistent hyphenation) but defer alias management to a future iteration. Is that correct, or should alias handling be included now?
**Answer:** Deferred - it's roadmap item #11, not part of this spec

**Q7:** For adding tags to existing notes, I assume we need `add_tags_to_note()`. Should this be a separate method, or should we have an `update_note` method that handles content and tag changes together?
**Answer:** Separate `add_tags_to_note()` method

**Q8:** I assume we should define a `NoteServiceError` enum with variants like `DatabaseError`, `NoteNotFound`, etc., using `thiserror` for ergonomic error handling. Should errors be surfaced directly, or wrapped in a more generic crate-level error type?
**Answer:** Use `anyhow::Result` for now (simple approach)

**Q9:** Is there anything that should explicitly be OUT of scope for this NoteService implementation?
**Answer:** Check roadmap - items after #3 are out of scope (AI integration #7-10, full-text search #12, etc.)

### Existing Code to Reference

No similar existing features identified for reference. However, the following existing code in this codebase should be referenced:

- `src/db.rs` and `src/db/schema.rs` - Database wrapper pattern to follow
- `src/models/*.rs` - Domain types (Note, NoteBuilder, Tag, TagAssignment, TagSource, NoteId, TagId)
- `src/lib.rs` - Re-export pattern to follow

### Follow-up Questions

No follow-up questions needed - user's answers were comprehensive.

## Visual Assets

### Files Provided:

No visual assets provided.

### Visual Insights:

N/A - This is a backend service layer with no UI components.

## Requirements Summary

### Functional Requirements

**Note CRUD Operations:**
- Create note with content and optional tags (tag names as strings, service handles tag creation/lookup)
- Get single note by ID, returning `Option<Note>`
- List notes with options struct (limit, tag filtering)
- Delete note by ID, returning `Result<()>` (ignore not-found)

**Tag Management:**
- Add tags to existing note via separate `add_tags_to_note()` method
- Service handles tag creation/lookup internally (deep abstraction)
- Tags provided as string names, not TagIds
- Support for both user tags and LLM-inferred tags (TagSource)

**Architecture:**
- NoteService owns Database instance (not borrowed)
- Module at `src/service.rs` (modern Rust convention, not mod.rs)
- Re-exported from `src/lib.rs`
- UI-independent, reusable across CLI/TUI/GUI

### Reusability Opportunities

- Follow the same struct pattern as `Database` in `src/db.rs`
- Use existing domain types from `src/models/`
- Follow the re-export pattern in `src/lib.rs`

### Scope Boundaries

**In Scope:**
- Note CRUD operations (create, read, list, delete)
- Tag assignment to notes (user tags)
- Options struct for list queries (limit, tag filtering)
- Basic service layer with Database ownership
- Unit tests for core user flows

**Out of Scope:**
- CLI commands (roadmap items #4, #5)
- Ollama HTTP client (roadmap item #7)
- Auto-tagging / AI integration (roadmap items #8, #9)
- Tag normalization (roadmap item #11)
- Full-text search with FTS5 (roadmap item #12)
- Note editing/updating content (roadmap item #26 - future)
- Tag alias management (future)

### Technical Considerations

- **Error handling:** Use `anyhow::Result` for simplicity (no custom error types)
- **Database:** SQLite via rusqlite, synchronous (no async needed for local DB)
- **Timestamps:** Use `time` crate with `OffsetDateTime`, store as INTEGER (Unix timestamp) in SQLite
- **Tag storage:** Tags are case-insensitive in DB (`COLLATE NOCASE`)
- **Junction table metadata:** `note_tags` includes confidence, source, verified, model_version columns for AI-first design
- **Foreign keys:** Enabled via PRAGMA, cascade deletes configured

### API Design Notes

**NoteService struct:**
```rust
pub struct NoteService {
    db: Database,
}
```

**List options struct:**
```rust
pub struct ListNotesOptions {
    pub limit: Option<usize>,
    pub tags: Option<Vec<String>>,
}
```

**Key method signatures (approximate):**
```rust
impl NoteService {
    pub fn new(db: Database) -> Self
    pub fn create_note(&self, content: &str, tags: Option<&[&str]>) -> Result<Note>
    pub fn get_note(&self, id: NoteId) -> Result<Option<Note>>
    pub fn list_notes(&self, options: ListNotesOptions) -> Result<Vec<Note>>
    pub fn delete_note(&self, id: NoteId) -> Result<()>
    pub fn add_tags_to_note(&self, note_id: NoteId, tags: &[&str], source: TagSource) -> Result<()>
}
```
