# Task Breakdown: FTS5 Full-Text Search

## Overview
Total Tasks: 4 Task Groups, approximately 18-22 sub-tasks

This feature adds full-text search capabilities to the cons CLI using SQLite FTS5. Users can search across note content, enhanced content, and tag names with a simple `cons search "query"` command.

## Task List

### Database Layer

#### Task Group 1: FTS5 Schema and Synchronization
**Dependencies:** None

- [x] 1.0 Complete FTS5 database layer
  - [x] 1.1 Write 4-6 focused tests for FTS5 functionality
    - Test FTS5 virtual table creation (idempotent via sqlite_master check)
    - Test FTS index population on database open
    - Test trigger-based sync on note INSERT/UPDATE/DELETE
    - Test trigger-based sync when note_tags changes
    - Test BM25 ranking returns results in relevance order
  - [x] 1.2 Create `notes_fts` virtual table in schema
    - Location: `/home/md/construct-app/src/db/schema.rs`
    - FTS5 does NOT support IF NOT EXISTS - must check `sqlite_master` first
    - Columns: `note_id` (unindexed, for joining), `content`, `content_enhanced`, `tags`
    - Use porter tokenizer: `tokenize='porter'`
    - Tags stored as space-separated string (denormalized for searchability)
  - [x] 1.3 Implement FTS index population on database open
    - Location: `/home/md/construct-app/src/db.rs` in `initialize_schema()` or new method
    - Rebuild FTS from notes/tags data: `INSERT INTO notes_fts SELECT ...`
    - Must handle case where FTS table exists but is empty or stale
    - Performance: synchronous execution acceptable for small-to-medium note collections
  - [x] 1.4 Create triggers for FTS synchronization
    - Location: `/home/md/construct-app/src/db/schema.rs`
    - Trigger on notes table: INSERT, UPDATE, DELETE
    - Trigger on note_tags junction table: INSERT, DELETE
    - Rebuild entire FTS entry when any component changes (content, enhanced, or tags)
    - Use pattern: DELETE from notes_fts WHERE note_id = X, then INSERT fresh entry
  - [x] 1.5 Ensure FTS5 database layer tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Verify FTS table creation is idempotent
    - Verify triggers keep FTS in sync

**Acceptance Criteria:**
- [x] The 8 tests written in 1.1 pass
- [x] FTS5 virtual table created with porter tokenizer
- [x] FTS index populated on database open
- [x] Triggers keep FTS synchronized with notes and note_tags tables
- [x] BM25 ranking orders results by relevance

---

### Service Layer

#### Task Group 2: NoteService Search Method
**Dependencies:** Task Group 1

- [x] 2.0 Complete search service layer
  - [x] 2.1 Write 4-6 focused tests for search_notes method
    - Test basic search returns matching notes
    - Test AND logic (all terms must match)
    - Test Porter stemming (e.g., "programming" matches "programs")
    - Test search across content, content_enhanced, and tags
    - Test empty query returns error with helpful message
    - Test limit parameter restricts result count
    - Test returns full Note objects with tags
  - [x] 2.2 Add `search_notes` method to NoteService
    - Location: `/home/md/construct-app/src/service.rs`
    - Signature: `pub fn search_notes(&self, query: &str, limit: Option<usize>) -> Result<Vec<Note>>`
    - Follow `list_notes()` method patterns
    - Return full Note objects (including tags) for consistent display
  - [x] 2.3 Implement query processing with AND logic
    - Split query on whitespace
    - All terms must match (FTS5 implicit AND)
    - Escape user input to prevent FTS5 syntax injection
    - Empty query: return `anyhow::bail!("Search query cannot be empty")`
  - [x] 2.4 Implement BM25 relevance ranking
    - Use FTS5's built-in `bm25()` function
    - ORDER BY relevance score ascending (lower scores more relevant in FTS5)
    - Join FTS results with notes table to get note IDs
    - Reuse `get_note()` to load full Note data after FTS returns matching IDs
  - [x] 2.5 Ensure search service tests pass
    - Run ONLY the 7 tests written in 2.1
    - Verify search returns correct results
    - Verify BM25 ordering works

**Acceptance Criteria:**
- [x] The 7 tests written in 2.1 pass
- [x] `search_notes` method follows existing NoteService patterns
- [x] AND logic correctly filters results
- [x] Porter stemming improves search recall
- [x] BM25 ranking orders by relevance
- [x] Empty query returns user-friendly error

---

### CLI Layer

#### Task Group 3: Search Command Implementation
**Dependencies:** Task Group 2

