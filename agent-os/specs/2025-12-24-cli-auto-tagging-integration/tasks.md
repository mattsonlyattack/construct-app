# Task Breakdown: CLI Auto-Tagging Integration

## Overview
Total Tasks: 4 task groups, 20 sub-tasks

## Task List

### CLI Layer

#### Task Group 1: Async Runtime Conversion
**Dependencies:** None

- [x] 1.0 Complete async runtime conversion
  - [x] 1.1 Write 2-4 focused tests for async main() and handle_add()
    - Test that main() can be called with async runtime
    - Test that handle_add() returns Result<()> from async context
    - Test that command dispatch uses .await correctly
    - Limit to 2-4 tests covering critical async behavior
  - [x] 1.2 Convert main() to async with #[tokio::main]
    - Change signature from `fn main()` to `#[tokio::main] async fn main()`
    - Update command dispatch to use `.await` for async calls
    - Verify tokio runtime features are enabled in Cargo.toml (already present)
  - [x] 1.3 Convert handle_add() to async function
    - Change signature from `fn handle_add()` to `async fn handle_add()`
    - Update all internal calls to use `.await` where needed
    - Keep existing validation and error handling logic
  - [x] 1.4 Update handle_list() to async (for consistency)
    - Change signature to `async fn handle_list()`
    - Update command dispatch to use `.await`
    - Ensure list command still works correctly
  - [x] 1.5 Ensure async conversion tests pass
    - Run ONLY the 2-4 tests written in 1.1
    - Verify main() and handle_add() work with async runtime
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 2-4 tests written in 1.1 pass
- main() successfully uses tokio runtime
- handle_add() and handle_list() are async functions
- Command dispatch uses .await correctly
- Existing functionality preserved (no regressions)

### Ollama Client Enhancement

#### Task Group 2: OLLAMA_MODEL Environment Variable Support
**Dependencies:** Task Group 1

- [x] 2.0 Complete OLLAMA_MODEL environment variable support
  - [x] 2.1 Write 2-4 focused tests for OLLAMA_MODEL env var handling
    - Test that builder reads OLLAMA_MODEL env var when not set via builder
    - Test that builder value takes precedence over env var
    - Test default fallback behavior when env var not set
    - Limit to 2-4 tests covering critical env var behavior
  - [x] 2.2 Add model field to OllamaClientBuilder struct
    - Add `model: Option<String>` field to builder struct
    - Follow existing pattern with `base_url: Option<String>`
    - Update Default implementation if needed
  - [x] 2.3 Add model() method to OllamaClientBuilder
    - Method signature: `pub fn model(mut self, model: impl Into<String>) -> Self`
    - Follow existing `base_url()` method pattern
    - Store model in builder state
  - [x] 2.4 Update build() method to check OLLAMA_MODEL env var
    - Check `std::env::var("OLLAMA_MODEL")` in build() method
    - Use builder value → env var → default pattern (matching OLLAMA_HOST)
    - Default to empty string if not set (model selection happens at call site)
    - Store model name in OllamaClient struct (add field if needed)
  - [x] 2.5 Add getter method for model name
    - Add `pub fn model(&self) -> &str` method to OllamaClient
    - Return model name for use in auto-tagging calls
    - Handle empty string default appropriately
  - [x] 2.6 Ensure OLLAMA_MODEL tests pass
    - Run ONLY the 2-4 tests written in 2.1
    - Verify env var reading works correctly
    - Verify precedence: builder → env var → default
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 2-4 tests written in 2.1 pass
- OllamaClientBuilder reads OLLAMA_MODEL env var
- Builder value takes precedence over env var
- Default fallback works when env var not set
- Model name accessible via getter method

### Auto-Tagging Integration

#### Task Group 3: Background Auto-Tagging Task
**Dependencies:** Task Groups 1, 2

