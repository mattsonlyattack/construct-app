# Task Breakdown: Tag Normalization

## Overview

**Feature**: Integrate existing `TagNormalizer` into `NoteService` layer to ensure consistent tag formatting across all entry points (CLI, TUI, future GUI).

**Total Tasks**: 11 (across 2 task groups)

**Complexity**: Low - This is a focused backend integration with minimal code changes.

## Task List

### Service Layer Integration

#### Task Group 1: NoteService Tag Normalization
**Dependencies:** None
**Specialist:** Backend/Rust Engineer

- [x] 1.0 Complete NoteService tag normalization integration
  - [x] 1.1 Write 4-6 focused tests for tag normalization in NoteService
    - Test: Creating note with "Machine Learning" tag results in "machine-learning" stored
    - Test: Querying tags via list_notes works with normalized form
    - Test: Duplicate detection works across formats (e.g., "Rust", "RUST", "rust" creates one tag)
    - Test: Special characters are stripped ("C++" becomes "c", "node.js" becomes "nodejs")
    - Test: Whitespace handling ("  rust  " normalizes to "rust")
    - Optional: add_tags_to_note normalizes before insertion
  - [x] 1.2 Import `TagNormalizer` into `src/service.rs`
    - Add `use crate::autotagger::TagNormalizer;` at top of file
    - Verify TagNormalizer is already exported via `pub use autotagger::TagNormalizer` in lib.rs
  - [x] 1.3 Modify `get_or_create_tag()` to normalize before database operations
    - Call `TagNormalizer::normalize_tag(name)` at the start of the method
    - Use normalized form for both SELECT query and INSERT statement
    - Location: `src/service.rs` line 309
  - [x] 1.4 Update deduplication logic in `create_note()`
    - Replace `tag_name.to_lowercase()` with `TagNormalizer::normalize_tag(tag_name)`
    - Location: `src/service.rs` line 108
    - This ensures deduplication matches the normalized form used in get_or_create_tag
  - [x] 1.5 Ensure NoteService tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Run existing service.rs tests to verify no regressions
    - Command: `cargo test service --lib`

**Acceptance Criteria:**
- Tags are normalized before database storage
- "Machine Learning" and "machine-learning" resolve to same tag
- "Rust", "RUST", "rust" create only one tag entry
- Existing tests continue to pass
- No changes needed to CLI layer (normalization is transparent)

---

### Test Review and Verification

#### Task Group 2: Test Review and Gap Analysis
**Dependencies:** Task Group 1
**Specialist:** QA/Test Engineer

- [x] 2.0 Review and verify test coverage for tag normalization
  - [x] 2.1 Review tests written in Task Group 1
    - Review the 4-6 tests written by backend engineer (Task 1.1)
    - Verify tests cover the specific requirements from spec.md
    - Check that test names are descriptive and follow conventions
  - [x] 2.2 Analyze test coverage gaps for this feature only
    - Verify normalization works through full create-retrieve cycle
    - Check that list_notes tag filter works with normalized tags
    - Ensure LLM-sourced tags (via add_tags_to_note with TagSource::Llm) are normalized
    - Focus ONLY on gaps related to tag normalization feature
  - [x] 2.3 Write up to 4 additional integration tests if needed
    - Add tests only if critical paths are missing from Task 1.1
    - Priority: End-to-end workflow (create with tags -> retrieve -> verify normalized)
    - Priority: Mixed case deduplication across user and LLM tags
    - Do NOT write exhaustive edge case tests
  - [x] 2.4 Run all feature-specific tests
    - Run: `cargo test` (full test suite to verify no regressions)
    - Verify all tag-related tests pass
    - Expected total: approximately 6-10 tests related to tag normalization

**Acceptance Criteria:**
- All tests pass (including existing TagNormalizer tests in normalizer.rs)
- Critical normalization workflows are covered
- No more than 4 additional tests added
- No regressions in existing functionality

---

## Execution Order

**Recommended implementation sequence:**

1. **Task Group 1: Service Layer Integration** (Backend Engineer)
   - Core implementation work
   - Write tests first (TDD approach)
   - Modify get_or_create_tag() and create_note()
   - Estimated effort: 1-2 hours

2. **Task Group 2: Test Review and Verification** (QA Engineer)
   - Review coverage after implementation
   - Fill critical gaps only
   - Verify no regressions
   - Estimated effort: 30 minutes - 1 hour

---

## Implementation Notes

### Files to Modify

| File | Changes |
|------|---------|
| `src/service.rs` | Import TagNormalizer, modify get_or_create_tag(), update create_note() deduplication |

### Files to Reference (No Changes Needed)

| File | Purpose |
|------|---------|
| `src/autotagger/normalizer.rs` | Contains TagNormalizer implementation and tests |
| `src/lib.rs` | Already exports TagNormalizer via autotagger module |
| `src/main.rs` | CLI layer - no changes needed (service handles normalization) |

### Code Changes Summary

**Change 1: Import (service.rs)**
```rust
use crate::autotagger::TagNormalizer;
```

**Change 2: get_or_create_tag() (service.rs, line 309)**
```rust
fn get_or_create_tag(&self, name: &str) -> Result<TagId> {
    let normalized = TagNormalizer::normalize_tag(name);  // ADD THIS LINE
    let conn = self.db.connection();

    // Use `normalized` instead of `name` in SELECT and INSERT
    // ...
}
```

**Change 3: create_note() deduplication (service.rs, line 108)**
```rust
// Change from:
let normalized = tag_name.to_lowercase();

// Change to:
let normalized = TagNormalizer::normalize_tag(tag_name);
```

---

## Out of Scope (Per Spec)

- Migration of existing database tags
- Tag aliases feature (e.g., "c++" -> "cpp")
- Changes to COLLATE NOCASE constraint
- CLI output formatting changes
- UI/display changes
- Changes to TagNormalizer implementation
- Changes to LLM prompt or auto-tagger behavior
