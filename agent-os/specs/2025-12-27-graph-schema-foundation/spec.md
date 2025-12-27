# Specification: Graph Schema Foundation

## Goal

Create the `edges` table to store weighted, typed, temporal relationships between tags, enabling spreading activation retrieval and historical knowledge queries per KNOWLEDGE.md cognitive science principles.

## User Stories

- As a knowledge worker, I want my tags connected by hierarchical relationships so that searching for "neural networks" automatically surfaces notes about "transformers" (which specialize neural-network)
- As a user reviewing past beliefs, I want edges to have validity windows so that I can query "what did I believe about X in 2023" without deleted or overwritten relationships

## Specific Requirements

**Create edges table with core relationship columns**
- Table name: `edges`
- Primary key: Auto-increment INTEGER `id` (allows duplicate source/target pairs with different validity windows)
- `source_tag_id` INTEGER NOT NULL referencing tags(id)
- `target_tag_id` INTEGER NOT NULL referencing tags(id)
- `confidence` REAL (no CHECK constraint per requirements; unconstrained for flexibility)
- Use CREATE TABLE IF NOT EXISTS for idempotent schema execution

**Implement hierarchy_type column for XKOS semantics**
- `hierarchy_type` TEXT with CHECK constraint: `hierarchy_type IN ('generic', 'partitive')` or NULL
- 'generic' = is-a relationships (transformer specializes neural-network) - supports inheritance-style queries
- 'partitive' = part-of relationships (attention isPartOf transformer) - supports decomposition but NOT inheritance
- NULL = non-hierarchical relationships (relatedTo, supports, contradicts) - deferred semantic types

**Add temporal validity columns**
- `valid_from` INTEGER (nullable) - UNIX timestamp when relationship became valid; NULL means "always valid from beginning"
- `valid_until` INTEGER (nullable) - UNIX timestamp when relationship ceased being valid; NULL means "still valid / no end date"
- Enables historical queries filtering by current date or specific time periods

**Include provenance metadata columns matching note_tags pattern**
- `source` TEXT - distinguishes 'user' from 'llm' for weight calculations in spreading activation
- `model_version` TEXT (nullable) - records which LLM version inferred the relationship
- `verified` INTEGER DEFAULT 0 - user confirmation flag; verified edges get weight boost

**Add audit timestamp columns**
- `created_at` INTEGER - UNIX timestamp when edge was created
- `updated_at` INTEGER - UNIX timestamp when edge was last modified
- Match existing codebase convention of INTEGER timestamps (not TEXT)

**Implement foreign key constraints with CASCADE**
- `FOREIGN KEY (source_tag_id) REFERENCES tags(id) ON DELETE CASCADE`
- `FOREIGN KEY (target_tag_id) REFERENCES tags(id) ON DELETE CASCADE`
- Ensures graph integrity when tags are deleted
- Foreign keys already enabled via PRAGMA in Database::open()

**Create indexes for query patterns**
- `idx_edges_source` on source_tag_id for efficient forward traversal
- `idx_edges_target` on target_tag_id for efficient reverse traversal
- `idx_edges_created_at` for temporal queries on creation date
- `idx_edges_updated_at` for temporal queries on modification date
- `idx_edges_hierarchy_type` for filtered traversal queries (generic vs partitive)

**Follow existing migration pattern**
- Add edges table creation to INITIAL_SCHEMA constant in schema.rs
- Use IF NOT EXISTS for idempotent execution on fresh and existing databases
- Index creation uses CREATE INDEX IF NOT EXISTS

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**note_tags junction table pattern (`/home/md/construct-app/src/db/schema.rs`)**
- Reuse exact provenance column pattern: confidence REAL, source TEXT, verified INTEGER DEFAULT 0, model_version TEXT, created_at INTEGER
- Reuse foreign key constraint pattern with ON DELETE CASCADE
- Reuse index pattern for efficient lookups on both directions

**tag_aliases table pattern (`/home/md/construct-app/src/db/schema.rs`)**
- Demonstrates SKOS-inspired relationship mapping with provenance tracking
- Shows case-insensitive lookup pattern (COLLATE NOCASE on alias primary key)
- Includes same provenance columns: source, confidence, created_at, model_version

**Database initialization pattern (`/home/md/construct-app/src/db.rs`)**
- Uses idempotent CREATE TABLE IF NOT EXISTS in INITIAL_SCHEMA constant
- execute_batch() runs all schema statements in single transaction
- PRAGMA foreign_keys = ON already set in initialize_schema()
- Pattern for handling ALTER TABLE migrations with graceful duplicate column errors

**Type-safe ID pattern (`/home/md/construct-app/src/models/ids.rs`)**
- NoteId and TagId newtype wrappers provide compile-time safety
- Consider adding EdgeId newtype following same pattern for future Edge model struct

**Tag model pattern (`/home/md/construct-app/src/models/tag.rs`)**
- Tag struct with TagId, name, and aliases demonstrates domain model approach
- Same pattern can be used for future Edge struct with source_tag_id, target_tag_id, confidence, etc.

## Out of Scope

- Separate `concepts` table (using tags directly as source/target per requirements decision)
- `relationship_type` column for semantic types like 'supports', 'contradicts' (deferred)
- Spreading activation query implementation (roadmap item 19)
- LLM population of edges with broader/narrower relationships (roadmap item 18)
- Degree centrality precomputation columns on tags table (roadmap item 22)
- Edge Rust model struct (can be added in future spec)
- EdgeId newtype wrapper (can be added in future spec)
- Triggers for graph synchronization (not requested)
- CLI commands for edge management (future spec)
- Service layer methods for edge CRUD operations (future spec)
