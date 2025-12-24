# Task Breakdown: Architecture Validation

## Overview
Total Tasks: 12

This spec validates the layered architecture by confirming NoteService and all library types can be used independently of CLI dependencies (clap, dirs), proving reusability for future TUI/GUI interfaces.

## Task List

### Integration Test Layer

#### Task Group 1: Integration Test File Setup
**Dependencies:** None

- [x] 1.0 Complete integration test file setup
  - [x] 1.1 Create `tests/architecture_validation.rs` integration test file
    - File must NOT import anything from main.rs or CLI modules
    - Must only use types exported from `cons::` crate root
    - Follow pattern from existing `tests/cli_add_integration.rs`
  - [x] 1.2 Add test helper function for NoteService instantiation
    - Use `Database::in_memory()` for isolation
    - Return `NoteService` instance ready for testing
    - No CLI context (clap, dirs) dependencies

**Acceptance Criteria:**
- Integration test file exists at `tests/architecture_validation.rs`
- File compiles without CLI crate dependencies
- Helper function provides isolated NoteService instance

---

#### Task Group 2: NoteService Isolation Verification
**Dependencies:** Task Group 1

- [x] 2.0 Complete NoteService isolation tests
  - [x] 2.1 Write test: `test_noteservice_instantiates_without_cli_context`
    - Instantiate NoteService with Database::in_memory()
    - Verify NoteService::new() succeeds without any CLI setup
    - Confirm database() accessor returns valid Database reference
  - [x] 2.2 Write test: `test_noteservice_database_accessor_returns_valid_reference`
    - Access underlying database via service.database()
    - Verify connection can execute simple query (e.g., SELECT 1)
  - [x] 2.3 Run isolation verification tests
    - Execute: `cargo test --test architecture_validation`
    - Verify tests from 2.1-2.2 pass

**Acceptance Criteria:**
- NoteService can be instantiated without CLI dependencies
- Database accessor provides working database connection
- All isolation tests pass

---

#### Task Group 3: Core CRUD Operations Validation
**Dependencies:** Task Group 2

- [x] 3.0 Complete CRUD operations validation tests
  - [x] 3.1 Write test: `test_create_note_content_only`
    - Create note with content, no tags
    - Verify note ID is positive
    - Verify content matches input
  - [x] 3.2 Write test: `test_create_note_with_tags`
    - Create note with content and tags
    - Verify note has expected tag count
    - Verify tags are user-sourced
  - [x] 3.3 Write test: `test_get_note_existing`
    - Create note, then retrieve by ID
    - Verify all fields match
  - [x] 3.4 Write test: `test_get_note_nonexistent_returns_none`
    - Query for non-existent NoteId
    - Verify returns None (not error)
  - [x] 3.5 Write test: `test_delete_note_removes_note`
    - Create note, delete it, verify get returns None
  - [x] 3.6 Run CRUD validation tests
    - Execute: `cargo test --test architecture_validation`
    - Verify tests from 3.1-3.5 pass

**Acceptance Criteria:**
- All CRUD operations work in complete isolation from CLI
- create_note, get_note, delete_note function correctly
- Tests pass without CLI dependencies

---

#### Task Group 4: Tag Operations Validation
**Dependencies:** Task Group 3

- [x] 4.0 Complete tag operations validation tests
  - [x] 4.1 Write test: `test_add_tags_with_user_source`
    - Create note, add tags with TagSource::User
    - Retrieve note and verify tags present
    - Verify source is user, confidence is 100%
  - [x] 4.2 Write test: `test_add_tags_with_llm_source`
    - Create note, add tags with TagSource::llm()
    - Include model name and confidence value
    - Verify tag metadata persists correctly
  - [x] 4.3 Run tag operations validation tests
    - Execute: `cargo test --test architecture_validation`
    - Verify tests from 4.1-4.2 pass

**Acceptance Criteria:**
- TagSource::User tags work correctly
- TagSource::llm() tags include model and confidence
- Tag assignments persist and can be retrieved

---

#### Task Group 5: List Operations Validation
**Dependencies:** Task Group 4

- [x] 5.0 Complete list operations validation tests
  - [x] 5.1 Write test: `test_list_notes_default_options`
    - Create multiple notes
    - List with ListNotesOptions::default()
    - Verify all notes returned in correct order
  - [x] 5.2 Write test: `test_list_notes_with_limit`
    - Create 5 notes
    - List with limit: Some(2)
    - Verify exactly 2 notes returned
  - [x] 5.3 Write test: `test_list_notes_with_tags_filter`
    - Create notes with various tag combinations
    - Filter by specific tags
    - Verify AND logic works correctly
  - [x] 5.4 Run list operations validation tests
    - Execute: `cargo test --test architecture_validation`
    - Verify tests from 5.1-5.3 pass

**Acceptance Criteria:**
- list_notes() works with default options
- Limit option restricts result count
- Tags filter applies AND logic correctly
- Returned notes include their tag assignments

---

### API Surface Validation

#### Task Group 6: Public API Cleanliness Check
**Dependencies:** Task Group 5

- [x] 6.0 Complete public API cleanliness validation
  - [x] 6.1 Write test: `test_all_required_types_accessible_from_crate_root`
    - Import and use: Database, NoteService, ListNotesOptions
    - Import and use: Note, NoteBuilder, NoteId
    - Import and use: Tag, TagId, TagSource, TagAssignment
    - Verify all compile and are usable
  - [x] 6.2 Document CLI types that should NOT be exported
    - Confirm no public exports of: Cli, Commands, AddCommand, ListCommand
    - This is a verification step - no code changes needed
    - Add comment in test file listing excluded types
  - [x] 6.3 Run complete architecture validation test suite
    - Execute: `cargo test --test architecture_validation`
    - Verify ALL tests in the file pass
  - [x] 6.4 Run cargo clippy to verify no warnings
    - Execute: `cargo clippy --test architecture_validation`
    - Address any warnings if present

**Acceptance Criteria:**
- All library types accessible from cons:: crate root
- No CLI types leak into public API
- Full test suite passes
- No clippy warnings

---

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: Integration Test File Setup** - Creates the test file foundation
2. **Task Group 2: NoteService Isolation Verification** - Proves core isolation
3. **Task Group 3: Core CRUD Operations Validation** - Validates primary functionality
4. **Task Group 4: Tag Operations Validation** - Validates tag functionality
5. **Task Group 5: List Operations Validation** - Validates query functionality
6. **Task Group 6: Public API Cleanliness Check** - Final validation and cleanup

## Reference Files

- **Existing integration test pattern**: `/home/md/construct-app/tests/cli_add_integration.rs`
- **NoteService implementation**: `/home/md/construct-app/src/service.rs`
- **Public API exports**: `/home/md/construct-app/src/lib.rs`
- **Rust standards**: `/home/md/construct-app/agent-os/standards/rust/standards.md`

## Notes

- All tests use `Database::in_memory()` for isolation (per Rust standards)
- Tests should follow existing patterns from `tests/cli_add_integration.rs`
- No file-based database testing required (out of scope)
- No async/Ollama testing required (out of scope - roadmap item #7)
- Test count is intentionally minimal as this is a validation spec, not a feature spec
