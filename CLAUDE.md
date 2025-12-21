# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**cons** is a structure-last personal knowledge management CLI tool. Users capture thoughts freely; AI handles all tagging and organization automatically. Local-first (SQLite + Ollama), privacy-focused, single Rust binary.

## Build and Test Commands

```bash
cargo build           # Build the project
cargo test            # Run all tests
cargo test <name>     # Run specific test
cargo clippy          # Run linter
cargo fmt             # Format code
```

## Architecture

### Layered Design

```
CLI (clap) ─────┐
                ├──> NoteService ──> SQLite
TUI (ratatui) ──┘         │
                          └──> OllamaClient ──> Ollama
```

- **NoteService**: Core business logic, UI-independent, reusable across CLI/TUI/GUI
- **OllamaClient**: Isolated AI integration, mockable for tests
- **CLI/TUI**: Thin presentation layers calling NoteService

### Current Module Structure

```
src/
  lib.rs          # Crate root, re-exports Database
  db/
    mod.rs        # Database struct, connection methods
    schema.rs     # INITIAL_SCHEMA constant (idempotent)
```

### Database Pattern

Uses **idempotent schema initialization** (IF NOT EXISTS) rather than versioned migrations. Schema runs automatically on `Database::open()` or `Database::in_memory()`. Foreign keys enabled via PRAGMA on every connection.

Tables: `notes`, `tags`, `note_tags` (junction). Indexes on created_at and both junction table foreign keys.

## Key Design Decisions

- **Fail-safe AI**: LLM failures never block note capture; notes save even if tagging fails
- **Local LLM**: Ollama with deepseek-r1:8b for privacy (no cloud API calls)
- **Async only for HTTP**: tokio for Ollama calls, sync for SQLite (local DB doesn't need async)
- **SKOS-inspired vocabulary**: Prefer/alternate labels for tag synonyms, broader/narrower for hierarchy

## Reference Documentation

- `ARCHITECTURE.md` - Database layer design decisions
- `KNOWLEDGE.md` - Information science principles for PKM design
- `agent-os/product/` - Mission, roadmap, and tech stack docs
