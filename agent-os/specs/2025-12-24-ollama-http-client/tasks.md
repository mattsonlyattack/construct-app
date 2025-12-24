# Task Breakdown: Ollama HTTP Client

## Overview
Total Tasks: 25 (across 5 task groups)

This spec implements an async HTTP client for the Ollama API using reqwest and tokio, with proper timeout and retry handling, to enable AI-powered auto-tagging of notes in the cons knowledge management system.

## Task List

### Infrastructure Layer

#### Task Group 1: Dependencies Setup
**Dependencies:** None

- [x] 1.0 Complete dependencies setup
  - [x] 1.1 Write 2 focused tests for dependency verification
    - Test that `reqwest` client can be created with timeout configuration
    - Test that `tokio` runtime can be used for async operations
  - [x] 1.2 Update `Cargo.toml` with required dependencies
    - Add `reqwest = { version = "0.12", features = ["json"] }`
    - Add `tokio = { version = "1", features = ["rt", "time"] }`
    - Add `thiserror = "1.0"`
    - Verify `serde` and `serde_json` are already present (used for JSON serialization)
  - [x] 1.3 Ensure dependencies compile successfully
    - Run `cargo check` to verify all dependencies resolve correctly
    - Verify no dependency conflicts with existing crates

**Acceptance Criteria:**
- The 2 tests written in 1.1 pass
- `cargo build` succeeds with new dependencies
- All dependencies resolve without conflicts
- `cargo clippy` passes with no warnings

### Error Handling Layer

#### Task Group 2: Custom Error Types
**Dependencies:** Task Group 1

- [x] 2.0 Complete error types implementation
  - [x] 2.1 Write 5 focused tests for OllamaError
    - Test `Network` error variant creation and display
    - Test `Timeout` error variant creation and display
    - Test `Http` error variant with status code
    - Test `Serialization` error variant wraps serde errors
    - Test `Api` error variant for Ollama-specific errors
  - [x] 2.2 Create `src/ollama/` directory structure
    - Create `src/ollama/mod.rs` for module declarations
    - Create `src/ollama/client.rs` for client implementation
  - [x] 2.3 Implement `OllamaError` enum in `src/ollama/client.rs`
    - Variants: `Network`, `Timeout`, `Http`, `Serialization`, `Api`
    - Use `thiserror` derive macro with `#[derive(thiserror::Error)]`
    - Derive `Debug`, `Display` via `thiserror` attributes
    - Use `#[source]` attribute to wrap underlying errors (reqwest::Error, serde_json::Error)
    - `Http` variant should include status code: `Http { status: u16 }`
    - `Api` variant should include error message: `Api { message: String }`
  - [x] 2.4 Ensure error type tests pass
    - Run ONLY the 5 tests written in 2.1
    - Verify error messages are user-friendly
    - Verify error source chaining works correctly

**Acceptance Criteria:**
- The 5 tests written in 2.1 pass
- Error types implement `std::error::Error` trait via `thiserror`
- Error messages are clear and actionable
- Error source chaining preserves underlying error context

### Client Infrastructure Layer

#### Task Group 3: Client Builder and Struct
**Dependencies:** Task Group 2

- [x] 3.0 Complete client builder and struct
  - [x] 3.1 Write 6 focused tests for OllamaClientBuilder
    - Test `OllamaClientBuilder::new()` creates builder with defaults
    - Test `base_url()` method sets custom URL
    - Test `build()` uses default URL when `base_url()` not called
    - Test `build()` reads `OLLAMA_HOST` environment variable if set
    - Test `build()` creates client with correct timeout configuration
    - Test `build()` panics or returns error if invalid URL provided
  - [x] 3.2 Implement `OllamaClientBuilder` struct in `src/ollama/client.rs`
    - Follow `NoteBuilder` pattern from `src/models/note.rs`
    - Fields: `base_url: Option<String>`
    - Methods: `new() -> Self`, `base_url(mut self, url: impl Into<String>) -> Self`, `build() -> Result<OllamaClient, OllamaError>`
    - Default base URL: `http://172.17.64.1:11434`
    - Read `OLLAMA_HOST` environment variable in `build()` if `base_url()` not called
    - Use `std::env::var("OLLAMA_HOST")` for environment variable reading
  - [x] 3.3 Implement `OllamaClient` struct in `src/ollama/client.rs`
    - Field: `client: reqwest::Client`
    - Field: `base_url: String`
    - Constructor should be private (only via builder)
    - Configure reqwest client with timeouts: connection 5s, read 60s
    - Use `reqwest::ClientBuilder` to set timeouts before creating client
  - [x] 3.4 Update `src/ollama/mod.rs` to export error type
    - Add `pub mod client;`
    - Add `pub use client::OllamaError;` (client and builder exported later)
  - [x] 3.5 Ensure client builder tests pass
    - Run ONLY the 6 tests written in 3.1
    - Verify builder pattern works correctly
    - Verify environment variable reading works

