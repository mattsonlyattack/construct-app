# Specification: Spreading Activation Retrieval

## Goal

Implement spreading activation retrieval using SQLite recursive CTE to surface semantically related notes through the tag hierarchy graph, enabling cognitive-inspired search that finds hub notes connecting multiple query concepts.

## User Stories

- As a user, I want to search "machine learning" and find notes about related concepts like "neural networks" and "deep learning" even if they don't contain my exact query terms, so that I can discover connections in my knowledge base
- As a user, I want to find notes related to a specific note I'm viewing, so that I can explore the semantic neighborhood of my knowledge

## Specific Requirements

**Spreading Activation Algorithm**
- Implement recursive CTE spreading activation: `activation_next = activation_current * edge.confidence * decay_factor`
- Seed initial activation (1.0) from query tags extracted via exact match and alias resolution using `expand_search_term()`
- Traverse edges bidirectionally: both source_tag_id -> target_tag_id and target_tag_id -> source_tag_id
- Apply activation threshold (default 0.1) to prune low-activation nodes during traversal
- Accumulate activation scores when concept receives activation from multiple paths (SUM aggregation)
- Limit traversal depth with max_hops parameter (default 3)

**Edge Type Handling**
- Generic (is-a) edges: Traverse with full weight (1.0 multiplier)
- Partitive (part-of) edges: Traverse with reduced weight (0.5 multiplier on top of decay)
- Both edge types contribute to spreading activation but partitive edges propagate less signal

**Note Scoring Formula**
- Calculate note score as: `SUM(tag_activation * note_tags.confidence)` for each activated tag on the note
- Normalize final scores to 0.0-1.0 range using the same pattern as FTS5: `1.0 / (1.0 + raw_score.abs())` or min-max normalization if scores don't have natural upper bound
- Return `Vec<SearchResult>` matching existing FTS5 interface for future dual-channel merge

**Environment Variable Configuration**
- `CONS_DECAY` (f64, default 0.7): Activation decay per hop
- `CONS_THRESHOLD` (f64, default 0.1): Minimum activation to continue spreading
- `CONS_MAX_HOPS` (usize, default 3): Maximum traversal depth
- Parse with `std::env::var()` at method call time, not at startup

**Query String Seeding**
- Parse query string into terms using whitespace splitting
- For each term, call `expand_search_term()` to get all related tag names (aliases + canonical)
- Look up TagIds for expanded terms using `resolve_alias()` and direct tag table query
- Assign initial activation 1.0 to each seed tag

**Note-Based Seeding**
- Accept NoteId as starting point for "find related notes" functionality
- Query note_tags to get all tags associated with the seed note
- Use note_tags.confidence as initial activation weight: `seed_activation = note_tags.confidence`
- Exclude the seed note itself from results

**Cold-Start Behavior**
- If no seed tags found (query terms don't match any tags), return empty `Vec<SearchResult>`
- If edges table is empty, activated tags only include seed tags (no spreading)
- Caller (dual-channel search) falls back to FTS when graph search returns empty

**CLI Command: `cons graph-search`**
- New command: `cons graph-search "query"` (separate from existing `cons search`)
- Accept positional query argument and optional `--limit` / `-l` flag (default 10)
- Display results using same format as `cons search` (ID, Created, Content, Tags)
- Output "No notes found via graph search" when results empty

## Existing Code to Leverage

**SearchResult Type and Score Normalization**
- `SearchResult` struct in `/home/md/construct-app/src/service.rs` (lines 9-39) provides the return type with `note: Note` and `relevance_score: f64`
- BM25 normalization pattern: `1.0 / (1.0 + raw_score.abs())` produces scores in 0.0-1.0 range
- Reuse this struct and pattern for graph-based relevance scores

**Alias Resolution and Query Expansion**
- `expand_search_term()` method in NoteService (lines 967-1040) returns `Vec<String>` of expanded tag names including aliases
- `resolve_alias()` method (lines 683-698) converts alias name to canonical TagId
- Use these for extracting seed tags from query string

**Tag Lookup Patterns**
- `get_or_create_tag()` method (lines 386-414) shows tag name to TagId resolution with alias handling
- Direct tag query pattern: `SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE`

**Edges Table Schema and Queries**
- Schema in `/home/md/construct-app/src/db/schema.rs` defines edges table with source_tag_id, target_tag_id, confidence, hierarchy_type
- `create_edge()` method in NoteService (lines 1357-1420) shows edge insertion pattern
- Indexes exist on idx_edges_source and idx_edges_target for efficient traversal

**CLI Command Pattern**
- `SearchCommand` struct and `execute_search()` function in `/home/md/construct-app/src/main.rs` show command structure pattern
- `handle_search()` dispatches to `execute_search()` with database setup
- Same output formatting pattern (timestamp, content, tags) should be reused

## Out of Scope

- Dual-channel search merge with FTS5 (item 20, future spec)
- Temporal validity filtering (valid_from/valid_until on edges) - edges are traversed regardless of validity windows
- Recency-weighted activation boost (notes.created_at does not affect scoring)
- Personalized PageRank or betweenness centrality measures
- Graph visualization or TUI integration
- Precomputed centrality metrics (degree_centrality, pagerank columns)
- Confidence-based edge filtering (all edges traversed, confidence used as weight)
- User-verified edge boost (verified column not used for weighting)
- Background job or async spreading (synchronous execution only)
- Concept schemes or namespace filtering
