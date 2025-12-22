/// Complete database schema for the notes application.
///
/// Uses CREATE TABLE/INDEX IF NOT EXISTS for idempotent execution.
/// All statements are designed to be run in a single transaction.
pub const INITIAL_SCHEMA: &str = r#"
-- Notes table: stores note content with timestamps
CREATE TABLE IF NOT EXISTS notes (
    id INTEGER PRIMARY KEY,
    content TEXT NOT NULL,
    created_at INTEGER,
    updated_at INTEGER
);

-- Tags table: stores unique tag names (case-insensitive)
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

-- Junction table: links notes to tags (many-to-many)
-- Includes AI-first metadata: confidence scores and source provenance
CREATE TABLE IF NOT EXISTS note_tags (
    note_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    confidence REAL DEFAULT 1.0,
    source TEXT DEFAULT 'user',
    created_at INTEGER,
    verified INTEGER DEFAULT 0,
    model_version TEXT,
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Tag aliases: SKOS-style prefLabel/altLabel synonym mapping
-- Maps alternate forms ("ML", "machine-learning") to canonical tag IDs
CREATE TABLE IF NOT EXISTS tag_aliases (
    alias TEXT PRIMARY KEY COLLATE NOCASE,
    canonical_tag_id INTEGER NOT NULL,
    FOREIGN KEY (canonical_tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Index for sorting notes by creation date
CREATE INDEX IF NOT EXISTS idx_notes_created ON notes(created_at);

-- Indexes for efficient junction table lookups
CREATE INDEX IF NOT EXISTS idx_note_tags_note ON note_tags(note_id);
CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);

-- Index for efficient tag alias lookup by canonical tag
CREATE INDEX IF NOT EXISTS idx_tag_aliases_canonical ON tag_aliases(canonical_tag_id);
"#;
