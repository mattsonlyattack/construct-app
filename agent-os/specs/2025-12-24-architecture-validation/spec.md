# Specification: Architecture Validation

## Goal
Verify that the layered architecture allows NoteService and all library types to be used independently of CLI dependencies (clap, dirs), proving reusability for future TUI/GUI interfaces.

## User Stories
- As a future TUI/GUI developer, I want to import and use NoteService without pulling in CLI dependencies so that I can build alternative interfaces efficiently
- As a library maintainer, I want to ensure no CLI-specific types leak into the public API so that the library boundary remains clean

## Specific Requirements

**Integration test file creation**
- Create a new integration test file at `tests/architecture_validation.rs`
- The test file must NOT import anything from main.rs or CLI modules
- The test file must only use types exported from `cons::` crate root
- Test must compile and run successfully without using clap or dirs crates

**NoteService isolation verification**
- Instantiate NoteService with Database::in_memory() in the test
- Verify NoteService::new() can be called without any CLI context
- Confirm database() accessor returns valid Database reference
- All operations should succeed in complete isolation from main.rs

**Core CRUD operations validation**
- Test create_note() with content only (no tags)
- Test create_note() with content and tags
- Test get_note() retrieves existing notes correctly
- Test get_note() returns None for non-existent IDs
- Test delete_note() removes notes successfully

**Tag operations validation**
- Test add_tags_to_note() with TagSource::User
- Test add_tags_to_note() with TagSource::llm() including model and confidence
- Verify tag assignments persist and can be retrieved via get_note()

**List operations validation**
- Test list_notes() with ListNotesOptions::default()
- Test list_notes() with limit option
- Test list_notes() with tags filter option
- Verify returned notes include their tag assignments

**Public API cleanliness check**
- Verify all required types are accessible from crate root: Database, NoteService, ListNotesOptions
- Verify domain types accessible: Note, NoteBuilder, NoteId, Tag, TagId, TagSource, TagAssignment
- Confirm no CLI types (Cli, Commands, AddCommand, ListCommand) are publicly exported

## Visual Design
No visual assets provided - this is an architecture validation task focused on code structure.

## Existing Code to Leverage

**tests/cli_add_integration.rs pattern**
- Demonstrates integration test file structure in tests/ directory
- Shows Database::in_memory() usage for isolated testing
- Pattern for using NoteService directly without CLI context
- Can replicate similar helper function approach if needed

**src/lib.rs public exports**
- Already exports: Database, Note, NoteBuilder, NoteId, Tag, TagAssignment, TagId, TagSource
- Already exports: NoteService, ListNotesOptions
- Integration test should use these exact exports to validate API surface

**src/service.rs unit tests**
- Contains comprehensive examples of all NoteService operations
- Demonstrates proper assertion patterns for CRUD and tag operations
- Shows ListNotesOptions usage patterns

**Database::in_memory() pattern**
- Established pattern for test isolation without file system dependencies
- Returns Result<Database> - handle appropriately in tests

## Out of Scope
- Creating a separate Cargo project or feature flag for compile-time validation
- Simulating hypothetical TUI workflow patterns beyond basic API calls
- Creating documentation artifacts beyond the test file itself
- Testing async behavior or Ollama client integration (roadmap item #7)
- Testing actual CLI binary invocation or clap parsing
- Performance benchmarking or stress testing
- Testing with file-based databases (only in-memory)
- Adding new public API methods to NoteService
- Modifying existing lib.rs exports or public API surface
