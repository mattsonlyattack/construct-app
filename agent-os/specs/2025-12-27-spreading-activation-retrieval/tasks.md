# Task Breakdown: Spreading Activation Retrieval

## Overview
Total Tasks: 18

This spec implements graph-based search using spreading activation through the tag hierarchy, enabling cognitive-inspired retrieval that surfaces semantically related notes through edge traversal.

## Task List

### Core Algorithm Layer

#### Task Group 1: Spreading Activation Engine
**Dependencies:** None

- [x] 1.0 Complete spreading activation core implementation
  - [x] 1.1 Write 4-6 focused tests for spreading activation logic
    - Test single seed tag activation spreads through generic edges
    - Test decay factor reduces activation per hop correctly
    - Test threshold pruning stops low-activation paths
    - Test max_hops limit terminates traversal
    - Test activation accumulates from multiple paths (SUM aggregation)
    - Test partitive edges use reduced weight multiplier (0.5)
  - [x] 1.2 Create environment variable configuration module
    - Parse `CONS_DECAY` (f64, default 0.7) at method call time using `std::env::var()`
    - Parse `CONS_THRESHOLD` (f64, default 0.1) at method call time
    - Parse `CONS_MAX_HOPS` (usize, default 3) at method call time
    - Return defaults when env vars not set or invalid
  - [x] 1.3 Implement recursive CTE for spreading activation
    - Seed CTE with initial activation 1.0 for seed tags
    - Traverse edges bidirectionally (source->target and target->source)
    - Apply formula: `activation_next = activation_current * edge.confidence * decay_factor`
    - Apply edge type multiplier: generic=1.0, partitive=0.5
    - Use activation threshold to prune low-activation nodes
    - Limit traversal with max_hops parameter
    - Accumulate scores with SUM when tag receives activation from multiple paths
  - [x] 1.4 Ensure spreading activation tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- Recursive CTE correctly spreads activation through edges
- Decay, threshold, and max_hops parameters work correctly
- Edge types (generic/partitive) apply correct weight multipliers
- Environment variables parsed at runtime with fallback defaults

---

### Service Layer

#### Task Group 2: NoteService Graph Search Methods
**Dependencies:** Task Group 1

- [x] 2.0 Complete NoteService graph search implementation
  - [x] 2.1 Write 4-6 focused tests for graph search service methods
    - Test `graph_search()` returns `Vec<SearchResult>` with normalized scores
    - Test query string parsed into seed tags via `expand_search_term()`
    - Test `graph_search_from_note()` seeds from note's tags with confidence weighting
    - Test cold-start returns empty results (no matching tags)
    - Test note scoring formula: `SUM(tag_activation * note_tags.confidence)`
    - Test seed note excluded from `graph_search_from_note()` results
  - [x] 2.2 Implement seed tag extraction from query string
    - Parse query string into terms using whitespace splitting
    - For each term, call `expand_search_term()` to get all related tag names
    - Look up TagIds for expanded terms using existing tag query patterns
    - Assign initial activation 1.0 to each seed tag
  - [x] 2.3 Implement seed tag extraction from note
    - Accept NoteId as starting point
    - Query note_tags to get all tags associated with the seed note
    - Use `note_tags.confidence` as initial activation weight
    - Store seed note ID for exclusion from results
  - [x] 2.4 Implement `graph_search()` method on NoteService
    - Signature: `graph_search(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>>`
    - Extract seed tags from query using 2.2 logic
    - Execute spreading activation CTE from Task Group 1
    - Score notes using formula: `SUM(tag_activation * note_tags.confidence)`
    - Normalize scores to 0.0-1.0 range using min-max normalization
    - Return results sorted by score descending, limited by `limit` parameter
  - [x] 2.5 Implement `graph_search_from_note()` method on NoteService
    - Signature: `graph_search_from_note(&self, note_id: NoteId, limit: Option<usize>) -> Result<Vec<SearchResult>>`
    - Extract seed tags from note using 2.3 logic
    - Execute spreading activation with confidence-weighted seeds
    - Exclude the seed note from results
    - Return results sorted by score descending
  - [x] 2.6 Ensure graph search service tests pass
    - Run ONLY the 4-6 tests written in 2.1
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- Both `graph_search()` and `graph_search_from_note()` return `Vec<SearchResult>`
- Query string correctly parsed and expanded to seed tags
- Note-based seeding uses tag confidence for initial activation
- Scores normalized to 0.0-1.0 range
- Seed note excluded from related notes results

---

### CLI Layer

#### Task Group 3: Graph Search Command
**Dependencies:** Task Group 2

