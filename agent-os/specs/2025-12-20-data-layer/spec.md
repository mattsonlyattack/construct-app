# Specification: Data Layer

## Goal

Establish the foundational SQLite database layer with schema for notes and tags, providing a working database connection that future CRUD operations can build upon.

## User Stories

- As a developer, I want a reliable database connection interface so that I can persist note data locally
- As a developer, I want an in-memory database option so that I can run fast unit tests without file I/O

## Specific Requirements

**Database Connection Management**
- Implement a `Database` struct that wraps rusqlite `Connection`
- Provide `open(path: impl AsRef<Path>)` method for file-based databases
- Provide `in_memory()` method for test databases
- Use `anyhow::Result` for all fallible operations
- Automatically run schema initialization on connection open

**Notes Table Schema**
- Create `notes` table with `id` as INTEGER PRIMARY KEY (auto-increment)
- Include `content` column as TEXT NOT NULL for note body
- Store `created_at` and `updated_at` as INTEGER (Unix timestamps)
- No title field; first line of content serves as preview in UI
- Create `idx_notes_created` index on `created_at` for sorting

**Tags Table Schema**
- Create `tags` table with `id` as INTEGER PRIMARY KEY
- Include `name` column as TEXT NOT NULL UNIQUE with COLLATE NOCASE
- Normalize tag names to lowercase on insert (application-level)
- Case-insensitive uniqueness enforced at database level

**Note-Tags Junction Table**
- Create `note_tags` table with composite primary key (note_id, tag_id)
- Define foreign keys to both `notes` and `tags` tables
- Use ON DELETE CASCADE for both foreign key references
- Include `confidence` column as REAL DEFAULT 1.0 for LLM confidence scores
- Include `source` column as TEXT DEFAULT 'user' to distinguish user-explicit vs llm-inferred tags
- Include `created_at` column as TEXT DEFAULT CURRENT_TIMESTAMP
- Create `idx_note_tags_note` index on `note_id`
- Create `idx_note_tags_tag` index on `tag_id`

**Tag Aliases Table (SKOS prefLabel/altLabel pattern)**
- Create `tag_aliases` table for mapping alternate forms to canonical tags
- Include `alias` column as TEXT PRIMARY KEY with COLLATE NOCASE
- Include `canonical_tag_id` column as INTEGER NOT NULL
- Define foreign key to `tags` table with ON DELETE CASCADE
- Supports synonym resolution: "ML" → "machine-learning", "NYC" → "new-york-city"

**Schema Initialization Strategy**
- Store complete schema as `INITIAL_SCHEMA` constant in `schema.rs`
- Use `CREATE TABLE IF NOT EXISTS` for idempotent execution
- Use `CREATE INDEX IF NOT EXISTS` for idempotent index creation
- Execute all statements in a single transaction for atomicity
- Skip migration tracking; document approach in ARCHITECTURE.md

**Module Organization**
- Create `src/db/` module directory
- Place schema constant in `src/db/schema.rs`
- Place Database struct and methods in `src/db/mod.rs`
- Re-export Database from `src/lib.rs`

**Testing Approach**
- Write happy path tests only per project standards
- Test that in-memory database opens successfully
- Test that schema tables exist after initialization
- Use in-memory databases for all tests to ensure fast execution

## Visual Design

No visual assets provided.

## Existing Code to Leverage

**`src/lib.rs` - Module structure pattern**
- Follow existing test module structure with `#[cfg(test)]` attribute
- Use `mod tests` pattern with `use super::*` import
- Apply consistent Rust 2024 edition conventions from Cargo.toml

## Out of Scope

- CRUD operations (create, read, update, delete methods)
- Query methods for retrieving notes or tags
- Connection pooling or async SQLite support
- Query builders or ORM abstractions
- Soft deletes or audit logging
- Full-text search capabilities
- Custom error types with thiserror (use anyhow only)
- Migration versioning or tracking tables
- Schema verification beyond IF NOT EXISTS
- Edge case, concurrency, or performance tests