- [x] 3.0 Complete background auto-tagging integration
  - [x] 3.1 Write 2-6 focused tests for background auto-tagging
    - Test that note creation succeeds even if Ollama client fails to construct
    - Test that background task spawns without blocking note creation
    - Test that auto-generated tags are added to note after creation
    - Test that manual and auto-generated tags coexist on same note
    - Test that failures in background task don't affect note creation
    - Limit to 2-6 tests covering critical background task behavior
  - [x] 3.2 Create helper function for background auto-tagging task
    - Function signature: `async fn auto_tag_note_background(note_id: NoteId, content: String, db_path: PathBuf, model: String)`
    - Function should own all resources (Database, OllamaClient, AutoTagger)
    - Use `let _ = tokio::spawn(...)` pattern for fire-and-forget
    - Silently catch and ignore all errors (no user-facing messages)
  - [x] 3.3 Implement background task logic
    - Open database connection using `Database::open(&db_path)`
    - Ensure directory exists before opening (reuse `ensure_database_directory()`)
    - Construct `OllamaClient` using `OllamaClientBuilder` with model from env var
    - Construct `AutoTagger` using `AutoTaggerBuilder` with client
    - Call `AutoTagger::generate_tags(model, content)` to get tag HashMap
  - [x] 3.4 Convert tags and add to note
    - Extract tag names from HashMap keys
    - Convert confidence scores from `f64` (0.0-1.0) to `u8` (0-100) by multiplying by 100
    - Create `TagSource::llm(model, confidence)` for each tag
    - Call `NoteService::add_tags_to_note()` with LLM source and all tags
    - Handle errors silently (background task should not fail loudly)
  - [x] 3.5 Integrate background task into execute_add()
    - After note creation succeeds, spawn background task
    - Pass note_id, content, db_path, and model name to background function
    - Read model from `OLLAMA_MODEL` env var (with fallback)
    - Use `tokio::spawn()` to detach task without awaiting
    - Ensure note creation output shows only manual tags (existing behavior)
  - [x] 3.6 Ensure auto-tagging integration tests pass
    - Run ONLY the 2-6 tests written in 3.1
    - Verify background task spawns correctly
    - Verify tags are added asynchronously
    - Verify fail-safe behavior (note saves even if tagging fails)
    - Do NOT run the entire test suite at this stage

**Acceptance Criteria:**
- The 2-6 tests written in 3.1 pass
- Background task spawns without blocking note creation
- Auto-generated tags are added to notes asynchronously
- Manual and auto-generated tags coexist on same note
- Fail-safe: note creation succeeds even if Ollama fails
- Silent error handling: no user-facing error messages

### Testing

#### Task Group 4: Test Review & Gap Analysis
**Dependencies:** Task Groups 1-3

- [x] 4.0 Review existing tests and fill critical gaps only
  - [x] 4.1 Review tests from Task Groups 1-3
    - Review the 2-4 tests written for async conversion (Task 1.1)
    - Review the 2-4 tests written for OLLAMA_MODEL support (Task 2.1)
    - Review the 2-6 tests written for background auto-tagging (Task 3.1)
    - Total existing tests: approximately 6-14 tests
  - [x] 4.2 Analyze test coverage gaps for THIS feature only
    - Identify critical user workflows that lack test coverage
    - Focus ONLY on gaps related to this spec's auto-tagging integration
    - Do NOT assess entire application test coverage
    - Prioritize end-to-end workflows: note creation → background tagging → tag persistence
  - [x] 4.3 Write up to 8 additional strategic tests maximum
    - Add maximum of 8 new tests to fill identified critical gaps
    - Focus on integration points: CLI command → background task → database persistence
    - Test tag merging: manual tags + auto-generated tags on same note
    - Test model selection: OLLAMA_MODEL env var → tag generation → model stored in TagSource
    - Test confidence score conversion: f64 → u8 → database storage
    - Do NOT write comprehensive coverage for all scenarios
    - Skip edge cases unless business-critical
  - [x] 4.4 Run feature-specific tests only
    - Run ONLY tests related to this spec's feature (tests from 1.1, 2.1, 3.1, and 4.3)
    - Expected total: approximately 14-22 tests maximum
    - Do NOT run the entire application test suite
    - Verify critical workflows pass: note creation with auto-tagging end-to-end

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 14-22 tests total)
- Critical user workflows for auto-tagging are covered
- No more than 8 additional tests added when filling in testing gaps
- Testing focused exclusively on this spec's auto-tagging integration
- End-to-end workflow verified: `cons add "note"` → note saved → tags added asynchronously

## Execution Order

Recommended implementation sequence:
1. CLI Layer - Async Runtime Conversion (Task Group 1)
2. Ollama Client Enhancement - OLLAMA_MODEL Support (Task Group 2)
3. Auto-Tagging Integration - Background Task (Task Group 3)
4. Test Review & Gap Analysis (Task Group 4)

## Notes

- Background task uses fire-and-forget pattern: `let _ = tokio::spawn(async move { ... })`
- All errors in background task are silently caught and ignored (fail-safe design)
- Model name is read from `OLLAMA_MODEL` env var at spawn time, not stored in OllamaClient
- Manual tags are shown immediately in output; auto-generated tags appear later when task completes
- Database connection in background task is independent from main command execution

