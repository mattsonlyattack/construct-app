# Building a knowledge graph PKM: what information science teaches engineers

**A local-first personal knowledge management tool with LLM-inferred relationships should adopt a property graph model on SQLite, use SKOS-inspired vocabulary patterns without full semantic web complexity, and implement folksonomy-first organization with lightweight faceted structure.** The critical insight from information science research is that personal knowledge systems have fundamentally different requirements than institutional ones—user vocabulary is inherently correct, emergent structure beats imposed taxonomy, and automated extraction should carry confidence metadata that enables later correction rather than blocking capture upfront.

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

## Concrete recommendations for cons

**Data model:** Property graph on SQLite with SKOS-inspired vocabulary. Nodes table for notes, concepts, and structured records (contacts/events/bibliographic entries) with JSON properties. Edges table with source, target, type, confidence, provenance, and timestamps. Full-text search via SQLite FTS5 for recall queries.

**MVP schema additions for AI-first tagging:**
- `note_tags` junction table includes `confidence` (REAL), `source` ('user' | 'llm'), and `created_at` columns
- `tag_aliases` table maps alternate forms to canonical tag IDs (SKOS prefLabel/altLabel pattern)
- Store all LLM inferences immediately; never block capture for confirmation

**Entity extraction pipeline (MVP):** LLM extracts tags and applies them immediately with confidence scores and `source: "llm"`. Accept LLM's canonical form for tag normalization. Build alias tables mapping variants to canonical IDs. Defer entity typing and structured extraction to v2.

**Relationship inference (defer to v2):** Semantic relationships like "supports" and "contradicts" are high-stakes errors that pollute discovery queries. Skip for MVP until you have: (1) a corpus to evaluate accuracy against, (2) UI for surfacing suggestions without asserting them as facts, (3) feedback loops to improve extraction.

**Query support:** Recall uses full-text search plus tag-to-note traversal. Synthesis and discovery features that depend on semantic relationships are deferred.

**What to skip:** Full RDF/SPARQL stack, OWL ontologies, complex inference engines, enterprise graph databases, confirmation workflows that block capture. These add complexity without proportional value for personal scale.

**Failure modes to design against:** LLM hallucination (store confidence metadata, build correction into retrieval UI), entity proliferation (aggressive alias detection and merge suggestions), tag explosion (autocomplete and cleanup dashboards), scale degradation (filtering controls, local rather than global graph views).

The deepest insight from this research is that **personal knowledge management is fundamentally about augmenting human judgment, not replacing it**—but for a personal tool, that augmentation happens during retrieval and synthesis, not during capture. Your LLMs should tag immediately with confidence metadata; correction happens when errors surface during use, not through upfront confirmation gates.