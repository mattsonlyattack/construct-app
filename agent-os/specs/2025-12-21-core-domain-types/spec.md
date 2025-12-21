# Specification: Core Domain Types

## Goal
Define the foundational Rust domain types (`Note`, `Tag`, `TagAssignment`, `TagSource`) with proper idioms, derive macros, and constructors to serve as the data layer between the SQLite database and the `NoteService` business logic.

## User Stories
- As a developer, I want well-defined domain types so that I can build the NoteService layer with clear data contracts
- As a developer, I want AI metadata (confidence, source, verified) on tag assignments so that LLM-inferred tags are distinguishable from user-created ones

## Specific Requirements

**Note struct**
- Fields: `id: i64`, `content: String`, `created_at: OffsetDateTime`, `updated_at: OffsetDateTime`, `tags: Vec<TagAssignment>`
- Use `time::OffsetDateTime` for all timestamp fields
- Implement builder pattern via `NoteBuilder` for flexible construction with optional fields
- Builder should allow setting `tags` as empty by default
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**Tag struct**
- Fields: `id: i64`, `name: String` (preferred label), `aliases: Vec<String>` (eagerly loaded)
- Provide `Tag::new(id, name)` constructor that sets `aliases` to empty vec
- Provide `Tag::with_aliases(id, name, aliases)` constructor for full initialization
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**TagAssignment struct**
- Models the note-tag relationship with AI-first metadata
- Fields: `tag_id: i64`, `confidence: u8`, `source: TagSource`, `created_at: OffsetDateTime`, `verified: bool`, `model_version: Option<String>`
- Confidence range: 0-100 (percentage); user-created tags default to 100
- `verified` defaults to `false` for all sources
- `model_version` tracks which LLM produced the inference (None for user-created)
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**TagSource enum**
- Variants: `User`, `Llm`
- Serde fails on unknown enum variants by default (no special attribute needed)
- Use `#[serde(rename_all = "lowercase")]` to serialize as "user"/"llm" matching Display output
- Implement `Display` trait for human-readable output
- Derive: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**Schema migration: note_tags.created_at**
- Change column type from TEXT to INTEGER (Unix timestamp in seconds)
- Existing data migration not required (no production data exists per ARCHITECTURE.md)
- Update INITIAL_SCHEMA constant in `src/db/schema.rs`

**Schema migration: note_tags.verified column**
- Add `verified INTEGER DEFAULT 0` column to note_tags table
- 0 = false, 1 = true (SQLite boolean convention)
- Default ensures existing rows and new LLM-inferred tags start unverified

**Schema migration: note_tags.model_version column**
- Add `model_version TEXT` column to note_tags table (nullable)
- Stores LLM model identifier (e.g., "deepseek-r1:8b") for provenance tracking
- NULL for user-created tags

**Schema migration: tag_aliases index**
- Add index `idx_tag_aliases_canonical` on `tag_aliases(canonical_tag_id)`
- Enables efficient lookup of all aliases for a given tag

**Schema migration: canonical name in tag_aliases**
- On tag creation, insert the canonical tag name into `tag_aliases` table
- Enables uniform alias lookup without special-casing the preferred label
- This is a behavioral requirement for NoteService, not a schema change

## Visual Design
No visual assets provided.

## Existing Code to Leverage

**`/home/md/construct-app/src/db/schema.rs` - INITIAL_SCHEMA constant**
- Contains current idempotent schema using IF NOT EXISTS pattern
- Modify existing note_tags table definition to add verified column and change created_at type
- Add new index for tag_aliases.canonical_tag_id
- Follow existing naming convention: `idx_{table}_{column}`

**`/home/md/construct-app/src/db/mod.rs` - Database struct**
- Provides connection management and schema initialization
- New domain types will be consumed by future NoteService that uses this Database
- Test patterns (in_memory database, table existence checks) should be replicated for new types

**`/home/md/construct-app/Cargo.toml` - Dependencies**
- Add `time` crate with serde feature: `time = { version = "0.3", features = ["serde", "serde-human-readable"] }`
- Add `serde` and `serde_json` for derive macros: `serde = { version = "1.0", features = ["derive"] }`
- Already has `rusqlite` and `anyhow` which will be used for database operations

**`/home/md/construct-app/src/lib.rs` - Crate root**
- Add new `models` module and re-export domain types
- Follow existing pattern: `pub mod models;` and `pub use models::{Note, Tag, TagAssignment, TagSource};`

## Out of Scope
- Structured record types (contacts, events, bibliographic entries)
- Semantic relationship types (supports, contradicts, extends)
- Note hierarchy or nesting
- Attachments or file references
- Tag hierarchy (broader/narrower SKOS relationships)
- Multi-device sync concerns
- Hash derive on any domain types
- NoteService implementation (separate spec)
- Database CRUD operations (separate spec)
- Ollama/LLM integration (separate spec)
