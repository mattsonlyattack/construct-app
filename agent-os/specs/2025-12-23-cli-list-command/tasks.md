# Task Breakdown: CLI List Command

## Overview
Total Tasks: 15 sub-tasks across 3 task groups

This is a Small (S) feature estimated at 2-3 days effort. The implementation leverages existing infrastructure:
- `NoteService::list_notes()` already supports limit and tag filtering
- `ListNotesOptions` struct already exists with `limit` and `tags` fields
- Existing `AddCommand` pattern provides template for CLI structure
- `parse_tags()` function already handles comma-separated input

## Task List

### CLI Layer

#### Task Group 1: List Command Implementation
**Dependencies:** None (existing service layer already supports the required functionality)

- [x] 1.0 Complete CLI list command implementation
  - [x] 1.1 Write 4 focused tests for list command functionality
    - Test `ListCommand` struct parsing with clap (short/long flags)
    - Test `execute_list()` with in-memory database returning notes
    - Test `execute_list()` with empty database showing "No notes found"
    - Test `execute_list()` with `--tags` filter applying correctly
  - [x] 1.2 Add `List` variant to `Commands` enum
    - Add `List(ListCommand)` to the `Commands` enum in `src/main.rs`
    - Add match arm in `main()` to call `handle_list()`
  - [x] 1.3 Create `ListCommand` struct with clap derives
    - Define `--limit` flag as `Option<usize>` with short `-l`
    - Define `--tags` flag as `Option<String>` with short `-t`
    - Add appropriate help text for each flag
    - Follow pattern from existing `AddCommand` struct
  - [x] 1.4 Implement `handle_list()` function
    - Get database path using existing `get_database_path()`
    - Ensure directory exists with `ensure_database_directory()`
    - Open database and delegate to `execute_list()`
    - Follow error handling pattern from `handle_add()`
  - [x] 1.5 Implement `execute_list()` for testability
    - Accept database instance for in-memory testing
    - Apply default limit of 10 when `--limit` not specified
    - Parse `--tags` using existing `parse_tags()` function
    - Call `NoteService::list_notes()` with `ListNotesOptions`
    - Return `Result<()>` following existing pattern
  - [x] 1.6 Ensure CLI list command tests pass
    - Run ONLY the 4 tests written in 1.1
    - Verify command parses correctly
    - Verify notes are retrieved from service layer

**Acceptance Criteria:**
- The 4 tests written in 1.1 pass
- `cons list` command is recognized by clap
- `--limit` and `--tags` flags parse correctly
- Default limit of 10 is applied when `--limit` not specified
- Service layer is called with correct `ListNotesOptions`

### Output Formatting Layer

#### Task Group 2: Tag Resolution and Note Display
**Dependencies:** Task Group 1

- [x] 2.0 Complete output formatting implementation
  - [x] 2.1 Write 4 focused tests for output formatting
    - Test tag name resolution from `TagId` to display name
    - Test timestamp formatting as "YYYY-MM-DD HH:MM"
    - Test note display with multiple tags showing `#tagname` format
    - Test empty results displaying "No notes found" message
  - [x] 2.2 Add tag name resolution to NoteService
    - Add `get_tag_name(&self, tag_id: TagId) -> Result<Option<String>>` method
    - Query `tags` table: `SELECT name FROM tags WHERE id = ?1`
    - Return `None` for non-existent tag IDs
    - Alternative: Add batch method `get_tag_names(&self, tag_ids: &[TagId]) -> Result<HashMap<TagId, String>>` for efficiency
  - [x] 2.3 Implement note display formatting
    - Create `format_note()` or `display_note()` helper function
    - Display note ID for reference (e.g., `[1]` or `ID: 1`)
    - Format `created_at` as "YYYY-MM-DD HH:MM" using `time` crate
    - Display full content without truncation
    - Display tags as space-separated hashtags (e.g., `#rust #programming`)
    - Separate notes with blank line for readability
  - [x] 2.4 Implement timestamp formatting
    - Use `time` crate formatting: `format_description!("[year]-[month]-[day] [hour]:[minute]")`
    - Handle UTC to local time conversion if needed
    - Ensure consistent 24-hour format
  - [x] 2.5 Implement empty results handling
    - Check if `list_notes()` returns empty vector
    - Display friendly "No notes found" message
    - Do not display any headers or formatting when empty
  - [x] 2.6 Ensure output formatting tests pass
    - Run ONLY the 4 tests written in 2.1
    - Verify tag names resolve correctly
    - Verify timestamps display in correct format