**Acceptance Criteria:**
- The 6 tests written in 3.1 pass
- Builder pattern follows `NoteBuilder` conventions
- Environment variable override works correctly
- Client created with proper timeout configuration
- Invalid URLs handled gracefully

### Retry Logic Layer

#### Task Group 4: Exponential Backoff Retry Implementation
**Dependencies:** Task Group 3

- [x] 4.0 Complete retry logic implementation
  - [x] 4.1 Write 5 focused tests for retry logic
    - Test retry succeeds after transient network error
    - Test retry stops after 3 attempts
    - Test retry delays increase exponentially (1s, 2s, 4s)
    - Test retry does NOT occur on HTTP 4xx errors
    - Test retry occurs on HTTP 5xx errors
  - [x] 4.2 Implement retry helper function in `src/ollama/client.rs`
    - Function signature: `async fn retry_with_backoff<F, Fut, T>(f: F) -> Result<T, OllamaError>` where `F: Fn() -> Fut, Fut: Future<Output = Result<T, OllamaError>>`
    - Maximum 3 retries
    - Delays: 1 second, 2 seconds, 4 seconds (exponential backoff)
    - Use `tokio::time::sleep` for delays
    - Only retry on transient errors: HTTP 5xx and network errors
    - Do NOT retry on HTTP 4xx client errors
  - [x] 4.3 Integrate retry logic into client methods
    - Wrap HTTP calls with retry helper function
    - Ensure retry logic is applied to all network operations
  - [x] 4.4 Ensure retry logic tests pass
    - Run ONLY the 5 tests written in 4.1
    - Verify exponential backoff timing is correct
    - Verify retry decision logic works correctly

**Acceptance Criteria:**
- The 5 tests written in 4.1 pass
- Retry logic implements exponential backoff correctly
- Retries only occur on transient errors (5xx, network)
- No retries on client errors (4xx)
- Maximum retry limit enforced

### API Implementation Layer

#### Task Group 5: Trait Interface and Generate Endpoint
**Dependencies:** Task Group 4

- [x] 5.0 Complete trait interface and generate endpoint
  - [x] 5.1 Write 6 focused tests for OllamaClientTrait and generate method
    - Test trait can be implemented by mock struct
    - Test `generate()` method calls correct Ollama endpoint
    - Test `generate()` serializes request body correctly
    - Test `generate()` parses response JSON correctly
    - Test `generate()` handles HTTP errors correctly
    - Test `generate()` applies retry logic on transient errors
  - [x] 5.2 Implement `OllamaClientTrait` trait in `src/ollama/client.rs`
    - Define `generate()` method: `async fn generate(&self, model: &str, prompt: &str) -> Result<String, OllamaError>`
    - Trait should be public for use in tests and by NoteService
    - Trait enables mocking in unit tests
  - [x] 5.3 Implement `generate()` method for `OllamaClient`
    - Call `/api/generate` endpoint at `{base_url}/api/generate`
    - Accept `model: &str` and `prompt: &str` parameters
    - Serialize request body with `serde_json::json!` macro: `{"model": model, "prompt": prompt}`
    - Send POST request with JSON body
    - Parse response JSON to extract `response` field (Ollama API format)
    - Return generated text as `String`
    - Wrap HTTP call with retry logic from Task Group 4
    - Handle errors appropriately (network, timeout, HTTP, serialization)
  - [x] 5.4 Implement trait for `OllamaClient`
    - Implement `OllamaClientTrait` for `OllamaClient`
    - Delegate to internal `generate()` implementation
  - [x] 5.5 Update `src/ollama/mod.rs` to export all public types
    - Add `pub use client::{OllamaClient, OllamaClientBuilder, OllamaClientTrait, OllamaError};`
  - [x] 5.6 Update `src/lib.rs` to expose ollama module
    - Add `pub mod ollama;`
    - Add `pub use ollama::{OllamaClient, OllamaClientBuilder, OllamaClientTrait, OllamaError};`
  - [x] 5.7 Ensure trait and generate endpoint tests pass
    - Run ONLY the 6 tests written in 5.1
    - Verify trait can be mocked in tests
    - Verify generate endpoint works correctly

