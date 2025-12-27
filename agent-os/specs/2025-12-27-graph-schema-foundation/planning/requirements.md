# Spec Requirements: Graph Schema Foundation

## Initial Description

From the roadmap item 17:
"Graph schema foundation -- Create edges table with confidence (REAL), hierarchy_type ('generic'|'partitive'|NULL), valid_from/valid_until (TIMESTAMP nullable); enables spreading activation and temporal queries"

Key context from KNOWLEDGE.md:
- Uses SKOS/XKOS semantics for vocabulary relationships
- Spreading activation is from cognitive psychology - activating one concept "spreads" to related concepts
- Generic hierarchy = is-a relationships (transformer is-a neural-network)
- Partitive hierarchy = part-of relationships (attention isPartOf transformer)
- The graph connects concepts/tags, not just notes directly
- Temporal queries allow asking "what did I believe about X in the past"

Existing database schema context:
- notes table (id, content, enhanced_content, created_at, updated_at)
- tags table (id, name - canonical tag names)
- tag_aliases table (id, alias, tag_id - maps alternate forms to canonical tags)
- note_tags junction table (note_id, tag_id) with confidence, source, verified, model_version

The edges table will connect tags/concepts to each other with weighted, typed, temporal relationships.

## Requirements Discussion

### First Round Questions

**Q1:** The roadmap specifies the `edges` table connects concepts/tags. I assume this means edges will use `tag_id` as source and target (connecting tags to each other), rather than creating a separate `concepts` table. Is that correct, or should we introduce a new `concepts` table that tags are a subset of?
**Answer:** Use tag_id for now - Use tag_id as source and target, no separate concepts table yet

**Q2:** For the `confidence` column (REAL), I assume we'll follow the existing pattern from `note_tags` where confidence ranges from 0.0 to 1.0. Should the schema enforce this range with a CHECK constraint, or leave it unconstrained for flexibility?
**Answer:** Unconstrained - No CHECK constraint on confidence range, leave flexible

**Q3:** The `hierarchy_type` column distinguishes generic (is-a) from partitive (part-of) relationships per XKOS semantics. I assume non-hierarchical relationships (like `relatedTo`, `supports`, `contradicts`) will have `hierarchy_type = NULL`. Is that the intended design, or should we add a separate `relationship_type` column for semantic relationship types?
**Answer:** NULL is fine, no relationship_type yet - hierarchy_type NULL for non-hierarchical relationships, defer relationship_type column

**Q4:** For `valid_from` / `valid_until` temporal fields, I assume these are UNIX timestamps (INTEGER) to match the existing `created_at` / `updated_at` pattern in the codebase, rather than TEXT timestamps. Is that correct?
**Answer:** Yes - Use UNIX timestamps (INTEGER) matching existing pattern

**Q5:** Should edges include provenance metadata like `note_tags` has (`source TEXT`, `model_version TEXT`, `verified INTEGER`)? This would track whether an edge was user-created vs. LLM-inferred, which is critical for spreading activation weight calculations mentioned in KNOWLEDGE.md.
**Answer:** Yes, fail open, store everything with provenance - Include source, model_version, verified columns like note_tags

**Q6:** I'm assuming we need `created_at` and `updated_at` timestamps on the edges table per your standards. Should there also be an index on these for temporal queries?
**Answer:** Yes - Add created_at/updated_at with indexes for temporal queries

**Q7:** The roadmap item 18 (Tag hierarchy population) will populate edges with LLM-suggested broader/narrower relationships. Should the schema include any foreign key constraints, or leave source/target as plain INTEGERs to support future entity types beyond tags?
**Answer:** Use FKs for both source and target - Foreign key constraints to tags table

**Q8:** Is there anything that should be explicitly excluded from this spec (deferred to later work)? For example, should we defer indexes beyond basic foreign key indexes, or defer triggers for graph synchronization?
**Answer:** No - Nothing explicitly excluded, implement complete solution

### Existing Code to Reference

**Similar Features Identified:**
- Feature: note_tags junction table - Path: `/home/md/construct-app/src/db/schema.rs`
  - Same provenance pattern (confidence, source, verified, model_version, created_at)
  - Same foreign key constraint pattern with ON DELETE CASCADE
  - Same index pattern for efficient lookups
- Feature: tag_aliases table - Path: `/home/md/construct-app/src/db/schema.rs`
  - Similar SKOS-inspired relationship mapping
  - Same provenance columns (source, confidence, created_at, model_version)
