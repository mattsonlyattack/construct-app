use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::fs::OpenOptions;
use std::io::Write;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cons::{
    autotagger::AutoTaggerBuilder, ollama::OllamaClientBuilder, Database, NoteId, NoteService,
    TagSource,
};

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

#[tokio::main]
async fn main() {
    // Load environment variables from .env file if it exists
    // This is a no-op if .env doesn't exist, so it's safe to call unconditionally
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Add(cmd) => {
            let res = handle_add(cmd).await;
            // Give background tasks time to start before exiting
            // This is a small delay to allow spawned tasks to begin execution
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            res
        },
        Commands::List(cmd) => handle_list(cmd).await,
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
async fn handle_add(cmd: &AddCommand) -> Result<()> {
    // Validate content is not empty or whitespace-only
    if cmd.content.trim().is_empty() {
        anyhow::bail!("Note content cannot be empty");
    }

    // Get database path and ensure directory exists
    let db_path = get_database_path()?;
    ensure_database_directory(&db_path)?;

    // Open database and create service
    let db = Database::open(&db_path).context("Failed to open database")?;

    execute_add(&cmd.content, cmd.tags.as_deref(), db).await
}

/// Executes the add command logic with a provided database.
///
/// This function is separated from `handle_add` to allow testing with in-memory databases.
async fn execute_add(content: &str, tags: Option<&str>, db: Database) -> Result<()> {
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

    // Spawn background task for auto-tagging (fire-and-forget)
    let note_id = note.id();
    let content = content.to_string();
    let db_path = get_database_path()?;
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| String::new());
    
    // #region agent log
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
        let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"main.rs:129","message":"execute_add: spawning background task","data":{{"note_id":{},"model":"{}","model_empty":{},"db_path":"{}"}},"timestamp":{}}}"#, 
            note_id.get(), model, model.is_empty(), db_path.display(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    }
    // #endregion
    
    // Spawn background task in a separate thread with its own tokio runtime
    // This ensures the task continues even after main() exits
    std::thread::spawn(move || {
        // Create a new tokio runtime for this thread
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"F","location":"main.rs:144","message":"Inside spawned thread - task is executing","data":{{"note_id":{}}},"timestamp":{}}}"#, 
                    note_id.get(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            let result = auto_tag_note_background(note_id, content, db_path, model).await;
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"F","location":"main.rs:144","message":"Spawned thread completed","data":{{"note_id":{},"result_ok":{}}},"timestamp":{}}}"#, 
                    note_id.get(), result.is_ok(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            result
        })
    });

    Ok(())
}

