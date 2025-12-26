# Task Breakdown: Note Text Enhancement

## Overview

**Total Tasks:** 22 (across 4 task groups)

This feature adds automatic LLM-based text enhancement to note capture, expanding fragmentary notes into complete thoughts while preserving the original content with provenance metadata and confidence scores.

## Architecture Context

```
CLI (clap) ─────┐
                ├──> NoteService ──> SQLite
TUI (ratatui) ──┘         │
                          └──> OllamaClient ──> Ollama
                          └──> NoteEnhancer ──> Ollama (NEW)
```

## Execution Order

The recommended implementation sequence accounts for dependencies:

1. **Database Layer** (Task Group 1) - Foundation: schema changes must come first
2. **Note Model & Service** (Task Group 2) - Depends on schema; adds enhancement fields to models
3. **NoteEnhancer Module** (Task Group 3) - Can be developed in parallel with Task Group 2 after schema is complete
4. **CLI Integration** (Task Group 4) - Depends on both NoteService and NoteEnhancer being complete

---

## Task List

### Database Layer

#### Task Group 1: Schema Extension for Enhancement Fields
**Dependencies:** None

- [x] 1.0 Complete database schema extension
  - [x] 1.1 Write 4 focused tests for schema changes
    - Test idempotent ALTER TABLE for `content_enhanced` column
    - Test idempotent ALTER TABLE for `enhanced_at` column
    - Test idempotent ALTER TABLE for `enhancement_model` column
    - Test idempotent ALTER TABLE for `enhancement_confidence` column
  - [x] 1.2 Add idempotent schema migration to `src/db/schema.rs`
    - Add `content_enhanced TEXT` column (nullable)
    - Add `enhanced_at INTEGER` column (nullable)
    - Add `enhancement_model TEXT` column (nullable)
    - Add `enhancement_confidence REAL` column (nullable)
    - Follow existing IF NOT EXISTS pattern for safe re-execution
    - Reference: Existing `INITIAL_SCHEMA` constant pattern in `src/db/schema.rs`
  - [x] 1.3 Verify schema runs on fresh and existing databases
    - Test fresh database creation includes new columns
    - Test existing database migration adds new columns safely
    - Verify column nullability allows NULL values
  - [x] 1.4 Ensure database schema tests pass
    - Run ONLY the 4 tests written in 1.1
    - Verify migrations run successfully on both fresh and existing DBs

**Acceptance Criteria:**
- All 4 schema tests pass
- Schema changes are idempotent (re-runnable without error)
- New columns are nullable (allow NULL when enhancement unavailable)
- Existing `notes` table data is preserved during migration

**Files to Modify:**
- `/home/md/construct-app/src/db/schema.rs`

---

### Note Model & Service Layer

#### Task Group 2: Note Model and NoteService Updates
**Dependencies:** Task Group 1 (schema must exist first)

- [x] 2.0 Complete Note model and NoteService updates
  - [x] 2.1 Write 6 focused tests for model and service changes
    - Test Note struct includes optional enhancement fields
    - Test NoteBuilder supports setting enhancement fields
    - Test Note accessors return correct values for enhancement fields
    - Test NoteService stores enhancement data on note creation
    - Test NoteService retrieves enhancement data from database
    - Test update_note_enhancement method updates existing note
  - [x] 2.2 Extend Note struct in `src/models/note.rs`
    - Add `content_enhanced: Option<String>` field
    - Add `enhanced_at: Option<OffsetDateTime>` field
    - Add `enhancement_model: Option<String>` field
    - Add `enhancement_confidence: Option<f64>` field
    - Update serde serialization to handle optional fields (skip_serializing_if for None values)
  - [x] 2.3 Extend NoteBuilder with enhancement field methods
    - Add `content_enhanced(impl Into<String>)` method
    - Add `enhanced_at(OffsetDateTime)` method
    - Add `enhancement_model(impl Into<String>)` method
    - Add `enhancement_confidence(f64)` method
    - Ensure `build()` uses `None` defaults for enhancement fields
  - [x] 2.4 Add Note accessor methods
    - Add `content_enhanced() -> Option<&str>` method
    - Add `enhanced_at() -> Option<OffsetDateTime>` method
    - Add `enhancement_model() -> Option<&str>` method
    - Add `enhancement_confidence() -> Option<f64>` method
  - [x] 2.5 Update NoteService to persist enhancement data
    - Modify `get_note` to SELECT enhancement columns
    - Add `update_note_enhancement` method for updating enhancement after save
    - Note: `create_note` doesn't need modification - it creates notes without enhancement by default
  - [x] 2.6 Ensure model and service tests pass
    - Run ONLY the 6 tests written in 2.1
    - Verify enhancement data roundtrips through database correctly

