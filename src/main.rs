use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cons::{Database, NoteService};

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

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Add(cmd) => handle_add(cmd),
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
}
