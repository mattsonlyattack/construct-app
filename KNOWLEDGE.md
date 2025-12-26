# Building a knowledge graph PKM: what information science teaches engineers

**A local-first personal knowledge management tool with LLM-inferred relationships should adopt a property graph model on SQLite, use SKOS-inspired vocabulary patterns without full semantic web complexity, implement folksonomy-first organization with lightweight faceted structure, and retrieve knowledge using spreading activation algorithms that model how human memory actually works.** The critical insight from information science research is that personal knowledge systems have fundamentally different requirements than institutional ones—user vocabulary is inherently correct, emergent structure beats imposed taxonomy, automated extraction should carry confidence metadata that enables later correction rather than blocking capture upfront, and retrieval should leverage both lexical search and graph structure through dual-channel architectures.

The "cons" tool you're building sits at an interesting intersection: it needs graph structures sophisticated enough to support semantic relationships like "contradicts" and "prerequisite for," yet simple enough for a solo developer to maintain. Research across knowledge organization systems, existing PKM tools, and ontology design reveals a clear path—borrow conceptual patterns from standards like SKOS and Dublin Core while implementing them on pragmatic infrastructure like SQLite property graphs rather than full RDF stacks.

---

## The folksonomy-first principle: why your users' vocabulary is the right vocabulary

Library science offers a foundational insight that engineers often miss: the concept of **warrant**. Literary warrant says classification terms should emerge from the content being classified. User warrant says terms should reflect how users actually think and search. For a personal tool, this means the user's idiosyncratic vocabulary—their inconsistent tags, their personal abbreviations—isn't a bug to be fixed but a feature to be preserved.

The spectrum of Knowledge Organization Systems runs from unstructured folksonomies through term lists, taxonomies, and thesauri to full ontologies. Each step adds semantic richness but also maintenance burden. Research consistently shows that for personal systems, **hybrid approaches combining lightweight controlled vocabularies with user-generated tags offer the best balance**. Pure folksonomies suffer from synonymy (car/auto/automobile), polysemy (java as coffee/language/island), and tag explosion—but these problems are manageable with autocomplete suggestions, merge tools, and post-hoc clustering rather than upfront ontology design.

Ranganathan's faceted classification from the 1930s provides a powerful pattern that scales down beautifully to personal collections. Rather than forcing notes into a single hierarchy, analyze subjects into independent facets—what type of note is this, what domain does it belong to, what's its status, when is it relevant. A note can be a "tutorial" (type) about "machine learning" (domain) that's "evergreen" (status) without these facets conflicting. For cons, consider implementing **3-5 fixed facets maximum**: type (note/article/reference/idea), domain (user-configurable), status (draft/active/archived/evergreen), and perhaps source (original/clipped/synthesized).

The critical distinction library science makes between **aboutness and mention** directly applies to entity extraction. A note that mentions "Python" might actually be about debugging strategies—the entity appears but isn't the subject. Your LLM extraction will capture mentions, but you'll need a mechanism (perhaps confidence scoring, perhaps user confirmation) to distinguish primary topics from incidental references.

---

## AI-first personal tools: a different calculus for errors

The recommendations in this document draw heavily from enterprise and institutional knowledge management research, where errors have legal, compliance, and organizational consequences. **For a personal tool where you're the only user, the calculus is fundamentally different.** A wrong tag in your personal notes doesn't propagate to colleagues or trigger audit failures—it just means you might miss something in search, with full-text search as your fallback.

This shifts the design philosophy from **confirmation gates** to **tiered confidence with correction affordances**:

| Inference Type | Error Severity | MVP Behavior | Stored Metadata |
|----------------|----------------|--------------|-----------------|
| Tags | Low (FTS fallback) | Apply immediately | `confidence`, `source: "llm"` |
| Entity mentions | Low | Extract and link | `confidence`, `mention_type` |
| Entity normalization | Medium | Accept LLM canonical form | `aliases[]`, `merge_candidates[]` |
| Semantic relationships | **High** | Skip for MVP | — |

**Why semantic relationships are deferred:** "Supports," "contradicts," and "prerequisite for" are where errors actually hurt—they pollute synthesis and discovery queries, actively misleading you. Tags and entity mentions are recoverable; a wrong "contradicts" relationship is not. Save relationship inference for v2 when you have a corpus to evaluate accuracy against and UI for surfacing suggestions without asserting them as facts.

