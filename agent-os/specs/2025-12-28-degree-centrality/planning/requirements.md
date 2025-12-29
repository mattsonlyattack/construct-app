# Degree Centrality - Requirements

## Summary

Precompute connection count per tag/concept, update incrementally on edge changes; use for "most connected" queries and importance signals in retrieval ranking.

**Effort**: S (2-3 days)
**Roadmap Item**: #22

---

## Key Decisions

### 1. Storage Strategy
**Decision**: Column on `tags` table

```sql
ALTER TABLE tags ADD COLUMN degree_centrality INTEGER DEFAULT 0;
```

**Rationale**: Consistent with existing schema patterns, avoids joins, simple to maintain.

### 2. Counting Method
**Decision**: Total edges bidirectionally

For tag with ID X: count rows where `source_tag_id = X OR target_tag_id = X`. This measures "how connected" the concept is overall, regardless of direction.

### 3. Update Strategy
**Decision**: Application-layer updates in NoteService

When inserting/deleting edges, increment/decrement the relevant tags' centrality counts in the same transaction.

**Rationale**: SQLite triggers add complexity; batch recomputation is eventually consistent. Application layer gives control and matches existing patterns.

### 4. Retrieval Ranking Integration
**Decision**: Multiply spreading activation scores

When spreading activation reaches a node, weight it by:
```
1.0 + (degree_centrality / max_degree) * 0.3
```

High-degree nodes (hubs) get modest boost. Not a separate channel—degree is a property of nodes that affects graph traversal.

### 5. Visualization Priority
**Decision**: Not needed for MVP

The roadmap mentions "use for visualization node sizing" but TUI items (#30-31) come later. Store the metric now for when needed, but don't build visualization yet.

### 6. API Surface
**Decision**: Include in existing output

`cons tags list` shows centrality alongside note count. No new command needed. Users discover high-degree tags naturally when browsing.

---

## Implementation Scope

### In Scope (MVP)
- [ ] Schema migration: Add `degree_centrality` column to `tags` table
- [ ] Backfill existing data: One-time computation for existing edges
- [ ] NoteService updates: Increment/decrement on edge insert/delete
- [ ] Spreading activation integration: Apply centrality boost to activated nodes
- [ ] CLI output: Show centrality in `cons tags list`

### Out of Scope (Deferred)
- TUI visualization node sizing (roadmap items #30-31)
- Separate in-degree/out-degree tracking
- Periodic recomputation or batch jobs
- New CLI commands for centrality queries

---

## Technical Notes

### Schema Change
```sql
ALTER TABLE tags ADD COLUMN degree_centrality INTEGER DEFAULT 0;
```

### Backfill Query
```sql
UPDATE tags SET degree_centrality = (
    SELECT COUNT(*) FROM edges
    WHERE source_tag_id = tags.id OR target_tag_id = tags.id
);
```

### Centrality Boost Formula
```rust
let boost = 1.0 + (tag.degree_centrality as f64 / max_degree as f64) * 0.3;
let boosted_activation = activation * boost;
```

### Update Points in NoteService
- `create_edge()` or equivalent: increment both source and target tag centrality
- `delete_edge()` or equivalent: decrement both source and target tag centrality
- Use same transaction to ensure consistency
