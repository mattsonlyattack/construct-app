# Task Breakdown: Degree Centrality

## Overview
Total Tasks: 4 Task Groups
Effort Estimate: S (2-3 days)

This feature adds degree centrality tracking to tags, enabling "most connected" queries and importance-weighted retrieval ranking through spreading activation boosts.

## Task List

### Database Layer

#### Task Group 1: Schema Migration and Data Backfill
**Dependencies:** None

- [ ] 1.0 Complete database layer for degree centrality
  - [ ] 1.1 Write 3-4 focused tests for schema and backfill
    - Test that degree_centrality column exists after schema initialization
    - Test that backfill correctly counts edges for tags with existing edges
    - Test that tags with no edges have degree_centrality = 0
    - Test idempotent re-run of backfill (values remain correct)
  - [ ] 1.2 Add ALTER TABLE migration to MIGRATIONS constant
    - Add `ALTER TABLE tags ADD COLUMN degree_centrality INTEGER DEFAULT 0;` to `src/db/schema.rs`
    - Follow existing enhancement_* column pattern in MIGRATIONS constant
    - Duplicate column errors handled by existing db.rs logic
  - [ ] 1.3 Implement backfill query in initialize_schema
    - Add backfill after migrations in `src/db.rs` initialize_schema()
    - Query: `UPDATE tags SET degree_centrality = (SELECT COUNT(*) FROM edges WHERE source_tag_id = tags.id OR target_tag_id = tags.id)`
    - Run after migrations but before FTS initialization
    - Safe for re-runs (idempotent UPDATE)
  - [ ] 1.4 Ensure database layer tests pass
    - Run ONLY the 3-4 tests written in 1.1
    - Verify migration runs successfully on fresh and existing databases
    - Verify backfill produces correct counts

**Acceptance Criteria:**
- The 3-4 tests written in 1.1 pass
- Column exists in tags table after database open
- Existing edges are counted correctly during backfill
- Schema initialization remains idempotent

---

### Service Layer

#### Task Group 2: NoteService Edge Operations
**Dependencies:** Task Group 1

- [ ] 2.0 Complete service layer centrality updates
  - [ ] 2.1 Write 4-6 focused tests for edge operations
    - Test create_edge increments degree_centrality for both source and target tags
    - Test idempotent create_edge does NOT double-increment centrality
    - Test delete_edge decrements degree_centrality for both source and target tags
    - Test delete_edge on non-existent edge returns error or no-op (define behavior)
    - Test centrality never goes negative (boundary case)
    - Test transaction atomicity (edge + centrality update succeed/fail together)
  - [ ] 2.2 Modify create_edge() to increment centrality
    - Location: `src/service.rs` lines 1475-1538
    - Add UPDATE statements for both source and target tags after edge insert
    - Execute within same transaction for atomicity
    - Only increment if edge was actually inserted (check `exists` flag)
    - SQL: `UPDATE tags SET degree_centrality = degree_centrality + 1 WHERE id = ?`
  - [ ] 2.3 Add delete_edge() method to NoteService
    - New method in `src/service.rs`
    - Takes source_tag_id and target_tag_id as parameters
    - Delete edge from edges table
    - Decrement degree_centrality for both tags in same transaction
    - Handle case where edge doesn't exist (no-op or error based on design)
    - SQL: `UPDATE tags SET degree_centrality = degree_centrality - 1 WHERE id = ?`
  - [ ] 2.4 Ensure service layer tests pass
    - Run ONLY the 4-6 tests written in 2.1
    - Verify increment/decrement operations work correctly
    - Verify idempotency is preserved

**Acceptance Criteria:**
- The 4-6 tests written in 2.1 pass
- Edge creation increments centrality for both connected tags
- Edge deletion decrements centrality for both connected tags
- Idempotent edge creation does not double-count
- All operations are transactionally atomic

---

### Algorithm Layer

#### Task Group 3: Spreading Activation Centrality Boost
**Dependencies:** Task Group 1

- [ ] 3.0 Complete spreading activation centrality integration
  - [ ] 3.1 Write 3-4 focused tests for centrality boost
    - Test high-degree tag receives up to 30% boost in activation score
    - Test zero-degree tag receives no boost (multiplier = 1.0)
    - Test division by zero is handled when max_degree = 0
    - Test boost scales linearly with degree relative to max
  - [ ] 3.2 Query max degree centrality at activation start
    - Location: `src/spreading_activation.rs` spread_activation() function
    - Add query: `SELECT MAX(degree_centrality) FROM tags`
    - Execute once before main activation query
    - Store as f64 for boost calculation
    - Handle NULL result (empty database) as 0
  - [ ] 3.3 Apply centrality boost to final activation scores
    - Modify result processing in spread_activation()
    - After getting base activation scores, join with tags to get degree_centrality
    - Apply formula: `boosted_activation = activation * (1.0 + (degree_centrality / max_degree) * 0.3)`
    - Guard against division by zero when max_degree = 0 (no boost applied)
    - Update return values with boosted scores
  - [ ] 3.4 Ensure spreading activation tests pass
    - Run ONLY the 3-4 tests written in 3.1
    - Verify boost calculations are correct
    - Verify edge cases are handled

