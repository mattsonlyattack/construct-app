# Task Breakdown: CLI list command

## Overview
Total Tasks: 18 sub-tasks across 4 task groups

This spec implements `cons list` command to display notes in chronological order with optional `--tags` filtering and `--limit` pagination, integrating with the existing NoteService layer.

## Integration Points with Existing Code

- **NoteService** (`src/service.rs`): Use `NoteService::list_notes(options)` with `ListNotesOptions`
- **ListNotesOptions** (`src/service.rs`): Use existing struct with `limit: Option<usize>` and `tags: Option<Vec<String>>`
- **parse_tags()** (`src/main.rs`): Reuse existing function for tag parsing
- **Database path resolution** (`src/main.rs`): Reuse `get_database_path()` and `ensure_database_directory()`
- **Note model** (`src/models/note.rs`): Access `note.id()`, `note.content()`, `note.created_at()`, `note.tags()`
- **Tag name resolution**: TagAssignment only has `tag_id()`, need to query tags table for display names

## Task List

### CLI Structure Layer

#### Task Group 1: Extend CLI with List Command
**Dependencies:** None

- [x] 1.0 Complete CLI structure extension
  - [x] 1.1 Extend Commands enum with List variant
    - Add `List(ListCommand)` variant to existing `Commands` enum in `src/main.rs`
    - Follow same pattern as existing `Add(AddCommand)` variant
  - [x] 1.2 Create ListCommand struct
    - Use `#[derive(Parser)]` attribute
    - Add `#[command(about = "...")]` for help text
    - Define optional `--tags` flag: `tags: Option<String>` with `#[arg(short, long)]`
    - Define optional `--limit` flag: `limit: Option<usize>` with `#[arg(short, long)]`
  - [x] 1.3 Add match arm for List command
    - Add `Commands::List(cmd)` case to match statement in main function
    - Call `handle_list(cmd)` handler function
  - [x] 1.4 Verify CLI structure builds and runs
    - Run `cargo build` to ensure compilation succeeds
    - Run `./target/debug/cons --help` to verify `list` command appears
    - Run `./target/debug/cons list --help` to verify flags are shown

**Acceptance Criteria:**
- `cargo build` succeeds with List command added
- `cons --help` shows `list` command
- `cons list --help` shows `--tags` and `--limit` flags
- Basic dispatch works (can call handle_list stub)

### Core Implementation Layer

#### Task Group 2: List Command Logic and Validation
**Dependencies:** Task Group 1

- [x] 2.0 Complete core list command implementation
  - [x] 2.1 Implement limit validation
    - Validate `--limit` is greater than 0 before calling NoteService
    - Return user-friendly error: "Limit must be greater than 0"
    - Use `anyhow::bail!` for error propagation
  - [x] 2.2 Implement tag parsing
    - Reuse existing `parse_tags()` function from `cons add` command
    - Convert `Option<String>` to `Option<Vec<String>>` for ListNotesOptions
    - Handle None case (no tag filtering)
  - [x] 2.3 Implement database and service setup
    - Reuse `get_database_path()` function for cross-platform paths
    - Reuse `ensure_database_directory()` for directory creation
    - Open database with `Database::open(&db_path)`
    - Create NoteService with `NoteService::new(db)`
  - [x] 2.4 Implement list_notes call and ordering
    - Build `ListNotesOptions` from parsed CLI arguments
    - Call `service.list_notes(options)` to get notes
    - Reverse the `Vec<Note>` results to achieve chronological order (oldest first)
    - Apply limit after reversing if specified (get oldest N notes)
  - [x] 2.5 Implement empty results handling
    - Check if notes vector is empty after fetching
    - Output "No notes found" message to stdout
    - Return success (exit code 0) - empty results are not an error
  - [x] 2.6 Verify core logic works with manual testing
    - Run `./target/debug/cons list` with existing notes and verify output
    - Run `./target/debug/cons list --limit 0` and verify validation error
    - Run `./target/debug/cons list --tags nonexistent` and verify "No notes found"

**Acceptance Criteria:**
- Limit validation rejects values <= 0 with clear error
- Tags parsed correctly using existing parse_tags function
- Database path resolution works (reuses existing functions)
- Notes fetched via NoteService successfully
- Notes displayed in chronological order (oldest first)
- Empty results show "No notes found" message
- Limit applied correctly after reversing order

### Output Formatting Layer

#### Task Group 3: Multi-line Output and Tag Name Resolution
**Dependencies:** Task Group 2

