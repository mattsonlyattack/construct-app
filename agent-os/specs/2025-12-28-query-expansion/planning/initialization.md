# Query Expansion Specification

## Initial Description

Query expansion for the cons personal knowledge management CLI tool. The feature involves:

- Expanding search queries using tag aliases (always)
- Using broader concepts for short queries (<3 terms)
- Aggressive noise control to prevent over-expansion
- Configurable expansion depth

## Context

From roadmap item 21:
> Query expansion -- Before FTS, expand query using aliases (always), broader concepts (for short queries <3 terms); aggressive noise control to prevent over-expansion; configurable expansion depth `S`

From KNOWLEDGE.md:
> Query expansion using graph structure is a powerful enhancement. Before executing FTS, expand the query using:
> 1. Alias expansion: "ML" -> "ML OR machine-learning OR machine learning" (from tag_aliases)
> 2. Broader concept inclusion (optional): "transformers" -> include "neural networks" as secondary term
> 3. Related concept inclusion (careful): Add relatedTo concepts with lower weight
>
> Control expansion aggressively--uninhibited expansion produces noise. A practical heuristic: expand aliases always, broader concepts only for short queries (<3 terms), related concepts only on explicit user request.

## Related Completed Work

- Item 16: Alias-expanded FTS (completed) - `expand_search_term()` already exists
- Item 19: Spreading activation retrieval (completed) - graph traversal exists
- Item 20: Dual-channel search (completed) - combines FTS with spreading activation

## Key Files

- `/home/md/construct-app/src/service.rs` - Contains `expand_search_term()`, `search_notes()`, `dual_search()`
- `/home/md/construct-app/src/spreading_activation.rs` - Spreading activation engine
- `/home/md/construct-app/src/db/schema.rs` - Schema with edges table and tag_aliases
