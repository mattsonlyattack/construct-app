# Spec Requirements: FTS5 Full-Text Search

## Initial Description

**Roadmap Item 15**: Full-text search with FTS5 -- Implement SQLite FTS5 virtual table for content search, with `cons search "query"` command

This is for a Rust CLI tool called "cons" - a personal knowledge management system. The feature adds full-text search capabilities using SQLite's FTS5 extension.

## Requirements Discussion

### First Round Questions

**Q1:** I assume the search should query only the original `content` field (not the `content_enhanced` field). Is that correct, or should search also include the enhanced content?
**Answer:** Search BOTH original AND enhanced content - this is critical to the AI-first system.

**Q2:** I'm assuming the search command should default to matching all words (AND logic), similar to how `list --tags` works. Should we also support phrase matching or prefix matching? Or keep MVP simple with just the AND-word default?
**Answer:** Keep it simple for MVP (AND logic only).

**Q3:** For result ordering, I assume we should use FTS5's built-in relevance ranking (BM25). Is that correct, or would you prefer chronological order?
**Answer:** Use built-in BM25 relevance ranking.

**Q4:** I assume search should NOT include tag names in the search - just note content. Is that correct?
**Answer:** YES - search should include tags. (Search should find notes by matching tag names as well as content.)

**Q5:** For enhanced notes, should search results display the enhanced content when available or always show original content?
**Answer:** Show BOTH original and enhanced content in results.

**Q6:** I'm planning to use a content FTS5 table (FTS5 stores its own copy of text) rather than an external content table. Does that sound right?
**Answer:** Yes, content table (stores own copy) is fine.

**Q7:** For tokenization, I assume default FTS5 Unicode tokenizer is fine. We don't need Porter stemmer for MVP. Is that correct?
**Answer:** Use Porter stemmer - stemming IS valuable.

**Q8:** Should search results show a snippet/excerpt highlighting the matching text, or just display the full note content like `cons list` does?
**Answer:** Full content display (not snippets).

### Existing Code to Reference

**Similar Features Identified:**
- `list_notes()` in NoteService (`/home/md/construct-app/src/service.rs`) - Query building and result formatting patterns
- `ListCommand` in main.rs (`/home/md/construct-app/src/main.rs`) - CLI argument parsing with clap
- `execute_list()` pattern - Separation of CLI handling from business logic (testability)
- `format_note_content()` - Stacked display format showing original + enhanced content with separator

### Follow-up Questions

No follow-up questions needed. User's answers were comprehensive and the AI-first philosophy clarification ("The AI-first, structure-last system is supposed to be doing all the work and should only surface simple mechanics/behavior") provides clear design guidance.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

**Search Command:**
- New CLI command: `cons search "query"`
- Searches across: original content, enhanced content, AND tag names
- Query syntax: Simple AND logic (all words must match)
- Results ordered by: FTS5 BM25 relevance ranking (most relevant first)

**FTS5 Virtual Table:**
- Content table type (FTS5 stores its own copy of searchable text)
- Porter stemmer tokenizer enabled (matches "running" when searching "run")
- Indexes: original content, enhanced content, tag names (concatenated or joined)

**Result Display:**
- Full note content display (same format as `cons list`)
- Show BOTH original and enhanced content using stacked format with separator
- Include note metadata: ID, created timestamp, tags

**Schema Changes:**
- New FTS5 virtual table for full-text search indexing
- Must stay synchronized with notes table (insert/update/delete triggers or manual sync)

### Reusability Opportunities

- Follow `ListCommand` pattern for `SearchCommand` struct definition
- Reuse `execute_list()` separation pattern: `handle_search()` calls `execute_search()`
- Reuse `format_note_content()` for displaying search results
- Reuse `get_tag_names()` for resolving tag IDs to display names
- Add `search_notes()` method to NoteService following `list_notes()` patterns

### Scope Boundaries

**In Scope:**
- FTS5 virtual table creation in schema
- `cons search "query"` CLI command
- Search across content, content_enhanced, and tags
- BM25 relevance-ranked results
- Porter stemmer for improved recall
- Full note display in results (matching `cons list` format)
- Keeping FTS5 index synchronized with notes table

**Out of Scope:**
- Phrase matching (`"exact phrase"`)
- Prefix matching (`word*`)
- Boolean operators (OR, NOT)
- Snippet/excerpt highlighting
- Search result pagination (can use --limit like list command)
- Semantic/vector search (roadmap item 27)
- Search within date ranges
- Fuzzy matching beyond stemming

### Technical Considerations

**Database Layer:**
- FTS5 virtual table with porter tokenizer: `CREATE VIRTUAL TABLE notes_fts USING fts5(content, content_enhanced, tags, tokenize='porter')`
- Idempotent schema pattern (IF NOT EXISTS not supported for virtual tables - need alternative approach)
- Synchronization strategy: triggers on notes/note_tags tables OR rebuild on search
- Consider: FTS5 content tables can't use external content with multiple source tables - may need to denormalize tags into searchable text

**Tag Inclusion Challenge:**
- Tags are in separate table with junction (note_tags)
- Options: (a) denormalize tag names into FTS table, (b) use contentless FTS + JOIN, (c) rebuild FTS index periodically
- Recommendation: Denormalize - concatenate tag names as space-separated string in FTS table

**NoteService Integration:**
- New method: `search_notes(&self, query: &str, limit: Option<usize>) -> Result<Vec<Note>>`
- Returns full Note objects (same as list_notes) for consistent display
- Query construction: Use FTS5 MATCH syntax with proper escaping

**CLI Integration:**
- New subcommand in Commands enum: `Search(SearchCommand)`
- SearchCommand struct with query positional arg and optional --limit flag
- Handler follows execute_list pattern for testability

**Error Handling:**
- Empty query: Return helpful error message
- No results: Display "No notes found matching query"
- FTS5 syntax errors: Catch and display user-friendly message

**Fail-Safe Design:**
- Search should never corrupt data
- Invalid queries fail gracefully with clear messages
- FTS index corruption should not prevent note access via `cons list`

### Design Philosophy Alignment

Per user guidance: "The AI-first, structure-last system is supposed to be doing all the work and should only surface simple mechanics/behavior"

This means:
- Simple CLI interface: just `cons search "words"`
- No complex query syntax to learn
- AI-enhanced content automatically improves search recall
- Tags (AI-generated) automatically included in search
- User doesn't need to know about FTS5, stemming, or relevance ranking - it just works
