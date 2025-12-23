# Task Breakdown: NoteService Implementation

## Overview
Total Tasks: 21

This task breakdown implements the core NoteService business logic layer for the cons CLI application. The service provides UI-independent note CRUD operations and tag management, sitting between presentation layers (CLI/TUI/GUI) and the SQLite database.

## Execution Order

Recommended implementation sequence:
1. Module Setup (Task Group 1) - Foundation for all subsequent work
2. Core CRUD Operations (Task Group 2) - Basic note operations
3. Tag Management (Task Group 3) - Tag-to-note relationships
4. Query Operations (Task Group 4) - Filtering and listing
5. Test Review (Task Group 5) - Fill critical gaps only

---

## Task List

### Module Setup

#### Task Group 1: NoteService Module Foundation
**Dependencies:** None

- [x] 1.0 Complete module setup
  - [x] 1.1 Create `src/service.rs` file with NoteService struct
    - Struct owns `Database` instance (not borrowed)
    - Pattern: `pub struct NoteService { db: Database }`
    - Constructor: `pub fn new(db: Database) -> Self`
    - Follow same wrapper pattern as `src/db.rs`
  - [x] 1.2 Create `ListNotesOptions` struct in `src/service.rs`
    - Fields: `limit: Option<usize>`, `tags: Option<Vec<String>>`
    - Implement `Default` trait for ergonomic usage
    - All fields optional with sensible defaults
  - [x] 1.3 Update `src/lib.rs` with module declaration and re-exports
    - Add `pub mod service;` to module declarations
    - Add `pub use service::{NoteService, ListNotesOptions};`
    - Group with existing domain type exports
  - [x] 1.4 Write 2 focused tests for module setup
    - Test: NoteService construction with in-memory database
    - Test: ListNotesOptions default implementation

**Acceptance Criteria:**
- [x] `cargo build` succeeds with new module
- [x] NoteService and ListNotesOptions accessible from crate root
- [x] Tests in 1.4 pass

---

### Core CRUD Operations

#### Task Group 2: Note Create, Get, Delete
**Dependencies:** Task Group 1

- [x] 2.0 Complete core CRUD operations
  - [x] 2.1 Write 4 focused tests for CRUD operations
    - Test: create_note with content only (no tags) returns Note with valid id
    - Test: get_note returns None for non-existent id
    - Test: get_note returns Some(Note) for existing note
    - Test: delete_note is idempotent (Ok for non-existent id)
  - [x] 2.2 Implement `create_note` method (without tags)
    - Signature: `pub fn create_note(&self, content: &str, tags: Option<&[&str]>) -> Result<Note>`
    - Insert into notes table with current Unix timestamp for created_at/updated_at
    - Use `time::OffsetDateTime::now_utc().unix_timestamp()` for timestamps
    - Return Note with NoteId from `last_insert_rowid()`
    - Initially ignore tags parameter (implement in Task Group 3)
    - Use `anyhow::Result` for error handling
  - [x] 2.3 Implement `get_note` method
    - Signature: `pub fn get_note(&self, id: NoteId) -> Result<Option<Note>>`
    - Query notes table by id
    - Return None if note does not exist (not an error)
    - Convert INTEGER timestamps to `OffsetDateTime` using `OffsetDateTime::from_unix_timestamp()`
    - Use NoteBuilder to construct Note
    - Initially return empty tags vec (tag loading in Task Group 3)
  - [x] 2.4 Implement `delete_note` method
    - Signature: `pub fn delete_note(&self, id: NoteId) -> Result<()>`
    - DELETE from notes table where id = ?
    - Return Ok(()) regardless of whether note existed (idempotent)
    - Cascade deletes handle note_tags cleanup automatically
  - [x] 2.5 Verify CRUD tests pass
    - Run only the 4 tests written in 2.1
    - All tests should pass

**Acceptance Criteria:**
- [x] The 4 tests from 2.1 pass
- [x] Notes can be created, retrieved, and deleted
- [x] get_note returns None for missing notes
- [x] delete_note is idempotent

---

### Tag Management

#### Task Group 3: Tag Operations and Note-Tag Relationships
**Dependencies:** Task Group 2

- [x] 3.0 Complete tag management
  - [x] 3.1 Write 4 focused tests for tag operations
    - Test: create_note with tags creates note and associates tags
    - Test: create_note with duplicate tag names only creates one tag
    - Test: add_tags_to_note with TagSource::User sets correct metadata
    - Test: add_tags_to_note with TagSource::Llm includes model_version and confidence
  - [x] 3.2 Implement helper method `get_or_create_tag`
    - Signature: `fn get_or_create_tag(&self, name: &str) -> Result<TagId>`
    - Query tags table by name (case-insensitive via COLLATE NOCASE)
    - If found, return existing TagId
    - If not found, INSERT and return new TagId from last_insert_rowid()
    - Private helper method (not pub)
  - [x] 3.3 Update `create_note` to handle tags parameter
    - When tags is Some, process each tag name
    - Use get_or_create_tag for each tag name
    - Insert note_tags entries with source='user', confidence=1.0, verified=0
    - Use transaction for atomicity (INSERT note, create tags, INSERT note_tags)
    - Build TagAssignment list for returned Note
  - [x] 3.4 Implement `add_tags_to_note` method
    - Signature: `pub fn add_tags_to_note(&self, note_id: NoteId, tags: &[&str], source: TagSource) -> Result<()>`
    - Verify note exists first (return error if not)
    - For TagSource::User: source='user', confidence=1.0, model_version=NULL
    - For TagSource::Llm: source='llm', confidence from variant (convert to f64), model_version from variant
    - Use INSERT OR IGNORE to skip duplicate tag assignments
    - Set created_at to current Unix timestamp
  - [x] 3.5 Update `get_note` to load tag assignments
    - JOIN note_tags and tags tables when loading note
    - Build TagAssignment for each row with correct TagSource
    - Handle NULL model_version for user tags
    - Convert confidence REAL to u8 (multiply by 100 if stored as 0.0-1.0)
  - [x] 3.6 Verify tag operation tests pass
    - Run only the 4 tests written in 3.1
    - All tests should pass