**Correction as a byproduct of use, not a workflow:** Instead of confirmation queues that interrupt capture, build correction affordances into surfaces you're already visiting:

- **Search results**: "Also tagged: X, Y, Z" with one-click removal
- **Note view**: inline tag chips that are deletable/editable
- **Entity pages**: show all notes mentioning this entity, make merge opportunities obvious
- **Dashboard (later)**: surface statistical anomalies—entities with single mentions, orphan tags, notes with zero connections

Errors get fixed *when they matter* (during retrieval and synthesis) rather than *when they happen* (interrupting capture). This means accepting that your graph will be noisy initially, optimizing for capture velocity, and trusting that noise is recoverable through use.

**The philosophical resolution:** "AI-first" ≠ "AI-infallible." It means AI does the work, human intervenes only when they notice and care. You're building for your future self who's searching, not your present self who's capturing.

---

## Property graphs beat RDF for personal tools, but steal SKOS patterns

The semantic web standards—RDF, OWL, SKOS—were designed for global interoperability and machine reasoning across organizational boundaries. For a single-user local tool, this is overkill. RDF's triple model requires verbose representation and awkward "reification" workarounds to put properties on relationships (like confidence scores on inferred links). OWL's formal ontology capabilities demand expertise to construct and perpetually lag behind evolving domains.

**Property graphs are the right choice for cons.** They treat relationships as first-class citizens with their own attributes—essential when you need to store that a "supports" relationship was LLM-inferred with **0.73 confidence** on a specific date. The model matches intuition (what you draw is what you store), supports schema evolution as your understanding develops, and has mature tooling. For local-first implementation, a SQLite-based approach like the simple-graph pattern (nodes table with JSON properties, edges table with from/to/type/properties) provides single-file portability, zero configuration, and adequate performance for thousands of notes.

However, **SKOS vocabulary patterns are worth adopting conceptually** even without RDF infrastructure:

- **Preferred and alternate labels** (`prefLabel`/`altLabel`) solve the synonym problem—"machine learning" is canonical, "ML" and "machine-learning" are aliases pointing to the same concept
- **Broader/narrower relationships** model natural hierarchies ("neural networks" narrower than "machine learning") without forcing rigid trees
- **Related relationships** capture associative connections that aren't hierarchical
- **Mapping properties** (`exactMatch`, `closeMatch`) enable future interoperability

A practical schema emerges: concepts have a unique ID, a preferred label, optional aliases, a type (tag/entity/topic), and optional definition. Relationships between concepts, notes, and structured records (contacts, events, bibliographic entries) live in an edges table with source, target, relationship type, confidence, provenance (user vs. LLM-inferred), and timestamps.

---

## Learning from Roam, Obsidian, and DEVONthink's architectural choices

Existing PKM tools have explored the design space extensively. **Roam Research** pioneered the block-as-atomic-unit model—every bullet point has a UUID, can be referenced from anywhere via `((block-id))` syntax, and changes propagate throughout the graph. This granularity enables powerful features but adds complexity. **Obsidian** chose plain Markdown files in folders, with links inferred from `[[wikilinks]]` at query time rather than stored in a separate database—maximizing portability and future-proofing at the cost of some query sophistication.

**Notion's architecture** offers relevant insights for mixing structured and unstructured data. Everything is a block with an ID, properties (decoupled from type for flexibility), content (array of child block IDs), and parent pointer (for permissions and hierarchy). This uniform model lets databases, pages, and inline content coexist. For cons, consider a similar approach where notes, contacts, events, and bibliographic entries are all "information entities" sharing common metadata properties while having type-specific fields.

**DEVONthink's "See Also" feature** demonstrates effective automated relationship discovery—it analyzes document contents to suggest related items with percentage confidence, improving as the database matures. Critically, it **shows suggestions rather than auto-organizing**, preserving user agency while reducing cognitive load. This pattern—surface LLM-inferred relationships with confidence scores, let users promote them to explicit links—respects the fundamental uncertainty in automated extraction.

For graph visualization, both Roam and Obsidian learned that **global graph views become useless "hairballs" at scale**. Local neighborhood views (showing 1-2 hops from a selected node) with filtering and depth controls are far more useful for exploration. Node sizing by connection count or recency, edge filtering by relationship type, and community detection for clustering related notes address the three query patterns: recall benefits from entity-to-note traversal, synthesis from neighborhood expansion with centrality measures, and discovery from path-finding between seemingly unrelated concepts.

