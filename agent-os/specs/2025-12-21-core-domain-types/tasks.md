# Task Breakdown: Core Domain Types

## Overview
Total Tasks: 19 (across 4 task groups)

This spec defines foundational Rust domain types (`Note`, `Tag`, `TagAssignment`, `TagSource`) with proper idioms, derive macros, and constructors to serve as the data layer between SQLite and the `NoteService` business logic.

## Task List

### Infrastructure Layer

#### Task Group 1: Dependencies and Schema Updates
**Dependencies:** None

- [ ] 1.0 Complete infrastructure layer
  - [ ] 1.1 Write 4 focused tests for schema changes
    - Test `note_tags` table has `verified` column with default 0
    - Test `note_tags` table has `model_version` column (nullable TEXT)
    - Test `note_tags.created_at` is INTEGER type
    - Test `idx_tag_aliases_canonical` index exists
  - [ ] 1.2 Update `Cargo.toml` with required dependencies
    - Add `time = { version = "0.3", features = ["serde", "serde-human-readable"] }`
    - Add `serde = { version = "1.0", features = ["derive"] }`
    - Add `serde_json = "1.0"` for JSON roundtripping
  - [ ] 1.3 Update `INITIAL_SCHEMA` in `src/db/schema.rs`
    - Change `note_tags.created_at` from TEXT to INTEGER
    - Add `verified INTEGER DEFAULT 0` column to `note_tags` table
    - Add `model_version TEXT` column to `note_tags` table (nullable)
    - Add `CREATE INDEX IF NOT EXISTS idx_tag_aliases_canonical ON tag_aliases(canonical_tag_id)` index
  - [ ] 1.4 Ensure infrastructure tests pass
    - Run ONLY the 4 tests written in 1.1
    - Verify schema initializes correctly with in-memory database

**Acceptance Criteria:**
- The 4 tests written in 1.1 pass
- `cargo build` succeeds with new dependencies
- Schema runs idempotently with new columns and index
- Existing database tests in `src/db/mod.rs` still pass

### Domain Types Layer

#### Task Group 2: TagSource Enum and Tag Struct
**Dependencies:** Task Group 1

- [ ] 2.0 Complete TagSource and Tag types
  - [ ] 2.1 Write 4 focused tests for TagSource and Tag
    - Test TagSource serializes to/from JSON correctly ("User", "Llm")
    - Test TagSource deserialization fails on unknown variant
    - Test `Tag::new(id, name)` creates tag with empty aliases
    - Test `Tag::with_aliases(id, name, aliases)` creates tag with aliases
  - [ ] 2.2 Create `src/models/mod.rs` module file
    - Add submodule declarations for `tag_source`, `tag`, `tag_assignment`, `note`
    - Re-export public types: `TagSource`, `Tag`, `TagAssignment`, `Note`, `NoteBuilder`
  - [ ] 2.3 Implement `TagSource` enum in `src/models/tag_source.rs`
    - Variants: `User`, `Llm`
    - Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
    - Serde fails on unknown enum variants by default (no attribute needed)
    - Use `#[serde(rename_all = "lowercase")]` to serialize as "user"/"llm" matching Display output
    - Implement `Display` trait: "user" for User, "llm" for Llm
  - [ ] 2.4 Implement `Tag` struct in `src/models/tag.rs`
    - Fields: `id: i64`, `name: String`, `aliases: Vec<String>`
    - Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
    - Constructor `Tag::new(id: i64, name: impl Into<String>)` with empty aliases
    - Constructor `Tag::with_aliases(id: i64, name: impl Into<String>, aliases: Vec<String>)`
  - [ ] 2.5 Update `src/lib.rs` to expose models module
    - Add `pub mod models;`
    - Add `pub use models::{Tag, TagSource};` (Note and TagAssignment added later)
  - [ ] 2.6 Ensure TagSource and Tag tests pass
    - Run ONLY the 4 tests written in 2.1
    - Verify JSON roundtripping works correctly

**Acceptance Criteria:**
- The 4 tests written in 2.1 pass
- TagSource fails to deserialize unknown variants
- Tag constructors work correctly
- Types are accessible from crate root

#### Task Group 3: TagAssignment and Note Structs
**Dependencies:** Task Group 2

