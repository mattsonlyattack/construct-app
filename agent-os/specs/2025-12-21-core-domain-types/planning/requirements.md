# Spec Requirements: Core Domain Types

## Initial Description
Define Note, Tag, and related structs with proper Rust idioms (derive macros, Display implementations, builder patterns where appropriate).

Key requirements from KNOWLEDGE.md include:
- SKOS-inspired vocabulary patterns (preferred/alternate labels, broader/narrower relationships)
- AI-first metadata on all LLM-inferred data (confidence scores, provenance tracking, timestamps, verification flags)
- Property graph model fundamentals
- Fail-safe design principle (notes capturable even if AI tagging fails)
- Folksonomy-first organization

## Requirements Discussion

### First Round Questions

**Q1:** I assume the `Note` struct will be straightforward with `id`, `content`, `created_at`, and `updated_at` fields matching the database schema, using `chrono::DateTime<Utc>` for timestamps. Is that correct, or do you prefer Unix timestamps (`i64`) in the domain model as well?
**Answer:** Use the `time` crate for timestamps (not chrono).

**Q2:** For the `Tag` struct, the database already supports the SKOS pattern via the `tag_aliases` table. I'm thinking the domain type should include a `Vec<String>` for aliases (preferred label + alternates). Should the `Tag` struct load its aliases eagerly, or should we use a separate `TagWithAliases` struct for when aliases are needed?
**Answer:** Tags load aliases eagerly. Single `Tag` struct includes aliases.

**Q3:** The `note_tags` junction table stores AI metadata (`confidence`, `source`, `created_at`). I assume we need a `NoteTag` or `TagAssignment` struct to model this relationship with its metadata. Should this be a standalone struct, or should `Note` have a `Vec<TaggedWith>` field that includes the confidence/source info inline?
**Answer:** Standalone `TagAssignment` struct. `Note` holds `Vec<TagAssignment>`.

**Q4:** For the `source` field distinguishing user vs. LLM-inferred tags, I'm planning to use a Rust enum like `enum TagSource { User, Llm }` rather than raw strings. Should we add a `verified: bool` field to the tag assignment struct now, even though verification UI isn't in the MVP scope?
**Answer:** `TagSource` enum with variants `User` and `Llm`. Fail on unknown values (no fallback/default).

**Q5:** The KNOWLEDGE.md mentions a "user-verified" flag for AI inferences that starts `false`. Should we add a `verified: bool` field to the tag assignment struct now, even though verification UI isn't in the MVP scope?
**Answer:** Yes, include `verified: bool` with default `false`.

**Q6:** For builder patterns, I assume we'd use builders primarily for `Note` (optional timestamps, optional tags at creation). Should `Tag` also have a builder, or is direct construction via `Tag::new("name")` sufficient given its simplicity?
**Answer:** Builder pattern for `Note`. For `Tag`, use simple constructors: `Tag::new()` and `Tag::with_aliases()`.

**Q7:** Regarding derive macros, I plan to include `#[derive(Debug, Clone, PartialEq)]` on all domain types, plus `serde::{Serialize, Deserialize}` for JSON roundtripping (needed for Ollama API responses). Should we also derive `Eq` and `Hash` for potential use in `HashSet`/`HashMap` lookups?
**Answer:** Derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`. Do NOT derive `Hash`.

**Q8:** Is there anything that should explicitly be **out of scope** for this spec?
**Answer:** Out of scope: structured records (contacts, events, bibliographic entries), semantic relationships (supports, contradicts), note hierarchy, attachments, tag hierarchy (broader/narrower), multi-device sync.

### Existing Code to Reference
No similar existing features identified for reference.

### Follow-up Questions
None required.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

**Note struct:**
- Fields: `id` (i64), `content` (String), `created_at` (OffsetDateTime from time crate), `updated_at` (OffsetDateTime)
- Holds `Vec<TagAssignment>` for associated tags with metadata
- Builder pattern for construction with optional fields
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**Tag struct:**
- Fields: `id` (i64), `name` (String - preferred label), `aliases` (Vec<String> - eagerly loaded)
- Simple constructors: `Tag::new(name)` and `Tag::with_aliases(name, aliases)`
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**TagAssignment struct:**
- Models the note-tag relationship with AI-first metadata
- Fields: `tag_id` (i64), `confidence` (u8, 0-100 percentage), `source` (TagSource enum), `created_at` (OffsetDateTime), `verified` (bool, default false), `model_version` (Option<String>, tracks LLM version)
- Standalone struct, not nested
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**TagSource enum:**
- Variants: `User`, `Llm`
- Fail on unknown values during deserialization (no fallback)
- Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`

**Schema changes required:**
- `note_tags.created_at`: Change from TEXT to INTEGER (Unix timestamp)
- `note_tags`: Add `verified INTEGER DEFAULT 0` column
- `note_tags`: Add `model_version TEXT` column (nullable, for LLM provenance)
- `tag_aliases`: Add index on `canonical_tag_id` for efficient lookups
- On tag creation: Insert canonical name into `tag_aliases` table (denormalized for consistent alias lookup)

### Reusability Opportunities
- No existing code patterns identified for reference
- These types will be consumed by `NoteService` (next roadmap item)
- Types designed for reuse across CLI/TUI/GUI layers per layered architecture

### Scope Boundaries

**In Scope:**
- `Note` struct with builder pattern
- `Tag` struct with eager alias loading
- `TagAssignment` struct for note-tag relationships with AI metadata
- `TagSource` enum (User, Llm)
- Derive macros: Debug, Clone, PartialEq, Eq, Serialize, Deserialize
- Display implementations where appropriate
- Schema migrations for new columns and indexes
- Unit tests for all types

**Out of Scope:**
- Structured record types (contacts, events, bibliographic entries)
- Semantic relationship types (supports, contradicts, extends)
- Note hierarchy/nesting
- Attachments/file references
- Tag hierarchy (broader/narrower SKOS relationships)
- Multi-device sync concerns
- Hash derive (explicitly excluded)

### Technical Considerations
- Use `time` crate for all timestamp handling (not chrono)
- Timestamps stored as INTEGER (Unix timestamps) in database
- Confidence scores as u8 percentage (0-100)
- TagSource enum must fail on unknown values (strict parsing)
- Canonical tag name inserted into tag_aliases on tag creation for uniform alias lookup
- All types must be serializable for Ollama API JSON roundtripping
- Types designed for use with rusqlite for database operations
