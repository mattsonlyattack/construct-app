# Spec Requirements: Query Expansion

## Initial Description

Query expansion for the cons personal knowledge management CLI tool. The feature involves:

- Expanding search queries using tag aliases (always)
- Using broader concepts for short queries (<3 terms)
- Aggressive noise control to prevent over-expansion
- Configurable expansion depth

From roadmap item 21:
> Query expansion -- Before FTS, expand query using aliases (always), broader concepts (for short queries <3 terms); aggressive noise control to prevent over-expansion; configurable expansion depth `S`

## Requirements Discussion

### First Round Questions

**Q1:** Where should query expansion integrate? I assume query expansion should happen in `search_notes()` (FTS) and `dual_search()`, not in `graph_search()` since spreading activation already handles graph traversal. Is that correct, or should expansion happen at a different integration point?
**Answer:** Yes - expand in `search_notes()` (FTS) and `dual_search()`, not in `graph_search()`

**Q2:** For broader concept expansion, should we traverse only "generic" hierarchy edges or also "partitive" edges? Per XKOS semantics in KNOWLEDGE.md, generic (is-a) supports inheritance-style queries ("transformers" -> "neural networks") while partitive (part-of) does not. I'm assuming we should only use generic edges for broader concept expansion. Correct?
**Answer:** Only generic edges (not partitive)

**Q3:** What should the default expansion depth be? I'm thinking 1 level of broader concepts (immediate parent only) as the default, with configurable depth via environment variable (e.g., `CONS_EXPANSION_DEPTH`). Should we allow deeper expansion (2+ levels) or is 1 level sufficient?
**Answer:** 1 level for now (immediate parent only)

**Q4:** How should broader concepts be weighted in FTS queries? Options include:
- A) Treat them as equal terms in an OR expression (same as alias expansion today)
- B) Use a separate secondary query and merge results with lower weight
- C) Add them as "optional" terms that boost but don't require matches

I'm leaning toward (A) for simplicity. Does that work, or do you prefer a different approach?
**Answer:** Option A - treat as equal terms in OR expression

**Q5:** What maximum expansion limits should we enforce for noise control? I'm thinking:
- Max 10 total expanded terms per original query term
- Only expand broader concepts if query has <3 terms (per KNOWLEDGE.md)
- Only include broader concepts with edge confidence >= 0.7

Are these thresholds reasonable, or do you have different values in mind?
**Answer:** Yes to all suggested thresholds (max 10 terms, <3 term queries only, confidence >= 0.7)

**Q6:** Should we expose this via CLI flags? For example:
- `cons search "rust" --no-expand` to disable all expansion
- `cons search "rust" --expand-depth=2` to include grandparent concepts

Or should expansion be purely automatic with environment variable configuration only?
**Answer:** No flags - purely automatic with environment variable configuration only

**Q7:** Is there anything that should explicitly be OUT of scope for this feature?
**Answer:** (Not explicitly answered) Assume "related concept" expansion is deferred per KNOWLEDGE.md guidance

### Existing Code to Reference

**Similar Features Identified:**
- Feature: Alias expansion - Path: `/home/md/construct-app/src/service.rs` (method `expand_search_term()`)
- Feature: Spreading activation - Path: `/home/md/construct-app/src/spreading_activation.rs`
- Feature: Dual-channel search - Path: `/home/md/construct-app/src/service.rs` (method `dual_search()`)
- Feature: FTS search - Path: `/home/md/construct-app/src/service.rs` (method `search_notes()`)

The existing `expand_search_term()` method handles alias expansion and can be extended or complemented for broader concept expansion.

### Follow-up Questions

No follow-up questions were needed - user answers were clear and complete.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

- **Alias expansion (always)**: Expand query terms using `tag_aliases` table (already implemented in `expand_search_term()`)
- **Broader concept expansion (conditional)**: For queries with <3 terms, expand using parent concepts from `edges` table
- **Hierarchy type filtering**: Only traverse `generic` (is-a) edges, not `partitive` (part-of) edges
- **Expansion depth**: 1 level (immediate parent only), configurable via environment variable
- **Confidence filtering**: Only include broader concepts from edges with confidence >= 0.7
- **Term limit**: Maximum 10 expanded terms per original query term
- **Integration points**: Apply expansion in `search_notes()` and `dual_search()`, not in `graph_search()`
- **FTS query format**: Treat broader concepts as equal terms in OR expression (same as alias expansion)

### Reusability Opportunities

- Extend or compose with existing `expand_search_term()` method
- Follow patterns from `SpreadingActivationConfig` for environment variable configuration
- Use similar confidence filtering patterns from alias expansion (which filters at 0.8 for LLM aliases)

### Scope Boundaries

**In Scope:**
- Broader concept expansion using generic hierarchy edges
- Configurable expansion depth via environment variable
- Noise control thresholds (term limits, query length limits, confidence filtering)
- Integration with `search_notes()` and `dual_search()`

**Out of Scope:**
- Related concept expansion (relatedTo edges) - deferred per KNOWLEDGE.md
- CLI flags for controlling expansion (environment variables only)
- Partitive edge traversal for broader concepts
- Expansion depth > 1 level (for now)
- Changes to `graph_search()` (spreading activation already handles graph traversal)

### Technical Considerations

- **Environment variables to add**:
  - `CONS_EXPANSION_DEPTH` (default: 1) - levels of broader concepts to include
  - `CONS_MAX_EXPANSION_TERMS` (default: 10) - max expanded terms per query term
  - `CONS_BROADER_MIN_CONFIDENCE` (default: 0.7) - minimum edge confidence for broader concepts

- **Query structure**: Broader concepts should be OR'd together with aliases, maintaining AND logic between original query terms

- **Edge traversal direction**: From tag (source) to broader concept (target) in the edges table, where `source_tag_id` is the narrower/child and `target_tag_id` is the broader/parent

- **Existing schema support**: The `edges` table already has `hierarchy_type` column with values 'generic' or 'partitive', and `confidence` column for filtering

- **Performance consideration**: Broader concept lookup requires additional queries per term; consider caching or batching if performance becomes an issue
