# Specification: Degree Centrality

## Goal
Precompute and maintain connection counts per tag to enable "most connected" queries and importance-weighted retrieval ranking, giving hub nodes a modest boost in spreading activation.

## User Stories
- As a user, I want to see which tags are most connected in my knowledge graph so I can identify key concepts
- As a user, I want search results to favor notes tagged with well-connected concepts so that hub topics surface more relevant results

## Specific Requirements

**Schema Migration: Add degree_centrality Column**
- Add `degree_centrality INTEGER DEFAULT 0` column to `tags` table via ALTER TABLE
- Follow existing MIGRATIONS pattern in `src/db/schema.rs` for idempotent column additions
- Handle "duplicate column" errors gracefully during schema initialization (matches existing db.rs pattern)
- Column stores total bidirectional edge count (sum of incoming and outgoing edges)

**Backfill Existing Data**
- Execute one-time computation during schema initialization for existing edges
- Query: `UPDATE tags SET degree_centrality = (SELECT COUNT(*) FROM edges WHERE source_tag_id = tags.id OR target_tag_id = tags.id)`
- Run backfill after migration but before any edge operations
- Safe to re-run on subsequent database opens (idempotent update)

**NoteService Edge Creation Updates**
- Modify `create_edge()` to increment `degree_centrality` for both source and target tags
- Use same transaction as edge insert for atomicity
- Increment both tags by 1 when new edge is created
- Handle idempotent edge creation (skip increment if edge already exists)

**NoteService Edge Deletion Updates**
- Add `delete_edge()` method if not present, or modify existing deletion logic
- Decrement `degree_centrality` for both source and target tags when edge is removed
- Use same transaction as edge delete for consistency
- Decrement both tags by 1 when edge is deleted

**Spreading Activation Centrality Boost**
- Modify `spread_activation()` to apply centrality boost to activated nodes
- Query max degree centrality once at start of activation
- Apply boost formula: `boosted_activation = activation * (1.0 + (degree_centrality / max_degree) * 0.3)`
- High-degree hub nodes receive up to 30% boost; zero-degree nodes receive no boost
- Handle division by zero when max_degree is 0 (no boost applied)

**CLI Tags List Output Enhancement**
- Extend existing tag listing to include degree centrality count
- Display format: `tag-name (N notes, M connections)` or similar
- Order by tag name (existing behavior) but centrality available for future sorting options
- Reuse `get_tags_with_notes()` pattern or create new method combining note count and centrality

## Existing Code to Leverage

**`src/db/schema.rs` MIGRATIONS Constant**
- Pattern for ALTER TABLE migrations executed line-by-line
- Duplicate column errors handled gracefully in db.rs `initialize_schema()`
- Add new ALTER TABLE statement following existing enhancement_* column pattern

**`src/service.rs` create_edge() Method**
- Lines 1475-1538 implement edge creation with validation and idempotency check
- Add centrality increment after successful edge insert within same transaction
- Follow existing pattern of checking `exists` before insert

**`src/spreading_activation.rs` spread_activation() Function**
- Lines 120-185 implement the core spreading activation algorithm
- Boost can be applied to final activation scores before returning
- Requires joining tags table to get degree_centrality values

**`src/db.rs` initialize_schema() Method**
- Lines 44-82 show pattern for running migrations and handling errors
- Backfill query should run after migrations but can be in same initialization flow

**`src/service.rs` get_tags_with_notes() Method**
- Lines 1413-1435 show pattern for listing tags with JOIN queries
- Extend or create similar method to include degree_centrality in results

## Out of Scope
- TUI visualization with node sizing based on centrality (roadmap items #30-31)
- Separate tracking of in-degree vs out-degree (only total degree counted)
- Periodic batch recomputation jobs (incremental updates only)
- New dedicated CLI commands for centrality queries (integrated into existing `tags list`)
- Centrality decay over time or temporal weighting
- Configurable boost multiplier via environment variables
- Centrality-based sorting in tag list output
- Graph pruning based on low centrality
- Export/import of centrality metrics
- Weighted edges affecting centrality counts differently
