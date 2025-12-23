# Task Breakdown: CLI add command

## Overview
Total Tasks: 19 sub-tasks across 4 task groups

This spec implements `cons add "<content>"` as the first CLI command for instant note capture, integrating with the existing NoteService layer.

## Integration Points with Existing Code

- **NoteService** (`src/service.rs`): Use `NoteService::new(db)` and `service.create_note(content, tags)`
- **Database** (`src/db/mod.rs`): Use `Database::open(path)` for file-based SQLite
- **Note model** (`src/models/note.rs`): Access `note.id()` for NoteId after creation
- **TagAssignment** (`src/models/tag_assignment.rs`): Access `tag_id()` for each assigned tag
- **Tag lookup**: TagAssignment only has `tag_id()`, need to query tags table for display names

## Task List

### Setup Layer

#### Task Group 1: Dependencies and CLI Structure
**Dependencies:** None

- [x] 1.0 Complete CLI setup layer
  - [x] 1.1 Add required dependencies to Cargo.toml
    - Add `clap = { version = "4.5", features = ["derive"] }` for CLI parsing
    - Add `dirs = "5.0"` for cross-platform data directory resolution
  - [x] 1.2 Create `src/main.rs` with basic clap structure
    - Define `Cli` struct with `#[command(name = "cons")]`
    - Use `#[derive(Parser)]` for the top-level CLI
    - Define `Commands` enum with `#[derive(Subcommand)]`
    - Add `AddCommand` variant as first subcommand
    - Include help text via `#[command(about = "...")]` attributes
  - [x] 1.3 Define AddCommand arguments
    - Required positional argument: `content: String` with `#[arg]`
    - Optional `--tags` flag: `tags: Option<String>` with `#[arg(short, long)]`
  - [x] 1.4 Implement basic command dispatch
    - Match on `Commands` enum in main function
    - Stub `Commands::Add(cmd)` handler that prints "Add command received"
  - [x] 1.5 Verify CLI structure builds and runs
    - Run `cargo build` to ensure compilation succeeds
    - Run `./target/debug/cons --help` to verify clap generates help
    - Run `./target/debug/cons add --help` to verify subcommand help

**Acceptance Criteria:**
- `cargo build` succeeds with clap and dirs dependencies
- `cons --help` shows available commands including `add`
- `cons add --help` shows content positional arg and --tags flag
- Basic dispatch works (prints stub message)

### Core Implementation Layer

#### Task Group 2: Database Path Resolution and Add Command Logic
**Dependencies:** Task Group 1

- [x] 2.0 Complete core add command implementation
  - [x] 2.1 Implement cross-platform database path resolution
    - Use `dirs::data_dir()` to get platform-specific data directory
    - Construct path as `{data_dir}/cons/notes.db`
    - Create helper function: `fn get_database_path() -> Result<PathBuf>`
    - Return error if data_dir() returns None (rare edge case)
  - [x] 2.2 Implement directory auto-creation
    - Check if parent directory exists before opening database
    - Use `std::fs::create_dir_all` to create `{data_dir}/cons/` if missing
    - Handle directory creation errors with user-friendly message
  - [x] 2.3 Implement tag parsing logic
    - Create helper function: `fn parse_tags(input: &str) -> Vec<String>`
    - Split on comma and trim whitespace from each tag
    - Filter out empty strings after trimming
    - Return Vec<String> for passing to NoteService
  - [x] 2.4 Implement content validation
    - Reject empty strings or whitespace-only content
    - Use `anyhow::bail!("Note content cannot be empty")` for clean error
    - Validate before any database operations
  - [x] 2.5 Implement add command handler
    - Get database path via helper function
    - Create directory if needed
    - Open database with `Database::open(path)`
    - Create NoteService with `NoteService::new(db)`
    - Parse tags if provided (convert Vec<String> to `&[&str]` for API)
    - Call `service.create_note(content, tags)`
  - [x] 2.6 Verify core logic works with manual testing
    - Run `./target/debug/cons add "Test note"` and check database file created
    - Run `./target/debug/cons add "Tagged note" --tags rust,learning` and verify