**Acceptance Criteria:**
- All 6 model/service tests pass
- Note struct has all four optional enhancement fields
- NoteBuilder can construct notes with enhancement data
- NoteService correctly persists and retrieves enhancement data
- Enhancement fields default to None when not provided

**Files to Modify:**
- `/home/md/construct-app/src/models/note.rs`
- `/home/md/construct-app/src/service.rs`

---

### NoteEnhancer Module

#### Task Group 3: NoteEnhancer LLM Integration
**Dependencies:** Task Group 1 (needs OllamaClient patterns, not schema)

- [x] 3.0 Complete NoteEnhancer module
  - [x] 3.1 Write 6 focused tests for NoteEnhancer
    - Test NoteEnhancerBuilder constructs NoteEnhancer with client
    - Test enhance_content returns EnhancementResult with content and confidence
    - Test JSON response parsing extracts enhanced_content and confidence
    - Test extract_json handles markdown code blocks and preamble
    - Test confidence clamping to 0.0-1.0 range
    - Test fail-safe behavior returns None on parse failure
  - [x] 3.2 Create `src/enhancer.rs` module file
    - Create module structure following `src/autotagger/tagger.rs` pattern
    - Add module to `src/lib.rs` exports
  - [x] 3.3 Define EnhancementResult struct
    - `enhanced_content: String` - the expanded note content
    - `confidence: f64` - model confidence score (0.0-1.0)
    - Implement Debug, Clone, PartialEq for testing
  - [x] 3.4 Implement NoteEnhancerBuilder
    - Follow `AutoTaggerBuilder` pattern from `src/autotagger/tagger.rs`
    - Accept `Arc<dyn OllamaClientTrait>` via `client()` method
    - Implement `build()` returning `NoteEnhancer`
  - [x] 3.5 Implement NoteEnhancer struct
    - Store `client: Arc<dyn OllamaClientTrait>`
    - Implement `enhance_content(&self, model: &str, content: &str) -> Result<EnhancementResult, OllamaError>`
    - Construct prompt using PROMPT_TEMPLATE constant
    - Parse JSON response with `extract_json` helper (copy from autotagger)
  - [x] 3.6 Design enhancement prompt template
    - Instruct model to expand abbreviations and complete fragments
    - Instruct model to clarify implicit context
    - Instruct model to preserve original intent (no new information)
    - Instruct model to return JSON: `{"enhanced_content": "...", "confidence": 0.0-1.0}`
    - Include few-shot examples for edge cases (short notes, code blocks, URLs)
  - [x] 3.7 Implement JSON parsing with fail-safe behavior
    - Copy `extract_json` helper from `src/autotagger/tagger.rs`
    - Parse `enhanced_content` and `confidence` fields
    - Clamp confidence to 0.0-1.0 range
    - Return error (not panic) on parse failure for caller to handle
  - [x] 3.8 Ensure NoteEnhancer tests pass
    - Run ONLY the 6 tests written in 3.1
    - Verify enhancement workflow with mock OllamaClient

**Acceptance Criteria:**
- All 6 NoteEnhancer tests pass
- NoteEnhancerBuilder follows AutoTagger pattern
- EnhancementResult contains enhanced content and confidence
- JSON parsing handles various LLM output formats
- Fail-safe: parse errors return error (caller decides how to handle)

**Files to Create:**
- `/home/md/construct-app/src/enhancer.rs`

**Files to Modify:**
- `/home/md/construct-app/src/lib.rs` (add enhancer module export)

**Reference Files:**
- `/home/md/construct-app/src/autotagger/tagger.rs` (pattern to follow)
- `/home/md/construct-app/src/ollama/client.rs` (OllamaClientTrait interface)

---

### CLI Integration

#### Task Group 4: CLI Integration and Display
**Dependencies:** Task Groups 2 and 3 (both must be complete)

