# Specification: Dual-Channel Search

## Goal

Combine FTS5 full-text search with spreading activation graph search into a unified `dual_search` method that scores results using additive RRF-style combination with an intersection bonus, and gracefully degrades to FTS-only when graph activation is sparse.

## User Stories

- As a user, I want to search my notes with a single command that leverages both text matching and semantic relationships so that I find relevant notes even when exact keywords differ
- As a user, I want notes found by both text and graph search to rank higher so that the most contextually relevant notes surface first

## Specific Requirements

**New DualSearchResult struct**
- Create `DualSearchResult` in `src/service.rs` with fields: `note: Note`, `final_score: f64`, `fts_score: Option<f64>`, `graph_score: Option<f64>`, `found_by_both: bool`
- All score fields use normalized 0.0-1.0 range consistent with existing `SearchResult`
- Export `DualSearchResult` from `src/lib.rs` alongside existing `SearchResult`
- Derive `Debug` and `Clone` traits matching existing `SearchResult` pattern

**New DualSearchMetadata struct**
- Create struct to capture search metadata: `graph_skipped: bool`, `skip_reason: Option<String>`, `fts_result_count: usize`, `graph_result_count: usize`
- Include in dual_search return type as `(Vec<DualSearchResult>, DualSearchMetadata)`
- When graph channel is skipped, populate `skip_reason` with message like "sparse graph activation"

**New DualSearchConfig struct**
- Create configuration struct with fields: `fts_weight: f64`, `graph_weight: f64`, `intersection_bonus: f64`, `min_avg_activation: f64`, `min_activated_tags: usize`
- Implement `from_env()` following `SpreadingActivationConfig` pattern in `src/spreading_activation.rs`
- Default values: `fts_weight=1.0`, `graph_weight=1.0`, `intersection_bonus=0.5`, `min_avg_activation=0.1`, `min_activated_tags=2`
- Environment variables: `CONS_FTS_WEIGHT`, `CONS_GRAPH_WEIGHT`, `CONS_INTERSECTION_BONUS`, `CONS_MIN_AVG_ACTIVATION`, `CONS_MIN_ACTIVATED_TAGS`

**New dual_search method in NoteService**
- Add `pub fn dual_search(&self, query: &str, limit: Option<usize>) -> Result<(Vec<DualSearchResult>, DualSearchMetadata)>`
- Internally call existing `search_notes()` and `graph_search()` independently (do not modify those methods)
- Merge results using note ID as key; calculate `final_score = (fts_score * fts_weight) + (graph_score * graph_weight) + intersection_bonus` where intersection_bonus only applies when both channels found the note
- Sort by `final_score` descending, apply limit, return results with metadata

**Cold-start detection and graceful degradation**
- After calling `graph_search`, inspect the spreading activation results to calculate average activation score
- Check two conditions: average activation below `min_avg_activation` threshold OR fewer than `min_activated_tags` activated
- If either condition is true, skip graph channel scoring entirely and return FTS-only results
- Set `graph_skipped=true` and populate `skip_reason` in metadata when degradation occurs

**Independent channel processing**
- FTS channel uses existing alias-expanded `search_notes()` with its own query preprocessing
- Graph channel uses existing `graph_search()` with its own tag ID resolution
- Do not create shared query preprocessing; each channel handles its input independently

**CLI integration**
- Modify existing `cons search` command to call `dual_search` instead of `search_notes`
- Display `final_score` instead of `relevance_score` in output
- When graph was skipped, print notice: "Note: Graph search skipped (sparse activation)"
- No new CLI flags needed; dual-channel is the default behavior

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**`SearchResult` struct in `/home/md/construct-app/src/service.rs`**
- Reference struct pattern with `note: Note` and `relevance_score: f64`
- Use same derive macros (`Debug`, `Clone`) for consistency
- `DualSearchResult` mirrors this pattern with additional fields

**`NoteService::search_notes()` in `/home/md/construct-app/src/service.rs`**
- Existing FTS5 search with alias expansion returning `Vec<SearchResult>`
- Call directly from `dual_search`; do not modify
- Results already have normalized 0.0-1.0 scores via BM25 transformation

**`NoteService::graph_search()` in `/home/md/construct-app/src/service.rs`**
- Existing spreading activation search returning `Vec<SearchResult>`
- Call directly from `dual_search`; do not modify
- Results already have normalized 0.0-1.0 scores via min-max normalization

**`SpreadingActivationConfig` in `/home/md/construct-app/src/spreading_activation.rs`**
- Reference pattern for environment-based configuration with `from_env()` method
- Use same approach for `DualSearchConfig` with fallback defaults
- Follow naming convention: `CONS_` prefix for environment variables

**CLI command structure in `/home/md/construct-app/src/main.rs`**
- Existing `handle_search()` and `execute_search()` functions around line 530-600
- Modify to call `dual_search` instead of `search_notes`
- Follow existing output formatting pattern for search results

## Out of Scope

- Query expansion using broader concepts (roadmap item 21 - separate feature)
- Temporal weighting or recency boost (roadmap item 38)
- LLM-tuned weight optimization (future enhancement)
- Mode flag for CLI (`--mode fts|graph|dual`) - explicitly rejected
- Modifications to existing `SearchResult` struct
- Changes to existing `search_notes` or `graph_search` method implementations
- Replacing existing search commands; `graph-search` CLI command remains separate
- Asynchronous execution of FTS and graph channels
- Caching of intermediate search results
- Score normalization changes for existing methods