**Acceptance Criteria:**
- Database file created at correct platform-specific path
- Directory auto-created if missing
- Empty content rejected with clear error message
- Tags parsed correctly (comma-separated, whitespace trimmed)
- Note created via NoteService successfully

### Output and Error Handling Layer

#### Task Group 3: User Output and Error Handling
**Dependencies:** Task Group 2

- [x] 3.0 Complete output and error handling
  - [x] 3.1 Implement success output formatting
    - On success, output: `Note created (id: {id})`
    - If tags applied, append: ` with tags: {tag_names}`
    - Example: `Note created (id: 42) with tags: rust, learning`
    - Note: TagAssignment only has tag_id - need to query tags table for names
  - [x] 3.2 Implement tag name lookup for output
    - Add helper to query tag names by IDs from database
    - Alternative: Store tag names during create flow before passing to service
    - Choose simpler approach: echo back the input tag names (already have them)
  - [x] 3.3 Implement error handling at main boundary
    - Wrap main logic in `anyhow::Result<()>`
    - Use `if let Err(e) = run()` pattern in main
    - Print user-friendly error via `eprintln!("Error: {}", e)`
    - Exit with appropriate status code: 1 for user errors, 2 for internal errors
    - No stack traces in user-facing output
  - [x] 3.4 Verify output format and error handling
    - Run successful add and verify output format
    - Run with empty content and verify error message
    - Run with invalid path scenario (if testable) and verify error

**Acceptance Criteria:**
- Success message shows note ID ✓
- Success message shows tags when provided ✓
- Errors displayed without stack traces ✓
- Exit code 1 for user errors, 2 for internal errors ✓
- User-friendly error messages ✓

### Testing Layer

#### Task Group 4: Test Coverage
**Dependencies:** Task Groups 1-3

- [x] 4.0 Complete test coverage for CLI add command
  - [x] 4.1 Write 2-4 focused unit tests for helper functions
    - Test `parse_tags` with normal input: `"rust,learning"` -> `["rust", "learning"]`
    - Test `parse_tags` with whitespace: `" rust , learning "` -> `["rust", "learning"]`
    - Test `parse_tags` with empty elements: `"rust,,learning"` -> `["rust", "learning"]`
    - Test content validation rejects whitespace-only strings
  - [x] 4.2 Write 2-3 focused integration tests for add command
    - Test successful note creation without tags
    - Test successful note creation with tags
    - Test empty content rejection
    - Use in-memory database for test isolation
  - [x] 4.3 Run feature-specific tests only
    - Run only the tests written in 4.1 and 4.2
    - Verify all critical paths covered
    - Do NOT run entire application test suite

**Acceptance Criteria:**
- All 5-7 tests pass (helper functions + integration) ✓
- Tag parsing handles edge cases correctly ✓
- Content validation works as specified ✓
- Integration tests verify end-to-end flow with in-memory database ✓

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: Dependencies and CLI Structure** - Set up clap CLI skeleton
2. **Task Group 2: Database Path Resolution and Add Command Logic** - Core implementation
3. **Task Group 3: User Output and Error Handling** - Polish user experience
4. **Task Group 4: Test Coverage** - Add strategic tests

## Technical Notes

### Tag Display Challenge

The spec notes that `TagAssignment` only provides `tag_id()`, not the tag name. Options for displaying tags in success output:

1. **Recommended**: Echo back the input tag names (we have them before calling NoteService)
2. Alternative: Query tags table by IDs after note creation
3. Alternative: Extend NoteService to return created tag names

Option 1 is simplest and avoids additional database queries.

### Error Message Examples

Per spec requirements, error messages should be user-friendly:
- Empty content: `"Note content cannot be empty"`
- Database path error: `"Failed to determine data directory"`
- Directory creation error: `"Failed to create database directory: {path}"`
- Database error: `"Failed to save note: {reason}"`

### Cross-Platform Paths

Using `dirs::data_dir()`:
- Linux: `~/.local/share/cons/notes.db`
- macOS: `~/Library/Application Support/cons/notes.db`
- Windows: `C:\Users\<user>\AppData\Roaming\cons\notes.db`
