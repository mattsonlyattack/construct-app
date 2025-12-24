# Spec Requirements: Architecture Validation

## Initial Description
Architecture validation -- Verify layered architecture by confirming NoteService can be used without CLI dependencies, proving reusability for future TUI/GUI

This is roadmap item #6, estimated as XS (1 day). It validates the architecture after completing SQLite schema, core domain types, NoteService implementation, and CLI commands (add, list).

## Requirements Discussion

### First Round Questions

**Q1:** Should we create a dedicated integration test file (not part of service.rs) that imports and uses NoteService without any CLI dependencies (clap, dirs)?
**Answer:** Yes, create a dedicated integration test file (not part of service.rs) that imports and uses NoteService without any CLI dependencies (clap, dirs)

**Q2:** What is the scope of the isolation check - verify that NoteService, Database, and all domain types can be instantiated in complete isolation from main.rs?
**Answer:** Yes, verify that NoteService, Database, and all domain types can be instantiated in complete isolation from main.rs, AND verify that no CLI-specific types leak into the library's public API

**Q3:** Should we add a compile-time check (feature flag test or separate minimal Cargo project)?
**Answer:** No additional compile-time check needed (feature flag test or separate minimal Cargo project) - the integration test approach is sufficient

**Q4:** Should the validation include simulating a hypothetical TUI workflow?
**Answer:** No - just demonstrate that library API operations (create_note, list_notes, get_note, delete_note, add_tags_to_note) work alone. Skip hypothetical TUI workflow simulation

**Q5:** Should any documentation artifact be produced?
**Answer:** No documentation artifact needed - passing tests is sufficient

**Q6:** Is there anything that should be explicitly excluded from this validation?
**Answer:** Exclude future roadmap items (like async behavior from Ollama client in roadmap #7)

### Existing Code to Reference

**Similar Features Identified:**
- Feature: Existing unit tests - Path: `tests/` directory (Rust integration tests location)
- Feature: NoteService - Path: `src/service.rs` (the service being validated)
- Feature: Database - Path: `src/db/mod.rs` (database layer)
- Feature: Domain types - Path: `src/` (Note, Tag, and related types)

### Follow-up Questions
None required - user provided clear and complete answers.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
Not applicable - this is an architecture validation task focused on code structure, not UI.

## Requirements Summary

### Functional Requirements
- Create a dedicated integration test file in `tests/` directory
- Import and use NoteService without any CLI dependencies (clap, dirs)
- Verify NoteService, Database, and all domain types can be instantiated in isolation from main.rs
- Verify no CLI-specific types leak into the library's public API
- Demonstrate these library API operations work in isolation:
  - create_note
  - list_notes
  - get_note
  - delete_note
  - add_tags_to_note

### Reusability Opportunities
- Existing unit tests in the codebase can serve as reference for test structure
- Database::in_memory() pattern already exists for testing

### Scope Boundaries
**In Scope:**
- Integration test proving NoteService independence from CLI
- Verification of public API cleanliness (no CLI types exposed)
- Testing core library operations (CRUD for notes, tag management)

**Out of Scope:**
- Compile-time checks via feature flags or separate Cargo projects
- Simulating TUI workflow patterns
- Documentation artifacts
- Async behavior testing (roadmap #7 - Ollama client)
- Any future roadmap items beyond current architecture validation

### Technical Considerations
- Tests should use Database::in_memory() for isolation
- Test file should NOT import anything from main.rs or CLI modules
- Test should compile and run without clap or dirs crates being used
- Follows Rust convention of integration tests in `tests/` directory