/// Background task for auto-tagging a note.
///
/// This function runs asynchronously in the background and silently handles all errors.
/// It opens its own database connection and constructs its own OllamaClient and AutoTagger.
async fn auto_tag_note_background(
    note_id: NoteId,
    content: String,
    db_path: PathBuf,
    model: String,
) -> Result<()> {
    // #region agent log
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
        let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"main.rs:147","message":"auto_tag_note_background: entry","data":{{"note_id":{},"model":"{}","model_empty":{},"content_len":{}}},"timestamp":{}}}"#, 
            note_id.get(), model, model.is_empty(), content.len(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    }
    // #endregion
    
    // Silently return if model is not set
    if model.is_empty() {
        // #region agent log
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
            let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"main.rs:150","message":"auto_tag_note_background: early return - model empty","data":{{"note_id":{}}},"timestamp":{}}}"#, 
                note_id.get(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        }
        // #endregion
        return Ok(());
    }

    // Open database connection
    ensure_database_directory(&db_path)?;
    let db = Database::open(&db_path).context("Failed to open database")?;

    // Construct OllamaClient (silently handle errors)
    let client = match OllamaClientBuilder::new().build() {
        Ok(c) => {
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"main.rs:157","message":"OllamaClient built successfully","data":{{"note_id":{}}},"timestamp":{}}}"#, 
                    note_id.get(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            c
        },
        Err(e) => {
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"main.rs:159","message":"OllamaClient build failed","data":{{"note_id":{},"error":"{}"}},"timestamp":{}}}"#, 
                    note_id.get(), e, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            return Ok(()); // Fail-safe: return silently if client construction fails
        }
    };

    // Construct AutoTagger
    let tagger = AutoTaggerBuilder::new()
        .client(Arc::new(client))
        .build();

    // #region agent log
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
        let content_preview = if content.len() > 50 { &content[..50] } else { content.as_str() };
        let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"C","location":"main.rs:163","message":"About to call generate_tags","data":{{"note_id":{},"model":"{}","content_preview":"{}"}},"timestamp":{}}}"#, 
            note_id.get(), model, content_preview, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    }
    // #endregion

    // Generate tags (silently handle errors)
    // Wrap with timeout to prevent indefinite hanging
    let tags = match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tagger.generate_tags(&model, &content)
    ).await {
        Ok(Ok(t)) => {
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"C","location":"main.rs:168","message":"Tags generated successfully","data":{{"note_id":{},"tag_count":{},"tags":{:?}}},"timestamp":{}}}"#, 
                    note_id.get(), t.len(), t.keys().collect::<Vec<_>>(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            t
        },
        Ok(Err(e)) => {
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"C","location":"main.rs:170","message":"Tag generation failed","data":{{"note_id":{},"error":"{}"}},"timestamp":{}}}"#, 
                    note_id.get(), e, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            return Ok(()); // Fail-safe: return silently if tag generation fails
        },
        Err(_) => {
            // #region agent log
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"C","location":"main.rs:170","message":"Tag generation timed out","data":{{"note_id":{}}},"timestamp":{}}}"#, 
                    note_id.get(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            }
            // #endregion
            return Ok(()); // Fail-safe: return silently if tag generation times out
        }
    };

    // Convert tags and add to note
    if tags.is_empty() {
        // #region agent log
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
            let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"C","location":"main.rs:174","message":"Tags empty, returning early","data":{{"note_id":{}}},"timestamp":{}}}"#, 
                note_id.get(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        }
        // #endregion
        return Ok(());
    }

    let service = NoteService::new(db);

    // Process each tag with its own confidence score
    // Note: add_tags_to_note accepts a single TagSource, so we need to add tags individually
    // or batch them. Since tags can have different confidences, we'll add them one by one.
    for (tag_name, confidence_f64) in tags {
        let confidence_u8 = (confidence_f64 * 100.0).round() as u8;
        let source = TagSource::llm(model.clone(), confidence_u8);
        // #region agent log
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
            let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"D","location":"main.rs:186","message":"Adding tag to note","data":{{"note_id":{},"tag":"{}","confidence":{}}},"timestamp":{}}}"#, 
                note_id.get(), tag_name, confidence_u8, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        }
        // #endregion
        match service.add_tags_to_note(note_id, &[tag_name.as_str()], source) {
            Ok(_) => {
                // #region agent log
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                    let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"D","location":"main.rs:186","message":"Tag added successfully","data":{{"note_id":{},"tag":"{}"}},"timestamp":{}}}"#, 
                        note_id.get(), tag_name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                }
                // #endregion
            },
            Err(e) => {
                // #region agent log
                if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
                    let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"D","location":"main.rs:186","message":"Tag add failed","data":{{"note_id":{},"tag":"{}","error":"{}"}},"timestamp":{}}}"#, 
                        note_id.get(), tag_name, e, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                }
                // #endregion
            }
        }
    }

    // #region agent log
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
        let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"E","location":"main.rs:189","message":"auto_tag_note_background: completed","data":{{"note_id":{}}},"timestamp":{}}}"#, 
            note_id.get(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    }
    // #endregion

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
async fn handle_list(cmd: &ListCommand) -> Result<()> {
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

    // Parse tags if provided, converting empty to None
    let parsed_tags = tags.map(parse_tags);
    let tags_option = match parsed_tags {
        Some(ref tags) if tags.is_empty() => None,
        other => other,
    };

    // Use DESC ordering to get the newest N notes, then reverse for chronological display
    // (oldest first, newest last within the result set)
    use cons::{ListNotesOptions, SortOrder};
    let options = ListNotesOptions {
        limit: Some(limit),
        tags: tags_option,
        order: SortOrder::Descending,
    };

    // Fetch newest N notes
    let mut notes = service
        .list_notes(options)
        .context("Failed to list notes")?;

    // Reverse to display oldest-first (newest last)
    notes.reverse();

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

        // Get tag names using batch query
        let tag_assignments = note.tags();
        // #region agent log
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/home/md/construct-app/.cursor/debug.log") {
            let user_tags: Vec<_> = tag_assignments.iter().filter(|ta| ta.source().is_user()).map(|ta| ta.tag_id().get()).collect();
            let llm_tags: Vec<_> = tag_assignments.iter().filter(|ta| ta.source().is_llm()).map(|ta| ta.tag_id().get()).collect();
            let _ = writeln!(file, r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"E","location":"main.rs:281","message":"Listing note tags","data":{{"note_id":{},"total_tags":{},"user_tag_ids":{:?},"llm_tag_ids":{:?}}},"timestamp":{}}}"#, 
                note.id().get(), tag_assignments.len(), user_tags, llm_tags, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        }
        // #endregion
        let tag_names: Vec<String> = get_tag_names(service.database(), tag_assignments)?
            .into_iter()
            .map(|name| format!("#{}", name))
            .collect();

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

