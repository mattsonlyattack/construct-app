# Specification: Query Expansion

## Goal

Expand search queries by including broader concepts from the tag hierarchy to improve recall, while maintaining aggressive noise control to prevent over-expansion and irrelevant results.

## User Stories

- As a user, I want my short search queries to automatically include parent concepts so that I find notes tagged with broader terms without manually searching for them
- As a user, I want my searches to remain precise even with expansion, so that I am not overwhelmed by loosely related results

## Specific Requirements

**Alias expansion (always applied)**
- Continue using existing `expand_search_term()` method for all queries
- Alias expansion runs unconditionally regardless of query length
- User-created aliases always included; LLM aliases require confidence >= 0.8

**Broader concept expansion (conditional)**
- Only apply to queries with fewer than 3 terms after whitespace splitting
- Traverse only `generic` hierarchy type edges (is-a relationships)
- Do not traverse `partitive` edges (part-of relationships)
- Edge direction: from `source_tag_id` (narrower) to `target_tag_id` (broader)

**Expansion depth control**
- Default depth: 1 level (immediate parent concepts only)
- Configurable via `CONS_EXPANSION_DEPTH` environment variable
- Do not implement multi-level traversal beyond configured depth

**Confidence filtering for broader concepts**
- Only include broader concepts from edges with `confidence >= 0.7`
- This is separate from alias confidence filtering (0.8 threshold)
- Configurable via `CONS_BROADER_MIN_CONFIDENCE` environment variable (default 0.7)

**Term limit enforcement**
- Maximum 10 expanded terms per original query term
- Configurable via `CONS_MAX_EXPANSION_TERMS` environment variable (default 10)
- When limit exceeded, prefer aliases over broader concepts

**FTS query construction**
- Treat broader concepts as equal terms in OR expression
- Maintain AND logic between original query terms
- Example: query "rust" with alias "rustlang" and broader "programming" becomes `("rust" OR "rustlang" OR "programming")`

**Integration points**
- Apply expansion in `search_notes()` method (FTS channel)
- Apply expansion in `dual_search()` method (which calls `search_notes()`)
- Do NOT apply to `graph_search()` since spreading activation handles graph traversal

**Configuration struct**
- Create `QueryExpansionConfig` struct with `from_env()` pattern
- Follow existing `DualSearchConfig` and `SpreadingActivationConfig` patterns
- Fields: `expansion_depth`, `max_expansion_terms`, `broader_min_confidence`

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**`expand_search_term()` in `/home/md/construct-app/src/service.rs`**
- Handles bi-directional alias expansion (alias -> canonical, canonical -> aliases)
- Applies confidence filtering for LLM aliases (>= 0.8)
- Returns `Vec<String>` of expanded terms including original
- Extend or compose with this method rather than duplicating alias logic

**`build_expanded_fts_term()` in `/home/md/construct-app/src/service.rs`**
- Builds FTS5 OR expressions from expanded terms
- Handles proper quoting and escaping
- Modify to accept broader concept terms in addition to alias expansions

**`SpreadingActivationConfig::from_env()` in `/home/md/construct-app/src/spreading_activation.rs`**
- Pattern for parsing environment variables with fallback defaults
- Use same pattern for `QueryExpansionConfig`
- Example: `std::env::var("CONS_...").ok().and_then(|s| s.parse().ok()).unwrap_or(default)`

**`edges` table in `/home/md/construct-app/src/db/schema.rs`**
- Contains `hierarchy_type` column with values 'generic' or 'partitive'
- Contains `confidence` column for filtering
- Query pattern: `WHERE source_tag_id = ? AND hierarchy_type = 'generic' AND confidence >= ?`

**`DualSearchConfig` struct in `/home/md/construct-app/src/service.rs`**
- Reference for config struct organization and documentation style
- Shows how to combine `Default` trait with `from_env()` method

## Out of Scope

- Related concept expansion (relatedTo edges) - deferred per KNOWLEDGE.md guidance
- CLI flags for controlling expansion (environment variables only)
- Partitive edge traversal for broader concepts
- Expansion depth greater than 1 level in initial implementation
- Changes to `graph_search()` method (spreading activation already handles graph traversal)
- Narrower concept expansion (children of search terms)
- Weighted/boosted broader concepts (all expanded terms treated equally)
- Caching of expansion results (optimize later if performance becomes an issue)
- Broader concept expansion for queries with 3+ terms
