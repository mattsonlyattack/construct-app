# Task Breakdown: Graph Schema Foundation

## Overview
Total Tasks: 11

This spec creates the `edges` table to store weighted, typed, temporal relationships between tags for spreading activation retrieval and historical knowledge queries. The implementation follows the existing idempotent schema pattern in `/home/md/construct-app/src/db/schema.rs`.

## Task List

### Database Layer

#### Task Group 1: Edges Table Schema
**Dependencies:** None

- [x] 1.0 Complete edges table schema
  - [x] 1.1 Write 6 focused tests for edges table functionality
    - Test edges table exists after Database::in_memory()
    - Test edges table has all required columns with correct types (source_tag_id, target_tag_id, confidence, hierarchy_type, valid_from, valid_until, source, model_version, verified, created_at, updated_at)
    - Test hierarchy_type CHECK constraint enforces 'generic', 'partitive', or NULL
    - Test foreign key CASCADE delete on source_tag_id (delete tag removes edges)
    - Test foreign key CASCADE delete on target_tag_id (delete tag removes edges)
    - Test edges table allows duplicate source/target pairs with different validity windows
  - [x] 1.2 Add edges table creation to INITIAL_SCHEMA in schema.rs
    - Table name: `edges`
    - Primary key: Auto-increment INTEGER `id`
    - `source_tag_id` INTEGER NOT NULL
    - `target_tag_id` INTEGER NOT NULL
    - `confidence` REAL (unconstrained, no CHECK)
    - `hierarchy_type` TEXT with CHECK constraint: `hierarchy_type IN ('generic', 'partitive')` or NULL
    - `valid_from` INTEGER (nullable) - UNIX timestamp
    - `valid_until` INTEGER (nullable) - UNIX timestamp
    - `source` TEXT DEFAULT 'user'
    - `model_version` TEXT (nullable)
    - `verified` INTEGER DEFAULT 0
    - `created_at` INTEGER
    - `updated_at` INTEGER
    - Use CREATE TABLE IF NOT EXISTS for idempotent execution
    - Pattern reference: `/home/md/construct-app/src/db/schema.rs` note_tags and tag_aliases tables
  - [x] 1.3 Add foreign key constraints with CASCADE
    - `FOREIGN KEY (source_tag_id) REFERENCES tags(id) ON DELETE CASCADE`
    - `FOREIGN KEY (target_tag_id) REFERENCES tags(id) ON DELETE CASCADE`
    - Pattern reference: note_tags and tag_aliases foreign key constraints
  - [x] 1.4 Ensure edges table tests pass
    - Run ONLY the 6 tests written in 1.1
    - Verify CREATE TABLE IF NOT EXISTS is idempotent
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 6 tests written in 1.1 pass
- Edges table created with all 11 columns
- CHECK constraint enforces valid hierarchy_type values
- Foreign key constraints work with CASCADE delete
- Schema initialization remains idempotent

---

#### Task Group 2: Edges Table Indexes
**Dependencies:** Task Group 1

- [x] 2.0 Complete edges table indexes
  - [x] 2.1 Write 5 focused tests for edges indexes
    - Test idx_edges_source index exists
    - Test idx_edges_target index exists
    - Test idx_edges_created_at index exists
    - Test idx_edges_updated_at index exists
    - Test idx_edges_hierarchy_type index exists
  - [x] 2.2 Add indexes for traversal queries to INITIAL_SCHEMA (ALREADY DONE in Task Group 1)
    - `CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_tag_id)`
    - `CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_tag_id)`
    - Pattern reference: idx_note_tags_note and idx_note_tags_tag in schema.rs
  - [x] 2.3 Add indexes for temporal queries to INITIAL_SCHEMA (ALREADY DONE in Task Group 1)
    - `CREATE INDEX IF NOT EXISTS idx_edges_created_at ON edges(created_at)`
    - `CREATE INDEX IF NOT EXISTS idx_edges_updated_at ON edges(updated_at)`
    - Pattern reference: idx_notes_created in schema.rs
  - [x] 2.4 Add index for filtered traversal queries to INITIAL_SCHEMA (ALREADY DONE in Task Group 1)
    - `CREATE INDEX IF NOT EXISTS idx_edges_hierarchy_type ON edges(hierarchy_type)`
    - Enables efficient filtering by generic vs partitive hierarchy type
  - [x] 2.5 Ensure edges index tests pass
    - Run ONLY the 5 tests written in 2.1
    - Verify indexes are created and usable
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 5 tests written in 2.1 pass
- All 5 indexes created with IF NOT EXISTS
- Indexes enable efficient forward/reverse traversal
- Indexes enable efficient temporal and filtered queries

