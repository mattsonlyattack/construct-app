# Task Breakdown: Tag Hierarchy Population

## Overview
Total Tasks: 4 task groups with 22 sub-tasks

This feature implements a CLI command (`cons hierarchy suggest`) that uses LLM to analyze existing tags and automatically populate the edges table with broader/narrower relationships, distinguishing between generic (is-a) and partitive (part-of) hierarchy types using XKOS semantics.

## Task List

### Core Module Layer

#### Task Group 1: HierarchySuggester Module
**Dependencies:** None

- [x] 1.0 Complete HierarchySuggester module
  - [x] 1.1 Write 4-6 focused tests for HierarchySuggester
    - Test PROMPT_TEMPLATE includes XKOS semantics explanation
    - Test `suggest_relationships()` returns parsed relationship suggestions
    - Test `extract_json()` handles markdown-wrapped responses
    - Test `parse_suggestions()` filters by confidence >= 0.7
    - Test fail-safe behavior returns empty Vec on parse failure
    - Test confidence clamping to 0.0-1.0 range
  - [x] 1.2 Create `src/hierarchy/mod.rs` module
    - Re-export HierarchySuggester and HierarchySuggesterBuilder
    - Follow pattern from `src/autotagger/mod.rs`
  - [x] 1.3 Create `src/hierarchy/suggester.rs` with PROMPT_TEMPLATE
    - Include XKOS semantics explanation: generic (is-a) vs partitive (part-of)
    - Include few-shot examples demonstrating both hierarchy types:
      - generic: "transformer" specializes "neural-network"
      - partitive: "attention" isPartOf "transformer"
    - Input format: JSON array of tag names
    - Output format: JSON array of relationship objects with source_tag, target_tag, hierarchy_type, confidence
    - Follow pattern from `src/autotagger/tagger.rs` PROMPT_TEMPLATE
  - [x] 1.4 Implement HierarchySuggesterBuilder
    - Accept `Arc<dyn OllamaClientTrait>` for testability
    - Follow `AutoTaggerBuilder` pattern exactly
  - [x] 1.5 Implement HierarchySuggester struct with `suggest_relationships()` method
    - Accept model name and Vec of tag names
    - Call OllamaClient.generate() with constructed prompt
    - Parse response using extract_json and parse_suggestions helpers
    - Return `Result<Vec<RelationshipSuggestion>, OllamaError>`
  - [x] 1.6 Implement extract_json and parse_suggestions helpers
    - `extract_json()`: Extract JSON array from model response (handle markdown, preamble)
    - `parse_suggestions()`: Parse JSON to Vec<RelationshipSuggestion>, clamp confidence, filter < 0.7
    - Follow fail-safe pattern: return empty Vec on parse failure
  - [x] 1.7 Define RelationshipSuggestion struct
    - Fields: source_tag (String), target_tag (String), hierarchy_type (String), confidence (f64)
    - Directional convention: source = narrower/child, target = broader/parent
  - [x] 1.8 Ensure HierarchySuggester tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Verify all core behaviors work correctly

**Acceptance Criteria:**
- The 4-6 tests written in 1.1 pass
- HierarchySuggester correctly constructs prompts with XKOS semantics
- JSON extraction handles various LLM response formats
- Suggestions with confidence < 0.7 are filtered out
- Fail-safe: parse failures return empty Vec (no panics)

### Database Layer

#### Task Group 2: Edge Creation in NoteService
**Dependencies:** Task Group 1

- [x] 2.0 Complete edge creation in NoteService
  - [x] 2.1 Write 4-6 focused tests for edge creation
    - Test `get_tags_with_notes()` returns only tags with associated notes
    - Test `create_edge()` inserts edge with correct metadata
    - Test `create_edge()` respects INSERT OR IGNORE for duplicates
    - Test edge stores correct hierarchy_type ('generic' or 'partitive')
    - Test transaction atomicity for bulk edge creation
    - Test empty tag set returns empty Vec (no LLM call needed)
  - [x] 2.2 Add `get_tags_with_notes()` method to NoteService
    - Query tags that have at least one associated note using JOIN with note_tags
    - Return Vec<(TagId, String)> with tag ID and name
    - Follow existing NoteService query patterns
  - [x] 2.3 Add `create_edge()` method to NoteService
    - Parameters: source_tag_id, target_tag_id, confidence, hierarchy_type, model_version
    - Populate all edge fields: source='llm', verified=0, created_at=now, valid_from/valid_until=NULL
    - Use explicit duplicate check for idempotent operation
    - Validate both tag IDs exist before insertion
  - [x] 2.4 Add `create_edges_batch()` method for atomic bulk insertion
    - Wrap multiple create_edge calls in transaction
    - Follow create_note transaction pattern: BEGIN/COMMIT/ROLLBACK
    - Return count of edges created (for CLI output)
  - [x] 2.5 Ensure edge creation tests pass
    - Run ONLY the 4-6 tests written in 2.1
    - Verify edge metadata is correctly populated
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 4-6 tests written in 2.1 pass
- Tags without notes are excluded from hierarchy analysis
- Edges are created with correct XKOS hierarchy_type values
- Duplicate edges are handled gracefully (no errors)
- Transactions ensure atomicity for bulk operations

### CLI Layer

#### Task Group 3: Hierarchy CLI Command
**Dependencies:** Task Groups 1-2