- [x] 4.0 Complete CLI integration
  - [x] 4.1 Write 6 focused tests for CLI integration
    - Test execute_add calls enhancement after note save
    - Test enhancement failure does not block note capture (fail-safe)
    - Test enhancement runs AFTER save but BEFORE tagging (order matters)
    - Test list command displays both original and enhanced content
    - Test show command displays stacked format with separator
    - Test confidence percentage display format (e.g., "85% confidence")
  - [x] 4.2 Integrate NoteEnhancer into execute_add flow
    - Create NoteEnhancer with shared OllamaClient
    - Call enhancement AFTER note is saved (fail-safe: original preserved)
    - Call enhancement BEFORE auto-tagging (tag original content, not enhanced)
    - Update note with enhancement result via `update_note_enhancement`
    - Reference: Existing `auto_tag_note` pattern in `src/main.rs`
  - [x] 4.3 Add fail-safe error handling for enhancement
    - Catch enhancement errors (LLM unavailable, parse failure)
    - Log error but continue (note capture succeeds)
    - Leave enhancement fields as NULL when enhancement fails
    - Follow existing auto-tag error handling pattern
  - [x] 4.4 Update execute_list display format
    - Display original content first
    - Add `---` separator when enhancement exists
    - Display enhanced content below separator
    - Show confidence as percentage: `(enhanced: 85% confidence)`
    - Skip separator and enhanced section when content_enhanced is NULL
  - [x] 4.5 Create helper function for stacked display format
    - Accept Note and return formatted string
    - Handle both enhanced and non-enhanced notes
    - Format confidence as integer percentage
  - [x] 4.6 Ensure CLI integration tests pass
    - Run ONLY the 6 tests written in 4.1
    - Verify end-to-end workflow: capture -> enhance -> tag -> display

**Acceptance Criteria:**
- All 6 CLI integration tests pass
- Enhancement runs automatically on `cons add`
- Enhancement failures never block note capture
- Enhancement runs AFTER save, BEFORE tagging
- List output shows stacked format with confidence percentage
- Original content always visible; enhanced shown when available

**Files to Modify:**
- `/home/md/construct-app/src/main.rs`

---

## Verification Checkpoints

### Checkpoint 1: After Task Group 1
- [x] Schema migration runs without error on fresh database
- [x] Schema migration runs without error on existing database with notes
- [x] All 4 schema tests pass: `cargo test schema`

### Checkpoint 2: After Task Group 2
- [x] Note model compiles with new enhancement fields
- [x] NoteService CRUD operations work with enhancement data
- [x] All 6 model/service tests pass: `cargo test note` and `cargo test service`

### Checkpoint 3: After Task Group 3
- [x] NoteEnhancer module compiles and is exported from lib.rs
- [x] Enhancement prompt produces valid JSON from mock responses
- [x] All 6 enhancer tests pass: `cargo test enhancer`

### Checkpoint 4: After Task Group 4
- [x] `cons add "quick thought"` creates note and attempts enhancement
- [x] `cons list` shows enhanced content with confidence when available
- [x] Enhancement failures are logged but don't block capture
- [x] All 6 CLI tests pass: `cargo test main`

### Final Verification
- [x] Full test suite passes: `cargo test`
- [x] Clippy passes: `cargo clippy`
- [x] Format check passes: `cargo fmt --check`

---

## Risk Mitigation

### Fail-Safe Guarantees
1. **Note capture never blocked**: Enhancement errors caught and logged
2. **Original content always preserved**: Enhancement stored in separate field
3. **Tagging uses original**: User intent preserved for tag extraction
4. **No retry complexity**: Single attempt, NULL on failure

### Rollback Strategy
- Schema changes are additive only (new nullable columns)
- Existing functionality unaffected if enhancement is NULL
- Feature can be disabled by skipping enhancement call in execute_add

---

## Testing Summary

| Task Group | Test Count | Focus Area |
|------------|------------|------------|
| 1 - Schema | 4 tests | Idempotent migration, column nullability |
| 2 - Models | 6 tests | Note struct, NoteBuilder, NoteService persistence |
| 3 - Enhancer | 6 tests | LLM integration, JSON parsing, fail-safe |
| 4 - CLI | 6 tests | Workflow integration, display format |
| **Total** | **22 tests** | Feature-specific coverage |
