# Task Breakdown: Query Expansion

## Overview
Total Tasks: 14

This feature extends the existing search system to include broader concepts from the tag hierarchy, improving recall while maintaining precision through aggressive noise control.

## Task List

### Configuration Layer

#### Task Group 1: Query Expansion Configuration
**Dependencies:** None

- [x] 1.0 Complete QueryExpansionConfig struct
  - [x] 1.1 Write 3 focused tests for configuration functionality
    - Test default values (depth=1, max_terms=10, broader_min_confidence=0.7)
    - Test environment variable parsing for all three config fields
    - Test fallback to defaults when env vars are invalid
  - [x] 1.2 Create `QueryExpansionConfig` struct in `/home/md/construct-app/src/service.rs`
    - Fields: `expansion_depth: usize`, `max_expansion_terms: usize`, `broader_min_confidence: f64`
    - Follow pattern from existing `DualSearchConfig` and `SpreadingActivationConfig`
  - [x] 1.3 Implement `Default` trait for `QueryExpansionConfig`
    - `expansion_depth`: 1
    - `max_expansion_terms`: 10
    - `broader_min_confidence`: 0.7
  - [x] 1.4 Implement `from_env()` method for `QueryExpansionConfig`
    - Parse `CONS_EXPANSION_DEPTH` (usize, default 1)
    - Parse `CONS_MAX_EXPANSION_TERMS` (usize, default 10)
    - Parse `CONS_BROADER_MIN_CONFIDENCE` (f64, default 0.7)
    - Use pattern: `std::env::var("...").ok().and_then(|s| s.parse().ok()).unwrap_or(default)`
  - [x] 1.5 Ensure configuration tests pass
    - Run ONLY the 3 tests written in 1.1
    - Verify defaults and env parsing work correctly

**Acceptance Criteria:**
- [x] The 3 tests written in 1.1 pass
- [x] Config struct follows existing patterns (`DualSearchConfig`, `SpreadingActivationConfig`)
- [x] All three environment variables are parsed with proper fallbacks

### Database Query Layer

#### Task Group 2: Broader Concept Retrieval
**Dependencies:** Task Group 1 (completed)

- [x] 2.0 Complete broader concept database query
  - [x] 2.1 Write 4 focused tests for broader concept retrieval
    - Test finding immediate parent via generic edge (source_tag_id -> target_tag_id)
    - Test filtering by confidence threshold (>= 0.7)
    - Test ignoring partitive edges (only traverse generic)
    - Test empty result when no broader concepts exist
  - [x] 2.2 Create `get_broader_concepts()` method in `NoteService`
    - Query pattern: `SELECT target_tag_id FROM edges WHERE source_tag_id = ? AND hierarchy_type = 'generic' AND confidence >= ?`
    - Return `Vec<TagId>` of broader concept tag IDs
    - Accept `tag_id: TagId` and `min_confidence: f64` parameters
  - [x] 2.3 Create `get_broader_concept_names()` helper method
    - Join with tags table to return tag names alongside IDs
    - Return `Vec<(TagId, String)>` for use in FTS query building
  - [x] 2.4 Ensure broader concept retrieval tests pass
    - Run ONLY the 4 tests written in 2.1
    - Verify edge direction and filtering work correctly

**Acceptance Criteria:**
- [x] The 5 tests (4 for get_broader_concepts + 1 for get_broader_concept_names) pass
- [x] Only generic edges are traversed (not partitive)
- [x] Edge direction is correct: source_tag_id (narrower) -> target_tag_id (broader)
- [x] Confidence filtering works (>= threshold)

### Query Expansion Logic Layer

#### Task Group 3: Term Expansion with Broader Concepts
**Dependencies:** Task Groups 1, 2