- [x] 3.0 Complete CLI graph-search command
  - [x] 3.1 Write 2-4 focused tests for graph-search CLI command
    - Test `cons graph-search "query"` returns formatted results
    - Test `--limit` flag restricts result count
    - Test empty results display "No notes found via graph search"
    - Test output format matches existing `cons search` (ID, Created, Content, Tags)
  - [x] 3.2 Add GraphSearchCommand struct to clap Commands enum
    - Add `GraphSearch(GraphSearchCommand)` variant to Commands enum
    - Define `GraphSearchCommand` struct with positional `query` argument
    - Add optional `--limit` / `-l` flag with default 10
    - Follow existing `SearchCommand` pattern from main.rs
  - [x] 3.3 Implement handle_graph_search() and execute_graph_search() functions
    - Follow existing `handle_search()` / `execute_search()` pattern
    - Open database and create NoteService
    - Call `service.graph_search()` with query and limit
    - Handle empty results with "No notes found via graph search" message
  - [x] 3.4 Implement result output formatting
    - Reuse `format_note_content()` and `get_tag_names()` helpers from existing search
    - Display ID, Created timestamp, Content, Tags for each result
    - Match exact output format of `cons search` command
  - [x] 3.5 Ensure graph-search CLI tests pass
    - Run ONLY the 2-4 tests written in 3.1
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- `cons graph-search "query"` command available and functional
- `--limit` / `-l` flag works correctly
- Output format matches existing `cons search` command
- Empty results display appropriate message

---

### Integration Testing

#### Task Group 4: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-3

- [x] 4.0 Review existing tests and fill critical gaps only
  - [x] 4.1 Review tests from Task Groups 1-3
    - Review the 6 tests written by algorithm implementer (Task 1.1)
    - Review the 6 tests written by service implementer (Task 2.1)
    - Review the 4 tests written by CLI implementer (Task 3.1)
    - Total existing tests: 16 tests
  - [x] 4.2 Analyze test coverage gaps for THIS feature only
    - Identify critical integration workflows that lack test coverage
    - Focus ONLY on gaps related to spreading activation retrieval
    - Prioritize end-to-end workflows: query -> seed extraction -> spreading -> scoring -> results
    - Do NOT assess entire application test coverage
  - [x] 4.3 Write up to 6 additional strategic tests maximum
    - Test multi-hop traversal finds distantly related notes (3-hop path verification)
    - Test hub note discovery: note with multiple activated tags scores highest (SUM aggregation)
    - Test environment variable override: custom CONS_DECAY affects results (runtime config)
    - Test alias expansion then spreading activation (integration of two features)
    - Test edge confidence impact: low-confidence edges contribute less activation (0.3 vs 0.9)
    - Test mixed edge types in path: both generic and partitive edges (multiplier composition)
  - [x] 4.4 Run feature-specific tests only
    - Run ONLY tests related to spreading activation retrieval
    - Total: 22 tests (6 algorithm + 6 service + 4 CLI + 6 integration)
    - Do NOT run the entire application test suite
    - Verify critical workflows pass

**Acceptance Criteria:**
- All feature-specific tests pass (22 tests total)
- End-to-end graph search workflow verified
- Hub note discovery confirmed working
- No more than 6 additional tests added when filling in testing gaps

---

## Execution Order

Recommended implementation sequence:

1. **Core Algorithm Layer (Task Group 1)** - Implement spreading activation CTE and env var config
2. **Service Layer (Task Group 2)** - Add NoteService methods for graph search
3. **CLI Layer (Task Group 3)** - Add `cons graph-search` command
4. **Integration Testing (Task Group 4)** - Review and fill test gaps

---

## Technical Notes

### Key Formulas

**Spreading Activation:**
```
activation_next = activation_current * edge.confidence * decay_factor * edge_type_multiplier
```
Where:
- `edge_type_multiplier` = 1.0 for generic, 0.5 for partitive

**Note Scoring:**
```
note_score = SUM(tag_activation * note_tags.confidence) for each activated tag on note
```

**Score Normalization:**
```
normalized_score = raw_score / max_score
```
Min-max normalization is used to ensure 0.0-1.0 range with higher raw scores yielding higher normalized scores.

### Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `CONS_DECAY` | f64 | 0.7 | Activation decay per hop |
| `CONS_THRESHOLD` | f64 | 0.1 | Minimum activation to continue spreading |
| `CONS_MAX_HOPS` | usize | 3 | Maximum traversal depth |

### Existing Code to Leverage

- **SearchResult type**: `/home/md/construct-app/src/service.rs` lines 9-39
- **expand_search_term()**: `/home/md/construct-app/src/service.rs` lines 967-1040
- **Edge schema**: `/home/md/construct-app/src/db/schema.rs` edges table with confidence, hierarchy_type
- **CLI pattern**: `/home/md/construct-app/src/main.rs` SearchCommand struct and execute_search() function

### Out of Scope

- Dual-channel search merge with FTS5 (future spec)
- Temporal validity filtering (valid_from/valid_until on edges)
- Recency-weighted activation boost
- Personalized PageRank or centrality measures
- Graph visualization or TUI integration