---

## Modeling LLM-inferred relationships with appropriate epistemic humility

The relationship types you've identified—semantic (contradicts, supports, extends), structural (contains, references), and temporal (supersedes, precedes)—map well to established ontology patterns. Start with a **minimal core of 8-10 relationship types**:

| Category | Relationships | Properties |
|----------|---------------|------------|
| Generic | `relatedTo` (symmetric) | Default for unclear relationships |
| Hierarchical | `broaderThan`/`narrowerThan` | For concept organization |
| Citation | `references`/`referencedBy` | Explicit links |
| Evidential | `supports`/`contradicts` | Semantic analysis |
| Temporal | `supersedes`/`supersededBy` | Versioning |
| Extraction | `mentions`/`about` | Entity-note connections |

For LLM-inferred relationships, the critical pattern is **distinguishing confidence and provenance**. Every inferred relationship should carry: a confidence score (0.0-1.0), a source indicator (user-explicit vs. llm-inferred), a timestamp, optionally the model version that produced it, and a user-verified flag that starts false. RDF-star notation provides a clean conceptual model—annotating triples with metadata—implementable as JSON properties on edges in your property graph.

**PROV-O (Provenance Ontology)** patterns are worth borrowing for tracking how knowledge was derived. Each inference activity has an agent (which LLM), inputs (which notes), outputs (which relationships), and timestamp. This enables debugging when relationships seem wrong and builds the feedback loop for improving extraction over time.

For handling **contradictions over time**, temporal knowledge graph research suggests storing facts as quadruples: (subject, relation, object, timestamp/validity-period). Rather than overwriting "Project X uses Python 2" with "Project X uses Python 3," store both with their validity windows. Create explicit "supersedes" relationships when updating understanding. For notes specifically, consider immutable notes with links (new notes reference old ones) or version tags (`#valid-2024`) for time-bound content.

---

## Entity resolution will break in ways you don't expect

LLM-based entity extraction performs **significantly worse** than classical neural extractors at normalization—a finding that surprises many engineers. Research from the GDELT Project found LLMs "must often be run multiple times over the same passage, yielding different results each time" and frequently hallucinate wrong normalized forms. The fundamental problem: LLMs lack access to real-time knowledge graphs that enterprise entity recognizers use for disambiguation.

**Practical entity resolution for cons requires a hybrid approach:**

