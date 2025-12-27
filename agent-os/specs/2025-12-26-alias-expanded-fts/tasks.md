# Task Breakdown: Alias-expanded FTS

## Overview
Total Tasks: 12
Size Estimate: S (small, 2-3 days)

## Context

This feature integrates the existing `tag_aliases` table into FTS5 search queries. When a user searches for "ML", the system automatically expands the query to include all related aliases and canonical forms (e.g., "ML OR machine-learning OR machine learning"), enabling synonym-based search without manual query construction.

**Key Implementation Points:**
- Modify `NoteService::search_notes()` to expand terms before FTS5 query execution
- Create `expand_search_term()` method for bi-directional alias lookup
- Apply confidence-based filtering (user aliases always, LLM aliases if >= 0.8)
- Construct FTS5 OR expressions with proper syntax for multi-word phrases

## Task List

### Service Layer

#### Task Group 1: Alias Expansion Logic
**Dependencies:** None

- [x] 1.0 Complete alias expansion implementation
  - [x] 1.1 Write 4-6 focused tests for expand_search_term functionality
    - Test: single term with no aliases returns only original term
    - Test: alias term expands to canonical tag name
    - Test: canonical tag expands to all its aliases
    - Test: user-created aliases always included regardless of confidence
    - Test: LLM aliases with confidence >= 0.8 included
    - Test: LLM aliases with confidence < 0.8 excluded
  - [x] 1.2 Create `expand_search_term(&self, term: &str) -> Result<Vec<String>>` method in NoteService
    - Normalize input term using `TagNormalizer::normalize_tag()`
    - Query `tag_aliases` to check if term is an alias -> get canonical_tag_id
    - Query `tags` table to check if term matches a canonical tag name
    - Query `tag_aliases` to get all aliases for matched canonical tag with confidence filtering
    - Collect unique expansion terms including original term
    - Apply confidence filter: source='user' always, source='llm' only if confidence >= 0.8
  - [x] 1.3 Ensure expansion tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Verify normalization works correctly
    - Verify bi-directional expansion works
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 4-6 tests written in 1.1 pass
- `expand_search_term()` returns original term when no aliases exist
- Bi-directional expansion works (alias->canonical and canonical->aliases)
- Confidence filtering correctly excludes low-confidence LLM aliases
- User aliases are always included regardless of confidence

**Reference Code:**
- `/home/md/construct-app/src/service.rs` - `resolve_alias()` method (lines 683-698) for alias lookup pattern
- `/home/md/construct-app/src/service.rs` - `list_aliases()` method (lines 827-873) for querying with confidence

---

#### Task Group 2: Search Integration
**Dependencies:** Task Group 1

- [x] 2.0 Complete search integration with alias expansion
  - [x] 2.1 Write 4-6 focused tests for search_notes with expansion
    - Test: search for alias term finds notes with canonical tag
    - Test: search for canonical term finds notes with alias tags
    - Test: multi-term search expands each term independently
    - Test: multi-word alias handled as phrase match in FTS5
    - Test: no performance degradation when no aliases exist (query passes through unchanged)
    - Test: BM25 scoring and SearchResult structure preserved after expansion
  - [x] 2.2 Modify `NoteService::search_notes()` to integrate expansion
    - Before FTS5 query construction, call `expand_search_term()` for each whitespace-separated term
    - Build FTS5 query: AND logic between original terms, OR within expansions
    - Handle multi-word aliases with FTS5 phrase syntax (quoted)
    - Single-word expansions unquoted for porter stemming
    - Example: "ML rust" -> `(ML OR machine-learning OR "machine learning") (rust)`
  - [x] 2.3 Update FTS5 query construction logic
    - Current: `"term1" "term2"` (each term quoted)
    - New: `(term1 OR alias1 OR "multi word") (term2 OR alias2)`
    - Use parentheses to group OR expansions
    - Preserve AND logic between original query terms
  - [x] 2.4 Ensure search integration tests pass
    - Run ONLY the 4-6 tests written in 2.1
    - Verify alias expansion works end-to-end
    - Verify BM25 scoring still works correctly
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 4-6 tests written in 2.1 pass
- Search for "ML" finds notes tagged with "machine-learning"
- Search for "machine-learning" finds notes with "ML" alias
- Multi-word aliases correctly use phrase matching
- BM25 relevance scoring preserved
- No regression in search behavior when no aliases exist

