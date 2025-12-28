# Spec Requirements: Spreading Activation Retrieval

## Initial Description

**Spreading activation retrieval** - Implement recursive CTE spreading activation from query tags through edges with decay=0.7, threshold=0.1, max_hops=3; accumulate scores to surface hub notes connecting multiple query concepts; cognitive psychology foundation per KNOWLEDGE.md

Key context about the cons project:
- Local-first personal knowledge management CLI tool
- SQLite database with edges table (source_tag_id, target_tag_id, confidence, hierarchy_type)
- Tag hierarchy population just implemented (item 18)
- This is foundation for dual-channel search (item 20) combining FTS + graph
- KNOWLEDGE.md contains cognitive psychology foundations

## Requirements Discussion

### First Round Questions

**Q1:** I assume spreading activation should be exposed as a new `NoteService` method (e.g., `graph_search`) that returns `Vec<SearchResult>` with normalized scores (0.0-1.0), matching the existing FTS5 `search_notes` interface. This would enable clean integration with the upcoming dual-channel search (item 20). Is that correct, or should it return a different result type?
**Answer:** Yes, return `Vec<SearchResult>` with normalized scores (0.0-1.0), matching FTS5 interface

**Q2:** I'm assuming the algorithm should start from tag IDs extracted from the query string using exact/alias matching, not from a specific note. For example, searching "transformer attention" would seed activation from the "transformer" and "attention" tags. Should we also support starting from a specific note's tags (e.g., "find notes related to note #42")?
**Answer:** Yes, support both query string tag extraction AND starting from a specific note's tags

**Q3:** Per KNOWLEDGE.md, generic (is-a) edges should be traversed bidirectionally for concept expansion, while partitive (part-of) edges should only be traversed when explicitly requested. I'm assuming for MVP we traverse generic edges in both directions but ignore partitive edges entirely. Is that correct, or should partitive edges also factor into spreading activation?
**Answer:** Include BOTH generic and partitive edges, but partitive edges at a lower weight multiplier

**Q4:** The parameters decay=0.7, threshold=0.1, max_hops=3 are specified in KNOWLEDGE.md. I assume these should be hardcoded constants for MVP rather than configurable. Should these be adjustable via CLI flags or NoteService method parameters for experimentation?
**Answer:** Configurable via environment variables (CONS_DECAY, CONS_THRESHOLD, CONS_MAX_HOPS) with defaults decay=0.7, threshold=0.1, max_hops=3

**Q5:** I assume edge confidence should be used as edge weight in the spreading formula: `activation_next = activation_current * edge.confidence * decay_factor`. This means low-confidence LLM-inferred edges (e.g., 0.6) naturally contribute less than user-confirmed edges (1.0). Is that the intended behavior?
**Answer:** Yes, `activation_next = activation_current * edge.confidence * decay`

**Q6:** For scoring notes, I assume we should sum the activation scores of all tags associated with a note, then normalize to 0.0-1.0 range. Notes with multiple activated tags (hub notes) would naturally score higher. Should we also incorporate tag-note confidence (`note_tags.confidence`) as an additional multiplier?
**Answer:** Yes, sum activation scores of tags on a note, AND multiply by note_tags.confidence

**Q7:** I assume we should add a `cons graph-search "query"` CLI command for MVP testing, separate from the existing `cons search` command. The dual-channel merge (item 20) will later combine them. Is that correct, or should we modify the existing search command with a `--graph` flag?
**Answer:** New command `cons graph-search "query"` (separate from existing search)

**Q8:** What should happen when the graph is too sparse for meaningful traversal (cold-start)?
**Answer:** Return empty results, let caller fall back to FTS

### Existing Code to Reference

No similar existing features identified for reference.

### Follow-up Questions

No follow-up questions needed.

## Visual Assets

### Files Provided:

No visual assets provided.

## Requirements Summary

### Functional Requirements

- Implement spreading activation algorithm using SQLite recursive CTE
- Seed activation from query string tags (via exact match and alias resolution)
- Support starting activation from a specific note's tags (find related notes)
- Traverse edges bidirectionally with confidence-weighted activation spreading
- Include both generic and partitive edge types (partitive at lower weight multiplier)
- Accumulate activation scores across multiple paths to surface hub notes
- Score notes by summing activated tag scores, weighted by note_tags.confidence
- Return `Vec<SearchResult>` with normalized relevance scores (0.0-1.0)
- Expose via new `cons graph-search "query"` CLI command
- Handle cold-start gracefully by returning empty results

### Algorithm Specification

**Spreading Activation Formula:**
```
activation_next = activation_current * edge.confidence * decay_factor
```

**Note Scoring Formula:**
```
note_score = SUM(tag_activation * note_tags.confidence) for each tag on note
```

**Default Parameters (configurable via environment variables):**
- `CONS_DECAY` = 0.7 (activation decay per hop)
- `CONS_THRESHOLD` = 0.1 (minimum activation to continue spreading)
- `CONS_MAX_HOPS` = 3 (maximum traversal depth)

**Edge Type Handling:**
- Generic (is-a) edges: Full weight traversal
- Partitive (part-of) edges: Lower weight multiplier (to be determined in spec)

### API Design

**NoteService Methods:**
1. `graph_search(query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>>` - Search by query string
2. `graph_search_from_note(note_id: NoteId, limit: Option<usize>) -> Result<Vec<SearchResult>>` - Find notes related to a specific note

**CLI Command:**
- `cons graph-search "query"` - New command separate from existing `cons search`
- Should display results similar to existing search output format

### Reusability Opportunities

- `SearchResult` type already exists with `note` and `relevance_score` fields
- Score normalization pattern exists in `search_notes` (BM25 normalization)
- Alias resolution via `expand_search_term` for query tag extraction
- `get_or_create_tag` and alias resolution for tag lookup
- Existing edge creation/query patterns in `create_edge` and `create_edges_batch`

### Scope Boundaries

**In Scope:**
- Recursive CTE implementation of spreading activation
- Environment variable configuration for algorithm parameters
- New NoteService methods for graph-based search
- New CLI command `cons graph-search`
- Both query-based and note-based activation seeding
- Score normalization to 0.0-1.0 range
- Unit tests for spreading activation logic

**Out of Scope:**
- Dual-channel search merge with FTS5 (item 20, future work)
- Temporal validity filtering on edges (valid_from/valid_until)
- Recency-weighted activation boost
- Personalized PageRank or other centrality measures
- Graph visualization
- TUI integration

### Technical Considerations

- SQLite recursive CTE for efficient graph traversal
- Environment variables for parameter configuration (std::env)
- Must integrate cleanly with existing NoteService architecture
- Return type must match FTS5 search for future dual-channel integration
- Consider index usage on edges table (idx_edges_source, idx_edges_target already exist)
- Handle bidirectional edge traversal (source->target and target->source)
- Prevent infinite loops in cyclic graphs via hop limit and visited tracking
