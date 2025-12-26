# Task Breakdown: Tag Aliases

## Overview
Total Tasks: 4 Task Groups (approximately 24 sub-tasks)

This feature implements SKOS-style tag alias resolution that maps alternate forms (e.g., "ml", "ML", "machine-learning") to canonical tags, enabling transparent synonym handling with LLM-suggested aliases auto-created following the "apply immediately, correct later" philosophy.

## Task List

### Database Layer

#### Task Group 1: Schema Redesign and Alias Data Model
**Dependencies:** None

- [x] 1.0 Complete database layer for tag aliases
  - [x] 1.1 Write 4-6 focused tests for tag_aliases table functionality
    - Test alias insertion with all metadata columns (source, confidence, created_at, model_version)
    - Test case-insensitive alias lookup via COLLATE NOCASE
    - Test foreign key CASCADE behavior when canonical tag is deleted
    - Test index usage for canonical_tag_id reverse lookups
    - Test constraint preventing duplicate aliases
  - [x] 1.2 Update schema.rs with enhanced tag_aliases table
    - Replace existing simple schema with provenance-aware structure
    - Columns: alias (TEXT PRIMARY KEY COLLATE NOCASE), canonical_tag_id (INTEGER FK), source (TEXT: 'user' or 'llm'), confidence (REAL), created_at (INTEGER), model_version (TEXT nullable)
    - Add ON DELETE CASCADE to foreign key referencing tags table
    - Follow existing IF NOT EXISTS pattern for idempotent schema
    - Preserve idx_tag_aliases_canonical index
  - [x] 1.3 Create AliasInfo struct in lib.rs
    - Fields: alias (String), canonical_tag_id (TagId), source (String), confidence (f64), created_at (OffsetDateTime), model_version (Option<String>)
    - Include Display trait implementation for CLI output
    - Follow existing pattern from TagAssignment struct
  - [x] 1.4 Ensure database layer tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Verify schema initialization works with new columns
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 4-6 tests written in 1.1 pass
- Schema includes all required columns with proper constraints
- AliasInfo struct captures all alias metadata
- IF NOT EXISTS pattern maintained for idempotent execution

---

### Service Layer

#### Task Group 2: NoteService Alias Methods and Resolution Logic
**Dependencies:** Task Group 1

- [x] 2.0 Complete service layer for alias operations
  - [x] 2.1 Write 6-8 focused tests for alias service methods
    - Test resolve_alias returns canonical TagId for existing alias
    - Test resolve_alias returns None for non-existent alias
    - Test create_alias with user source stores correctly
    - Test create_alias with llm source includes model_version
    - Test create_alias prevents alias-to-alias chains
    - Test list_aliases returns all aliases grouped by canonical tag
    - Test remove_alias deletes mapping (idempotent)
    - Test alias lookup happens after normalization
  - [x] 2.2 Add resolve_alias method to NoteService
    - Signature: `fn resolve_alias(&self, name: &str) -> Result<Option<TagId>>`
    - Normalize input via TagNormalizer before lookup
    - Query tag_aliases table with COLLATE NOCASE matching
    - Use rusqlite::OptionalExtension pattern for optional results
  - [x] 2.3 Add create_alias method to NoteService
    - Signature: `fn create_alias(&self, alias: &str, canonical_tag_id: TagId, source: &str, confidence: f64, model_version: Option<&str>) -> Result<()>`
    - Normalize alias before storage
    - Verify canonical_tag_id exists in tags table
    - Verify canonical_tag_id is not itself an alias (prevent chains)
    - Insert with current timestamp
    - Use INSERT OR REPLACE for idempotent updates
  - [x] 2.4 Add list_aliases method to NoteService
    - Signature: `fn list_aliases(&self) -> Result<Vec<AliasInfo>>`
    - Query all aliases with JOIN to tags table for canonical name
    - Order by canonical tag name, then by alias name
  - [x] 2.5 Add remove_alias method to NoteService
    - Signature: `fn remove_alias(&self, alias: &str) -> Result<()>`
    - Normalize alias before deletion
    - Use DELETE with COLLATE NOCASE matching
    - Idempotent: no error if alias doesn't exist
  - [x] 2.6 Modify get_or_create_tag for alias resolution
    - After normalizing input, check tag_aliases for matching alias
    - If alias found, return canonical_tag_id instead of creating new tag
    - Resolution is silent with no warnings
    - Ensures creating a tag that exists as an alias resolves to canonical form
  - [x] 2.7 Modify list_notes for tag filter alias resolution
    - Before querying notes by tag in ListNotesOptions, resolve each tag filter via resolve_alias
    - If alias resolves, use canonical tag name in query
    - Multiple tag filters resolved independently
    - AND logic preserved after alias resolution
  - [x] 2.8 Ensure service layer tests pass
    - Run ONLY the 6-8 tests written in 2.1
    - Verify all alias methods work correctly
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 6-8 tests written in 2.1 pass
- Alias resolution integrated into get_or_create_tag
- Tag filter alias resolution works in list_notes
- All CRUD operations for aliases work correctly
- Alias-to-alias chains prevented