**Acceptance Criteria:**
- The 4 tests written in 2.1 pass
- Tag IDs are resolved to human-readable names
- Timestamps display as "YYYY-MM-DD HH:MM"
- Tags display uniformly as `#tagname` regardless of source
- Empty results show "No notes found" message

### Integration Layer

#### Task Group 3: End-to-End Integration and Test Review
**Dependencies:** Task Groups 1-2

- [x] 3.0 Complete integration and verify all tests pass
  - [x] 3.1 Review tests from Task Groups 1-2
    - Review the 4 tests written in Task 1.1 (CLI parsing/execution)
    - Review the 4 tests written in Task 2.1 (output formatting)
    - Total existing tests for this feature: 8 tests
  - [x] 3.2 Write up to 4 additional integration tests if needed
    - Test full `cons list` workflow with multiple notes
    - Test `cons list --limit 5` respects limit
    - Test `cons list --tags rust,programming` filters correctly
    - Test `cons list -l 3 -t rust` with combined short flags
  - [x] 3.3 Run all feature-specific tests
    - Run tests from 1.1, 2.1, and 3.2 (approximately 12 tests total)
    - Verify all CLI integration works correctly
    - Do NOT run the entire application test suite
  - [x] 3.4 Manual smoke test verification
    - Create a few test notes with `cons add`
    - Run `cons list` and verify output format
    - Run `cons list --limit 2` and verify limit works
    - Run `cons list --tags <tag>` and verify filtering works
    - Verify error handling for edge cases

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 12 tests total)
- `cons list` displays notes in verbose format with correct formatting
- `--limit` flag correctly limits output
- `--tags` flag correctly filters notes (AND logic)
- Empty database shows "No notes found" message
- Error handling follows existing patterns (exit codes 1/2)

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: CLI List Command Implementation** (1 day)
   - Define `ListCommand` struct and integrate with clap
   - Implement `handle_list()` and `execute_list()` functions
   - Wire up to existing `NoteService::list_notes()`

2. **Task Group 2: Output Formatting** (0.5-1 day)
   - Add tag name resolution capability
   - Implement verbose note display with timestamp formatting
   - Handle empty results gracefully

3. **Task Group 3: Integration and Verification** (0.5 day)
   - Review and fill critical test gaps
   - Run all feature tests
   - Manual smoke testing

## Implementation Notes

### Existing Code to Leverage

| Component | Location | Usage |
|-----------|----------|-------|
| `AddCommand` pattern | `/home/md/claude-cli-list/src/main.rs` | Template for `ListCommand` struct |
| `parse_tags()` | `/home/md/claude-cli-list/src/main.rs` | Parse `--tags` comma-separated input |
| `get_database_path()` | `/home/md/claude-cli-list/src/main.rs` | Database path resolution |
| `NoteService::list_notes()` | `/home/md/claude-cli-list/src/service.rs` | Core query method (already implemented) |
| `ListNotesOptions` | `/home/md/claude-cli-list/src/service.rs` | Query parameters struct |
| `Note` model | `/home/md/claude-cli-list/src/models/note.rs` | Note with `id()`, `content()`, `created_at()`, `tags()` |
| `TagAssignment` | `/home/md/claude-cli-list/src/models/tag_assignment.rs` | Tag with `tag_id()` accessor |
| `tags` table | `/home/md/claude-cli-list/src/db/schema.rs` | Tag name storage |

### Key Implementation Details

1. **Default Limit**: Apply limit of 10 when `--limit` is not specified:
   ```rust
   let limit = cmd.limit.unwrap_or(10);
   ```

2. **Tag Name Resolution**: Query tags table to get names for display:
   ```sql
   SELECT name FROM tags WHERE id = ?1
   ```

3. **Timestamp Formatting**: Use `time` crate macros:
   ```rust
   use time::macros::format_description;
   let format = format_description!("[year]-[month]-[day] [hour]:[minute]");
   note.created_at().format(&format)
   ```

4. **Tag Display**: Uniform hashtag format regardless of source:
   ```rust
   let tag_display: Vec<String> = tags.iter()
       .map(|t| format!("#{}", tag_name))
       .collect();
   ```

### Out of Scope

- Offset or cursor-based pagination (only limit)
- Interactive note selection
- Output to file export
- JSON output format option
- Compact single-line output mode
- Relative timestamp formatting ("2 hours ago")
- Visual distinction between user vs LLM tags
- Search by note content (text search)
- Sorting options other than newest-first
- Note editing or deletion from list view