- [x] 3.0 Complete expanded term collection logic
  - [x] 3.1 Write 5 focused tests for term expansion
    - Test alias expansion still works (existing behavior)
    - Test broader concept expansion for single-term query
    - Test broader concept expansion for two-term query
    - Test NO broader expansion for three-term query (conditional)
    - Test term limit enforcement (max 10, prefer aliases)
  - [x] 3.2 Create `expand_search_term_with_broader()` method in `NoteService`
    - Call existing `expand_search_term()` for alias expansion first
    - Look up TagId for the normalized term
    - If TagId exists, call `get_broader_concepts()` with configured confidence threshold
    - Get broader concept tag names to add to expansions
    - Apply term limit: max `max_expansion_terms` total, prefer aliases over broader concepts
    - Return `Vec<String>` of all expanded terms
  - [x] 3.3 Create `should_expand_broader()` helper function
    - Accept query string, return bool
    - Split query by whitespace, check term count < 3
    - Return false if query has 3+ terms
  - [x] 3.4 Ensure term expansion tests pass
    - Run ONLY the 5 tests written in 3.1
    - Verify conditional expansion and term limits work

**Acceptance Criteria:**
- [x] The 5 tests written in 3.1 pass
- [x] Alias expansion always applied (existing behavior preserved)
- [x] Broader concept expansion only for queries with < 3 terms
- [x] Term limit of 10 enforced with alias preference

### FTS Query Construction Layer

#### Task Group 4: Enhanced FTS Query Building
**Dependencies:** Task Group 3 (completed)

- [x] 4.0 Complete FTS query construction with broader concepts
  - [x] 4.1 Write 3 focused tests for FTS query building
    - Test single term with alias + broader expands to correct OR expression
    - Test multi-term query maintains AND between terms, OR within expansions
    - Test proper quoting and escaping in generated FTS query
  - [x] 4.2 Modify `build_expanded_fts_term()` to use `expand_search_term_with_broader()`
    - Replace call to `expand_search_term()` with new method
    - Pass `QueryExpansionConfig` for threshold and limit settings
    - Maintain existing FTS5 OR expression formatting
  - [x] 4.3 Update `build_expanded_fts_term()` signature if needed
    - May need to accept config or use `from_env()` internally
    - Ensure backward compatibility with existing callers
  - [x] 4.4 Ensure FTS query construction tests pass
    - Run ONLY the 3 tests written in 4.1
    - Verify FTS syntax is correct with broader concepts included

**Acceptance Criteria:**
- [x] The 3 tests written in 4.1 pass
- [x] FTS query includes broader concepts in OR expression
- [x] AND logic between original query terms preserved
- [x] Example: "rust" -> `("rust" OR "rustlang" OR "programming")`

### Integration Layer

#### Task Group 5: Search Method Integration
**Dependencies:** Task Group 4 (completed)

- [x] 5.0 Complete search method integration
  - [x] 5.1 Write 4 focused tests for integrated search
    - Test `search_notes()` returns notes tagged with broader concept
    - Test `dual_search()` applies expansion correctly to FTS channel
    - Test `graph_search()` does NOT apply broader expansion (spreading activation handles it)
    - Test end-to-end: note tagged "rust", search "transformer", find via hierarchy
  - [x] 5.2 Update `search_notes()` to use broader concept expansion
    - Load `QueryExpansionConfig::from_env()` at method entry
    - Check `should_expand_broader()` for the query
    - Use `expand_search_term_with_broader()` when building FTS terms
    - Existing FTS query execution remains unchanged
  - [x] 5.3 Verify `dual_search()` inherits expansion (calls `search_notes()`)
    - `dual_search()` calls `search_notes()` internally for FTS channel
    - No changes needed to `dual_search()` itself
  - [x] 5.4 Confirm `graph_search()` remains unchanged
    - Spreading activation in `graph_search()` already traverses hierarchy
    - Do NOT add broader expansion to avoid double-expansion
  - [x] 5.5 Ensure integration tests pass
    - Run ONLY the 4 tests written in 5.1
    - Verify search results include broader concept matches

**Acceptance Criteria:**
- [x] The 4 tests written in 5.1 pass
- [x] `search_notes()` finds notes via broader concepts
- [x] `dual_search()` benefits from expansion via FTS channel
- [x] `graph_search()` unchanged (spreading activation already handles hierarchy)