1. Use LLMs for initial extraction (they're good at finding entity boundaries)
2. Build local alias tables mapping variants to canonical IDs (store "NYC," "New York City," "New York, NY" all pointing to one entity)
3. Consider linking to stable external identifiers (Wikidata QIDs) for well-known entities
4. Implement confirmation workflows—don't auto-merge without user validation
5. Store extraction provenance so you can debug and improve

The **error rates you should expect** are sobering: even high-quality knowledge graphs like DBpedia have estimated **2.8% error rates** from source Wikipedia. LLM tagging "suffers from accuracy/precision issues when fed a large taxonomy" and produces inconsistent results across runs. Budget for 5-15% errors in automated systems and design interfaces that make correction low-friction.

Common engineer blind spots from an information science perspective include: assuming categories are stable (meaning evolves), assuming one entity belongs to one category (faceted classification exists for good reason), assuming more structure is always better (over-specification creates maintenance burden), and assuming automation can replace curation entirely. The Open World Assumption in semantic web reasoning—that missing information means "unknown" rather than "doesn't exist"—also trips up engineers who expect database-style closed-world semantics.

---

## Scale problems emerge around 1,000 notes—plan for graceful degradation

Research on personal knowledge bases at scale (8,000+ notes) consistently finds that **manual organization breaks down past 1,000 notes**, but also that heavy automation creates its own maintenance burden. Graph visualizations become unusable. Full-text search becomes noisy. Link density can reach a point where everything connects to everything, making the graph meaningless.

Sustainable patterns for long-term PKM include:

- **Atomic notes** (small, focused) enable better linking and reduce the cost of outdated content
- **Plain text formats** (Markdown) maximize portability and longevity
- **Maps of Content (MOCs)** as curated entry points to topic clusters
- **Progressive disclosure** in interfaces—show high-confidence, high-relevance connections first
- **Regular maintenance workflows** built into the tool—surfacing orphan notes, duplicate entities, stale tags

For temporal concerns, the "this was true then but not now" problem requires explicit validity windows on facts, version relationships between notes that supersede each other, and interfaces that surface temporal context when showing older content. Don't overwrite—append with timestamps. Store conflicting information with confidence scores rather than forcing premature resolution.

---

## Graph retrieval: how cognitive science informs finding what you know

The preceding sections focus on **representation**—how to structure and store knowledge. But representation without retrieval is a library without a search strategy. This section addresses the critical question: once you have a property graph of notes, tags, entities, and relationships, **how do you find what you need?**

The surprising insight from cognitive psychology is that human memory itself operates as a spreading activation network. The most effective retrieval algorithms for personal knowledge aren't arbitrary graph traversals—they're computational models of how your brain already works. This explains why "I know I wrote something about this" often succeeds when you mentally walk through related concepts, and why isolated keyword search frequently fails.

### Spreading activation: your graph should work like memory

The **spreading activation** model, introduced by Collins and Loftus in 1975 and foundational to cognitive psychology ever since, proposes that concepts in memory are nodes connected by associative links. When you think of "dog," activation spreads to connected concepts—"pet," "bark," "fur," "loyalty"—with strength decaying over distance. This priming effect is measurable: people recognize "butter" faster after seeing "bread" than after seeing "window."

**This is exactly the retrieval model cons should implement.** A December 2024 paper demonstrated that integrating spreading activation into RAG systems yields up to **39% improvement** in answer correctness compared to naive retrieval. The algorithm:

```
1. Parse query, identify seed nodes (tags, entities, concepts mentioned)
2. Assign initial activation A₀ = 1.0 to each seed
3. For each iteration (typically 2-4 hops):
   - For each active node n with activation Aₙ:
     - For each neighbor m connected by edge e:
       - Aₘ += Aₙ × weight(e) × decay
   - Apply threshold: discard nodes below minimum activation
4. Rank all activated nodes by final activation score
5. Retrieve notes connected to top-N activated concepts
```

The key parameters, each with cognitive and practical justification:

| Parameter | Typical Range | Rationale |
|-----------|---------------|-----------|
| **Decay factor** | 0.5–0.8 per hop | Models cognitive distance; 3-hop connections are weakly relevant |
| **Edge weight** | 0.0–1.0 by type | User-confirmed links > LLM-inferred; `about` > `mentions` |
| **Activation threshold** | 0.1–0.2 | Prevents explosion; focuses on meaningfully connected nodes |
| **Max hops** | 2–4 | Beyond 4 hops, almost everything connects to everything |

**Confidence metadata integrates naturally.** An LLM-inferred relationship with confidence 0.6 should have edge weight 0.6; a user-confirmed relationship has weight 1.0. This means uncertain inferences contribute less to retrieval without requiring you to delete them—the graph self-corrects toward high-confidence paths.

**Accumulation is the key insight.** If a concept receives activation from multiple paths, its scores accumulate. A note connected to three query-relevant concepts ranks higher than one connected to a single concept, even if that single connection is strong. This naturally surfaces **hub notes** that synthesize multiple ideas—exactly the notes most valuable for recall.

For cons, implement spreading activation as a SQL query pattern:

```sql
-- Pseudocode: iterative activation spreading
WITH RECURSIVE activated(node_id, activation, hop) AS (
  -- Seeds from query
  SELECT id, 1.0, 0 FROM concepts WHERE name IN (query_terms)
  UNION ALL
  -- Spread with decay
  SELECT e.target_id,
         a.activation * e.confidence * 0.7,  -- decay = 0.7
         a.hop + 1
  FROM activated a
  JOIN edges e ON a.node_id = e.source_id
  WHERE a.hop < 3 AND a.activation * e.confidence * 0.7 > 0.1
)
SELECT node_id, SUM(activation) as total_activation
FROM activated
GROUP BY node_id
ORDER BY total_activation DESC;
```

### XKOS hierarchy distinctions: not all "broader" is the same

SKOS provides `broader` and `narrower` relationships for hierarchical organization, and KNOWLEDGE.md recommends adopting these patterns. But SKOS deliberately avoids specifying **what kind** of hierarchy these represent—a decision that creates ambiguity at retrieval time.

The **XKOS extension** (eXtended Knowledge Organization System) distinguishes two fundamentally different hierarchical relationships that SKOS conflates:

| Relation Type | XKOS Properties | Meaning | Example |
|---------------|-----------------|---------|---------|
| **Generic** | `generalizes` / `specializes` | Type–subtype (is-a) | "Transformer" specializes "neural network" |
| **Partitive** | `hasPart` / `isPartOf` | Whole–part (has-a) | "Attention mechanism" isPartOf "transformer" |

**This distinction matters critically for retrieval semantics:**

**Generic (is-a) relationships support inheritance-style queries.** When you search for "neural networks," you probably want notes about transformers, CNNs, and RNNs—they're all neural networks. The query should traverse `specializes` edges downward, expanding scope. Conversely, a note tagged "transformer" is implicitly about neural networks, so `generalizes` edges should be traversable upward for aboutness inference.

**Partitive (part-of) relationships support composition queries but NOT inheritance.** When you search for "transformer components," you want attention mechanisms, layer normalization, feed-forward networks. But when searching for "neural networks," you don't want every note about matrix multiplication just because matrices are part of neural network implementations. Partitive edges should only be traversed when the query semantics explicitly request decomposition.

The practical implementation for cons:

```sql
-- Add relationship subtype to edges table
ALTER TABLE edges ADD COLUMN hierarchy_type
  TEXT CHECK(hierarchy_type IN ('generic', 'partitive', NULL));

-- Generic broader/narrower use hierarchy_type = 'generic'
-- Part-of relationships use hierarchy_type = 'partitive'
-- Non-hierarchical relationships leave it NULL
```

**Query-time behavior:**
- **Broad topic queries** ("What do I know about ML?"): Traverse `generic` narrower edges, ignore `partitive`
- **Decomposition queries** ("What are the parts of X?"): Traverse `partitive` hasPart edges only
- **Contextualization queries** ("What is X part of?"): Traverse `partitive` isPartOf edges upward

For MVP, you can default all `broader`/`narrower` relationships to generic (the common case) and add partitive as an explicit edge type when needed. But the schema should support the distinction from day one.

### Dual-channel retrieval: combining lexical and structural search

Neither full-text search nor graph traversal alone provides complete retrieval. They fail in complementary ways:

**FTS failures (the lexical gap):**
- You search "ML optimization" but your note says "training neural networks efficiently"
- Synonyms, paraphrases, and conceptual equivalents are invisible to keyword matching
- A note about a related concept that doesn't use your query terms won't surface

**Graph traversal failures (the structural gap):**
- You wrote a note but never tagged it or linked it—it's an orphan
- The LLM missed an entity, so there's no edge to traverse
- New notes aren't yet connected to the broader concept graph

**The solution is dual-channel retrieval with merge.** Microsoft's GraphRAG and recent academic work formalize this pattern:

```
Query → [Channel 1: FTS/Vector] → Candidate set A (lexically similar)
      → [Channel 2: Graph]      → Candidate set B (structurally connected)
      → [Merge & Rank]          → Final results
```

**Merge strategies, from simple to sophisticated:**

| Strategy | Method | Tradeoff |
|----------|--------|----------|
| **Union** | Return A ∪ B, interleave by score | Maximum recall, may surface irrelevant results |
| **Intersection boost** | Score(x) += bonus if x ∈ A ∩ B | Rewards notes found by both channels |
| **Graph-then-FTS** | Use graph to expand query terms, then FTS | Query expansion, controlled noise |
| **FTS-then-graph** | FTS seeds graph traversal | Finds lexically-matched hubs, expands structurally |

For cons MVP, implement **intersection boost** as the simplest effective strategy:

```sql
-- Simplified dual-channel with intersection boost
WITH fts_results AS (
  SELECT note_id, rank as fts_score FROM notes_fts WHERE notes_fts MATCH ?
),
graph_results AS (
  -- spreading activation query from above
  SELECT note_id, total_activation as graph_score FROM ...
),
merged AS (
  SELECT
    COALESCE(f.note_id, g.note_id) as note_id,
    COALESCE(f.fts_score, 0) as fts_score,
    COALESCE(g.graph_score, 0) as graph_score,
    -- Intersection boost: notes in both channels get multiplier
    CASE WHEN f.note_id IS NOT NULL AND g.note_id IS NOT NULL
         THEN 1.5 ELSE 1.0 END as boost
  FROM fts_results f
  FULL OUTER JOIN graph_results g ON f.note_id = g.note_id
)
SELECT note_id, (fts_score + graph_score) * boost as final_score
FROM merged
ORDER BY final_score DESC;
```

**Query expansion using graph structure** is a powerful enhancement. Before executing FTS, expand the query using:

1. **Alias expansion**: "ML" → "ML OR machine learning OR machine-learning" (from tag_aliases)
2. **Broader concept inclusion** (optional): "transformers" → include "neural networks" as secondary term
3. **Related concept inclusion** (careful): Add `relatedTo` concepts with lower weight

Control expansion aggressively—uninhibited expansion produces noise. A practical heuristic: expand aliases always, broader concepts only for short queries (<3 terms), related concepts only on explicit user request.

### Personalized PageRank and centrality measures: importance beyond connection count

KNOWLEDGE.md mentions "node sizing by connection count" for visualization, but connection count (degree centrality) is the crudest measure of importance. Different centrality measures answer fundamentally different questions:

| Measure | Question It Answers | Computation | Use in Cons |
|---------|---------------------|-------------|-------------|
| **Degree** | "What's most connected?" | COUNT(edges) | Quick importance proxy |
| **PageRank** | "What's linked by important things?" | Iterative; importance flows through links | Authoritative notes |
| **Personalized PageRank** | "What's important relative to X?" | PageRank seeded from X | "Related to X" queries |
| **Betweenness** | "What bridges different clusters?" | Path counting | Cross-domain insights |
| **Closeness** | "What can reach everything quickly?" | Inverse of average path length | Good entry points |

**Personalized PageRank (PPR)** is particularly valuable for personal knowledge. Unlike global PageRank (which finds universally important nodes), PPR finds nodes important **from a specific starting point**. When you're looking at a note about "distributed systems," PPR seeded from that note surfaces related concepts weighted by structural proximity—not just globally popular tags.

The algorithm (simplified):
```
1. Seed node(s) get initial PageRank = 1.0
2. Iteratively:
   - Each node distributes (1 - α) of its rank to neighbors
   - Each node receives α × (original seed rank) as "teleport" back to seeds
3. Converge when rank changes < ε
```

The teleport probability α (typically 0.15–0.3) controls locality: higher α = more focused on immediate neighborhood, lower α = more exploration.

**For cons, precompute centrality measures as node properties:**

```sql
ALTER TABLE concepts ADD COLUMN degree_centrality INTEGER DEFAULT 0;
ALTER TABLE concepts ADD COLUMN pagerank REAL DEFAULT 0.0;
-- Update periodically (on write, or background job)

-- Degree is trivial
UPDATE concepts SET degree_centrality = (
  SELECT COUNT(*) FROM edges
  WHERE source_id = concepts.id OR target_id = concepts.id
);
```

PageRank requires iteration but converges quickly for graphs under 10K nodes. Compute it in Rust on startup or after significant changes, store results. Betweenness is expensive—compute offline for "insight discovery" features, not real-time queries.

### Temporal retrieval: when you knew it matters

Personal knowledge has a temporal dimension that enterprise knowledge graphs often ignore. A note from yesterday about a meeting is highly relevant today; the same note in six months is archival. But a conceptual insight from years ago might be eternally relevant. **Recency and validity are orthogonal to importance.**

**Temporal considerations for retrieval:**

1. **Recency-weighted activation**: Boost recent notes in spreading activation
   ```
   temporal_boost = 1.0 + (0.5 × e^(-days_old / 30))  // Decays over ~month
   final_activation = base_activation × temporal_boost
   ```

2. **Validity windows on relationships**: Some facts are time-bound
   - "Project X uses Python 2" was true 2018-2020
   - "Project X uses Python 3" is true 2020-present
   - Store `valid_from` and `valid_until` on edges; filter by current date in queries

3. **Freshness signals in ranking**: Surface notes updated recently higher than stale notes with same relevance score

4. **Temporal clustering**: "What was I thinking about last week?" is a legitimate query—support date-range filters that interact with graph structure

**The "this was true then" problem** from earlier in KNOWLEDGE.md connects here. Don't delete superseded information; mark it with validity windows. Retrieval should default to current validity but support historical queries ("What did I believe about X in 2023?").

### Concept schemes: namespacing for context

SKOS **Concept Schemes** group related concepts into coherent vocabularies. For personal knowledge, this enables:

- **Work vs. personal separation**: Professional concepts in one scheme, hobbies in another
- **Project namespacing**: Each major project gets its own concept scheme
- **Context-aware retrieval**: When searching "in work context," prefer work-scheme concepts

Implementation is lightweight:

```sql
CREATE TABLE concept_schemes (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT
);

ALTER TABLE concepts ADD COLUMN scheme_id INTEGER REFERENCES concept_schemes(id);
```

**Cross-scheme links remain possible**—a methodology concept might belong to your "research" scheme but be referenced from "work" projects. The scheme provides a default context, not a hard boundary.

For retrieval, scheme membership becomes a **filter or boost**, not a wall:
- "Search in work context": scheme_id = work_scheme gets 1.5x boost
- "Search everywhere": no scheme filter
- "Search only personal": scheme_id = personal_scheme required

This is v2 functionality for cons—the schema should support it, but the UI and retrieval logic can wait until the base system proves useful.

---

## Concrete recommendations for cons

**Data model:** Property graph on SQLite with SKOS-inspired vocabulary. Nodes table for notes, concepts, and structured records (contacts/events/bibliographic entries) with JSON properties. Edges table with source, target, type, confidence, provenance, and timestamps. Full-text search via SQLite FTS5 for recall queries.

**MVP schema additions for AI-first tagging:**
- `note_tags` junction table includes `confidence` (REAL), `source` ('user' | 'llm'), and `created_at` columns
- `tag_aliases` table maps alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern)
- Store all LLM inferences immediately; never block capture for confirmation

