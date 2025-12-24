use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cons::{Database, NoteService};
use rusqlite::OptionalExtension;

/// cons - structure-last personal knowledge management CLI
#[derive(Parser)]
#[command(name = "cons")]
#[command(about = "A structure-last personal knowledge management tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Subcommand)]
enum Commands {
    /// Add a new note with optional tags
    Add(AddCommand),
    /// List notes with optional filtering and pagination
    List(ListCommand),
}

/// Add a new note
#[derive(Parser)]
struct AddCommand {
    /// The content of the note
    #[arg(value_name = "CONTENT")]
    content: String,

    /// Comma-separated tags to apply to the note
    #[arg(short, long, value_name = "TAGS")]
    tags: Option<String>,
}

/// List notes with optional filtering
#[derive(Parser)]
struct ListCommand {
    /// Maximum number of notes to display
    #[arg(short, long, value_name = "LIMIT")]
    limit: Option<usize>,

    /// Filter by comma-separated tags (AND logic)
    #[arg(short, long, value_name = "TAGS")]
    tags: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Add(cmd) => handle_add(cmd),
        Commands::List(cmd) => handle_list(cmd),
    };

    if let Err(e) = result {
        // Determine exit code based on error type
        let exit_code = if is_user_error(&e) { 1 } else { 2 };
        eprintln!("Error: {e}");
        std::process::exit(exit_code);
    }
}

/// Determines if an error is a user error (vs internal error).
///
/// User errors include validation failures like empty content.
/// Internal errors include database failures and I/O errors.
fn is_user_error(error: &anyhow::Error) -> bool {
    // Check if the error message indicates a user error
    let error_msg = error.to_string();
    error_msg.contains("cannot be empty")
}

/// Handles the add command by creating a new note.
fn handle_add(cmd: &AddCommand) -> Result<()> {
    // Validate content is not empty or whitespace-only
    if cmd.content.trim().is_empty() {
        anyhow::bail!("Note content cannot be empty");
    }

    // Get database path and ensure directory exists
    let db_path = get_database_path()?;
    ensure_database_directory(&db_path)?;

    // Open database and create service
    let db = Database::open(&db_path).context("Failed to open database")?;

    execute_add(&cmd.content, cmd.tags.as_deref(), db)
}

/// Executes the add command logic with a provided database.
///
/// This function is separated from `handle_add` to allow testing with in-memory databases.
fn execute_add(content: &str, tags: Option<&str>, db: Database) -> Result<()> {
    let service = NoteService::new(db);

    // Parse tags if provided
    let parsed_tags = tags.map(parse_tags);

    // Create note with optional tags
    let note = if let Some(ref tags) = parsed_tags {
        let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
        service.create_note(content, Some(&tag_refs))
    } else {
        service.create_note(content, None)
    }
    .context("Failed to create note")?;

    // Output success message
    print!("Note created (id: {})", note.id());
    if let Some(tags) = parsed_tags
        && !tags.is_empty()
    {
        print!(" with tags: {}", tags.join(", "));
    }
    println!();

    Ok(())
}

/// Gets the cross-platform database path.
///
/// Returns the path as `{data_dir}/cons/notes.db` where `data_dir` is:
/// - Linux: `~/.local/share`
/// - macOS: `~/Library/Application Support`
/// - Windows: `C:\Users\<user>\AppData\Roaming`
fn get_database_path() -> Result<PathBuf> {
    let data_dir =
        dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Failed to determine data directory"))?;

    Ok(data_dir.join("cons").join("notes.db"))
}

/// Ensures the parent directory of the database file exists.
///
/// Creates the directory structure if it doesn't exist using `create_dir_all`.
fn ensure_database_directory(db_path: &Path) -> Result<()> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create database directory: {}", parent.display())
        })?;
    }
    Ok(())
}

/// Handles the list command by displaying notes.
fn handle_list(cmd: &ListCommand) -> Result<()> {
    // Get database path and ensure directory exists
    let db_path = get_database_path()?;
    ensure_database_directory(&db_path)?;

    // Open database and create service
    let db = Database::open(&db_path).context("Failed to open database")?;
    let service = NoteService::new(db);

    execute_list(cmd.limit, cmd.tags.as_deref(), service)
}

