mod schema;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use schema::{FTS_TABLE_CREATION, FTS_TRIGGERS, INITIAL_SCHEMA, MIGRATIONS};

/// Database wrapper providing connection management and schema initialization.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens an in-memory SQLite database.
    ///
    /// Automatically initializes the schema on connection open.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Opens a file-based SQLite database at the given path.
    ///
    /// Creates the database file if it does not exist.
    /// Automatically initializes the schema on connection open.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Initializes the database schema.
    ///
    /// Executes all schema statements in a single transaction.
    /// Uses IF NOT EXISTS for idempotent execution.
    /// Runs migrations for column additions, ignoring "duplicate column" errors.
    /// Creates FTS5 virtual table and triggers if they don't exist.
    /// Populates FTS index from existing notes.
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute("PRAGMA foreign_keys = ON", [])?;
        self.conn.execute_batch(INITIAL_SCHEMA)?;

        // Execute migrations line by line, ignoring "duplicate column" errors
        // This allows idempotent execution on both fresh and existing databases
        for statement in MIGRATIONS.lines() {
            let trimmed = statement.trim();
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }

            // Execute ALTER TABLE statements, ignoring duplicate column errors
            match self.conn.execute(trimmed, []) {
                Ok(_) => {}
                Err(rusqlite::Error::SqliteFailure(err, msg)) => {
                    // Check if this is a "duplicate column name" error
                    // SQLite returns SQLITE_ERROR (code 1) for duplicate columns
                    let is_duplicate_column = msg
                        .as_ref()
                        .map(|s| s.contains("duplicate column"))
                        .unwrap_or(false);

                    if !is_duplicate_column {
                        // Not a duplicate column error, propagate it
                        return Err(rusqlite::Error::SqliteFailure(err, msg).into());
                    }
                    // Otherwise, silently ignore duplicate column errors
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Initialize FTS5 virtual table and triggers
        self.initialize_fts()?;

        Ok(())
    }

    /// Initializes FTS5 virtual table, triggers, and populates the index.
    ///
    /// FTS5 does NOT support IF NOT EXISTS, so we check sqlite_master first.
    /// After creating the table and triggers, we populate the index from existing notes.
    fn initialize_fts(&self) -> Result<()> {
        // Check if FTS table already exists
        let fts_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='notes_fts')",
            [],
            |row| row.get(0),
        )?;

        if !fts_exists {
            // Create FTS virtual table
            self.conn.execute_batch(FTS_TABLE_CREATION)?;
        }

        // Create triggers (idempotent with IF NOT EXISTS)
        self.conn.execute_batch(FTS_TRIGGERS)?;

        // Populate/rebuild FTS index from existing notes
        // This handles both fresh creation and existing databases where FTS might be stale
        self.populate_fts_index()?;

        Ok(())
    }

    /// Populates the FTS index from existing notes and tags.
    ///
    /// Clears the existing FTS index and rebuilds it from the notes table.
    /// This operation is idempotent and safe to run on every database open.
    fn populate_fts_index(&self) -> Result<()> {
        // Clear existing FTS index
        self.conn.execute("DELETE FROM notes_fts", [])?;

        // Rebuild from notes and tags
        self.conn.execute(
            "INSERT INTO notes_fts (note_id, content, content_enhanced, tags)
             SELECT
                 n.id,
                 n.content,
                 n.content_enhanced,
                 (SELECT GROUP_CONCAT(t.name, ' ')
                  FROM note_tags nt
                  JOIN tags t ON nt.tag_id = t.id
                  WHERE nt.note_id = n.id)
             FROM notes n",
            [],
        )?;

        Ok(())
    }

    /// Returns a reference to the underlying connection.
    ///
    /// Useful for executing custom queries in tests or future CRUD operations.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests;