**Entity extraction pipeline (MVP):** LLM extracts tags and applies them immediately with confidence scores and `source: "llm"`. Accept LLM's canonical form for tag normalization. Build alias tables mapping variants to canonical IDs. Defer entity typing and structured extraction to v2.

**Relationship inference (defer to v2):** Semantic relationships like "supports" and "contradicts" are high-stakes errors that pollute discovery queries. Skip for MVP until you have: (1) a corpus to evaluate accuracy against, (2) UI for surfacing suggestions without asserting them as facts, (3) feedback loops to improve extraction.

**Query support:** Recall uses full-text search plus tag-to-note traversal. Synthesis and discovery features that depend on semantic relationships are deferred.

**Retrieval architecture (phased):**

| Phase | Capability | Implementation |
|-------|------------|----------------|
| **MVP** | FTS + tag filtering | SQLite FTS5, simple tag-to-note joins |
| **v1.1** | Alias-expanded FTS | Query expansion via tag_aliases before FTS |
| **v1.2** | Spreading activation | Recursive CTE from seed concepts, 2-3 hops, decay 0.7 |
| **v2** | Dual-channel retrieval | FTS ∪ graph with intersection boost |
| **v2+** | PPR, centrality, temporal | Precomputed metrics, validity windows |

**MVP retrieval schema additions:**
- `edges.hierarchy_type` TEXT ('generic' | 'partitive' | NULL) — for XKOS-style traversal semantics
- `edges.confidence` REAL — used as edge weight in spreading activation
- `edges.valid_from` / `edges.valid_until` TIMESTAMP — for temporal validity (nullable, default to always-valid)
- `concepts.degree_centrality` INTEGER — precomputed, updated on edge changes
- `concepts.pagerank` REAL — precomputed periodically (startup or background)

