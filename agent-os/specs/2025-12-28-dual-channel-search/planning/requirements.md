# Spec Requirements: Dual-Channel Search

## Initial Description

**Dual-channel search** -- Combine FTS5 results with spreading activation using intersection boost (1.5x multiplier for notes found by both channels); graceful degradation to FTS-only when graph density below threshold (cold-start handling)

Context from the roadmap:
- Item 15 (done): Full-text search with FTS5 implemented
- Item 16 (done): Alias-expanded FTS implemented
- Item 17 (done): Graph schema foundation with edges table
- Item 19 (done): Spreading activation retrieval implemented

## Requirements Discussion

### First Round Questions

**Q1:** I assume the dual-channel search should be a new method (e.g., `dual_search`) that internally calls both `search_notes` and `graph_search`, then merges results. Is that correct, or should this replace the existing `search_notes` method?
**Answer:** New method - Dual-channel search should be a new method (not replacing existing).

**Q2:** For the intersection boost (1.5x multiplier), I'm thinking the formula should be: `final_score = (fts_score + graph_score) * boost` where `boost = 1.5` if found by both channels, else `1.0`. Is that the intended calculation, or should it be a different combination?
**Answer:** User rejected the multiplicative boost approach as it "double-counts". They want an additive RRF-style approach:
```rust
final_score = (fts_score * fts_weight) + (graph_score * graph_weight) + (intersection_bonus if both)

// Concrete example:
fts_weight = 1.0
graph_weight = 1.0
intersection_bonus = 0.5

// Note found by both channels:
final = (0.8 * 1.0) + (0.7 * 1.0) + 0.5 = 2.0

// Note found by FTS only:
final = (0.8 * 1.0) + (0.0 * 1.0) + 0.0 = 0.8
```

**Q3:** For score combination, since both FTS and graph already produce normalized 0.0-1.0 scores, should they be weighted equally, or should one channel have priority?
**Answer:** Equal weights for now, eventually LLM should help tweak.

**Q4:** For graph density threshold (cold-start handling), I'm thinking we should measure the ratio of `edges COUNT / tags COUNT`. If this ratio is below a threshold, fall back to FTS-only. What metric and threshold would you prefer for detecting "sparse graph"?
**Answer:** User prefers "average activation score per query" as the metric (not edges/tags ratio).

**Q5:** For the CLI interface, I assume we should add a flag like `--mode fts|graph|dual` to `cons search`, defaulting to dual-channel when available. Is that correct?
**Answer:** No mode flag, dual-channel is always the default. Don't make users think about it.

**Q6:** Regarding query term handling: should the query be passed identically to both channels, or should there be any preprocessing differences between them?
**Answer:** Keep channels independent:
- FTS needs text expansion ("ML" -> "ML OR machine-learning")
- Graph needs concept IDs (["machine-learning", "neural-networks"] -> tag IDs [42, 73])
- Don't try to make one "preprocessed query" work for both

**Q7:** Should the dual-channel search return additional metadata (e.g., which channel(s) found each result, raw scores from each channel) for debugging/transparency, or just the final merged SearchResult with combined score?
**Answer:** Return all metadata/scores/channels for transparency.

**Q8:** Is there anything that should explicitly be out of scope for this feature?
**Answer:** Not explicitly addressed - assume standard scope boundaries apply.

### Existing Code to Reference

No similar existing features identified for reference. However, the implementation should build directly on:
- `NoteService::search_notes()` in `/home/md/construct-app/src/service.rs` - FTS5 search returning `SearchResult`
- `NoteService::graph_search()` in `/home/md/construct-app/src/service.rs` - Spreading activation returning `SearchResult`
- `spread_activation()` in `/home/md/construct-app/src/spreading_activation.rs` - Core graph traversal algorithm
- `SearchResult` struct in `/home/md/construct-app/src/service.rs` - Existing result type with `note` and `relevance_score`

### Follow-up Questions

**Follow-up 1:** For the "average activation score per query" threshold: After running spreading activation, if the average score across all activated tags is below some threshold, we fall back to FTS-only. What threshold value should trigger degradation? And should we also consider a minimum number of activated tags?
**Answer:** Use both conditions in an OR relationship:
- Average activation score below threshold, OR
- Fewer than minimum activated tags
Either condition triggers FTS-only fallback.

**Follow-up 2:** When degrading to FTS-only, should the system silently return FTS-only results, or include a flag/indicator in the response metadata?
**Answer:** Include an indicator in the response metadata showing "graph channel skipped due to sparse activation" (option b - visible degradation).

**Follow-up 3:** Should the SearchResult struct be extended with new fields, or should we create a new DualSearchResult struct?
**Answer:** Create a new `DualSearchResult` struct (not extend existing `SearchResult`).

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

- **New dual_search method**: Create a new method in `NoteService` that combines FTS5 and spreading activation results
- **Additive scoring formula**:
  ```
  final_score = (fts_score * fts_weight) + (graph_score * graph_weight) + intersection_bonus
  ```
  - `fts_weight = 1.0`
  - `graph_weight = 1.0`
  - `intersection_bonus = 0.5` (only applied when note found by both channels)
- **Independent channel processing**:
  - FTS channel: Use existing alias-expanded FTS (`search_notes`)
  - Graph channel: Use existing spreading activation (`graph_search`)
  - Each channel processes query independently with its own preprocessing
- **Cold-start/sparse graph detection**: Fall back to FTS-only when EITHER:
  - Average activation score across activated tags is below threshold, OR
  - Number of activated tags is below minimum threshold
- **Visible degradation**: When graph channel is skipped, include indicator in response metadata
- **New result struct**: Create `DualSearchResult` with full transparency:
  ```rust
  pub struct DualSearchResult {
      pub note: Note,
      pub final_score: f64,           // Combined score
      pub fts_score: Option<f64>,     // None if not found by FTS
      pub graph_score: Option<f64>,   // None if not found by graph
      pub found_by_both: bool,        // True if intersection bonus applied
  }
  ```
- **CLI default behavior**: `cons search` should use dual-channel by default with no mode flags

### Reusability Opportunities

- Reuse existing `search_notes()` for FTS channel
- Reuse existing `graph_search()` for graph channel
- Reuse `SpreadingActivationConfig` for configurable thresholds
- Consider environment variables for tunable parameters (weights, thresholds) following existing pattern in `SpreadingActivationConfig::from_env()`

### Scope Boundaries

**In Scope:**
- New `dual_search` method in `NoteService`
- New `DualSearchResult` struct with channel metadata
- Additive RRF-style score combination with intersection bonus
- Cold-start detection using average activation score and minimum tag count
- Graceful degradation to FTS-only with metadata indicator
- Update CLI `cons search` to use dual-channel by default

**Out of Scope:**
- Query expansion using broader concepts (roadmap item 21 - separate feature)
- Temporal weighting/recency boost (roadmap item 38)
- LLM-tuned weight optimization (future enhancement)
- Mode flag for CLI (`--mode fts|graph|dual`) - not wanted
- Modifications to existing `SearchResult` struct
- Changes to existing `search_notes` or `graph_search` methods

### Technical Considerations

- Both existing channels already return normalized 0.0-1.0 scores, enabling direct combination
- Threshold values for cold-start detection should be configurable (environment variables)
- The `DualSearchResult` struct should include enough metadata for debugging and future LLM tuning
- Graph channel's `spread_activation` returns `HashMap<TagId, f64>` which can be used to calculate average activation score
- Consider adding a `DualSearchMetadata` or similar for the degradation indicator rather than a simple boolean
