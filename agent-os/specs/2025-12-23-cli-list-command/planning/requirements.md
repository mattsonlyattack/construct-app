# Spec Requirements: CLI List Command

## Initial Description

Implement `cons list` showing recent notes with `--tags` filtering and `--limit` pagination. This is for a personal knowledge management CLI tool called "cons" - a Rust-based, local-first PKM tool where users capture thoughts freely and AI handles tagging. The architecture is layered with NoteService as the core business logic layer.

## Requirements Discussion

### First Round Questions

**Q1:** Default limit behavior - should `cons list` without any flags show a reasonable default number of notes (e.g., 10 or 20 most recent) rather than all notes?
**Answer:** Default to 10 notes.

**Q2:** Output format - should each note display as a compact single line by default, or should we support a `--full` or `--verbose` flag for multi-line output showing complete content?
**Answer:** Show verbose by default, no flags needed.

**Q3:** Tag display format - should we distinguish between user-added tags and LLM-generated tags in the output, or treat them identically in display?
**Answer:** Treat user and LLM tags the same.

**Q4:** Tag filtering with `--tags` - the existing `NoteService::list_notes()` uses AND logic (notes must have ALL specified tags). Is that the desired behavior?
**Answer:** AND logic (must have ALL specified tags) - confirmed.

**Q5:** Empty results messaging - when no notes match, should we display a friendly message like "No notes found" rather than empty output?
**Answer:** Yes, show friendly "No notes found" message.

**Q6:** Timestamp format - should we use human-readable relative format ("2 hours ago") or consistent absolute format ("2025-12-23 14:30")?
**Answer:** Consistent absolute format (e.g., "2025-12-23 14:30").

**Q7:** Is there anything specific to EXCLUDE from this command's scope?
**Answer:** No exclusions mentioned - keep scope as defined.

### Existing Code to Reference

**Similar Features Identified:**
- Feature: Add Command - Path: `/home/md/claude-cli-list/src/main.rs`
  - CLI structure pattern with clap
  - `handle_*` and `execute_*` function separation for testability
  - Database path resolution and directory creation
  - Error handling with exit codes (user errors vs internal errors)
  - Tag parsing from comma-separated input
- Feature: NoteService - Path: `/home/md/claude-cli-list/src/service.rs`
  - `list_notes(ListNotesOptions)` method already implemented
  - `ListNotesOptions` struct with `limit` and `tags` fields
- Feature: Note Model - Path: `/home/md/claude-cli-list/src/models/note.rs`
  - Note structure with `id()`, `content()`, `created_at()`, `updated_at()`, `tags()` accessors

### Follow-up Questions

No follow-up questions needed - user's answers were clear and comprehensive.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A - No visual files found in the planning/visuals directory.

## Requirements Summary

### Functional Requirements
- Implement `cons list` subcommand using clap
- Display up to 10 most recent notes by default (newest first)
- Support `--limit N` flag to override default limit
- Support `--tags tag1,tag2` flag for filtering (AND logic - must have ALL specified tags)
- Show verbose/full note output by default (multi-line format with complete content)
- Display timestamp in absolute format: "YYYY-MM-DD HH:MM"
- Display tags uniformly regardless of source (user vs LLM)
- Show friendly "No notes found" message when results are empty

### Output Format Details
- Verbose output by default (no flags needed)
- Each note should show:
  - Note ID
  - Created timestamp in "YYYY-MM-DD HH:MM" format
  - Full content (not truncated)
  - Tags (displayed uniformly, e.g., `#rust #programming`)
- Notes ordered by creation time, newest first

### Reusability Opportunities
- Reuse existing `NoteService::list_notes()` method - already supports limit and tag filtering
- Reuse `ListNotesOptions` struct for query parameters
- Follow `add` command pattern for CLI structure (`handle_list`, `execute_list`)
- Reuse `parse_tags()` function for `--tags` argument parsing
- Reuse database path resolution from `get_database_path()` and `ensure_database_directory()`

### Scope Boundaries

**In Scope:**
- `cons list` subcommand
- `--limit N` flag for pagination
- `--tags tag1,tag2` flag for filtering
- Verbose note display with full content
- Absolute timestamp formatting
- Empty results messaging

**Out of Scope:**
- Offset/cursor-based pagination
- Interactive selection
- Output to file
- JSON output format
- Compact/single-line output mode
- Relative timestamp formatting ("2 hours ago")
- Distinguishing user vs LLM tags in display

### Technical Considerations
- Integration with existing clap-based CLI in `src/main.rs`
- Use existing `NoteService::list_notes()` - no new service methods needed
- Follow existing error handling pattern with `anyhow` and exit codes
- Tag names need to be resolved from TagId for display (may need to query tags table or extend Note/TagAssignment to include tag name)
- Timestamp formatting will need `time` crate formatting capabilities
