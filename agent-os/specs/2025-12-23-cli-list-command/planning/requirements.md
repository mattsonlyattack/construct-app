# Spec Requirements: CLI list command

## Initial Description

**CLI: list command** - Implement `cons list` showing recent notes with `--tags` filtering and `--limit` pagination

Roadmap item #5, estimated size S (2-3 days). This is the second CLI command being implemented for the cons personal knowledge management tool. The CLI uses clap for argument parsing and calls the existing NoteService layer for business logic.

## Requirements Discussion

### First Round Questions

**Q1:** I assume `cons list` with no flags should show all notes ordered by most recent first (using the existing `ListNotesOptions::default()` which has no limit). Is that correct, or should there be a sensible default limit (e.g., 20 or 50 most recent)?
**Answer:** Most recent last (chronological order - oldest first, newest last)

**Q2:** For the `--tags` flag, I'm thinking the same comma-separated format as `cons add`, like `cons list --tags rust,learning`. The NoteService already implements AND logic (notes must have ALL specified tags). Should we match this behavior, or do you want OR logic (notes with ANY of the tags)?
**Answer:** AND logic (notes must have ALL specified tags)

**Q3:** For the `--limit` flag, I assume it should accept a positive integer like `cons list --limit 10`. Should we validate that the limit is greater than 0 and show an error if invalid?
**Answer:** Yes, validate that limit is greater than 0

**Q4:** For the output format, I'm thinking we should display each note with its ID, content (possibly truncated if long), tags, and timestamp. Something like:
   ```
   [42] Created: 2025-12-23 10:30:15
   Tags: rust, learning
   Content: This is my note content...
   ```
   Or would you prefer a more compact single-line format, or a different structure?
**Answer:** Yes, multi-line format

**Q5:** I assume if no notes match (e.g., `cons list --tags nonexistent-tag`), we should output a friendly message like "No notes found" rather than just showing nothing. Is that correct?
**Answer:** Yes, show "No notes found" message

**Q6:** Should `cons list` support combining both flags, like `cons list --tags rust --limit 5` to show the 5 most recent notes tagged with "rust"? The NoteService already supports this.
**Answer:** Yes, support combining both flags

**Q7:** For notes with very long content, should we truncate the content in the list view (e.g., first 100 characters with "...") or show the full content? Should there be a `--full` flag to show complete content?
**Answer:** Show the full content for now

**Q8:** Is there anything you explicitly want to exclude from this first implementation? For example: no sorting options (always newest first), no date range filtering, no reverse order flag, no JSON output format?
**Answer:** No explicit exclusions mentioned

### Existing Code to Reference

No similar existing features identified for reference.

### Follow-up Questions

No follow-up questions were needed.

## Visual Assets

### Files Provided:

No visual assets provided.

## Requirements Summary

### Functional Requirements

- Implement `cons list` command using clap with subcommand structure (extending existing Commands enum)
- Support optional `--tags` flag with comma-separated values (same format as `cons add`: `--tags rust,learning`)
- Support optional `--limit` flag accepting a positive integer (e.g., `--limit 10`)
- Support combining both flags: `cons list --tags rust --limit 5`
- Order notes chronologically (oldest first, newest last) - Note: This differs from NoteService's default `created_at DESC` ordering, so we may need to reverse results or modify the query
- Display notes in multi-line format showing:
  - Note ID in brackets
  - Created timestamp
  - Tags (comma-separated)
  - Full note content
- Show "No notes found" message when no notes match the filters
- Validate that `--limit` is greater than 0, show error if invalid

### Reusability Opportunities

- Reuse existing `ListNotesOptions` struct from NoteService (already supports `limit` and `tags` fields)
- Reuse existing `NoteService::list_notes()` method
- Follow same CLI structure pattern as `cons add` command (extend Commands enum, add ListCommand variant)
- Reuse tag parsing logic from `cons add` (comma-separated, trim whitespace)
- Reuse database path resolution from `cons add` command
- Reuse error handling patterns (user-friendly messages, exit codes)

### Scope Boundaries

**In Scope:**
- `cons list` command implementation
- `--tags` flag with comma-separated values (AND logic - notes must have ALL tags)
- `--limit` flag with positive integer validation
- Combining both flags
- Multi-line output format showing ID, timestamp, tags, and full content
- Chronological ordering (oldest first, newest last)
- "No notes found" message for empty results
- User-friendly error messages

**Out of Scope:**
- Sorting options (always chronological)
- Date range filtering
- Reverse order flag
- JSON output format
- Content truncation (show full content)
- OR logic for tags (only AND logic)
- Default limit when no flags provided (show all notes)
- Other commands (search, edit)

### Technical Considerations

- Extend existing `Commands` enum in `src/main.rs` with `List(ListCommand)` variant
- Create `ListCommand` struct with optional `--tags` and `--limit` flags using clap derive macros
- Parse tags using same logic as `cons add`: split on comma, trim whitespace
- Call `NoteService::list_notes()` with `ListNotesOptions` struct
- **Important**: NoteService currently orders by `created_at DESC` (newest first), but requirement is chronological (oldest first). Options:
  1. Reverse the Vec<Note> results after fetching
  2. Add ordering parameter to `ListNotesOptions` (future enhancement)
  3. Modify NoteService query to support ASC ordering
- Format timestamps using `time` crate (Note already has `OffsetDateTime` for `created_at`)
- Extract tag names from `TagAssignment` objects for display
- Handle empty results with user-friendly message
- Validate `--limit` > 0 before calling NoteService
- Reuse database path resolution and error handling from `cons add` command

### Dependencies

- Existing: NoteService, Database, Note, ListNotesOptions types from cons crate
- Existing: clap, dirs, anyhow dependencies
- May need: time crate formatting utilities (if not already available) for timestamp display

