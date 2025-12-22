# Rust Standards for cons

Modern Rust guidelines for an AI-first CLI knowledge management tool. Rust 2021 edition.

---

## Project Structure

- **Layered architecture**: `domain` → `data` → `application` → `interface`
- Domain models own no infrastructure dependencies
- Database and LLM integrations live in dedicated modules, not scattered
- Binary crates (`src/main.rs`) are thin wrappers around library crates (`src/lib.rs`)
- Feature flags for optional integrations (e.g., `ollama`, `tui`)

## Error Handling

- Use `anyhow::Result` at application boundaries (CLI, main)
- Use `thiserror` for library code where callers need to match on error variants
- Propagate with `?`; avoid `.unwrap()` except in tests or provably-safe cases
- Fail fast on LLM/network operations—don't retry silently
- Context errors at boundaries: `.context("loading note")?`

## Ownership & Borrowing

- Prefer `&str` over `String` in function parameters
- Return owned types (`String`, `Vec<T>`) when caller needs ownership
- Use `Cow<'_, str>` when input may or may not need allocation
- Clone deliberately, not defensively—if you're cloning to satisfy the borrow checker, reconsider the design
- Avoid `Rc`/`Arc` unless shared ownership is genuinely required

## Type Design

- Newtypes for domain identifiers: `struct NoteId(i64)`
- Enums with data for tagged unions: `TagSource::Llm { model: String, confidence: f32 }`
- `#[non_exhaustive]` on public enums you may extend
- Derive `Debug` on everything; derive `Clone`, `PartialEq` when semantically meaningful
- Keep domain types free of serialization concerns—add `serde` derives only where needed

## Database (rusqlite)

- Single connection for CLI; connection pooling only if concurrency is required
- Statements via `conn.prepare_cached()` for repeated queries
- Map rows to domain types at the boundary—don't leak `rusqlite::Row` into domain
- Store timestamps as Unix INTEGER; convert to `time::OffsetDateTime` in domain
- Transactions for multi-statement writes: `conn.execute_batch()` or explicit `BEGIN`/`COMMIT`
- Migrations as numbered SQL files applied at startup

## CLI (clap)

- Derive-based definitions: `#[derive(Parser)]`
- Subcommands as enums: `#[derive(Subcommand)]`
- Keep argument parsing in the interface layer—pass parsed values to application logic
- Exit codes: 0 success, 1 user error, 2 internal error
- Stderr for errors and diagnostics; stdout for data output

## Async & LLM Integration

- Prefer sync for CLI tools unless I/O-bound concurrency is needed
- If async: `tokio` with `#[tokio::main]` or `block_on` at the boundary
- Treat LLM calls as fallible and slow—timeouts, retries with backoff if appropriate
- Store all LLM outputs immediately with provenance metadata (model, timestamp, confidence)
- Never gate capture on LLM success—fail open, log the error, continue

## API Design

- Builder pattern for complex constructors: `NoteBuilder::new().title("...").build()`
- Accept `impl Into<String>` for ergonomic string parameters
- Return `impl Iterator` over collecting into `Vec` when caller may short-circuit
- Avoid boolean parameters—use enums or builder methods for clarity
- Public API surface should be minimal; `pub(crate)` by default

## Testing

- Unit tests in `#[cfg(test)]` modules alongside code
- Integration tests in `tests/` directory
- Use `tempfile` for filesystem tests; in-memory SQLite (`:memory:`) for DB tests
- Property-based tests (`proptest`) for parsers and serialization roundtrips
- Avoid mocking where possible—use real implementations against test fixtures

## Performance & Safety

- Measure before optimizing—`cargo bench` with `criterion`
- Prefer stack allocation; `Box` only for recursive types or large data
- No `unsafe` without a safety comment explaining the invariant
- Audit dependencies: `cargo deny`, `cargo audit`
- Compile with `--release` for benchmarks and production

## Style

- `cargo fmt` with default settings, no custom rustfmt.toml unless necessary
- `cargo clippy -- -W clippy::pedantic` as baseline; suppress with justification
- Document public items with `///`; skip obvious getters
- Prefer explicit imports over globs except in preludes
- Group imports: std, external crates, crate-internal

## Dependencies

- Minimize dependency count—each dep is maintenance burden
- Pin major versions in `Cargo.toml`; use `cargo update` deliberately
- Prefer well-maintained crates with recent activity
- Vendor or feature-gate heavy dependencies (e.g., full tokio runtime)

---

*Optimized for: rusqlite, clap, time, anyhow/thiserror, Ollama integration, local-first CLI architecture.*