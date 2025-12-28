# Task Breakdown: Dual-Channel Search

## Overview
Total Tasks: 18 (across 4 task groups)

This feature combines FTS5 full-text search with spreading activation graph search into a unified `dual_search` method. Results are scored using additive RRF-style combination with an intersection bonus, and the system gracefully degrades to FTS-only when graph activation is sparse (cold-start handling).

## Task List

### Service Layer

#### Task Group 1: Configuration and Data Structures
**Dependencies:** None

- [x] 1.0 Complete configuration and data structures
  - [x] 1.1 Write 3 focused tests for DualSearchConfig and new structs
    - Test DualSearchConfig::from_env() with defaults
    - Test DualSearchConfig::from_env() with custom env vars
    - Test DualSearchResult struct instantiation with all fields
  - [x] 1.2 Create DualSearchConfig struct in `src/service.rs`
    - Fields: `fts_weight: f64`, `graph_weight: f64`, `intersection_bonus: f64`, `min_avg_activation: f64`, `min_activated_tags: usize`
    - Implement `Default` trait with values: fts_weight=1.0, graph_weight=1.0, intersection_bonus=0.5, min_avg_activation=0.1, min_activated_tags=2
    - Implement `from_env()` method following `SpreadingActivationConfig` pattern in `src/spreading_activation.rs`
    - Environment variables: `CONS_FTS_WEIGHT`, `CONS_GRAPH_WEIGHT`, `CONS_INTERSECTION_BONUS`, `CONS_MIN_AVG_ACTIVATION`, `CONS_MIN_ACTIVATED_TAGS`
  - [x] 1.3 Create DualSearchResult struct in `src/service.rs`
    - Fields: `note: Note`, `final_score: f64`, `fts_score: Option<f64>`, `graph_score: Option<f64>`, `found_by_both: bool`
    - Derive `Debug`, `Clone` to match existing `SearchResult` pattern
  - [x] 1.4 Create DualSearchMetadata struct in `src/service.rs`
    - Fields: `graph_skipped: bool`, `skip_reason: Option<String>`, `fts_result_count: usize`, `graph_result_count: usize`
    - Derive `Debug`, `Clone` for consistency
  - [x] 1.5 Export new types from `src/lib.rs`
    - Add `DualSearchResult`, `DualSearchMetadata`, `DualSearchConfig` to pub use statement
  - [x] 1.6 Ensure Task Group 1 tests pass
    - Run ONLY the 3 tests written in 1.1
    - Verify all structs compile and have correct defaults

**Acceptance Criteria:**
- The 3 tests written in 1.1 pass
- All new structs compile with correct field types
- `DualSearchConfig::from_env()` parses environment variables with fallback defaults
- Types are exported from crate root

---

#### Task Group 2: Dual Search Core Logic
**Dependencies:** Task Group 1

- [x] 2.0 Complete dual_search method implementation
  - [x] 2.1 Write 5 focused tests for dual_search method
    - Test dual_search returns FTS-only results when graph has no matching tags (cold-start)
    - Test dual_search returns combined results with correct final_score calculation
    - Test intersection_bonus applied only when note found by both channels
    - Test graceful degradation sets metadata.graph_skipped and skip_reason when activation sparse
    - Test results sorted by final_score descending with limit applied
  - [x] 2.2 Implement dual_search method in NoteService
    - Signature: `pub fn dual_search(&self, query: &str, limit: Option<usize>) -> Result<(Vec<DualSearchResult>, DualSearchMetadata)>`
    - Call existing `search_notes()` for FTS channel (do not modify)
    - Call existing `graph_search()` for graph channel (do not modify)
    - Use HashMap keyed by NoteId to merge results
  - [x] 2.3 Implement cold-start detection logic
    - After calling `graph_search`, calculate average activation score from the raw graph results
    - Check two conditions (OR relationship):
      - Average activation score below `config.min_avg_activation` threshold
      - Fewer than `config.min_activated_tags` activated
    - If either condition true, skip graph channel scoring entirely
    - Note: Need to expose activation data from graph_search - may need helper method
  - [x] 2.4 Implement result merging and scoring
    - For each note, calculate: `final_score = (fts_score * fts_weight) + (graph_score * graph_weight) + intersection_bonus`
    - Apply intersection_bonus only when `found_by_both == true`
    - Set `fts_score = None` for notes found only by graph channel
    - Set `graph_score = None` for notes found only by FTS channel
  - [x] 2.5 Implement result sorting and limiting
    - Sort results by `final_score` descending
    - Apply limit after sorting
    - Populate DualSearchMetadata with counts and skip status
  - [x] 2.6 Ensure Task Group 2 tests pass
    - Run ONLY the 5 tests written in 2.1
    - Verify all scoring and merging logic works correctly

**Acceptance Criteria:**
- The 5 tests written in 2.1 pass
- dual_search correctly calls both channels without modifying them
- Results are merged with correct final_score calculation
- Cold-start detection triggers FTS-only fallback when graph is sparse
- Metadata correctly indicates when graph channel was skipped

---

### CLI Layer

#### Task Group 3: CLI Integration
**Dependencies:** Task Group 2

