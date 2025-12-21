# Tech Stack

## Language and Runtime

### Rust
- **Purpose**: Primary implementation language
- **Rationale**:
  - Single-binary distribution with no runtime dependencies
  - Memory safety without garbage collection overhead
  - Excellent error handling with Result types
  - Strong type system catches bugs at compile time
  - Performance suitable for responsive CLI tool
  - Demonstrates systems programming competence for Oxide application

### Cargo
- **Purpose**: Build system and package manager
- **Rationale**: Standard Rust toolchain, handles dependencies and builds

## Database

### SQLite
- **Purpose**: Local persistent storage for notes, tags, and relationships
- **Rationale**:
  - Local-first: data stays on user's machine
  - Embedded: no separate database server to install or manage
  - Proven reliability: billions of deployments
  - Single file: easy backup and portability
  - FTS5 extension: built-in full-text search capability

### SQLite FTS5
- **Purpose**: Full-text search indexing
- **Rationale**:
  - Native SQLite extension, no external dependencies
  - Handles tokenization, stemming, and relevance ranking
  - Efficient for the expected note volume (thousands of notes)

## AI / LLM

### Ollama
- **Purpose**: Local LLM inference for auto-tagging
- **Rationale**:
  - Local-first: no API keys, no cloud dependency, no per-request costs
  - Privacy: note content never leaves user's machine
  - Offline capable: works without internet
  - Simple HTTP API: easy integration via reqwest

### deepseek-r1:8b
- **Purpose**: Language model for tag extraction
- **Rationale**:
  - Good balance of capability and resource requirements
  - Runs on consumer hardware (8GB+ RAM)
  - Sufficient for tag extraction task (not general chat)
  - Can be swapped for other Ollama-compatible models

## CLI Framework

### clap
- **Purpose**: Command-line argument parsing
- **Rationale**:
  - Derive macros for clean, declarative CLI definitions
  - Automatic help generation
  - Subcommand support (add, list, search)
  - Type-safe argument handling
  - De facto standard in Rust ecosystem

## Async Runtime

### tokio
- **Purpose**: Async runtime for HTTP calls to Ollama
- **Rationale**:
  - Required for async/await with reqwest
  - Only used for Ollama API calls, not SQLite (sync is fine for local DB)
  - Industry-standard async runtime for Rust

### reqwest
- **Purpose**: HTTP client for Ollama API
- **Rationale**:
  - Ergonomic async HTTP client
  - Built on tokio
  - Handles JSON serialization with serde

## Serialization

### serde + serde_json
- **Purpose**: JSON serialization for Ollama API communication
- **Rationale**:
  - Standard Rust serialization framework
  - Derive macros for zero-boilerplate struct serialization
  - Required for Ollama HTTP API

## Future: Terminal UI

### ratatui
- **Purpose**: Terminal UI framework (Week 4 stretch goal)
- **Rationale**:
  - Modern fork of tui-rs with active maintenance
  - Declarative widget-based rendering
  - Same process as CLI, reuses NoteService directly
  - Proves layered architecture works

## Future: Desktop GUI

### Tauri (or egui)
- **Purpose**: Desktop application (post-MVP)
- **Rationale**:
  - Tauri: web-based UI with Rust backend, small binary size
  - egui: immediate-mode GUI, pure Rust, simpler but less polished
  - Decision deferred until after MVP validation

## Development Tools

### clippy
- **Purpose**: Rust linter
- **Rationale**: Catches common mistakes, enforces idioms

### rustfmt
- **Purpose**: Code formatter
- **Rationale**: Consistent style, automated formatting

### GitHub Actions
- **Purpose**: CI/CD pipeline
- **Rationale**:
  - Automated testing on push/PR
  - Linting and format checks
  - Standard for open source projects

## Testing

### Rust built-in test framework
- **Purpose**: Unit and integration testing
- **Rationale**:
  - Built into language, no external dependencies
  - `#[test]` attribute for test functions
  - `cargo test` runs all tests

### tempfile (crate)
- **Purpose**: Temporary directories for integration tests
- **Rationale**: Clean test isolation for SQLite database tests

## Architecture Principles

### Layered Architecture
```
CLI (clap) ─────┐
                ├──> NoteService ──> SQLite
TUI (ratatui) ──┘         │
                          └──> OllamaClient ──> Ollama
```

- **NoteService**: Core business logic, no UI dependencies
- **OllamaClient**: Isolated AI integration, mockable for tests
- **CLI/TUI/GUI**: Thin presentation layers calling NoteService

### Error Handling
- Use `thiserror` or `anyhow` for ergonomic error types
- Fail-safe design: LLM failures never block note capture
- User-friendly messages for CLI errors

### Configuration
- XDG Base Directory compliance for database location
- Environment variables for Ollama URL override
- Sensible defaults that work out of the box
