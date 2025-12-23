# Specification: CLI list command

## Goal

Implement `cons list` command to display notes in chronological order with optional `--tags` filtering and `--limit` pagination, using clap subcommand structure and integrating with the existing NoteService layer.

## User Stories

- As a user, I want to list my notes with `cons list` so that I can review my captured thoughts in chronological order
- As a user, I want to filter notes by tags with `cons list --tags rust,learning` so that I can find notes related to specific topics
- As a user, I want to limit the number of notes shown with `cons list --limit 10` so that I can focus on recent entries

## Specific Requirements

**CLI subcommand structure**
- Extend existing `Commands` enum in `src/main.rs` with `List(ListCommand)` variant
- Create `ListCommand` struct using clap derive macros with `#[derive(Parser)]`
- Add `ListCommand` variant to `Commands` enum alongside existing `Add` variant
- Add match arm in main function to handle `Commands::List(cmd)` and call `handle_list(cmd)`

**List command arguments**
- Support optional `--tags` flag with comma-separated values: `#[arg(short, long)]` on `tags: Option<String>`
- Support optional `--limit` flag accepting positive integer: `#[arg(short, long)]` on `limit: Option<usize>`
- Parse tags using same `parse_tags()` function from `cons add` (split on comma, trim whitespace, filter empty)
- Validate `--limit` is greater than 0 before calling NoteService, return user-friendly error if invalid

**Note ordering**
- NoteService currently orders by `created_at DESC` (newest first)
- Requirement is chronological order (oldest first, newest last)
- Reverse the `Vec<Note>` results after fetching from NoteService to achieve chronological ordering
- Apply limit after reversing to get oldest N notes when limit is specified

**Output format**
- Display each note in multi-line format with blank line separator between notes
- Format: `[ID] Created: YYYY-MM-DD HH:MM:SS` on first line
- Format: `Tags: tag1, tag2` on second line (or omit if no tags)
- Format: `Content: <full note content>` on third line
- Show full content without truncation (no character limits)
- Use `time` crate formatting for timestamps (Note has `OffsetDateTime` for `created_at`)

**Tag name resolution**
- Note objects contain `TagAssignment` objects with `tag_id()` but not tag names
- Query tags table to resolve tag IDs to tag names for display
- Use `NoteService` database connection or add helper method to resolve tag names
- Handle notes with no tags gracefully (show "Tags: " line or omit tags line)

**Empty results handling**
- When no notes match filters, output "No notes found" message
- Return success (exit code 0) even when no notes found (not an error condition)
- Display message to stdout, not stderr

**Error handling**
- Reuse error handling patterns from `cons add` command
- Use `anyhow::Result` for error propagation
- Validate `--limit > 0` with user-friendly error: "Limit must be greater than 0"
- Catch database errors and format with user-friendly messages
- No stack traces in user-facing output
- Exit with appropriate status codes (1 for user errors, 2 for internal errors)

**Database and service integration**
- Reuse `get_database_path()` and `ensure_database_directory()` functions from `cons add`
- Open database using `Database::open(&db_path)` pattern
- Create `NoteService` instance with `NoteService::new(db)`
- Call `service.list_notes(options)` with `ListNotesOptions` struct
- Build `ListNotesOptions` from parsed CLI arguments (tags as `Option<Vec<String>>`, limit as `Option<usize>`)

## Existing Code to Leverage

**NoteService::list_notes() (src/service.rs)**
- Use existing `list_notes(&self, options: ListNotesOptions) -> Result<Vec<Note>>` method
- Supports `ListNotesOptions` with `limit: Option<usize>` and `tags: Option<Vec<String>>`
- Returns notes ordered by `created_at DESC` (will need to reverse for chronological display)
- Handles tag filtering with AND logic (notes must have ALL specified tags)
- Returns fully populated Note objects with TagAssignment list

**parse_tags() function (src/main.rs)**
- Reuse existing `parse_tags(input: &str) -> Vec<String>` function
- Splits on comma, trims whitespace, filters empty strings
- Already tested and used by `cons add` command

**Database path resolution (src/main.rs)**
- Reuse `get_database_path() -> Result<PathBuf>` function for cross-platform paths
- Reuse `ensure_database_directory(db_path: &Path) -> Result<()>` for directory creation
- Same XDG Base Directory compliance as `cons add` command

**Error handling patterns (src/main.rs)**
- Reuse `is_user_error()` function to determine exit codes
- Follow same error message formatting and propagation patterns
- Use `anyhow::Context` for error context when appropriate

**CLI structure (src/main.rs)**
- Follow same pattern as `AddCommand` struct definition
- Use clap derive macros with `#[derive(Parser)]` and `#[arg(short, long)]` attributes
- Extend `Commands` enum following existing pattern

**Note model (src/models/note.rs)**
- Access `note.id()` for NoteId display
- Access `note.content()` for content display
- Access `note.created_at()` for timestamp (returns `OffsetDateTime`)
- Access `note.tags()` for `&[TagAssignment]` slice

**Tag name resolution**
- Need to query tags table to get tag names from TagId
- Use `NoteService` database connection or add helper method
- Query: `SELECT name FROM tags WHERE id = ?1` for each tag_id

## Out of Scope

- Sorting options beyond chronological (always oldest to newest)
- Date range filtering (no `--from` or `--to` flags)
- Reverse order flag (no `--reverse` to show newest first)
- JSON output format (plain text only)
- Content truncation (always show full content, no `--truncate` flag)
- OR logic for tags (only AND logic - notes must have ALL specified tags)
- Default limit when no flags provided (show all notes if no limit specified)
- Tag normalization (deferred to roadmap item #11)
- Other CLI commands (search, edit implemented separately)
- TUI or GUI interfaces
- Pagination beyond simple limit (no offset/cursor-based pagination)
- Colorized or formatted output (plain text terminal output)

