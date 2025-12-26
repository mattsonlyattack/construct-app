# Specification: FTS5 Full-Text Search

## Goal
Add full-text search capabilities to the cons CLI using SQLite FTS5, enabling users to find notes by searching across original content, enhanced content, and tag names with a simple `cons search "query"` command.

## User Stories
- As a user, I want to search my notes using natural language words so that I can quickly find relevant thoughts without manually browsing
- As a user, I want search to find notes based on their tags so that AI-categorized notes are discoverable through search

## Specific Requirements

**FTS5 Virtual Table Creation**
- Create `notes_fts` virtual table using FTS5 with porter tokenizer for stemming
- Columns: `note_id` (for joining), `content`, `content_enhanced`, `tags` (space-separated tag names)
- Denormalize tag names into the FTS table as a single space-separated string for searchability
- FTS5 does not support IF NOT EXISTS; handle idempotent creation by checking sqlite_master first

**FTS Index Synchronization**
- Populate FTS table on database open by rebuilding from notes/tags data
- Use triggers on notes table for INSERT/UPDATE/DELETE to keep FTS in sync
- Use triggers on note_tags junction table to update the tags column when tags change
- Rebuild entire FTS entry when any component (content, enhanced content, or tags) changes

**Search Query Processing**
- Implement simple AND logic: split query on whitespace, all terms must match
- Use FTS5 MATCH syntax with proper escaping of user input to prevent injection
- Porter stemmer automatically handles word variations (e.g., "running" matches "run")
- Empty query should return error with helpful message

**BM25 Relevance Ranking**
- Use FTS5's built-in bm25() function for result ordering
- Order results by relevance score descending (most relevant first)
- BM25 considers term frequency and document length automatically

**NoteService Integration**
- Add `search_notes(&self, query: &str, limit: Option<usize>) -> Result<Vec<Note>>` method
- Return full Note objects including tags (same as list_notes) for consistent display
- Reuse existing `get_note()` method to load full Note data after FTS returns matching IDs
- Follow established query patterns from `list_notes()` method

**CLI Command Implementation**
- Add `Search(SearchCommand)` variant to Commands enum in main.rs
- SearchCommand struct: query positional arg, optional --limit flag (default 10)
- Follow `handle_list/execute_list` separation pattern for testability
- Reuse `format_note_content()` and `get_tag_names()` for output formatting

**Result Display Format**
- Match existing `cons list` output format exactly
- Show: ID, Created timestamp, Content (original), Enhanced content with separator, Tags
- Display "No notes found matching query" when no results

**Error Handling**
- Empty query: Return user-friendly error "Search query cannot be empty"
- FTS5 syntax errors: Catch and display clear message without exposing technical details
- FTS index corruption should not prevent note access via `cons list` (fail-safe)

## Visual Design
No visual assets provided.

## Existing Code to Leverage

**`/home/md/construct-app/src/main.rs` - ListCommand and execute_list pattern**
- Copy ListCommand struct pattern for SearchCommand (positional query arg, --limit flag)
- Reuse handle_list/execute_list separation for testability with in-memory databases
- Reuse format_note_content() for displaying search results with original + enhanced content
- Reuse get_tag_names() batch query for resolving tag IDs to display names

**`/home/md/construct-app/src/service.rs` - NoteService list_notes method**
- Follow list_notes query building patterns for search_notes implementation
- Reuse get_note() to load full Note objects after FTS returns matching note IDs
- Follow ListNotesOptions pattern if search needs additional options in future

**`/home/md/construct-app/src/db/schema.rs` - Schema and migrations pattern**
- FTS5 virtual table cannot use IF NOT EXISTS; check sqlite_master before CREATE
- Add FTS rebuild triggers to INITIAL_SCHEMA after virtual table creation
- Follow existing idempotent migration pattern for schema additions

**`/home/md/construct-app/src/db.rs` - Database initialization**
- Extend initialize_schema() to handle FTS5 virtual table creation
- Add FTS index population logic that runs on database open
- FTS rebuild should be fast enough for synchronous execution on open

**`/home/md/construct-app/src/main.rs` - parse_tags function**
- Reference for parsing user input with trimming and empty filtering
- Similar pattern may be useful for query term tokenization

## Out of Scope
- Phrase matching with quotes (e.g., `"exact phrase"`)
- Prefix/wildcard matching (e.g., `word*`)
- Boolean operators (OR, NOT)
- Search result snippet/excerpt highlighting
- Search result pagination beyond --limit
- Semantic/vector search (roadmap item 27)
- Date range filtering within search
- Fuzzy matching beyond Porter stemming
- Search configuration options (case sensitivity, stemmer selection)
- Search history or saved searches