**Reference Code:**
- `/home/md/construct-app/src/service.rs` - `search_notes()` method (lines 971-1029) for current FTS5 query construction
- `/home/md/construct-app/src/db/schema.rs` - FTS5 table definition with porter tokenizer (lines 77-85)

---

### Testing

#### Task Group 3: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-2

- [x] 3.0 Review existing tests and fill critical gaps only
  - [x] 3.1 Review tests from Task Groups 1-2
    - Review the 6 tests written in Task 1.1 (expand_search_term)
    - Review the 6 tests written in Task 2.1 (search integration)
    - Total existing tests: 12 tests
  - [x] 3.2 Analyze test coverage gaps for alias-expanded FTS feature
    - Identified critical user workflows that lack test coverage
    - Focus ONLY on gaps related to alias expansion feature
    - Prioritized end-to-end workflows over unit test gaps
  - [x] 3.3 Write up to 6 additional strategic tests if needed
    - Added 4 new tests to fill identified critical gaps:
      - `expand_search_term_case_insensitive_lookup` - Case sensitivity handling in expansion
      - `expand_search_term_with_special_characters_normalized` - Expansion with special characters in alias names
      - `search_alias_in_enhanced_content` - Integration with enhanced content search
      - `expand_search_term_exact_confidence_boundary` - LLM alias at exactly 0.8 threshold
    - Fixed `multi_word_alias_handled_as_phrase_match` test (removed conflicting multi-word alias that normalized to same name as canonical tag)
  - [x] 3.4 Run all feature-specific tests
    - Ran `cargo test expand` - 10 tests pass
    - Ran `cargo test search` - 24 tests pass (includes 5 CLI tests)
    - Total feature-specific tests: 16 (expand_search_term: 10, search integration: 6)
    - All critical workflows verified passing

**Acceptance Criteria:**
- All feature-specific tests pass (16 tests total)
- Critical user workflows for alias expansion are covered
- 4 additional tests added when filling in testing gaps
- Testing focused exclusively on alias-expanded FTS feature requirements

---

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: Alias Expansion Logic** - Core expansion method
   - Foundation: Create the `expand_search_term()` method that will be used by search
   - Must be completed first as Task Group 2 depends on it

2. **Task Group 2: Search Integration** - Modify search_notes to use expansion
   - Depends on Task Group 1 being complete
   - Integrates expansion into the existing search workflow

3. **Task Group 3: Test Review and Gap Analysis** - Verify and fill gaps
   - Depends on Task Groups 1-2 being complete
   - Reviews all feature tests and adds strategic gap-filling tests

---

## Technical Notes

### FTS5 Query Syntax Reference

Current query construction (from `search_notes()`):
```rust
// Current: quoted terms joined with spaces (AND logic)
let fts_query = terms.join(" ");  // e.g., "\"ML\" \"rust\""
```

New query construction with expansion:
```rust
// New: grouped OR expansions with AND between groups
// e.g., "(ML OR machine-learning OR \"machine learning\") (rust)"
```

### Confidence Filtering SQL Pattern

```sql
-- Get aliases for a canonical tag with confidence filtering
SELECT alias FROM tag_aliases
WHERE canonical_tag_id = ?1
  AND (source = 'user' OR (source = 'llm' AND confidence >= 0.8))
```

### Key Files to Modify

1. `/home/md/construct-app/src/service.rs`
   - Add `expand_search_term()` method
   - Modify `search_notes()` to call expansion before FTS5 query

2. `/home/md/construct-app/src/service/tests.rs`
   - Add tests for expansion logic
   - Add tests for search integration

### Out of Scope Reminders

Per the spec, these are explicitly out of scope:
- Broader concept expansion using tag hierarchies (roadmap item 21)
- Related concept expansion through graph relationships
- Changes to FTS index content or triggers
- User-configurable expansion settings or command-line flags
- Displaying which expansions were applied in search output
- Adding new database tables or schema changes
- Caching of expansion lookups
- Performance optimization for large alias sets
