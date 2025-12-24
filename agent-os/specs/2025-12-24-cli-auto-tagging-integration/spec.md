# Specification: CLI Auto-Tagging Integration

## Goal

Integrate auto-tagging into the `cons add` command by calling Ollama asynchronously in the background to generate tags from note content, ensuring note capture succeeds immediately even if tagging fails.

## User Stories

- As a user, I want my notes to be automatically tagged when I run `cons add "thought"` so that I can find them later without manual organization
- As a user, I want note capture to succeed instantly even if Ollama is slow or unavailable so that I'm never blocked from saving my thoughts

## Specific Requirements

**Convert main() to async runtime**
- Change `main()` function signature to `#[tokio::main] async fn main()`
- Update `handle_add()` to be async: `async fn handle_add(cmd: &AddCommand) -> Result<()>`
- Update command dispatch to use `.await` for async calls
- Ensure tokio runtime features are enabled in Cargo.toml (already present)

**Add OLLAMA_MODEL environment variable support**
- Extend `OllamaClientBuilder` to check `OLLAMA_MODEL` environment variable in `build()` method
- Use `std::env::var("OLLAMA_MODEL")` with fallback to empty string or sensible default
- Store model name for use in auto-tagging calls (separate from base URL configuration)
- Follow existing pattern: builder value → env var → default (matching `OLLAMA_HOST` pattern)

**Create async background task for auto-tagging**
- After note creation succeeds, spawn background task using `tokio::spawn()` for fire-and-forget pattern
- Background task should own its own `Database` instance (open new connection)
- Task should construct `OllamaClient` and `AutoTagger` independently
- Use `tokio::spawn()` to detach task from main execution flow

**Integrate AutoTagger into add command flow**
- In background task, call `AutoTagger::generate_tags(model, content)` with model from env var
- Convert confidence scores from `HashMap<String, f64>` (0.0-1.0) to `u8` (0-100) for `TagSource::llm()`
- Extract tag names from HashMap keys, create `TagSource::llm(model, confidence)` for each tag
- Call `NoteService::add_tags_to_note()` with LLM source and all generated tags

**Merge manual and auto-generated tags**
- Manual tags from `--tags` flag are applied during `create_note()` call (existing behavior)
- Auto-generated tags are added asynchronously after note creation via `add_tags_to_note()`
- Both tag sets coexist on the same note (no override or conflict resolution)
- Database uses `INSERT OR IGNORE` in `add_tags_to_note()` to handle duplicates gracefully

**Fail-safe error handling**
- Note creation must succeed even if Ollama client construction fails
- Background task silently catches and ignores all errors (no user-facing error messages)
- Use `let _ = tokio::spawn(async move { ... })` pattern to detach task without awaiting
- If `generate_tags()` fails, background task exits silently without affecting note

**Output all tags together**
- Success message shows manual tags immediately: `Note created (id: 42) with tags: rust, learning`
- Auto-generated tags appear later when background task completes (but user doesn't wait)
- When listing notes later, both manual and auto-generated tags appear together
- No distinction in output between tag sources (all shown as unified tag list)

**Database connection handling in background task**
- Background task opens its own `Database::open(&db_path)` connection
- Task owns `Database` instance for its lifetime (no shared references)
- Follow existing pattern: get database path via `get_database_path()` helper
- Ensure directory exists before opening database (reuse `ensure_database_directory()`)

**Model selection and configuration**
- Read model name from `OLLAMA_MODEL` environment variable
- Default to empty string or sensible fallback if env var not set (user can specify via env)
- Pass model name to `AutoTagger::generate_tags(model, content)` as first parameter
- Model name stored in `TagSource::Llm` variant for provenance tracking

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**`src/main.rs` - CLI entry point and add command handler**
- Reuse `handle_add()` and `execute_add()` function structure
- Follow existing tag parsing logic with `parse_tags()` helper
- Maintain existing output format: `Note created (id: X) with tags: ...`
- Keep separation between `handle_add()` (setup) and `execute_add()` (logic) for testability

**`src/service.rs` - NoteService with tag management**
- Use `NoteService::add_tags_to_note()` method for adding auto-generated tags
- Pass `TagSource::llm(model, confidence)` to distinguish LLM tags from user tags
- Method already handles `INSERT OR IGNORE` for duplicate tag assignments
- Confidence scores converted from u8 (0-100) to f64 (0.0-1.0) internally

**`src/autotagger/tagger.rs` - AutoTagger for tag generation**
- Use `AutoTaggerBuilder` pattern to construct tagger with `OllamaClient`
- Call `generate_tags(model, content)` async method returning `HashMap<String, f64>`
- Tags already normalized (lowercase, hyphenated) by `TagNormalizer`
- Returns empty HashMap on JSON parsing failures (fail-safe behavior)

**`src/ollama/client.rs` - OllamaClient for HTTP communication**
- Use `OllamaClientBuilder` to construct client with environment variable support
- Follow existing `OLLAMA_HOST` pattern for `OLLAMA_MODEL` env var handling
- Client already handles timeouts, retries, and error types via `OllamaError`
- Use `OllamaClientTrait` for dependency injection and testability

**`src/models/tag_source.rs` - TagSource enum for tag provenance**
- Use `TagSource::llm(model, confidence)` constructor for LLM tags
- Confidence stored as `u8` (0-100), convert from `f64` (0.0-1.0) by multiplying by 100
- Model name stored in variant for tracking which LLM generated each tag
- Enum already serializes correctly for database storage

## Out of Scope

- Synchronous auto-tagging that blocks note creation until tags complete
- `--no-auto-tags` flag to disable auto-tagging for specific notes
- Distinguishing manual vs auto-generated tags in CLI output messages
- Error messages or warnings when auto-tagging fails (silent degradation)
- Model selection via CLI flag (`--model`) - only environment variable support
- Tag conflict resolution where manual tags override auto-generated ones
- Waiting for background task completion before command exit
- Progress indicators or status messages for background tagging
- Retry logic for failed auto-tagging attempts (single attempt only)
- Configurable confidence thresholds for tag filtering

