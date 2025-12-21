# Architecture

## Database Layer

### Schema Initialization Strategy

The application uses an **idempotent schema initialization** approach rather than a versioned migration system:

1. **IF NOT EXISTS Pattern**: All `CREATE TABLE` and `CREATE INDEX` statements use `IF NOT EXISTS` clauses, making them safe to run multiple times without error.

2. **Automatic Initialization**: Schema initialization runs automatically when opening a database connection via `Database::open()` or `Database::in_memory()`. This ensures the schema always exists before any operations.

3. **Single Transaction**: All schema statements execute within a single `execute_batch` call for atomicity.

4. **Foreign Key Enforcement**: `PRAGMA foreign_keys = ON` is set on every connection to enable referential integrity.

### Why No Migration Tracking?

Migration versioning is intentionally deferred for this MVP phase:

- The application is pre-1.0 with no production data to migrate
- IF NOT EXISTS provides sufficient safety for schema changes that only add new tables/indexes
- A full migration system adds complexity without immediate benefit

When the application matures and requires schema modifications to existing tables, a proper migration tracking system will be implemented.

### Module Structure

```
src/
  lib.rs          # Crate root, re-exports Database
  db/
    mod.rs        # Database struct and connection methods
    schema.rs     # INITIAL_SCHEMA constant
```

### Database Tables

| Table | Purpose |
|-------|---------|
| notes | Stores note content with timestamps |
| tags | Stores unique tag names (case-insensitive) |
| note_tags | Junction table for many-to-many relationship |

### Indexes

| Index | Purpose |
|-------|---------|
| idx_notes_created | Sorting notes by creation date |
| idx_note_tags_note | Finding tags for a note |
| idx_note_tags_tag | Finding notes with a tag |