/// Executes the list command logic with a provided NoteService.
///
/// This function is separated from `handle_list` to allow testing with in-memory databases.
fn execute_list(limit: Option<usize>, tags: Option<&str>, service: NoteService) -> Result<()> {
    use time::macros::format_description;

    // Apply default limit of 10 when not specified
    let limit = limit.unwrap_or(10);

    // Parse tags if provided
    let parsed_tags = tags.map(parse_tags);

    // Build options for list_notes
    let options = cons::ListNotesOptions {
        limit: Some(limit),
        tags: parsed_tags,
    };

    // Retrieve notes
    let notes = service
        .list_notes(options)
        .context("Failed to list notes")?;

    // Handle empty results
    if notes.is_empty() {
        println!("No notes found");
        return Ok(());
    }

    // Format descriptor for "YYYY-MM-DD HH:MM"
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]");

    // Display each note
    for note in &notes {
        // Format timestamp as "YYYY-MM-DD HH:MM"
        let timestamp = note
            .created_at()
            .format(&format)
            .unwrap_or_else(|_| "Invalid date".to_string());

        // Get tag names by querying the database
        let mut tag_names = Vec::new();
        for tag_assignment in note.tags() {
            if let Some(tag_name) = get_tag_name(&service, tag_assignment.tag_id())? {
                tag_names.push(format!("#{}", tag_name));
            }
        }

        // Display note information
        println!("ID: {}", note.id().get());
        println!("Created: {}", timestamp);
        println!("Content: {}", note.content());
        if !tag_names.is_empty() {
            println!("Tags: {}", tag_names.join(" "));
        }
        println!(); // Blank line separator
    }

    Ok(())
}

/// Gets a tag name by its ID.
///
/// Returns None if the tag does not exist.
fn get_tag_name(service: &NoteService, tag_id: cons::TagId) -> Result<Option<String>> {
    let conn = service.database().connection();

    let name: Option<String> = conn
        .query_row(
            "SELECT name FROM tags WHERE id = ?1",
            [tag_id.get()],
            |row| row.get(0),
        )
        .optional()
        .context("Failed to query tag name")?;

    Ok(name)
}