**Acceptance Criteria:**
- The 6 tests written in 5.1 pass
- Trait interface enables mocking for tests
- Generate endpoint calls correct Ollama API endpoint
- Request/response serialization works correctly
- Retry logic is applied to generate calls
- All public types are accessible from crate root

### Testing Layer

#### Task Group 6: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-5

- [x] 6.0 Review existing tests and fill critical gaps only
  - [x] 6.1 Review tests from Task Groups 1-5
    - Review 2 tests written in Task 1.1 (dependencies)
    - Review 5 tests written in Task 2.1 (error types)
    - Review 6 tests written in Task 3.1 (client builder)
    - Review 5 tests written in Task 4.1 (retry logic)
    - Review 6 tests written in Task 5.1 (trait and generate)
    - Total existing tests: 24 tests
  - [x] 6.2 Analyze test coverage gaps for THIS feature only
    - Identify critical behaviors lacking coverage
    - Focus on integration between components (builder → client → retry → generate)
    - Check error propagation through retry logic
    - Verify environment variable handling edge cases
  - [x] 6.3 Write up to 6 additional strategic tests maximum
    - Test full integration: builder → client → generate with real HTTP mock
    - Test environment variable override precedence (env var vs builder method)
    - Test error types propagate correctly through retry logic
    - Additional tests only if critical gaps identified in 6.2
  - [x] 6.4 Run feature-specific tests only
    - Run `cargo test` for all tests in `src/ollama/` module
    - Expected total: approximately 24-30 tests maximum
    - Verify all client behaviors work correctly
    - Do NOT run the entire application test suite

**Acceptance Criteria:**
- All feature-specific tests pass (approximately 24-30 tests total)
- Critical client behaviors are covered
- No more than 6 additional tests added
- `cargo clippy` passes with no warnings
- `cargo fmt --check` passes
- Integration between components works correctly

## Execution Order

Recommended implementation sequence:
1. Infrastructure Layer (Task Group 1) - Dependencies setup
2. Error Handling Layer (Task Group 2) - Error types foundation
3. Client Infrastructure Layer (Task Group 3) - Builder and client struct
4. Retry Logic Layer (Task Group 4) - Exponential backoff implementation
5. API Implementation Layer (Task Group 5) - Trait interface and generate endpoint
6. Test Review (Task Group 6) - Final verification and gap analysis

## File Structure After Completion

```
src/
  lib.rs                    # Updated: add ollama module and re-exports
  ollama/
    mod.rs                  # New: module declarations and re-exports
    client.rs               # New: OllamaClient, OllamaClientBuilder, OllamaClientTrait, OllamaError

Cargo.toml                  # Updated: reqwest, tokio, thiserror dependencies
```

## Technical Notes

- Use `reqwest::Client` with timeout configuration via `ClientBuilder`
- Default base URL: `http://172.17.64.1:11434` (WSL networking address)
- Environment variable: `OLLAMA_HOST` for endpoint override
- Timeouts: connection 5s, read 60s (hardcoded for MVP)
- Retry strategy: exponential backoff (1s, 2s, 4s), max 3 retries
- Only retry on transient errors (5xx, network), not client errors (4xx)
- Trait-based design enables mocking for unit tests
- Error types use `thiserror` for ergonomic error handling
- Builder pattern follows `NoteBuilder` conventions from `src/models/note.rs`
- All async operations use `tokio` runtime
- JSON serialization uses `serde` and `serde_json` (already in dependencies)

