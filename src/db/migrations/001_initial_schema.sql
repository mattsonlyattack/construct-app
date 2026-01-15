-- Initial database schema
-- Creates all base tables for the notes application
-- Version: 001

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

-- Schema migrations tracking table
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL,
    description TEXT
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