- [ ] 3.0 Complete TagAssignment and Note types
  - [ ] 3.1 Write 5 focused tests for TagAssignment and Note
    - Test TagAssignment serialization/deserialization roundtrip
    - Test TagAssignment equality with same confidence values
    - Test `NoteBuilder` creates Note with default empty tags
    - Test `NoteBuilder` allows setting all fields
    - Test Note serialization/deserialization roundtrip
  - [ ] 3.2 Implement `TagAssignment` struct in `src/models/tag_assignment.rs`
    - Fields: `tag_id: i64`, `confidence: u8`, `source: TagSource`, `created_at: OffsetDateTime`, `verified: bool`, `model_version: Option<String>`
    - Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
    - Constructor `TagAssignment::new(tag_id, confidence, source, created_at, model_version)` with verified=false
    - Constructor `TagAssignment::user_created(tag_id, created_at)` with confidence=100, source=User, model_version=None
  - [ ] 3.3 Implement `Note` struct in `src/models/note.rs`
    - Fields: `id: i64`, `content: String`, `created_at: OffsetDateTime`, `updated_at: OffsetDateTime`, `tags: Vec<TagAssignment>`
    - Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`
  - [ ] 3.4 Implement `NoteBuilder` in `src/models/note.rs`
    - Required field: `id`, `content`
    - Optional fields with defaults: `created_at` (now), `updated_at` (now), `tags` (empty vec)
    - Builder methods: `id()`, `content()`, `created_at()`, `updated_at()`, `tags()`, `build()`
    - Follow standard Rust builder pattern with owned self
  - [ ] 3.5 Update `src/lib.rs` to expose remaining types
    - Update re-exports: `pub use models::{Note, NoteBuilder, Tag, TagAssignment, TagSource};`
  - [ ] 3.6 Ensure TagAssignment and Note tests pass
    - Run ONLY the 5 tests written in 3.1
    - Verify builder pattern works correctly

**Acceptance Criteria:**
- The 5 tests written in 3.1 pass
- TagAssignment properly implements Eq despite f64 field
- NoteBuilder allows flexible Note construction
- All types are accessible from crate root

### Testing Layer

#### Task Group 4: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-3

- [ ] 4.0 Review existing tests and fill critical gaps only
  - [ ] 4.1 Review tests from Task Groups 1-3
    - Review 4 tests written in Task 1.1 (schema changes)
    - Review 4 tests written in Task 2.1 (TagSource, Tag)
    - Review 5 tests written in Task 3.1 (TagAssignment, Note)
    - Total existing tests: 13 tests
  - [ ] 4.2 Analyze test coverage gaps for THIS feature only
    - Identify critical behaviors lacking coverage
    - Focus on integration between types (e.g., Note with TagAssignments)
    - Check Display trait implementations have basic coverage
  - [ ] 4.3 Write up to 5 additional strategic tests maximum
    - Test Note with multiple TagAssignments of different sources
    - Test TagSource Display implementations
    - Additional tests only if critical gaps identified in 4.2
  - [ ] 4.4 Run feature-specific tests only
    - Run `cargo test` for all tests in `src/models/` module
    - Run updated schema tests from Task 1.1
    - Expected total: approximately 12-17 tests maximum
    - Verify all domain type behaviors work correctly

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 12-17 tests total)
- Core domain type behaviors are covered
- No more than 5 additional tests added
- `cargo clippy` passes with no warnings
- `cargo fmt --check` passes

## Execution Order

Recommended implementation sequence:
1. Infrastructure Layer (Task Group 1) - Dependencies and schema updates
2. TagSource and Tag (Task Group 2) - Simple types first, no dependencies on other domain types
3. TagAssignment and Note (Task Group 3) - Depends on TagSource, builds on Tag patterns
4. Test Review (Task Group 4) - Final verification and gap analysis

## File Structure After Completion

```
src/
  lib.rs                    # Updated: add models module and re-exports
  db/
    mod.rs                  # Unchanged
    schema.rs               # Updated: note_tags changes, new index
  models/
    mod.rs                  # New: module declarations and re-exports
    tag_source.rs           # New: TagSource enum
    tag.rs                  # New: Tag struct
    tag_assignment.rs       # New: TagAssignment struct
    note.rs                 # New: Note struct and NoteBuilder

Cargo.toml                  # Updated: time, serde, serde_json dependencies
```

## Technical Notes

- Use `time::OffsetDateTime` for all timestamps, not `chrono`
- TagSource must use strict deserialization (fail on unknown variants)
- TagAssignment uses `u8` for confidence (0-100 percentage)
- NoteBuilder follows Rust builder pattern with method chaining
- All types derive Serialize/Deserialize for Ollama API JSON roundtripping
- Schema uses idempotent IF NOT EXISTS pattern (no versioned migrations)