- Feature: Migration pattern - Path: `/home/md/construct-app/src/db/schema.rs`
  - Uses idempotent CREATE TABLE IF NOT EXISTS
  - ALTER TABLE statements handled separately with graceful duplicate column handling

### Follow-up Questions

No follow-up questions needed - user answers were comprehensive.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements
- Create `edges` table to store relationships between tags
- Store relationship confidence as REAL (unconstrained range)
- Store hierarchy type distinguishing generic (is-a) from partitive (part-of) relationships
- Store temporal validity windows (valid_from, valid_until) for time-bound facts
- Store full provenance metadata (source, model_version, verified) matching note_tags pattern
- Store audit timestamps (created_at, updated_at) for all edges
- Enable efficient queries for spreading activation algorithm
- Enable efficient temporal queries for historical knowledge retrieval

### Schema Design Requirements
- **Source/Target columns**: INTEGER foreign keys referencing tags(id)
- **Confidence column**: REAL, unconstrained (no CHECK constraint)
- **Hierarchy type column**: TEXT with CHECK constraint for 'generic'|'partitive'|NULL
- **Temporal columns**: valid_from INTEGER (nullable), valid_until INTEGER (nullable) - UNIX timestamps
- **Provenance columns**: source TEXT, model_version TEXT, verified INTEGER (matching note_tags)
- **Audit columns**: created_at INTEGER, updated_at INTEGER
- **Foreign keys**: Both source_tag_id and target_tag_id with ON DELETE CASCADE
- **Primary key**: Composite of (source_tag_id, target_tag_id) or separate id column (implementation decision)

### Index Requirements
- Index on source_tag_id for efficient forward traversal
- Index on target_tag_id for efficient reverse traversal
- Index on created_at for temporal queries
- Index on updated_at for temporal queries
- Consider index on hierarchy_type for filtered traversal queries

### Reusability Opportunities
- Follow exact provenance pattern from note_tags table
- Follow exact foreign key pattern from note_tags table
- Follow existing migration pattern in schema.rs (idempotent CREATE TABLE IF NOT EXISTS)
- Follow existing timestamp pattern (UNIX INTEGER timestamps)

### Scope Boundaries

**In Scope:**
- Create edges table with all specified columns
- Add appropriate indexes for query patterns
- Foreign key constraints to tags table
- Provenance metadata columns
- Temporal validity columns
- Audit timestamp columns
- Integration with existing schema initialization pattern

**Out of Scope:**
- Separate concepts table (deferred - using tags directly)
- relationship_type column for semantic relationships (deferred)
- Spreading activation query implementation (roadmap item 19)
- LLM population of edges (roadmap item 18)
- Triggers for graph synchronization (not requested)
- Degree centrality precomputation columns on tags (roadmap item 22)

### Technical Considerations
- Schema follows existing idempotent pattern (CREATE TABLE IF NOT EXISTS)
- Foreign keys require PRAGMA foreign_keys = ON (already enabled in Database::open())
- Timestamps use UNIX epoch integers, not TEXT, matching codebase convention
- No CHECK constraint on confidence to allow flexibility
- CHECK constraint on hierarchy_type to enforce valid values
- ON DELETE CASCADE ensures graph integrity when tags are deleted
- Consider whether edges should allow duplicate relationships (same source/target with different validity windows) - this affects primary key design

### XKOS Semantics Reference
Per KNOWLEDGE.md, the hierarchy_type column enables XKOS-style traversal:
- **'generic'** (is-a): "Transformer" specializes "neural-network" - supports inheritance-style queries where searching "neural networks" returns transformer notes
- **'partitive'** (part-of): "Attention mechanism" isPartOf "transformer" - supports decomposition queries but NOT inheritance (searching "neural networks" should NOT return matrix multiplication notes)
- **NULL**: Non-hierarchical relationships like relatedTo, supports, contradicts (semantic relationships deferred)

### Spreading Activation Context
The edges table enables the spreading activation algorithm (roadmap item 19):
- confidence column serves as edge weight in activation spreading
- verified column can boost weight for user-confirmed relationships
- source column distinguishes user-created (higher weight) from LLM-inferred (weight = confidence)
- hierarchy_type enables query-aware traversal (generic vs partitive handling)
- valid_from/valid_until enables temporal filtering in queries

### Default Values to Consider
- confidence: Could default to 1.0 for user-created edges
- source: Could default to 'user' for manually created edges
- verified: Should default to 0 (false) for LLM-inferred, 1 (true) for user-created
- valid_from: NULL means "always valid from the beginning"
- valid_until: NULL means "still valid / no end date"
- created_at: Should be set on insert
- updated_at: Should be set on insert and updated on modification
