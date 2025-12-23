# Specification: CLI add command

## Goal

Implement `cons add "<content>"` as the first CLI command for instant note capture, using clap with subcommand structure and integrating with the existing NoteService layer.

## User Stories

- As a user, I want to quickly capture a thought with `cons add "my thought"` so that ideas are saved instantly without friction
- As a user, I want to optionally tag my notes with `--tags rust,learning` so that I can manually organize related thoughts

## Specific Requirements

**CLI binary and subcommand structure**
- Create `src/main.rs` as the CLI entry point using clap with derive macros
- Define top-level `Cli` struct with `#[command(name = "cons")]` attribute
- Use `#[derive(Parser, Subcommand)]` pattern for extensible subcommands
- Define `AddCommand` variant under Commands enum for `cons add`
- Structure supports future `list`, `search`, `edit` commands without refactoring

**Add command arguments**
- Accept note content as a required positional argument: `#[arg]` on `content: String`
- Support optional `--tags` flag with comma-separated values: `#[arg(short, long)]` on `tags: Option<String>`
- Parse tags by splitting on comma and trimming whitespace from each tag
- Pass tags as-is to NoteService (no normalization per requirements)

**Content validation**
- Reject empty strings or whitespace-only content before calling NoteService
- Return user-friendly error: "Note content cannot be empty"
- Use `anyhow::bail!` for error propagation with clean message

**Database path resolution**
- Use `dirs` crate `data_dir()` function for cross-platform paths
- Construct path as `{data_dir}/cons/notes.db`
- Linux: `~/.local/share/cons/notes.db`
- macOS: `~/Library/Application Support/cons/notes.db`
- Windows: `C:\Users\<user>\AppData\Roaming\cons\notes.db`

**Auto-create database directory**
- Check if parent directory exists before opening database
- Create directory recursively with `std::fs::create_dir_all` if missing
- Handle directory creation errors with user-friendly message

**Success output format**
- On successful note creation, output: `Note created (id: {id})`
- If tags were applied, append: ` with tags: {comma-separated tags}`
- Example with tags: `Note created (id: 42) with tags: rust, learning`
- Example without tags: `Note created (id: 42)`

**Error handling**
- Use `anyhow::Result` for error propagation
- Catch and format errors at main boundary with user-friendly messages
- No stack traces in user-facing output
- Exit with non-zero status code on error

**Help text**
- Provide clear description via `#[command(about = "...")]` attributes
- Include usage examples in command help where possible
- Standard clap `--help` and `--version` flags auto-generated

## Existing Code to Leverage

**NoteService (src/service.rs)**
- Use `NoteService::new(db)` constructor taking Database ownership
- Call `service.create_note(content, tags)` where tags is `Option<&[&str]>`
- Returns `Result<Note>` with fully populated Note including ID and tag assignments
- Handles tag deduplication internally (case-insensitive)

**Database (src/db.rs)**
- Use `Database::open(path)` for file-based database
- Auto-initializes schema on connection (idempotent)
- Creates database file if it does not exist
- Enables foreign keys automatically via PRAGMA

**Note model (src/models/note.rs)**
- Access `note.id()` for NoteId after creation
- Access `note.tags()` for Vec<TagAssignment> to display applied tags
- TagAssignment has `tag_id()` method but no tag name directly - may need to query tags table

**TagAssignment (src/models/tag_assignment.rs)**
- Provides `tag_id()` to get TagId for each assigned tag
- Use for iterating and displaying applied tags in output

## Out of Scope

- Reading note content from stdin or editor (positional argument only)
- `--quiet` flag or other output control flags
- `--database` flag for custom database location override
- Tag normalization (deferred to roadmap item #11)
- Auto-tagging with LLM integration (deferred to roadmap item #9)
- Multi-line note special handling beyond what shell quoting provides
- Reading content from files
- Other CLI commands (list, search, edit implemented separately)
- TUI or GUI interfaces
- Backward compatibility with previous versions (new implementation)