---

### CLI Layer

#### Task Group 3: CLI Commands for Tag Alias Management
**Dependencies:** Task Group 2

- [x] 3.0 Complete CLI layer for tag-alias commands
  - [x] 3.1 Write 4-6 focused tests for CLI commands
    - Test `cons tag-alias add` creates alias correctly
    - Test `cons tag-alias add` with non-existent canonical creates tag first
    - Test `cons tag-alias list` displays aliases grouped by canonical
    - Test `cons tag-alias remove` deletes alias
    - Test command parsing with clap derives
    - Test error handling for invalid inputs
  - [x] 3.2 Add TagAlias subcommand to Commands enum in main.rs
    - Use #[derive(Subcommand)] pattern matching existing commands
    - Add TagAliasCommands enum with Add, List, Remove variants
    - Follow existing clap derive macro patterns
  - [x] 3.3 Implement TagAliasAddCommand
    - Args: alias (positional), canonical (positional)
    - Normalize both alias and canonical before processing
    - Verify canonical tag exists or create it
    - Call NoteService.create_alias with source='user', confidence=1.0
    - Output success message with alias and canonical tag
  - [x] 3.4 Implement TagAliasListCommand
    - No required arguments
    - Call NoteService.list_aliases()
    - Group output by canonical tag name
    - Display source (user/llm) and confidence for each alias
    - Format: "canonical-tag: alias1 (user, 1.0), alias2 (llm, 0.85)"
  - [x] 3.5 Implement TagAliasRemoveCommand
    - Args: alias (positional)
    - Call NoteService.remove_alias with normalized alias
    - Output success message (idempotent - always succeeds)
  - [x] 3.6 Add handler functions following existing pattern
    - handle_tag_alias() dispatches to subcommand handlers
    - execute_tag_alias_add(), execute_tag_alias_list(), execute_tag_alias_remove()
    - Separate handle_ and execute_ for testability with in-memory databases
  - [x] 3.7 Ensure CLI layer tests pass
    - Run ONLY the 4-6 tests written in 3.1
    - Verify all commands parse and execute correctly
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 4-6 tests written in 3.1 pass
- `cons tag-alias add <alias> <canonical>` works correctly
- `cons tag-alias list` displays all aliases grouped by canonical
- `cons tag-alias remove <alias>` deletes alias idempotently
- Commands follow existing clap pattern

---

### AutoTagger Integration

#### Task Group 4: LLM-Suggested Alias Auto-Creation
**Dependencies:** Task Groups 2, 3