**Acceptance Criteria:**
- [x] The 4 tests from 3.1 pass
- [x] Tags are created or reused correctly (case-insensitive)
- [x] Note-tag relationships have correct metadata based on source
- [x] get_note returns Note with populated tags

---

### Query Operations

#### Task Group 4: List Notes with Filtering
**Dependencies:** Task Group 3

- [x] 4.0 Complete query operations
  - [x] 4.1 Write 3 focused tests for list operations
    - Test: list_notes with default options returns notes in created_at DESC order
    - Test: list_notes with limit option respects limit
    - Test: list_notes with tags filter returns only notes with ALL specified tags
  - [x] 4.2 Implement `list_notes` method
    - Signature: `pub fn list_notes(&self, options: ListNotesOptions) -> Result<Vec<Note>>`
    - Base query: SELECT from notes ORDER BY created_at DESC
    - Apply LIMIT if options.limit is Some
    - Include tag assignments for each note (sub-query or separate queries)
  - [x] 4.3 Implement tag filtering in `list_notes`
    - When options.tags is Some, filter to notes having ALL specified tags
    - Use subquery or JOIN with GROUP BY and HAVING COUNT = tag_count
    - Tag matching is case-insensitive (tags.name COLLATE NOCASE)
    - AND logic: note must have every specified tag
  - [x] 4.4 Verify list operation tests pass
    - Run only the 3 tests written in 4.1
    - All tests should pass

**Acceptance Criteria:**
- [x] The 3 tests from 4.1 pass
- [x] Notes returned in newest-first order
- [x] Limit option works correctly
- [x] Tag filtering uses AND logic

---

### Test Review

#### Task Group 5: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-4

- [x] 5.0 Review tests and fill critical gaps only
  - [x] 5.1 Review existing tests from Task Groups 1-4
    - Review 2 tests from module setup (Task 1.4)
    - Review 4 tests from CRUD operations (Task 2.1)
    - Review 4 tests from tag operations (Task 3.1)
    - Review 3 tests from list operations (Task 4.1)
    - Total existing: 13 tests
  - [x] 5.2 Analyze gaps for critical user workflows only
    - Identify any untested critical paths for this feature
    - Focus on integration points between methods
    - Do NOT assess entire application coverage
  - [x] 5.3 Write up to 5 additional tests if needed
    - Consider: transaction rollback on failure
    - Consider: timestamp conversion accuracy
    - Consider: empty list handling
    - Skip edge cases unless business-critical
    - Maximum 5 new tests
  - [x] 5.4 Run all NoteService tests
    - Run tests in src/service.rs only
    - Expected total: 13-18 tests
    - Verify all pass
    - Run `cargo clippy` for lint check
    - Run `cargo fmt --check` for formatting

**Acceptance Criteria:**
- [x] All NoteService tests pass (13-18 tests total)
- [x] No clippy warnings
- [x] Code is properly formatted
- [x] Critical user workflows are covered

---

## Technical Notes

### Database Schema Reference
From `src/db/schema.rs`:
- `notes`: id (INTEGER PK), content (TEXT), created_at (INTEGER), updated_at (INTEGER)
- `tags`: id (INTEGER PK), name (TEXT UNIQUE COLLATE NOCASE)
- `note_tags`: note_id, tag_id (PK), confidence (REAL), source (TEXT), created_at (INTEGER), verified (INTEGER), model_version (TEXT nullable)

### Domain Types Reference
From `src/models/`:
- `NoteId(i64)` - newtype for note IDs
- `TagId(i64)` - newtype for tag IDs
- `Note` - built via NoteBuilder, has id, content, created_at, updated_at, tags
- `Tag` - has id, name, aliases
- `TagAssignment` - has tag_id, source, created_at, verified
- `TagSource::User` | `TagSource::Llm { model, confidence }`

### Patterns to Follow
- Wrapper pattern from `src/db.rs` for NoteService struct
- Re-export pattern from `src/lib.rs`
- Use `anyhow::Result` for all fallible operations
- Use `time::OffsetDateTime` for timestamp handling
- Store timestamps as Unix INTEGER in SQLite

### Key Decisions from Spec
1. NoteService owns Database (no lifetime parameters)
2. Module at `src/service.rs` (modern Rust style, not mod.rs)
3. Deep abstractions - service handles tag creation/lookup internally
4. Combined list method with `ListNotesOptions` struct
5. Delete returns `Result<()>`, idempotent (no error for missing notes)
6. Separate `add_tags_to_note()` method for adding tags post-creation
