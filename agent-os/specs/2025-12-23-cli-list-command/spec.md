# Specification: CLI List Command

## Goal
Implement `cons list` subcommand to display recent notes with optional tag filtering and pagination, enabling users to quickly review their captured thoughts.

## User Stories
- As a user, I want to list my recent notes so that I can review what I have captured
- As a user, I want to filter notes by tags so that I can find notes related to specific topics

## Specific Requirements

**List subcommand with clap**
- Add `List` variant to `Commands` enum with `ListCommand` struct
- Define `--limit` flag as `Option<usize>` with short `-l`
- Define `--tags` flag as `Option<String>` with short `-t` for comma-separated input
- Default limit is 10 notes when `--limit` is not specified

**Command handler pattern**
- Implement `handle_list()` function following existing `handle_add()` pattern
- Implement `execute_list()` for testability with in-memory database
- Resolve database path using existing `get_database_path()` and `ensure_database_directory()`

**Tag name resolution**
- Current `TagAssignment` only stores `TagId`, not the tag name
- Query the `tags` table to resolve `TagId` to tag name for display
- Add a helper method or extend service to fetch tag names by their IDs

**Output formatting**
- Display notes in verbose multi-line format (no compact mode needed)
- Format timestamp as "YYYY-MM-DD HH:MM" using `time` crate formatting
- Display tags uniformly as `#tagname` regardless of source (user vs LLM)
- Separate each note with a blank line for readability

**Note display structure**
- Show note ID for reference
- Show created timestamp in absolute format
- Show full content without truncation
- Show all associated tags formatted as hashtags

**Empty results handling**
- Display friendly message "No notes found" when query returns zero results
- Do not display column headers or formatting when empty

**Error handling**
- Follow existing pattern with `anyhow` for error propagation
- Use exit code 1 for user errors, 2 for internal errors
- Display user-friendly error messages via `eprintln!`

## Visual Design
No visual assets provided.

## Existing Code to Leverage

**`src/main.rs` - Add Command Pattern**
- Follow `AddCommand` struct pattern for defining `ListCommand` with clap derives
- Reuse `parse_tags()` function for parsing `--tags` comma-separated input
- Reuse `get_database_path()` and `ensure_database_directory()` for database setup
- Follow `handle_add`/`execute_add` separation pattern for testability

**`src/service.rs` - NoteService::list_notes()**
- Existing method already supports `ListNotesOptions` with `limit` and `tags` fields
- Uses AND logic for tag filtering (notes must have ALL specified tags)
- Returns notes ordered by `created_at` DESC (newest first)
- Returns fully populated `Note` objects with `TagAssignment` data

**`src/models/tag_assignment.rs` - TagAssignment**
- Provides `tag_id()` accessor for the `TagId`
- Tag names must be resolved separately from the `tags` table
- Source distinction (user vs LLM) exists but display will treat them uniformly

**`src/db/schema.rs` - Tags Table**
- Tags table has `id` and `name` columns
- Use simple SELECT query to resolve tag IDs to names for display

## Out of Scope
- Offset or cursor-based pagination (only limit is supported)
- Interactive note selection
- Output to file export
- JSON output format option
- Compact single-line output mode
- Relative timestamp formatting ("2 hours ago")
- Visual distinction between user vs LLM tags in output
- Search by note content (text search)
- Sorting options other than newest-first
- Note editing or deletion from list view
