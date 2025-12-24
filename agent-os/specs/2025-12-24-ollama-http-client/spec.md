# Specification: Ollama HTTP Client

## Goal

Build an async HTTP client for the Ollama API using reqwest and tokio, with proper timeout and retry handling, to enable AI-powered auto-tagging of notes in the cons knowledge management system.

## User Stories

- As a developer, I want a reusable Ollama HTTP client with timeout and retry handling so that NoteService can reliably call Ollama for auto-tagging without blocking note capture
- As a user, I want the system to gracefully handle Ollama connection failures so that note capture always succeeds even if AI tagging fails

## Specific Requirements

**OllamaClient struct with builder pattern**
- Create `OllamaClient` struct that owns a `reqwest::Client` instance
- Implement `OllamaClientBuilder` following the `NoteBuilder` pattern from `src/models/note.rs`
- Builder methods: `new()`, `base_url()`, `build()` with owned `self` pattern
- Default base URL: `http://172.17.64.1:11434` (WSL networking address)
- Allow override via `OLLAMA_HOST` environment variable using `std::env::var()`
- Builder should read environment variable in `build()` method if `base_url()` not called

**Timeout configuration**
- Hardcode sensible defaults: connection timeout 5 seconds, read timeout 60 seconds
- Configure reqwest client with both timeouts during builder `build()` method
- Use `reqwest::ClientBuilder` to set timeouts before creating client

**Exponential backoff retry logic**
- Implement retry logic with 3 maximum retries
- Retry delays: 1 second, 2 seconds, 4 seconds (exponential backoff)
- Only retry on transient errors: HTTP 5xx server errors and network errors
- Do NOT retry on HTTP 4xx client errors (bad request, not found, etc.)
- Use `tokio::time::sleep` for delays between retries

**Custom error types with thiserror**
- Create `OllamaError` enum using `thiserror` derive macro
- Error variants: `Network`, `Timeout`, `Http`, `Serialization`, `Api`
- `Network` for connection failures, `Timeout` for timeout errors
- `Http` for HTTP errors with status code, `Api` for Ollama API-specific errors
- `Serialization` for JSON serialization/deserialization failures
- Derive `Debug`, `Display` via `thiserror`

**Trait-based interface for testability**
- Create `OllamaClientTrait` trait with async methods for Ollama operations
- Define `generate()` method signature: `async fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError>`
- Implement trait for `OllamaClient` struct
- Trait enables mocking in unit tests without real Ollama instance

**/api/generate endpoint support**
- Implement `generate()` method that calls Ollama `/api/generate` endpoint
- Accept `model: &str` and `prompt: &str` parameters
- Serialize request body with serde_json (model, prompt fields)
- Parse response JSON to extract generated text
- Return generated text as `String` or propagate errors

**Module organization**
- Create `src/ollama/` directory with `mod.rs` and `client.rs` files
- `client.rs` contains `OllamaClient`, `OllamaClientBuilder`, `OllamaError`
- `mod.rs` exports public types: `OllamaClient`, `OllamaClientTrait`, `OllamaError`
- Add `pub mod ollama;` to `src/lib.rs` module declarations
- Re-export types from `lib.rs`: `pub use ollama::{OllamaClient, OllamaClientTrait, OllamaError};`

**Dependencies**
- Add `reqwest = { version = "0.12", features = ["json"] }` to `Cargo.toml`
- Add `tokio = { version = "1", features = ["rt", "time"] }` to `Cargo.toml`
- Add `thiserror = "1.0"` to `Cargo.toml`
- Use `serde` and `serde_json` already in dependencies for request/response serialization

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**NoteBuilder pattern in `src/models/note.rs`**
- Follow same builder pattern: `#[derive(Debug, Default)]` for builder struct
- Use `Option<T>` fields for optional configuration
- Methods take `mut self` and return `Self` for method chaining
- `build()` method consumes builder and returns constructed type
- Use `unwrap_or()` or `unwrap_or_default()` for defaults in `build()`

**Error handling patterns in `agent-os/standards/rust/standards.md`**
- Use `thiserror` for library code error types (not `anyhow`)
- Derive `Debug` and `Display` via `thiserror` attributes
- Use `#[source]` attribute to wrap underlying errors
- Propagate errors with `?` operator, avoid `.unwrap()` except in tests

**Async patterns in `agent-os/standards/rust/standards.md`**
- Use tokio for async/await with HTTP operations
- Client methods should be `async fn`
- Use `tokio::time::sleep` for retry delays
- Follow pattern: async at boundary, sync for SQLite (not applicable here, but shows async usage)

**Module structure from `src/db/` and `src/models/`**
- Follow same directory structure: `src/ollama/mod.rs` and `src/ollama/client.rs`
- Use `mod.rs` for module declarations and re-exports
- Keep implementation in separate files, expose via `mod.rs`

## Out of Scope

- `/api/chat` or other Ollama API endpoints (add in future iterations)
- Config file support (`.env` via `dotenvy` crate can be added later if needed)
- Configurable timeout values (hardcoded sensible defaults for MVP)
- Integration tests with real Ollama instance (trait-based mocking for now)
- Advanced retry strategies beyond exponential backoff (circuit breakers, jitter, etc.)
- Connection pooling configuration (use reqwest defaults)
- Request/response logging or metrics (add later if needed)
- Streaming responses from Ollama (only support complete responses for now)
- Custom HTTP headers or authentication (Ollama API doesn't require auth)
- Rate limiting or throttling (add later if needed)

