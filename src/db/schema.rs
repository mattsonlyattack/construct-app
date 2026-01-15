pub use super::migration::apply_pending_migrations;
// Re-export for tests
#[cfg(test)]
pub use super::migration::{Migration, MIGRATIONS};

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