- [x] 3.0 Complete output formatting
  - [x] 3.1 Implement tag name resolution helper
    - Create helper function to query tag names from database by TagId
    - Query: `SELECT name FROM tags WHERE id = ?1` for each tag_id
    - Use NoteService database connection or add helper method
    - Return Vec<String> of tag names for a note
  - [x] 3.2 Implement timestamp formatting
    - Use `time` crate to format `OffsetDateTime` from `note.created_at()`
    - Format as: `YYYY-MM-DD HH:MM:SS`
    - Use `time::format_description` or `format!` macro with time formatting
  - [x] 3.3 Implement multi-line note output format
    - Format each note with blank line separator between notes
    - First line: `[ID] Created: YYYY-MM-DD HH:MM:SS`
    - Second line: `Tags: tag1, tag2` (or omit if no tags)
    - Third line: `Content: <full note content>`
    - Show full content without truncation
  - [x] 3.4 Implement output loop
    - Iterate through reversed notes vector
    - For each note: resolve tag names, format timestamp, output multi-line format
    - Handle notes with no tags gracefully (show "Tags: " or omit tags line)
  - [x] 3.5 Verify output format
    - Run `./target/debug/cons list` and verify format matches specification
    - Test with notes that have tags and notes without tags
    - Verify chronological ordering (oldest first)
    - Verify full content displayed (no truncation)

**Acceptance Criteria:**
- Tag names resolved correctly from database
- Timestamps formatted as `YYYY-MM-DD HH:MM:SS`
- Multi-line output format matches specification exactly
- Notes with no tags handled gracefully
- Full content displayed without truncation
- Blank lines separate notes correctly

### Testing Layer

#### Task Group 4: Test Coverage
**Dependencies:** Task Groups 1-3

- [x] 4.0 Complete test coverage for CLI list command
  - [x] 4.1 Write 2-4 focused unit tests for helper functions
    - Test limit validation rejects 0 and negative values
    - Test tag name resolution helper with valid tag IDs
    - Test timestamp formatting produces correct format
    - Test empty tag list handling in output
  - [x] 4.2 Write 3-5 focused integration tests for list command
    - Test `cons list` with no flags shows all notes chronologically
    - Test `cons list --limit N` shows oldest N notes
    - Test `cons list --tags tag1,tag2` filters correctly (AND logic)
    - Test `cons list --tags tag --limit N` combines both flags
    - Test `cons list --tags nonexistent` shows "No notes found"
    - Use in-memory database for test isolation
  - [x] 4.3 Run feature-specific tests only
    - Run only the tests written in 4.1 and 4.2
    - Verify all critical paths covered
    - Do NOT run entire application test suite

**Acceptance Criteria:**
- All 5-9 tests pass (helper functions + integration)
- Limit validation tested with edge cases
- Tag filtering tested with AND logic
- Chronological ordering verified
- Empty results handling tested
- Integration tests verify end-to-end flow with in-memory database

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: Extend CLI with List Command** - Add CLI structure
2. **Task Group 2: List Command Logic and Validation** - Core implementation
3. **Task Group 3: Multi-line Output and Tag Name Resolution** - Formatting and display
4. **Task Group 4: Test Coverage** - Add strategic tests

## Technical Notes

### Tag Name Resolution

The spec notes that `TagAssignment` only provides `tag_id()`, not the tag name. For displaying tags in list output:

1. **Recommended**: Query tags table by IDs after fetching notes
   - Create helper function: `fn get_tag_names(db: &Database, tag_ids: &[TagId]) -> Result<Vec<String>>`
   - Query: `SELECT name FROM tags WHERE id IN (...)` for batch lookup
   - Map tag_ids to names for each note

2. Alternative: Extend NoteService to return tag names in Note objects (future enhancement)

### Chronological Ordering

NoteService currently returns notes ordered by `created_at DESC` (newest first). Requirement is chronological (oldest first, newest last).

**Solution**: Reverse the `Vec<Note>` after fetching from NoteService:
```rust
let mut notes = service.list_notes(options)?;
notes.reverse(); // Convert DESC to ASC (chronological)
```

Apply limit after reversing to get oldest N notes when limit is specified.

### Output Format Example

```
[1] Created: 2025-12-20 10:30:15
Tags: rust, learning
Content: My first note about Rust

[2] Created: 2025-12-21 14:20:30
Content: A note without tags

[3] Created: 2025-12-23 09:15:45
Tags: project
Content: Latest note with project tag
```

### Error Message Examples

Per spec requirements, error messages should be user-friendly:
- Invalid limit: `"Limit must be greater than 0"`
- Database error: `"Failed to list notes: {reason}"`
- Tag resolution error: `"Failed to load tag names: {reason}"`

