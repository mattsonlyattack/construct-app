# Spec Requirements: Ollama HTTP Client

## Initial Description

**Roadmap Item #7**: Ollama HTTP client -- Build async client for Ollama API using reqwest and tokio, with proper timeout and retry handling

## Requirements Discussion

### First Round Questions

**Q1:** Ollama endpoint configuration: I'm assuming the client should default to `http://localhost:11434` (Ollama's default), but allow override via environment variable (e.g., `OLLAMA_HOST`). Is that correct, or should we use a different configuration approach?

**Answer:** Default to `http://172.17.64.1:11434` (WSL networking address). Allow override via environment variable.

**Q2:** Timeout values: I'm thinking we should have separate timeouts for connection (e.g., 5 seconds) and read/response (e.g., 60 seconds for LLM inference). Should these be configurable or hardcoded with sensible defaults?

**Answer:** Hardcode sensible defaults for now.

**Q3:** Retry strategy: I'm assuming exponential backoff retries (e.g., 3 max retries with 1s, 2s, 4s delays) for transient network errors, but NOT retrying on HTTP 4xx client errors. Is that the right approach, or should we handle retries differently?

**Answer:** Good enough for now (exponential backoff with 3 max retries, no retries on 4xx errors).

**Q4:** Error handling: Since this is library code that will be used by NoteService, I'm assuming we should use `thiserror` for custom error types (network errors, timeout errors, API errors) rather than `anyhow`. Is that correct?

**Answer:** Yes, use `thiserror` for custom error types.

**Q5:** Client initialization: I'm thinking a builder pattern like `OllamaClient::new().base_url("...").timeout(...).build()` would be ergonomic, with sensible defaults. Should we go with builder pattern or a simpler constructor?

**Answer:** Builder pattern (consistent with existing `NoteBuilder` pattern in codebase).

**Q6:** Ollama API endpoints: For the initial implementation, should we focus on the `/api/generate` endpoint (for tag extraction), or do we also need `/api/chat` or other endpoints? I'm assuming we start with `/api/generate` since roadmap item #8 mentions tag extraction.

**Answer:** `/api/generate` only for now. We'll add more endpoints later.

**Q7:** Configuration: Should the client read configuration from environment variables only, or do we need a config file? I'm assuming environment variables (`OLLAMA_HOST`, `OLLAMA_TIMEOUT`) with defaults are sufficient for MVP.

**Answer:** Environment variables. Note: `.env` file support can be added later via `dotenvy` crate if needed, but `std::env::var()` is sufficient for MVP.

**Q8:** Testing approach: Should we design the client to be easily mockable (trait-based) for unit tests, or rely on integration tests against a real Ollama instance? I'm assuming both—a trait interface for mocking AND integration tests with real Ollama.

**Answer:** Mockable trait-based for now.

### Existing Code to Reference

**Similar Features Identified:**
- `NoteBuilder` pattern in `src/models/note.rs` - Follow same builder pattern approach
- Error handling patterns in `agent-os/standards/rust/standards.md` - Use `thiserror` for library code
- Async patterns in `agent-os/standards/rust/standards.md` - Use tokio with `#[tokio::main]` or `block_on` at boundary

No similar HTTP client implementations found in codebase (this is the first HTTP client).

### Follow-up Questions

None needed - all questions answered clearly.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements
- Build async HTTP client for Ollama API using `reqwest` and `tokio`
- Support `/api/generate` endpoint initially (expandable to other endpoints later)
- Default endpoint: `http://172.17.64.1:11434` (WSL networking)
- Allow endpoint override via environment variable (e.g., `OLLAMA_HOST`)
- Implement proper timeout handling (connection and read/response timeouts with sensible defaults)
- Implement exponential backoff retry strategy (3 max retries, 1s/2s/4s delays)
- Do not retry on HTTP 4xx client errors
- Use builder pattern for client initialization (consistent with `NoteBuilder`)
- Design trait-based interface for mockability in tests

### Reusability Opportunities
- Follow `NoteBuilder` pattern from `src/models/note.rs` for builder implementation
- Follow error handling patterns from `agent-os/standards/rust/standards.md` (use `thiserror`)
- Follow async patterns from standards (tokio for HTTP, sync for SQLite)

### Scope Boundaries
**In Scope:**
- Async HTTP client using reqwest and tokio
- `/api/generate` endpoint support
- Timeout configuration (hardcoded sensible defaults)
- Exponential backoff retry logic (3 max retries)
- Builder pattern for client construction
- Trait-based interface for testability
- Environment variable configuration for endpoint override
- Custom error types using `thiserror`

**Out of Scope:**
- `/api/chat` or other Ollama endpoints (add later)
- Config file support (can add `.env` via `dotenvy` later if needed)
- Configurable timeout values (hardcoded for MVP)
- Integration tests with real Ollama instance (trait-based mocking for now)
- Advanced retry strategies beyond exponential backoff

### Technical Considerations
- **Dependencies**: Need to add `reqwest` (with `tokio` runtime), `tokio`, `thiserror` to `Cargo.toml`
- **Error Types**: Use `thiserror` for custom error enum (network errors, timeout errors, API errors, serialization errors)
- **Builder Pattern**: Follow `NoteBuilder` pattern - owned `self` in methods, `build()` returns `OllamaClient`
- **Async Runtime**: Use tokio for async/await; client methods should be `async fn`
- **Configuration**: Read from `std::env::var()` for `OLLAMA_HOST` override, default to `http://172.17.64.1:11434`
- **Trait Design**: Create `OllamaClientTrait` or similar for mockability in tests
- **Timeout Values**: Hardcode sensible defaults (e.g., connection: 5s, read: 60s for LLM inference)
- **Retry Logic**: Exponential backoff: 1s, 2s, 4s delays, max 3 retries, only retry on transient errors (5xx, network errors), not 4xx
- **Module Location**: Create new module `src/ollama/` or `src/client/ollama.rs` following layered architecture
- **Integration**: This client will be used by `NoteService` in roadmap item #9 for auto-tagging