- [x] 3.0 Complete CLI search command
  - [x] 3.1 Write 3-5 focused tests for search command
    - Test SearchCommand struct parsing with clap (positional query, --limit flag)
    - Test execute_search with in-memory database returns results
    - Test execute_search with empty database shows "No notes found matching query"
    - Test empty query validation at CLI level
  - [x] 3.2 Add SearchCommand to CLI
    - Location: `/home/md/construct-app/src/main.rs`
    - Add `Search(SearchCommand)` variant to `Commands` enum
    - SearchCommand struct: positional `query` arg, optional `--limit` flag (default 10)
    - Follow `ListCommand` pattern exactly
  - [x] 3.3 Implement handle_search and execute_search functions
    - `handle_search`: Opens database, calls `execute_search`
    - `execute_search`: Calls `service.search_notes()`, formats output
    - Follow `handle_list`/`execute_list` separation pattern for testability
  - [x] 3.4 Implement search result display
    - Reuse `format_note_content()` for displaying original + enhanced content
    - Reuse `get_tag_names()` batch query for resolving tag IDs
    - Match existing `cons list` output format exactly:
      - ID, Created timestamp, Content (original), Enhanced content with separator, Tags
    - Display "No notes found matching query" when no results
  - [x] 3.5 Implement error handling for search command
    - Empty query: "Search query cannot be empty" (exit code 1)
    - FTS5 syntax errors: Catch and display clear message without technical details
    - Standard error handling: use `is_user_error()` pattern for exit codes
  - [x] 3.6 Ensure CLI search tests pass
    - Run ONLY the 5 tests written in 3.1
    - Verify command parses correctly
    - Verify output matches expected format

**Acceptance Criteria:**
- [x] The 5 tests written in 3.1 pass
- [x] `cons search "query"` command works
- [x] Output matches `cons list` format exactly
- [x] `--limit` flag controls result count (default 10)
- [x] User-friendly error messages for empty query and syntax errors

---

### Testing

#### Task Group 4: Test Review and Integration Verification
**Dependencies:** Task Groups 1-3

- [x] 4.0 Review tests and verify feature integration
  - [x] 4.1 Review tests from Task Groups 1-3
    - Reviewed 8 tests from Task Group 1 (FTS5 database layer)
    - Reviewed 7 tests from Task Group 2 (search service)
    - Reviewed 5 tests from Task Group 3 (CLI command)
    - Total existing tests: 20 tests
  - [x] 4.2 Analyze test coverage gaps for this feature only
    - Identified critical gap: BM25 ordering verification at service layer (only tested at DB layer)
    - Identified critical gap: Fail-safe behavior (FTS corruption shouldn't block list_notes)
    - All other workflows well-covered by existing tests
  - [x] 4.3 Write up to 5 additional strategic tests if needed
    - Added 2 strategic tests (well under 5-test limit):
      1. `search_notes_orders_results_by_bm25_relevance` - End-to-end BM25 ranking verification
      2. `list_notes_works_independently_of_fts_functionality` - Fail-safe requirement verification
    - Location: `/home/md/construct-app/src/service/tests.rs`
  - [x] 4.4 Run feature-specific tests only
    - Ran FTS5-related tests: `cargo test fts` - 9 tests passing
    - Ran search-related tests: `cargo test search` - 13 tests passing
    - Total FTS5 feature tests: 22 tests (within 16-22 range)
    - All critical workflows verified and passing

**Acceptance Criteria:**
- [x] All feature-specific tests pass (22 tests total - within 16-22 range)
- [x] Search finds notes by content, enhanced content, and tag names (verified in tests)
- [x] BM25 relevance ranking orders results appropriately (verified at DB and service layers)
- [x] Fail-safe: FTS issues don't block note access via `cons list` (explicitly tested)
- [x] Only 2 additional tests added (well under 5-test limit)

---

## Execution Order

Recommended implementation sequence:

1. **Database Layer (Task Group 1)** - FTS5 schema and synchronization must exist before search can work
2. **Service Layer (Task Group 2)** - `search_notes` method depends on FTS5 table existing
3. **CLI Layer (Task Group 3)** - CLI command depends on service method
4. **Test Review (Task Group 4)** - Final verification after all components are implemented

---

## Technical Notes

### FTS5 Idempotent Creation Pattern

FTS5 virtual tables do NOT support `IF NOT EXISTS`. Use this pattern:

```sql
-- Check if FTS table exists before creating
SELECT name FROM sqlite_master WHERE type='table' AND name='notes_fts';
-- If not found, create:
CREATE VIRTUAL TABLE notes_fts USING fts5(
    note_id UNINDEXED,
    content,
    content_enhanced,
    tags,
    tokenize='porter'
);
```

### FTS5 Query Syntax

```sql
-- Basic MATCH query with BM25 ranking
SELECT note_id, bm25(notes_fts) as score
FROM notes_fts
WHERE notes_fts MATCH ?
ORDER BY score;
```

### Query Escaping

User input must be escaped to prevent FTS5 syntax injection. Terms should be double-quoted:

```rust
// Split query, escape each term, join with spaces
let terms: Vec<String> = query.split_whitespace()
    .map(|term| format!("\"{}\"", term.replace("\"", "\"\"")))
    .collect();
let fts_query = terms.join(" ");
```

### Existing Code References

- **Schema patterns**: `/home/md/construct-app/src/db/schema.rs`
- **Service patterns**: `/home/md/construct-app/src/service.rs` (see `list_notes()`)
- **CLI patterns**: `/home/md/construct-app/src/main.rs` (see `ListCommand`, `execute_list()`)
- **Output formatting**: `/home/md/construct-app/src/main.rs` (see `format_note_content()`, `get_tag_names()`)

---

## Out of Scope (Per Spec)

These features are explicitly out of scope and should NOT be implemented:

- Phrase matching with quotes (`"exact phrase"`)
- Prefix/wildcard matching (`word*`)
- Boolean operators (OR, NOT)
- Search result snippet/excerpt highlighting
- Search result pagination beyond --limit
- Semantic/vector search
- Date range filtering within search
- Fuzzy matching beyond Porter stemming
- Search configuration options
- Search history or saved searches