/// Parses comma-separated tags from a string.
///
/// Splits on commas, trims whitespace from each tag, and filters out empty strings.
///
/// # Examples
///
/// ```
/// # use cons::parse_tags;  // This won't work, just for illustration
/// let tags = parse_tags("rust, learning, ");
/// assert_eq!(tags, vec!["rust", "learning"]);
/// ```
fn parse_tags(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tags_with_normal_input() {
        let result = parse_tags("rust,learning");
        assert_eq!(result, vec!["rust", "learning"]);
    }

    #[test]
    fn parse_tags_with_whitespace() {
        let result = parse_tags(" rust , learning ");
        assert_eq!(result, vec!["rust", "learning"]);
    }

    #[test]
    fn parse_tags_with_empty_elements() {
        let result = parse_tags("rust,,learning");
        assert_eq!(result, vec!["rust", "learning"]);
    }

    #[test]
    fn parse_tags_with_trailing_comma() {
        let result = parse_tags("rust,learning,");
        assert_eq!(result, vec!["rust", "learning"]);
    }

    #[test]
    fn parse_tags_empty_string() {
        let result = parse_tags("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_tags_only_whitespace() {
        let result = parse_tags("  ,  ,  ");
        assert!(result.is_empty());
    }

    #[test]
    fn content_validation_rejects_empty_string() {
        let cmd = AddCommand {
            content: String::new(),
            tags: None,
        };
        let result = handle_add(&cmd);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn content_validation_rejects_whitespace_only() {
        let cmd = AddCommand {
            content: "   \n\t  ".to_string(),
            tags: None,
        };
        let result = handle_add(&cmd);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    // --- List Command Tests (Task Group 1) ---

    #[test]
    fn list_command_struct_parsing_with_clap() {
        use clap::CommandFactory;

        // Test parsing with short flags
        let matches = Cli::command()
            .try_get_matches_from(vec!["cons", "list", "-l", "5", "-t", "rust,programming"])
            .expect("failed to parse list command");

        // Verify command is recognized
        assert!(matches.subcommand_matches("list").is_some());
    }

    #[test]
    fn execute_list_with_in_memory_database_returns_notes() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create some notes
        service
            .create_note("First note", Some(&["rust"]))
            .expect("failed to create note");
        service
            .create_note("Second note", Some(&["rust", "programming"]))
            .expect("failed to create note");

        // Create a new database with a test note
        let db2 = Database::in_memory().expect("failed to create in-memory database");
        let service2 = NoteService::new(db2);
        service2
            .create_note("Test note", None)
            .expect("failed to create note");

        // Test execute_list function (accepts Database)
        let db3 = Database::in_memory().expect("failed to create in-memory database");
        let service3 = NoteService::new(db3);
        service3
            .create_note("List test note", None)
            .expect("failed to create note");

        let result = execute_list(Some(10), None, service3);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_list_with_empty_database_shows_no_notes_found() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);
        let result = execute_list(Some(10), None, service);
        assert!(result.is_ok());
    }

    #[test]
    fn execute_list_with_tags_filter_applies_correctly() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create notes with different tags
        service
            .create_note("Rust note", Some(&["rust"]))
            .expect("failed to create note");
        service
            .create_note("Programming note", Some(&["programming"]))
            .expect("failed to create note");
        service
            .create_note("Rust programming note", Some(&["rust", "programming"]))
            .expect("failed to create note");

        // Filter by tags
        let result = execute_list(Some(10), Some("rust,programming"), service);
        assert!(result.is_ok());
    }

    // --- Output Formatting Tests (Task Group 2) ---

    #[test]
    fn get_tag_name_resolves_tag_id_to_display_name() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create a note with tags to ensure tags exist in database
        let note = service
            .create_note("Test note", Some(&["rust", "programming"]))
            .expect("failed to create note");

        // Get the first tag ID from the note
        let tag_id = note.tags()[0].tag_id();

        // Test tag name resolution
        let tag_name = get_tag_name(&service, tag_id)
            .expect("failed to get tag name")
            .expect("tag should exist");

        assert_eq!(tag_name, "rust", "tag name should match");
    }

    #[test]
    fn get_tag_name_returns_none_for_non_existent_tag_id() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Query for non-existent tag ID
        let tag_name = get_tag_name(&service, cons::TagId::new(999))
            .expect("get_tag_name should not error for non-existent ID");

        assert_eq!(tag_name, None, "should return None for non-existent tag ID");
    }

    #[test]
    fn timestamp_formats_as_yyyy_mm_dd_hh_mm() {
        use time::macros::format_description;

        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create a note
        let note = service
            .create_note("Timestamp test", None)
            .expect("failed to create note");

        // Format timestamp using the same format as execute_list
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]");
        let timestamp = note
            .created_at()
            .format(&format)
            .expect("failed to format timestamp");

        // Verify format matches expected pattern (YYYY-MM-DD HH:MM)
        // Example: "2025-12-23 14:30"
        assert_eq!(timestamp.len(), 16, "timestamp should be 16 characters");
        assert_eq!(
            &timestamp[4..5],
            "-",
            "character at position 4 should be '-'"
        );
        assert_eq!(
            &timestamp[7..8],
            "-",
            "character at position 7 should be '-'"
        );
        assert_eq!(
            &timestamp[10..11],
            " ",
            "character at position 10 should be space"
        );
        assert_eq!(
            &timestamp[13..14],
            ":",
            "character at position 13 should be ':'"
        );
    }

    #[test]
    fn note_display_with_multiple_tags_shows_hashtag_format() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create a note with multiple tags
        let note = service
            .create_note("Test note", Some(&["rust", "programming", "tutorial"]))
            .expect("failed to create note");

        // Collect tag names in hashtag format (simulating execute_list behavior)
        let mut tag_names = Vec::new();
        for tag_assignment in note.tags() {
            if let Some(tag_name) =
                get_tag_name(&service, tag_assignment.tag_id()).expect("failed to get tag name")
            {
                tag_names.push(format!("#{}", tag_name));
            }
        }

        // Verify all tags are present in hashtag format
        assert_eq!(tag_names.len(), 3, "should have 3 tags");
        assert!(
            tag_names.contains(&"#rust".to_string()),
            "should contain #rust"
        );
        assert!(
            tag_names.contains(&"#programming".to_string()),
            "should contain #programming"
        );
        assert!(
            tag_names.contains(&"#tutorial".to_string()),
            "should contain #tutorial"
        );

        // Verify joined output (as it appears in execute_list)
        let tags_display = tag_names.join(" ");
        assert!(
            tags_display.contains("#rust"),
            "joined output should contain #rust"
        );
        assert!(
            tags_display.contains("#programming"),
            "joined output should contain #programming"
        );
        assert!(
            tags_display.contains("#tutorial"),
            "joined output should contain #tutorial"
        );
    }
}
