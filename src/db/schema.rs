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
-- Includes provenance tracking: source (user/llm), confidence, timestamps, model version
CREATE TABLE IF NOT EXISTS tag_aliases (
    alias TEXT PRIMARY KEY COLLATE NOCASE,
    canonical_tag_id INTEGER NOT NULL,
    source TEXT NOT NULL,
    confidence REAL NOT NULL,
    created_at INTEGER NOT NULL,
    model_version TEXT,
    FOREIGN KEY (canonical_tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Edges table: stores weighted, typed, temporal relationships between tags
-- Enables spreading activation retrieval and historical knowledge queries
-- Uses XKOS hierarchy semantics: generic (is-a), partitive (part-of), or NULL (non-hierarchical)
CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY,
    source_tag_id INTEGER NOT NULL,
    target_tag_id INTEGER NOT NULL,
    confidence REAL,
    hierarchy_type TEXT CHECK (hierarchy_type IN ('generic', 'partitive')),
    valid_from INTEGER,
    valid_until INTEGER,
    source TEXT DEFAULT 'user',
    model_version TEXT,
    verified INTEGER DEFAULT 0,
    created_at INTEGER,
    updated_at INTEGER,
    FOREIGN KEY (source_tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    FOREIGN KEY (target_tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Index for sorting notes by creation date
CREATE INDEX IF NOT EXISTS idx_notes_created ON notes(created_at);

-- Indexes for efficient junction table lookups
CREATE INDEX IF NOT EXISTS idx_note_tags_note ON note_tags(note_id);
CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);

-- Index for efficient tag alias lookup by canonical tag
CREATE INDEX IF NOT EXISTS idx_tag_aliases_canonical ON tag_aliases(canonical_tag_id);

-- Indexes for efficient edges table queries
CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_tag_id);
CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_tag_id);
CREATE INDEX IF NOT EXISTS idx_edges_created_at ON edges(created_at);
CREATE INDEX IF NOT EXISTS idx_edges_updated_at ON edges(updated_at);
CREATE INDEX IF NOT EXISTS idx_edges_hierarchy_type ON edges(hierarchy_type);
"#;

/// Schema migrations for adding new columns to existing tables.
///
/// These ALTER TABLE statements are executed after INITIAL_SCHEMA.
/// SQLite doesn't support IF NOT EXISTS for ALTER TABLE ADD COLUMN,
/// so we handle duplicate column errors gracefully in initialize_schema().
pub const MIGRATIONS: &str = r#"
-- Add enhancement fields to notes table
-- These columns store LLM-enhanced versions of fragmentary notes with provenance metadata
ALTER TABLE notes ADD COLUMN content_enhanced TEXT;
ALTER TABLE notes ADD COLUMN enhanced_at INTEGER;
ALTER TABLE notes ADD COLUMN enhancement_model TEXT;
ALTER TABLE notes ADD COLUMN enhancement_confidence REAL;

-- Add degree centrality tracking to tags table
-- Stores the total number of edges connected to this tag (incoming + outgoing)
ALTER TABLE tags ADD COLUMN degree_centrality INTEGER DEFAULT 0;
"#;

/// FTS5 virtual table creation SQL.
///
/// FTS5 does NOT support IF NOT EXISTS, so this must be executed conditionally
/// by checking sqlite_master first in initialize_schema().
pub const FTS_TABLE_CREATION: &str = r#"
CREATE VIRTUAL TABLE notes_fts USING fts5(
    note_id UNINDEXED,
    content,
    content_enhanced,
    tags,
    tokenize='porter'
);
"#;

/// FTS5 synchronization triggers.
///
/// These triggers keep the FTS index in sync with the notes and note_tags tables.
/// Triggers must be created AFTER the FTS virtual table exists.
pub const FTS_TRIGGERS: &str = r#"
-- Trigger: Sync FTS on note INSERT
CREATE TRIGGER IF NOT EXISTS notes_fts_insert AFTER INSERT ON notes
BEGIN
    INSERT INTO notes_fts (note_id, content, content_enhanced, tags)
    SELECT
        NEW.id,
        NEW.content,
        NEW.content_enhanced,
        (SELECT GROUP_CONCAT(t.name, ' ')
         FROM note_tags nt
         JOIN tags t ON nt.tag_id = t.id
         WHERE nt.note_id = NEW.id);
END;

-- Trigger: Sync FTS on note UPDATE
CREATE TRIGGER IF NOT EXISTS notes_fts_update AFTER UPDATE ON notes
BEGIN
    DELETE FROM notes_fts WHERE note_id = OLD.id;
    INSERT INTO notes_fts (note_id, content, content_enhanced, tags)
    SELECT
        NEW.id,
        NEW.content,
        NEW.content_enhanced,
        (SELECT GROUP_CONCAT(t.name, ' ')
         FROM note_tags nt
         JOIN tags t ON nt.tag_id = t.id
         WHERE nt.note_id = NEW.id);
END;

-- Trigger: Sync FTS on note DELETE
CREATE TRIGGER IF NOT EXISTS notes_fts_delete AFTER DELETE ON notes
BEGIN
    DELETE FROM notes_fts WHERE note_id = OLD.id;
END;

-- Trigger: Sync FTS on note_tags INSERT
CREATE TRIGGER IF NOT EXISTS notes_fts_tags_insert AFTER INSERT ON note_tags
BEGIN
    DELETE FROM notes_fts WHERE note_id = NEW.note_id;
    INSERT INTO notes_fts (note_id, content, content_enhanced, tags)
    SELECT
        n.id,
        n.content,
        n.content_enhanced,
        (SELECT GROUP_CONCAT(t.name, ' ')
         FROM note_tags nt
         JOIN tags t ON nt.tag_id = t.id
         WHERE nt.note_id = n.id)
    FROM notes n
    WHERE n.id = NEW.note_id;
END;

-- Trigger: Sync FTS on note_tags DELETE
CREATE TRIGGER IF NOT EXISTS notes_fts_tags_delete AFTER DELETE ON note_tags
BEGIN
    DELETE FROM notes_fts WHERE note_id = OLD.note_id;
    INSERT INTO notes_fts (note_id, content, content_enhanced, tags)
    SELECT
        n.id,
        n.content,
        n.content_enhanced,
        (SELECT GROUP_CONCAT(t.name, ' ')
         FROM note_tags nt
         JOIN tags t ON nt.tag_id = t.id
         WHERE nt.note_id = n.id)
    FROM notes n
    WHERE n.id = OLD.note_id;
END;
"#;