---

### Testing

#### Task Group 3: Integration Tests and Verification
**Dependencies:** Task Groups 1-2

- [x] 3.0 Verify integration and fill critical gaps
  - [x] 3.1 Review tests from Task Groups 1-2
    - Review the 6 tests from Task 1.1 (schema tests)
    - Review the 5 tests from Task 2.1 (index tests)
    - Total existing tests: 11 tests
  - [x] 3.2 Write up to 5 additional integration tests if needed
    - Test inserting edge with all columns populated
    - Test inserting edge with minimal columns (relying on defaults)
    - Test query performance uses indexes (EXPLAIN QUERY PLAN verification)
    - Test edges with temporal validity windows (valid_from/valid_until filtering)
    - Test schema reopen is idempotent (Database::open twice on same file)
  - [x] 3.3 Run all edges-related tests
    - Run tests from 1.1, 2.1, and 3.2 (up to 16 tests total)
    - Verify all tests pass
    - Do NOT run the entire application test suite
  - [x] 3.4 Verify schema integration with existing tables
    - Ensure notes, tags, note_tags, tag_aliases tables unaffected
    - Verify foreign keys from edges to tags work correctly
    - Confirm existing tests in db.rs still pass

**Acceptance Criteria:**
- All 11-16 edges-related tests pass
- Schema changes do not break existing functionality
- Edge insertion and querying work correctly
- Temporal queries can filter by validity windows

## Execution Order

Recommended implementation sequence:
1. **Task Group 1: Edges Table Schema** - Create the edges table with all columns and constraints
2. **Task Group 2: Edges Table Indexes** - Add indexes for efficient query patterns
3. **Task Group 3: Integration Tests** - Verify everything works together and fill test gaps

## Implementation Notes

### Schema Location
Add edges table and indexes to `/home/md/construct-app/src/db/schema.rs` in the `INITIAL_SCHEMA` constant, following the existing pattern of CREATE TABLE IF NOT EXISTS and CREATE INDEX IF NOT EXISTS.

### Column Types Reference
| Column | Type | Nullable | Default | Constraint |
|--------|------|----------|---------|------------|
| id | INTEGER | No | AUTO | PRIMARY KEY |
| source_tag_id | INTEGER | No | - | FK tags(id) CASCADE |
| target_tag_id | INTEGER | No | - | FK tags(id) CASCADE |
| confidence | REAL | Yes | - | None |
| hierarchy_type | TEXT | Yes | - | CHECK ('generic', 'partitive') |
| valid_from | INTEGER | Yes | - | UNIX timestamp |
| valid_until | INTEGER | Yes | - | UNIX timestamp |
| source | TEXT | Yes | 'user' | None |
| model_version | TEXT | Yes | - | None |
| verified | INTEGER | No | 0 | None |
| created_at | INTEGER | Yes | - | UNIX timestamp |
| updated_at | INTEGER | Yes | - | UNIX timestamp |

### Index Reference
| Index Name | Column(s) | Purpose |
|------------|-----------|---------|
| idx_edges_source | source_tag_id | Forward graph traversal |
| idx_edges_target | target_tag_id | Reverse graph traversal |
| idx_edges_created_at | created_at | Temporal queries by creation |
| idx_edges_updated_at | updated_at | Temporal queries by modification |
| idx_edges_hierarchy_type | hierarchy_type | Filtered traversal (generic vs partitive) |

### Out of Scope (per spec)
- EdgeId newtype wrapper (deferred to future spec)
- Edge Rust model struct (deferred to future spec)
- Service layer methods for edge CRUD (deferred to future spec)
- CLI commands for edge management (deferred to future spec)
- Spreading activation query implementation (roadmap item 19)
- LLM population of edges (roadmap item 18)