**Spreading activation defaults:** decay=0.7, threshold=0.1, max_hops=3. These work well for graphs under 5K nodes; tune if retrieval becomes noisy at scale.

**What to skip:** Full RDF/SPARQL stack, OWL ontologies, complex inference engines, enterprise graph databases, confirmation workflows that block capture. Real-time betweenness centrality (compute offline). Embedding-based vector search (FTS5 is sufficient for MVP; add embeddings when you have data proving FTS fails).

**Failure modes to design against:** LLM hallucination (store confidence metadata, build correction into retrieval UI), entity proliferation (aggressive alias detection and merge suggestions), tag explosion (autocomplete and cleanup dashboards), scale degradation (filtering controls, local rather than global graph views), **retrieval noise from over-expansion** (aggressive thresholds, controlled query expansion), **cold-start sparsity** (fall back to FTS when graph is too sparse for meaningful traversal).

The deepest insight from this research is that **personal knowledge management is fundamentally about augmenting human judgment, not replacing it**—but for a personal tool, that augmentation happens during retrieval and synthesis, not during capture. Your LLMs should tag immediately with confidence metadata; correction happens when errors surface during use, not through upfront confirmation gates. **Retrieval should model how memory works**: spreading activation from query concepts, accumulated relevance from multiple paths, recency-weighted results, and graceful degradation when the graph is sparse.