- [x] 3.0 Complete hierarchy CLI command
  - [x] 3.1 Write 4-6 focused tests for CLI command
    - Test clap parsing of `cons hierarchy suggest`
    - Test `execute_hierarchy_suggest()` with in-memory database
    - Test output format: "Analyzed X tags, found Y relationships"
    - Test graceful handling when OLLAMA_MODEL not set
    - Test empty tag set displays message without LLM call
    - Test fail-safe: LLM errors don't crash command (exit code 0)
  - [x] 3.2 Add HierarchyCommand and HierarchyCommands enum to main.rs
    - Follow TagAliasCommand pattern exactly
    - Add `Hierarchy(HierarchyCommand)` variant to Commands enum
    - Add `Suggest` variant to HierarchyCommands enum
  - [x] 3.3 Add handle_hierarchy dispatch function
    - Follow handle_tag_alias pattern for database path resolution
    - Dispatch to execute_hierarchy_suggest for Suggest variant
  - [x] 3.4 Implement execute_hierarchy_suggest function
    - Read OLLAMA_MODEL env var (fail with clear message if not set)
    - Call get_tags_with_notes() (return early with message if empty)
    - Build OllamaClient and HierarchySuggester
    - Call suggest_relationships() with tag names
    - Call create_edges_batch() for auto-accepted suggestions
    - Display summary output (analyzed, found, discarded counts)
  - [x] 3.5 Implement CLI output formatting
    - "Analyzed X tags, found Y relationships"
    - List each relationship: "transformer -> neural-network (generic, 0.85 confidence)"
    - Report discarded count: "Discarded Z low-confidence suggestions"
    - On LLM failure: display error message, exit 0 (fail-safe)
  - [x] 3.6 Ensure CLI tests pass
    - Run ONLY the 4-6 tests written in 3.1
    - Verify command parsing works correctly
    - Verify fail-safe error handling

**Acceptance Criteria:**
- The 4-6 tests written in 3.1 pass
- `cons hierarchy suggest` command is recognized by clap
- Command handles missing OLLAMA_MODEL with clear error
- Empty tag sets are handled gracefully without LLM call
- LLM failures logged but don't crash (fail-safe, exit code 0)
- Output format is clear and informative

### Testing

#### Task Group 4: Test Review & Gap Analysis
**Dependencies:** Task Groups 1-3

- [x] 4.0 Review existing tests and fill critical gaps only
  - [x] 4.1 Review tests from Task Groups 1-3
    - Reviewed 11 tests from HierarchySuggester (Task 1.1)
    - Reviewed 8 tests for edge creation (Task 2.1)
    - Reviewed 5 tests for CLI command (Task 3.1)
    - Total existing tests: 24 tests (more than expected due to thorough coverage)
  - [x] 4.2 Analyze test coverage gaps for THIS feature only
    - Identified critical integration workflows that lack coverage
    - Focused ONLY on gaps related to hierarchy population feature
    - Prioritized end-to-end workflow over unit test gaps
  - [x] 4.3 Write up to 6 additional strategic tests maximum
    - Integration test: full workflow from tags to edges (`hierarchy_population_full_end_to_end_workflow`)
    - Test edge direction convention (`edge_direction_convention_narrower_to_broader`)
    - Test idempotency: running suggest twice doesn't duplicate edges (`hierarchy_suggest_idempotency_no_duplicate_edges`)
    - Test mixed hierarchy types in single batch (`mixed_hierarchy_types_in_single_batch`)
    - Test tag name resolution before edge creation (`tag_name_resolution_before_edge_creation`)
    - Test transaction rollback on failure (`create_edges_batch_rollback_on_failure`)
  - [x] 4.4 Run feature-specific tests only
    - Ran ONLY tests related to hierarchy population feature
    - Total: 30 hierarchy-specific tests (24 existing + 6 new)
    - All critical workflows pass successfully
    - No warnings or errors

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 18-24 tests total)
- Critical integration workflows for hierarchy population are covered
- No more than 6 additional tests added when filling in testing gaps
- Testing focused exclusively on hierarchy population feature

## Execution Order

Recommended implementation sequence:
1. **Core Module Layer** (Task Group 1) - HierarchySuggester with LLM integration
2. **Database Layer** (Task Group 2) - Edge creation methods in NoteService
3. **CLI Layer** (Task Group 3) - CLI command and dispatch
4. **Testing** (Task Group 4) - Test review and gap analysis

## Key Implementation Notes

### File Locations
- `src/hierarchy/mod.rs` - Module re-exports
- `src/hierarchy/suggester.rs` - HierarchySuggester, PROMPT_TEMPLATE, parsing logic
- `src/service.rs` - Add get_tags_with_notes(), create_edge(), create_edges_batch()
- `src/main.rs` - Add HierarchyCommand, HierarchyCommands, handle_hierarchy, execute_hierarchy_suggest
- `src/lib.rs` - Add `pub mod hierarchy;` and re-exports

### Patterns to Follow
- **AutoTaggerBuilder** (`src/autotagger/tagger.rs`) - Builder pattern with Arc-wrapped client
- **TagAliasCommand** (`src/main.rs`) - Nested subcommand structure
- **create_note transaction** (`src/service.rs`) - BEGIN/COMMIT/ROLLBACK pattern
- **auto_tag_note fail-safe** (`src/main.rs`) - Errors logged, don't block execution

### XKOS Hierarchy Types
- `generic` (is-a): Specialization relationship. "transformer" specializes "neural-network"
- `partitive` (part-of): Composition relationship. "attention" isPartOf "transformer"

### Edge Direction Convention
- `source_tag_id` = narrower/child concept (more specific)
- `target_tag_id` = broader/parent concept (more general)
- Edges point "up" the hierarchy

### Confidence Thresholds
- >= 0.7: Auto-accept, create edge immediately
- < 0.7: Discard, do not store or display