- [x] 3.0 Complete CLI integration
  - [x] 3.1 Write 3 focused tests for CLI search behavior
    - Test `cons search` command parses correctly (clap)
    - Test execute_search calls dual_search and formats output correctly
    - Test graph skipped notice displayed when metadata.graph_skipped is true
  - [x] 3.2 Modify execute_search in `src/main.rs` to use dual_search
    - Replace `search_notes()` call with `dual_search()` call
    - Extract results and metadata from tuple return type
    - Keep existing limit handling (default: 10)
  - [x] 3.3 Update search result display format
    - Display `final_score` instead of `relevance_score` in output (or keep internal, match existing pattern)
    - Use same note display format as current (ID, Created, Content, Tags)
    - Reuse existing `format_note_content()` for stacked format
  - [x] 3.4 Add graph skip notice to output
    - When `metadata.graph_skipped == true`, print: "Note: Graph search skipped (sparse activation)"
    - Print to stdout after results (user-facing info, not error)
  - [x] 3.5 Ensure Task Group 3 tests pass
    - Run ONLY the 3 tests written in 3.1
    - Verify CLI commands parse and execute correctly

**Acceptance Criteria:**
- The 3 tests written in 3.1 pass
- `cons search <query>` calls dual_search instead of search_notes
- Results display in same format as before
- Graph skip notice appears when degradation occurs
- No new CLI flags introduced (dual-channel is default)

---

### Testing

#### Task Group 4: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-3

- [x] 4.0 Review existing tests and fill critical gaps only
  - [x] 4.1 Review tests from Task Groups 1-3
    - Review the 3 tests written in Task Group 1 (config/structs)
    - Review the 5 tests written in Task Group 2 (dual_search logic)
    - Review the 5 tests written in Task Group 3 (CLI integration)
    - Total existing tests: 13 tests
  - [x] 4.2 Analyze test coverage gaps for dual-channel search feature only
    - Identify critical user workflows that lack test coverage
    - Focus ONLY on gaps related to this spec's feature requirements
    - Do NOT assess entire application test coverage
    - Prioritize end-to-end workflows over unit test gaps
  - [x] 4.3 Write up to 7 additional strategic tests maximum
    - Integration test: dual_search with populated database and realistic ranking verification
    - Edge case: all notes found by both channels (max intersection bonus)
    - Edge case: no results from either channel (empty result handling)
    - Config test: custom weights actually affect final_score calculation
    - Edge case: graph-only results (notes found via spreading but not FTS)
    - Edge case: limit=None returns all results
    - Formula test: intersection_bonus independent of weights
  - [x] 4.4 Run feature-specific tests only
    - Run ONLY tests related to dual-channel search feature (tests from 1.1, 2.1, 3.1, and 4.3)
    - Total: 20 tests (3 + 5 + 5 + 7)
    - Do NOT run the entire application test suite
    - Verify all critical workflows pass

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 11-18 tests total)
- Critical user workflows for dual-channel search are covered
- No more than 7 additional tests added when filling in testing gaps
- Testing focused exclusively on this spec's feature requirements

---

## Execution Order

Recommended implementation sequence:
1. **Task Group 1** - Configuration and Data Structures (service layer foundation)
2. **Task Group 2** - Dual Search Core Logic (main feature implementation)
3. **Task Group 3** - CLI Integration (user-facing integration)
4. **Task Group 4** - Test Review and Gap Analysis (quality assurance)

## Technical Notes

### Key Patterns to Follow

1. **SpreadingActivationConfig pattern** (`/home/md/construct-app/src/spreading_activation.rs` lines 12-76)
   - Use for DualSearchConfig::from_env() implementation
   - Parse env vars with fallback defaults using `unwrap_or()`

2. **SearchResult struct pattern** (`/home/md/construct-app/src/service.rs` lines 9-39)
   - Use for DualSearchResult struct design
   - Match derive macros: `#[derive(Debug, Clone)]`

3. **CLI execute functions** (`/home/md/construct-app/src/main.rs` lines 546-598)
   - Follow `execute_search` pattern for modifications
   - Keep database/service creation in `handle_search`, logic in `execute_search`

### Implementation Considerations

1. **Cold-start detection requires activation data**
   - Current `graph_search` returns `Vec<SearchResult>` with normalized scores
   - To calculate "average activation", may need to either:
     - Add a helper method that returns raw activation data alongside results, OR
     - Infer from result count and score distribution
   - Consider: use result count < min_activated_tags and max_score < threshold as proxy

2. **Existing methods must not be modified**
   - `search_notes()` and `graph_search()` are called as-is
   - All merging/scoring logic lives in `dual_search()`

3. **Score ranges**
   - Both channels already return normalized 0.0-1.0 scores
   - With fts_weight=1.0, graph_weight=1.0, intersection_bonus=0.5, max possible score is 2.5
   - This is acceptable per spec (additive RRF-style)

### Files to Modify

| File | Changes |
|------|---------|
| `/home/md/construct-app/src/service.rs` | Add DualSearchConfig, DualSearchResult, DualSearchMetadata structs; add dual_search method |
| `/home/md/construct-app/src/lib.rs` | Export new types (DualSearchResult, DualSearchMetadata, DualSearchConfig) |
| `/home/md/construct-app/src/main.rs` | Modify execute_search to call dual_search; add graph skip notice |

### Out of Scope Reminders

- Do NOT modify existing `search_notes()` or `graph_search()` methods
- Do NOT add `--mode` flag to CLI
- Do NOT implement query expansion using broader concepts (roadmap item 21)
- Do NOT implement temporal weighting/recency boost (roadmap item 38)
- Do NOT modify existing `SearchResult` struct
- The `graph-search` CLI command remains separate and unchanged
