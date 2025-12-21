# Task Breakdown: Data Layer

## Overview
Total Tasks: 14

This spec establishes the foundational SQLite database layer for the notes application. Since this is a backend-only data layer with no API or UI components, the task groups focus on schema definition, database connection management, and verification testing.

## Task List

### Database Schema

#### Task Group 1: Schema Definition
**Dependencies:** None

- [x] 1.0 Complete schema definition
  - [x] 1.1 Create `src/db/` module directory structure
    - Create `src/db/mod.rs` as module root
    - Create `src/db/schema.rs` for schema constant
  - [x] 1.2 Define INITIAL_SCHEMA constant in `src/db/schema.rs`
    - Notes table: id (INTEGER PRIMARY KEY), content (TEXT NOT NULL), created_at (INTEGER), updated_at (INTEGER)
    - Tags table: id (INTEGER PRIMARY KEY), name (TEXT NOT NULL UNIQUE COLLATE NOCASE)
    - Note_tags junction table: composite primary key (note_id, tag_id), foreign keys with ON DELETE CASCADE
    - Use CREATE TABLE IF NOT EXISTS for idempotent execution
  - [x] 1.3 Define index creation statements in INITIAL_SCHEMA
    - idx_notes_created index on notes(created_at)
    - idx_note_tags_note index on note_tags(note_id)
    - idx_note_tags_tag index on note_tags(tag_id)
    - Use CREATE INDEX IF NOT EXISTS for idempotent execution
  - [x] 1.4 Add AI-first metadata columns to note_tags table
    - confidence REAL DEFAULT 1.0 for LLM confidence scores
    - source TEXT DEFAULT 'user' to distinguish user-explicit vs llm-inferred
    - created_at TEXT DEFAULT CURRENT_TIMESTAMP
  - [x] 1.5 Add tag_aliases table for SKOS-style synonym mapping
    - alias TEXT PRIMARY KEY COLLATE NOCASE
    - canonical_tag_id INTEGER NOT NULL with foreign key to tags(id)
    - ON DELETE CASCADE for foreign key reference

**Acceptance Criteria:**
- `src/db/schema.rs` contains complete INITIAL_SCHEMA constant
- All CREATE statements use IF NOT EXISTS pattern
- Schema follows spec exactly (no title field on notes, COLLATE NOCASE on tags.name)
- note_tags table includes confidence, source, and created_at columns
- tag_aliases table exists for synonym resolution

---

### Database Connection

#### Task Group 2: Database Struct and Connection Methods
**Dependencies:** Task Group 1

- [x] 2.0 Complete database connection layer
  - [x] 2.1 Add required dependencies to Cargo.toml
    - rusqlite = { version = "0.32", features = ["bundled"] }
    - anyhow = "1.0"
  - [x] 2.2 Implement Database struct in `src/db/mod.rs`
    - Wrap rusqlite::Connection in Database struct
    - Use anyhow::Result for all fallible operations
  - [x] 2.3 Implement `in_memory()` constructor method
    - Open in-memory SQLite connection
    - Call schema initialization automatically
    - Return anyhow::Result<Database>
  - [x] 2.4 Implement `open(path: impl AsRef<Path>)` constructor method
    - Open file-based SQLite connection at given path
    - Call schema initialization automatically
    - Return anyhow::Result<Database>
  - [x] 2.5 Implement private schema initialization method
    - Execute all INITIAL_SCHEMA statements
    - Wrap in single transaction for atomicity
    - Enable foreign key enforcement with PRAGMA foreign_keys = ON
  - [x] 2.6 Re-export Database from `src/lib.rs`
    - Add `pub mod db;` declaration
    - Add `pub use db::Database;` for convenience

**Acceptance Criteria:**
- Database struct wraps rusqlite Connection
- Both in_memory() and open(path) methods work correctly
- Schema initializes automatically on connection open
- Foreign keys are enabled via PRAGMA
- Database is publicly accessible from crate root

---

### Testing

#### Task Group 3: Database Layer Tests
**Dependencies:** Task Group 2

- [x] 3.0 Complete database layer tests
  - [x] 3.1 Write 4-6 focused tests for database functionality
    - Test: in_memory() opens successfully and returns Ok
    - Test: schema tables exist after initialization (notes, tags, note_tags)
    - Test: schema indexes exist after initialization
    - Test: foreign key constraint is enforced (PRAGMA foreign_keys check)
    - Test: open(path) creates database file successfully
    - Test: re-opening existing database is idempotent (IF NOT EXISTS works)
  - [x] 3.2 Ensure all tests use in-memory databases for speed
    - Use Database::in_memory() for most tests
    - Use tempfile for open(path) test if needed
  - [x] 3.3 Run database layer tests and verify all pass
    - Run: cargo test
    - All 4-6 tests should pass
    - No warnings or errors

**Acceptance Criteria:**
- 4-6 focused tests written covering happy paths only
- All tests use in-memory databases (fast execution)
- All tests pass with cargo test
- Tests verify: connection opens, tables exist, indexes exist, foreign keys enabled

---

### Documentation

#### Task Group 4: Architecture Documentation
**Dependencies:** Task Group 3

- [x] 4.0 Complete architecture documentation
  - [x] 4.1 Document schema approach in ARCHITECTURE.md
    - Explain idempotent IF NOT EXISTS strategy
    - Note that migration tracking is intentionally deferred
    - Describe schema initialization on connection open
  - [x] 4.2 Verify project compiles and tests pass
    - Run: cargo build
    - Run: cargo test
    - Run: cargo clippy (if available)

**Acceptance Criteria:**
- ARCHITECTURE.md documents the schema initialization approach
- Project compiles without errors
- All tests pass
- Code follows Rust conventions

---

## Execution Order

Recommended implementation sequence:

1. **Schema Definition (Task Group 1)** - Define the database schema constant first as it has no dependencies
2. **Database Connection (Task Group 2)** - Implement the Database struct and connection methods that use the schema
3. **Testing (Task Group 3)** - Write and run tests to verify the implementation
4. **Documentation (Task Group 4)** - Document the approach and do final verification

## File Deliverables Summary

| File | Purpose |
|------|---------|
| `src/db/mod.rs` | Database struct with open() and in_memory() methods |
| `src/db/schema.rs` | INITIAL_SCHEMA constant with CREATE TABLE/INDEX statements |
| `src/lib.rs` | Re-export Database from db module |
| `Cargo.toml` | Add rusqlite and anyhow dependencies |
| `ARCHITECTURE.md` | Document schema initialization approach |

## Notes

- **No CRUD operations**: This spec only covers schema and connection; CRUD is future scope
- **Happy paths only**: Per project standards, skip edge case and error path tests
- **anyhow only**: Use anyhow::Result everywhere; defer thiserror to post-MVP
- **~200 LOC target**: Keep implementation minimal and focused