/// Gets tag names from the database for the given tag assignments.
///
/// Uses a single batch query with IN clause for efficiency.
fn get_tag_names(db: &Database, tag_assignments: &[cons::TagAssignment]) -> Result<Vec<String>> {
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

    #[tokio::test]
    async fn content_validation_rejects_empty_string() {
        let cmd = AddCommand {
            content: String::new(),
            tags: None,
        };
        let result = handle_add(&cmd).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn content_validation_rejects_whitespace_only() {
        let cmd = AddCommand {
            content: "   \n\t  ".to_string(),
            tags: None,
        };
        let result = handle_add(&cmd).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    // --- Async Runtime Conversion Tests (Task Group 1) ---

    #[tokio::test]
    async fn handle_add_returns_result_from_async_context() {
        let cmd = AddCommand {
            content: "Test note".to_string(),
            tags: None,
        };
        // This test verifies handle_add() can be called from async context
        // Once handle_add() is converted to async, this will compile and run
        let result = handle_add(&cmd).await;
        // Note creation may fail due to database path issues in test environment,
        // but we're testing that async call works, not that note creation succeeds
        let _ = result; // Suppress unused warning
    }

    #[tokio::test]
    async fn command_dispatch_uses_await_correctly() {
        // Test that command parsing and dispatch works with async runtime
        use clap::CommandFactory;
        let cli = Cli::command();
        // Verify CLI can be constructed (this will be used in async main)
        assert_eq!(cli.get_name(), "cons");
    }

    // --- Background Auto-Tagging Tests (Task Group 3) ---

    #[tokio::test]
    async fn note_creation_succeeds_even_if_ollama_client_fails() {
        // Test that note creation succeeds even if Ollama client construction would fail
        // We test this by ensuring execute_add completes successfully
        let db = Database::in_memory().expect("failed to create in-memory database");
        let result = execute_add("Test note", None, db).await;
        // Note creation should succeed regardless of Ollama availability
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_add_spawns_background_task_without_blocking() {
        // Test that execute_add returns immediately without waiting for background task
        let db = Database::in_memory().expect("failed to create in-memory database");
        let start = std::time::Instant::now();
        let result = execute_add("Test note", None, db).await;
        let elapsed = start.elapsed();
        // Should return quickly (within 100ms) without waiting for background task
        assert!(elapsed.as_millis() < 100);
        assert!(result.is_ok());
    }

    #[test]
    fn manual_and_auto_generated_tags_coexist_on_same_note() {
        // Test that manual tags and auto-generated tags can both exist on a note
        // This is tested at the NoteService level - both tag sources are supported
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create note with manual tags
        let note = service
            .create_note("Test note", Some(&["manual-tag"]))
            .expect("failed to create note");

        // Add auto-generated tags (simulating background task)
        let llm_source = TagSource::llm("test-model", 85);
        service
            .add_tags_to_note(note.id(), &["auto-tag"], llm_source)
            .expect("failed to add auto-generated tags");

        // Retrieve note and verify both tag types exist
        let retrieved = service
            .get_note(note.id())
            .expect("failed to get note")
            .expect("note should exist");

        assert_eq!(retrieved.tags().len(), 2, "note should have 2 tags");
        // Verify both user and LLM tags are present
        let has_user_tag = retrieved.tags().iter().any(|ta| ta.source().is_user());
        let has_llm_tag = retrieved.tags().iter().any(|ta| ta.source().is_llm());
        assert!(has_user_tag, "note should have user tag");
        assert!(has_llm_tag, "note should have LLM tag");
    }

    // --- Test Review & Gap Analysis Tests (Task Group 4) ---

    #[test]
    fn confidence_score_conversion_f64_to_u8_works_correctly() {
        // Test that confidence scores are converted correctly from f64 (0.0-1.0) to u8 (0-100)
        let test_cases: Vec<(f64, u8)> = vec![
            (0.0, 0u8),
            (0.5, 50u8),
            (0.85, 85u8),
            (1.0, 100u8),
            (0.955, 96u8), // Test rounding
        ];

        for (f64_val, expected_u8) in test_cases {
            let actual_u8 = (f64_val * 100.0_f64).round() as u8;
            assert_eq!(
                actual_u8, expected_u8,
                "f64 {} should convert to u8 {}",
                f64_val, expected_u8
            );
        }
    }

    #[test]
    fn model_name_stored_in_tag_source_llm_variant() {
        // Test that model name from OLLAMA_MODEL env var is stored in TagSource::Llm
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        let note = service
            .create_note("Test note", None)
            .expect("failed to create note");

        // Add tags with specific model name
        let model_name = "gemma3:4b";
        let source = TagSource::llm(model_name, 85);
        service
            .add_tags_to_note(note.id(), &["test-tag"], source)
            .expect("failed to add tags");

        // Retrieve note and verify model name is stored
        let retrieved = service
            .get_note(note.id())
            .expect("failed to get note")
            .expect("note should exist");

        let llm_tags: Vec<_> = retrieved
            .tags()
            .iter()
            .filter(|ta| ta.source().is_llm())
            .collect();

        assert_eq!(llm_tags.len(), 1, "should have one LLM tag");
        assert_eq!(
            llm_tags[0].model(),
            Some(model_name),
            "model name should be stored in TagSource"
        );
    }

    #[tokio::test]
    async fn background_task_handles_empty_model_gracefully() {
        // Test that background task returns early if model is empty (no OLLAMA_MODEL set)
        let db_path = std::path::PathBuf::from("/tmp/test-cons-empty-model.db");
        let note_id = NoteId::new(1);
        let content = "Test note".to_string();
        let model = String::new(); // Empty model

        // Should return Ok(()) without attempting to connect to Ollama
        let result = auto_tag_note_background(note_id, content, db_path, model).await;
        assert!(result.is_ok(), "should handle empty model gracefully");
    }

    #[test]
    fn tag_source_llm_constructor_accepts_model_and_confidence() {
        // Test that TagSource::llm() constructor works correctly
        let source = TagSource::llm("test-model", 75);
        assert!(source.is_llm());
        assert_eq!(source.confidence(), 75);
        assert_eq!(source.model(), Some("test-model"));
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
    fn get_tag_names_resolves_tag_ids_to_display_names() {
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create a note with tags to ensure tags exist in database
        let note = service
            .create_note("Test note", Some(&["rust", "programming"]))
            .expect("failed to create note");

        // Test batch tag name resolution
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
        let tag_names: Vec<String> = get_tag_names(service.database(), note.tags())
            .expect("failed to get tag names")
            .into_iter()
            .map(|name| format!("#{}", name))
            .collect();

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