**Acceptance Criteria:**
- The 3-4 tests written in 3.1 pass
- Hub nodes (high degree) receive up to 30% activation boost
- Zero-degree nodes receive no boost
- Division by zero is handled gracefully
- Boost scales proportionally with relative degree

---

### CLI Layer

#### Task Group 4: Tags List Output Enhancement
**Dependencies:** Task Group 1

- [ ] 4.0 Complete CLI output enhancement
  - [ ] 4.1 Write 2-3 focused tests for tag listing
    - Test that tag listing includes degree centrality count
    - Test output format: `tag-name (N notes, M connections)`
    - Test tags with zero connections display correctly
  - [ ] 4.2 Create or extend method to include centrality
    - Option A: Extend `get_tags_with_notes()` return type to include centrality
    - Option B: Create new `get_tags_with_stats()` method
    - Query should return: tag_id, tag_name, note_count, degree_centrality
    - SQL pattern: JOIN note_tags for count, include degree_centrality column
    - Maintain existing order by tag name
  - [ ] 4.3 Update CLI tags list command output
    - Locate tags list command handler (likely in `src/main.rs` or CLI module)
    - Update output formatting to include connections count
    - Format: `tag-name (N notes, M connections)` or similar
    - Handle pluralization (1 note vs 2 notes, 1 connection vs 2 connections)
  - [ ] 4.4 Ensure CLI layer tests pass
    - Run ONLY the 2-3 tests written in 4.1
    - Verify output format is correct
    - Verify integration with new service method

**Acceptance Criteria:**
- The 2-3 tests written in 4.1 pass
- `cons tags list` shows degree centrality for each tag
- Output format is clear and consistent
- Zero-connection tags display correctly

---

### Testing

#### Task Group 5: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-4

- [ ] 5.0 Review existing tests and fill critical gaps only
  - [ ] 5.1 Review tests from Task Groups 1-4
    - Review the 3-4 tests written for database layer (Task 1.1)
    - Review the 4-6 tests written for service layer (Task 2.1)
    - Review the 3-4 tests written for algorithm layer (Task 3.1)
    - Review the 2-3 tests written for CLI layer (Task 4.1)
    - Total existing tests: approximately 12-17 tests
  - [ ] 5.2 Analyze test coverage gaps for degree centrality feature only
    - Identify critical end-to-end workflows lacking coverage
    - Focus ONLY on gaps related to degree centrality requirements
    - Prioritize integration between layers over unit test gaps
    - Consider: full edge lifecycle with centrality tracking
  - [ ] 5.3 Write up to 5 additional strategic tests if necessary
    - Maximum of 5 new tests to fill identified critical gaps
    - Focus on integration tests spanning multiple components
    - Example: Create note with tags, create edges, verify centrality, run search
    - Skip edge cases already covered by unit tests
  - [ ] 5.4 Run feature-specific tests only
    - Run ONLY tests related to degree centrality feature
    - Expected total: approximately 15-22 tests maximum
    - Do NOT run the entire application test suite
    - Verify all critical workflows pass

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 15-22 tests total)
- Critical user workflows for degree centrality are covered
- No more than 5 additional tests added for gap filling
- Testing focused exclusively on degree centrality feature requirements

---

## Execution Order

Recommended implementation sequence:

1. **Database Layer (Task Group 1)** - Foundation: schema migration and backfill
2. **Service Layer (Task Group 2)** - Core logic: edge operation updates
3. **Algorithm Layer (Task Group 3)** - Enhancement: spreading activation boost (can run in parallel with Group 2 after Group 1)
4. **CLI Layer (Task Group 4)** - User-facing: output enhancement (can run in parallel with Groups 2-3 after Group 1)
5. **Test Review (Task Group 5)** - Validation: gap analysis and integration tests

**Parallelization Note:** After Task Group 1 completes, Task Groups 2, 3, and 4 can be worked on in parallel as they have no dependencies on each other, only on the database layer.

---

## Technical Reference

### Key Files to Modify
- `src/db/schema.rs` - Add migration to MIGRATIONS constant
- `src/db.rs` - Add backfill query to initialize_schema()
- `src/service.rs` - Modify create_edge(), add delete_edge()
- `src/spreading_activation.rs` - Add centrality boost logic
- CLI command handler - Update tags list output

### SQL Patterns

**Migration:**
```sql
ALTER TABLE tags ADD COLUMN degree_centrality INTEGER DEFAULT 0;
```

**Backfill:**
```sql
UPDATE tags SET degree_centrality = (
    SELECT COUNT(*) FROM edges
    WHERE source_tag_id = tags.id OR target_tag_id = tags.id
);
```

**Centrality Boost Formula:**
```rust
let boost = if max_degree > 0 {
    1.0 + (degree_centrality as f64 / max_degree as f64) * 0.3
} else {
    1.0
};
let boosted_activation = activation * boost;
```

### Out of Scope Reminders
- TUI visualization (roadmap items #30-31)
- In-degree vs out-degree separation
- Periodic batch recomputation
- Configurable boost multiplier
- Centrality-based sorting in output