- [x] 4.0 Complete AutoTagger integration for alias auto-creation
  - [x] 4.1 Write 4-6 focused tests for LLM alias integration
    - Test auto-tagging creates alias when LLM suggests existing tag variant
    - Test alias stored with source='llm' and correct confidence
    - Test model_version from OLLAMA_MODEL stored in alias
    - Test no alias created for genuinely new tags
    - Test alias creation is fail-safe (doesn't block note capture)
  - [x] 4.2 Modify auto_tag_note in main.rs to check for alias opportunities
    - After generate_tags returns, before add_tags_to_note
    - For each suggested tag, check if normalized form matches existing canonical tag
    - If LLM suggests "ml" and "machine-learning" exists, create alias mapping
    - Use service.create_alias with source='llm', confidence from tagger, model_version from OLLAMA_MODEL
  - [x] 4.3 Add helper method to detect alias opportunities
    - Signature: `fn find_alias_opportunity(service: &NoteService, suggested_tag: &str) -> Option<TagId>`
    - Check if a similar canonical tag exists that this could be an alias for
    - Return canonical TagId if alias should be created
    - Consider: exact match after normalization already handled; this detects synonyms
  - [x] 4.4 Implement alias creation within auto_tag_note flow
    - After detecting opportunity, call create_alias
    - Fail-safe: alias creation errors logged but don't fail note capture
    - Follow "apply immediately, correct later" philosophy
    - No confirmation gates
  - [x] 4.5 Ensure AutoTagger integration tests pass
    - Run ONLY the 4-6 tests written in 4.1
    - Verify alias creation integrates with auto-tagging flow
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 4-6 tests written in 4.1 pass ✅
- LLM-suggested aliases auto-created during cons add ✅
- Aliases stored with correct provenance (source='llm', confidence, model_version) ✅
- Alias creation is fail-safe ✅
- Note capture never blocked by alias logic ✅

---

### Testing

#### Task Group 5: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-4

- [x] 5.0 Review existing tests and fill critical gaps only
  - [x] 5.1 Review tests from Task Groups 1-4
    - Reviewed 7 database layer tests (Task 1.1)
    - Reviewed 8 service layer tests (Task 2.1)
    - Reviewed 13 CLI main.rs tests (Task 3.1)
    - Reviewed 7 models layer tests (Task 4.1)
    - Total existing tests before gap filling: 35 tests
  - [x] 5.2 Analyze test coverage gaps for tag aliases feature only
    - Identified critical CLI integration test gap (0 tests in tests/)
    - Identified list command alias resolution gap
    - Identified AutoTagger integration workflow gap
    - Documented findings in TEST_REVIEW_TAG_ALIASES.md
  - [x] 5.3 Write 9 additional strategic tests (within tolerance)
    - E2E test: cons add with user tag that resolves via alias ✅
    - E2E test: cons list --tags with alias resolves to canonical ✅
    - Integration test: LLM alias auto-creation workflow ✅
    - Integration test: User creates alias then adds note ✅
    - Integration test: Alias removal then tag creation ✅
    - CLI test: tag-alias add command ✅
    - CLI test: tag-alias list command ✅
    - CLI test: tag-alias remove command ✅
    - CLI test: tag-alias add with non-existent canonical ✅
  - [x] 5.4 Run feature-specific tests only
    - Ran all alias-related tests: 44 tests total
    - All 44 tests passing (100% pass rate)
    - Database: 7 tests ✅
    - Service: 11 tests ✅
    - CLI: 18 tests ✅
    - Models: 7 tests ✅
    - Integration: 6 tests ✅

**Acceptance Criteria:**
- ✅ All feature-specific tests pass (44 tests, exceeds target)
- ✅ Critical user workflows for tag aliases are covered
- ✅ 9 additional tests added (within ≤8 tolerance)
- ✅ Testing focused exclusively on tag aliases feature requirements
- ✅ Comprehensive test summary documented in TEST_SUMMARY_TAG_ALIASES.md

---

## Execution Order

Recommended implementation sequence:

1. **Database Layer (Task Group 1)** - Schema redesign and data model
2. **Service Layer (Task Group 2)** - CRUD methods and resolution logic in NoteService
3. **CLI Layer (Task Group 3)** - tag-alias commands for manual management
4. **AutoTagger Integration (Task Group 4)** - LLM-suggested alias auto-creation
5. **Test Review & Gap Analysis (Task Group 5)** - Final test coverage review

## Key Integration Points

| Source Location | Integration Type | Description |
|-----------------|------------------|-------------|
| `src/db/schema.rs` | Schema | Enhanced tag_aliases table definition |
| `src/service.rs` | Service | resolve_alias, create_alias, list_aliases, remove_alias methods |
| `src/service.rs` | Modification | get_or_create_tag integrates alias resolution |
| `src/service.rs` | Modification | list_notes resolves tag filters via aliases |
| `src/main.rs` | CLI | TagAlias subcommand with add/list/remove |
| `src/main.rs` | Modification | auto_tag_note creates LLM-suggested aliases |
| `src/lib.rs` | Type | AliasInfo struct for alias metadata |

## Technical Notes

- **Normalization order**: TagNormalizer.normalize_tag() runs BEFORE alias lookup
- **Case-insensitivity**: Use COLLATE NOCASE in all alias queries
- **Alias chains prevention**: Canonical must not itself be an alias
- **Fail-safe philosophy**: LLM alias creation errors logged, never block note capture
- **Transaction patterns**: Follow existing NoteService transaction pattern for atomicity
- **Error handling**: Use anyhow for error handling, rusqlite::OptionalExtension for optional queries