### Testing

#### Task Group 6: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-5

- [x] 6.0 Review existing tests and fill critical gaps only
  - [x] 6.1 Review tests from Task Groups 1-5
    - Review the 3 tests from config (Task 1.1)
    - Review the 5 tests from database query (Task 2.1) - includes get_broader_concept_names
    - Review the 10 tests from term expansion (Task 3.1) - includes 5 should_expand_broader tests
    - Review the 3 tests from FTS construction (Task 4.1)
    - Review the 4 tests from integration (Task 5.1)
    - Total existing tests: 25 tests
  - [x] 6.2 Analyze test coverage gaps for this feature only
    - Check for missing edge cases in conditional expansion logic - COVERED
    - Verify confidence threshold boundary cases are covered - GAP: exactly 0.7 threshold
    - Ensure term limit edge cases are tested (exactly 10 terms, 11 terms) - GAP: exact boundaries
  - [x] 6.3 Write up to 5 additional strategic tests if needed
    - Test 1: get_broader_concepts_exact_confidence_threshold_included (0.7 exactly)
    - Test 2: expand_search_term_with_broader_exactly_ten_terms_no_truncation
    - Test 3: expand_search_term_with_broader_eleven_terms_truncates_broader_first
    - Test 4: expand_search_term_with_broader_multiple_broader_concepts_all_included
    - Test 5: expand_search_term_with_broader_no_broader_but_expansion_enabled
  - [x] 6.4 Run all query expansion feature tests
    - Run tests from groups 1-5 plus 5 new tests from 6.3
    - Total tests: 30 tests
    - All tests pass successfully

**Acceptance Criteria:**
- [x] All feature-specific tests pass (30 tests total)
- [x] Critical user workflows for query expansion are covered
- [x] Exactly 5 additional tests added when filling gaps
- [x] Testing focused exclusively on query expansion feature

## Execution Order

Recommended implementation sequence:

1. **Configuration Layer (Task Group 1)** - Foundation for all other components
2. **Database Query Layer (Task Group 2)** - Core database access for broader concepts
3. **Query Expansion Logic Layer (Task Group 3)** - Business logic combining aliases and broader concepts
4. **FTS Query Construction Layer (Task Group 4)** - Modify FTS query building
5. **Integration Layer (Task Group 5)** - Wire expansion into search methods
6. **Testing (Task Group 6)** - Review and fill gaps

## Files to Modify

| File | Changes |
|------|---------|
| `/home/md/construct-app/src/service.rs` | Add `QueryExpansionConfig`, `get_broader_concepts()`, `get_broader_concept_names()`, `expand_search_term_with_broader()`, `should_expand_broader()`, modify `build_expanded_fts_term()`, update `search_notes()` |
| `/home/md/construct-app/src/service/tests.rs` | Add tests for all new functionality |

## Implementation Notes

### Existing Code to Leverage
- `expand_search_term()` - Reuse for alias expansion (composition, not duplication)
- `build_expanded_fts_term()` - Extend to include broader concepts
- `DualSearchConfig::from_env()` - Pattern for environment variable parsing
- `SpreadingActivationConfig::from_env()` - Same pattern reference

### Key Constraints
- Edge direction: `source_tag_id` (narrower) -> `target_tag_id` (broader)
- Only traverse `generic` hierarchy type (not `partitive`)
- Alias confidence threshold: 0.8 (existing behavior)
- Broader concept confidence threshold: 0.7 (configurable)
- Broader expansion only for queries < 3 terms
- Max 10 expanded terms per original term (configurable)
- Prefer aliases over broader concepts when enforcing limit

### Out of Scope
- Related concept expansion (relatedTo edges)
- CLI flags for controlling expansion
- Partitive edge traversal
- Multi-level broader traversal (depth > 1)
- Changes to `graph_search()` method
- Narrower concept expansion
- Weighted/boosted broader concepts
- Caching of expansion results
