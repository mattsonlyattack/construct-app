//! Shared utility functions for database and tag operations.
//!
//! These functions are reused across the CLI and TUI interfaces.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{Database, TagAssignment};

/// Gets the cross-platform database path.
///
/// Returns the path as `{data_dir}/cons/notes.db` where `data_dir` is:
/// - Linux: `~/.local/share`
/// - macOS: `~/Library/Application Support`
/// - Windows: `C:\Users\<user>\AppData\Roaming`
///
/// # Errors
///
/// Returns an error if the data directory cannot be determined.
pub fn get_database_path() -> Result<PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Failed to determine data directory"))?;

    Ok(data_dir.join("cons").join("notes.db"))
}

/// Ensures the parent directory of the database file exists.
///
/// Creates the directory structure if it doesn't exist using `create_dir_all`.
///
/// # Errors
///
/// Returns an error if directory creation fails.
pub fn ensure_database_directory(db_path: &Path) -> Result<()> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create database directory: {}", parent.display())
        })?;
    }
    Ok(())
}

/// Gets tag names from the database for the given tag assignments.
///
/// Uses a single batch query with IN clause for efficiency.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn get_tag_names(db: &Database, tag_assignments: &[TagAssignment]) -> Result<Vec<String>> {
    if tag_assignments.is_empty() {
        return Ok(Vec::new());
    }

    let conn = db.connection();
    let tag_ids: Vec<i64> = tag_assignments.iter().map(|ta| ta.tag_id().get()).collect();

    // Build query with placeholders
    let placeholders: Vec<String> = (0..tag_ids.len()).map(|_| "?".to_string()).collect();
    let query = format!(
        "SELECT name FROM tags WHERE id IN ({})",
        placeholders.join(", ")
    );

    let mut stmt = conn
        .prepare(&query)
        .context("Failed to prepare tag query")?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(tag_ids.iter()), |row| {
            row.get::<_, String>(0)
        })
        .context("Failed to query tag names")?;

    let mut names = Vec::new();
    for row_result in rows {
        names.push(row_result.context("Failed to read tag name")?);
    }

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::NoteService;

    #[test]
    fn get_database_path_returns_valid_path() {
        let path = get_database_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("cons"));
        assert!(path.to_string_lossy().contains("notes.db"));
    }

    #[test]
    fn get_tag_names_resolves_tag_ids_to_display_names() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create a note with tags to ensure tags exist in database
        let note = service
            .create_note("Test note", Some(&["rust", "programming"]))
            .expect("failed to create note");

        // Test batch tag name resolution using the database from the service
        let tag_names =
            get_tag_names(service.database(), note.tags()).expect("failed to get tag names");

        assert_eq!(tag_names.len(), 2, "should have 2 tags");
        assert!(
            tag_names.contains(&"rust".to_string()),
            "should contain rust"
        );
        assert!(
            tag_names.contains(&"programming".to_string()),
            "should contain programming"
        );
    }

    #[test]
    fn get_tag_names_returns_empty_for_empty_assignments() {
        let db = Database::in_memory().expect("failed to create in-memory database");

        // Query with empty tag assignments
        let tag_names =
            get_tag_names(&db, &[]).expect("get_tag_names should not error for empty assignments");

        assert!(
            tag_names.is_empty(),
            "should return empty vec for empty assignments"
        );
    }
}
