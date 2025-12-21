# Spec Requirements: Data Layer

**Milestone 1.1** | Target: ~200 LOC | Philosophy: "Ship in 3 weeks with good bones"

## Goal

Working SQLite database with schema for notes and tags.

## Dependencies

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
anyhow = "1.0"
thiserror = "1.0"  # included but not actively used yet
```

## Schema

| Table | Columns | Notes |
|-------|---------|-------|
| notes | id (PK), content (TEXT NOT NULL), created_at (INTEGER), updated_at (INTEGER) | No title field - first line is preview |
| tags | id (PK), name (TEXT NOT NULL UNIQUE COLLATE NOCASE) | Normalize to lowercase on insert |
| note_tags | note_id, tag_id (composite PK) | ON DELETE CASCADE for both FKs |

**Indexes:** idx_notes_created, idx_note_tags_note, idx_note_tags_tag

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Migration tracking | Defer | Use idempotent IF NOT EXISTS; document in ARCHITECTURE.md |
| Schema verification | Skip | Trust existing file; IF NOT EXISTS is safe |
| Error handling | anyhow only | Defer thiserror to post-MVP |
| Testing | Happy paths only | Skip FK/concurrent/edge/perf tests |

## Deliverables

- `src/db/schema.rs` - INITIAL_SCHEMA constant with CREATE TABLE/INDEX statements
- `src/db/mod.rs` - Database struct with `open(path)` and `in_memory()` methods
- Basic tests verifying database opens and schema exists

## Out of Scope

- CRUD operations, query methods (future milestone)
- Connection pooling, async SQLite
- Query builders, ORM abstractions
- Soft deletes, audit logs
- Full-text search (week 3)
- Custom error types, migration versioning

## Patterns to Apply

- **NoteService pattern:** Single source of truth (similar to Redux)
- **Graceful degradation:** AI components fail safe without breaking core functionality
