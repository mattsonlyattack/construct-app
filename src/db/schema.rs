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
CREATE TABLE IF NOT EXISTS note_tags (
    note_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (note_id, tag_id),
    FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Index for sorting notes by creation date
CREATE INDEX IF NOT EXISTS idx_notes_created ON notes(created_at);

-- Indexes for efficient junction table lookups
CREATE INDEX IF NOT EXISTS idx_note_tags_note ON note_tags(note_id);
CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);
"#;
