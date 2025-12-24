# Spec Requirements: CLI Auto-Tagging Integration

## Initial Description

CLI: Integrate auto-tagging into `cons add` command, calling Ollama asynchronously and updating note tags

Roadmap item #9, estimated size S (2-3 days). This integrates the existing AutoTagger and OllamaClient infrastructure into the `cons add` command to automatically generate tags for notes using LLM analysis.

## Requirements Discussion

### First Round Questions

**Q1:** I assume auto-tagging should happen asynchronously (non-blocking) so that note capture succeeds immediately even if Ollama is slow or unavailable. The note is saved first, then tags are added in the background. Is that correct, or should we wait for tags before returning success?
**Answer:** Async - auto-tagging should happen asynchronously (non-blocking)

**Q2:** I'm thinking that when a user provides manual tags via `--tags`, those should be merged with the auto-generated tags (both sets of tags applied to the note). Should we allow manual tags to override auto-generated ones if there's a conflict, or always merge them?
**Answer:** Merge - manual tags should be merged with auto-generated tags

**Q3:** I assume we should use the existing `deepseek-r1:8b` model (as referenced in your codebase) for auto-tagging. Should this be hardcoded, or should we support a `--model` flag or environment variable to override it?
**Answer:** Environment variable for model, currently have gemma3:4b available

**Q4:** Following your fail-safe design principle, I'm assuming that if Ollama fails (network error, timeout, etc.), the note should still be created successfully without tags, and we should silently continue (no error message to the user). Is that correct, or should we show a warning message?
**Answer:** Yes, save without auto-tagging if Ollama fails (fail-safe)

**Q5:** I'm thinking the success output should show both manual and auto-generated tags together, like `Note created (id: 42) with tags: rust, learning, async, tokio` (where "rust, learning" are manual and "async, tokio" are auto-generated). Should we distinguish between manual and auto-generated tags in the output, or just show all tags together?
**Answer:** Show all tags together

**Q6:** Should we add a `--no-auto-tags` flag to allow users to disable auto-tagging for a specific note, or is auto-tagging always-on when available?
**Answer:** No flag needed, always auto-tag

**Q7:** I assume we should use the existing `AutoTagger` and `OllamaClient` from your codebase, constructing them in `handle_add` and calling `generate_tags()` then `add_tags_to_note()` with `TagSource::Llm`. Should we reuse the existing code patterns, or do you want a different integration approach?
**Answer:** Reuse existing code, expose inconsistencies where found

### Existing Code to Reference

**Similar Features Identified:**
- Feature: Ollama HTTP Client - Path: `src/ollama/client.rs`
  - Components to potentially reuse: `OllamaClientBuilder`, `OllamaClientTrait`, error handling patterns
  - Backend logic to reference: Environment variable handling (`OLLAMA_HOST`), async HTTP client patterns

- Feature: AutoTagger - Path: `src/autotagger/tagger.rs`
  - Components to potentially reuse: `AutoTaggerBuilder`, `AutoTagger::generate_tags()` method
  - Backend logic to reference: Tag generation with confidence scores, JSON parsing, tag normalization

- Feature: NoteService - Path: `src/service.rs`
  - Components to potentially reuse: `NoteService::add_tags_to_note()` method with `TagSource::Llm`
  - Backend logic to reference: Tag assignment with LLM source, confidence scores, model version tracking

- Feature: CLI Add Command - Path: `src/main.rs`
  - Components to potentially reuse: `handle_add()`, `execute_add()` functions, tag parsing logic
  - Backend logic to reference: Note creation flow, tag merging patterns

**Inconsistencies Found:**
- `main()` function is currently synchronous but needs to be async (`#[tokio::main]`) to support async auto-tagging
- `OLLAMA_MODEL` environment variable is referenced in tests (`tests/ollama_integration.rs`) but not used in `OllamaClientBuilder` - should be added for model selection
- Current `handle_add()` is synchronous - needs async support for background tag generation

### Follow-up Questions

No follow-up questions needed.

## Visual Assets

### Files Provided:
No visual files found.

### Visual Insights:
No visual assets provided.

## Requirements Summary

### Functional Requirements
- Auto-tagging runs asynchronously in the background after note creation
- Note is saved immediately, tags added asynchronously without blocking
- Manual tags (`--tags` flag) are merged with auto-generated tags (both applied)
- Model selection via `OLLAMA_MODEL` environment variable (defaults to available model)
- Fail-safe design: note creation succeeds even if Ollama fails
- Silent failure: no error messages shown to user if auto-tagging fails
- Output shows all tags together (manual + auto-generated) without distinction
- Auto-tagging is always-on (no flag to disable)

### Reusability Opportunities
- Reuse `AutoTagger` and `AutoTaggerBuilder` from `src/autotagger/tagger.rs`
- Reuse `OllamaClient` and `OllamaClientBuilder` from `src/ollama/client.rs`
- Reuse `NoteService::add_tags_to_note()` with `TagSource::Llm` for tag assignment
- Follow existing async patterns using tokio runtime
- Use `OllamaClientTrait` for dependency injection and testability
- Follow existing error handling patterns (fail-safe, silent degradation)

### Scope Boundaries
**In Scope:**
- Converting `main()` to async with `#[tokio::main]`
- Adding async background task for auto-tagging after note creation
- Integrating `AutoTagger` into `handle_add()` flow
- Adding `OLLAMA_MODEL` environment variable support to `OllamaClientBuilder`
- Merging manual and auto-generated tags
- Fail-safe error handling (note saves even if tagging fails)
- Output showing all tags together

**Out of Scope:**
- Synchronous auto-tagging (blocking until tags complete)
- Flag to disable auto-tagging (`--no-auto-tags`)
- Distinguishing manual vs auto-generated tags in output
- Error messages for auto-tagging failures
- Model selection via CLI flag (only environment variable)
- Tag conflict resolution (manual tags don't override auto-generated)

### Technical Considerations
- Integration points: `handle_add()` → `execute_add()` → `NoteService::create_note()` → background task → `AutoTagger::generate_tags()` → `NoteService::add_tags_to_note()`
- Existing system constraints: `main()` currently synchronous, needs async conversion
- Technology preferences: tokio for async runtime, existing `AutoTagger` and `OllamaClient` infrastructure
- Similar code patterns to follow: `OllamaClientBuilder` pattern, `AutoTaggerBuilder` pattern, `NoteService::add_tags_to_note()` with `TagSource::Llm`
- Inconsistencies to address: Add `OLLAMA_MODEL` env var support to `OllamaClientBuilder`, convert `main()` to async

