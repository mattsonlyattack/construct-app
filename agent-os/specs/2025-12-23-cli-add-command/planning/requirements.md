# Spec Requirements: CLI add command

## Initial Description

**CLI: add command** - Implement `cons add "thought"` for instant note capture with optional manual tags via `--tags` flag

Roadmap item #4, estimated size S (2-3 days). This is the first CLI command being implemented for the cons personal knowledge management tool. The CLI uses clap for argument parsing and calls the existing NoteService layer for business logic.

## Requirements Discussion

### First Round Questions

**Q1:** I assume the primary usage is `cons add "my thought here"` where the entire note content is passed as a single positional argument. Is that correct, or should we also support reading from stdin (e.g., `echo "thought" | cons add` or `cons add` to open an editor)?
**Answer:** No, positional argument only for now - `cons add "thought"`

**Q2:** For the `--tags` flag, I'm thinking comma-separated format like `--tags rust,learning,project`. Should we normalize tags to lowercase and allow spaces around commas (e.g., `"rust, Learning"` becomes `["rust", "learning"]`)? Or do you prefer a different format like repeated flags (`--tag rust --tag learning`)?
**Answer:** CSV format (comma-separated). Tag normalization is deferred to roadmap item #11

**Q3:** I assume empty or whitespace-only content should be rejected with a clear error message (e.g., "Note content cannot be empty"). Is that correct?
**Answer:** Yes, reject with clear error message

**Q4:** After successfully adding a note, I'm thinking we should output a confirmation like `Note created (id: 42)` or possibly show more details like `Note created (id: 42) with tags: rust, learning`. What level of feedback do you prefer? Should there be a `--quiet` flag to suppress output for scripting?
**Answer:** Show all data (id + tags), no extra flags like --quiet

**Q5:** I assume errors should be user-friendly messages without stack traces (following your error-handling standards), like "Failed to create note: database file not found at ~/.local/share/cons/notes.db". Is that the right approach?
**Answer:** Yes, user-friendly without stack traces

**Q6:** The NoteService needs a Database instance, which needs a file path. I assume we should follow XDG Base Directory spec (`~/.local/share/cons/notes.db` on Linux, appropriate equivalents on macOS/Windows). Should the first `cons add` auto-create this directory and database file if they don't exist? And should there be a `--database` flag to override the location?
**Answer:** Use the most idiomatic cross-platform approach for Rust, no --database override flag

**Q7:** Since this is the first command and future commands will include `list`, `search`, and `edit`, I'm assuming we should structure the CLI with subcommands from the start (e.g., `cons add`, `cons list`) rather than making `add` the default action. Is that correct?
**Answer:** Yes, subcommands from the start

**Q8:** Is there anything you explicitly want to exclude from this first implementation? For example: no `--auto-tag` integration yet (roadmap item #9), no special handling of multi-line notes, no reading from files?
**Answer:** None explicitly mentioned

### Existing Code to Reference

No similar existing features identified for reference.

### Follow-up Questions

No follow-up questions were needed.

## Visual Assets

### Files Provided:

No visual assets provided.

## Requirements Summary

### Functional Requirements

- Implement `cons add "<content>"` command using clap with subcommand structure
- Accept note content as a required positional argument (string)
- Support optional `--tags` flag with comma-separated values (e.g., `--tags rust,learning`)
- Reject empty or whitespace-only content with a clear error message
- Create the note via NoteService.create_note() with provided content and tags
- Output confirmation showing note ID and any tags applied (e.g., "Note created (id: 42) with tags: rust, learning")
- Auto-create database directory and file on first use if they don't exist
- Use idiomatic cross-platform data directory (dirs crate recommended for Rust)

### Technical Considerations

- Use clap with derive macros for CLI definition (per tech-stack.md)
- Structure CLI with subcommands from the start to support future `list`, `search`, `edit` commands
- Database location: use `dirs` crate for cross-platform data directory resolution
  - Linux: `~/.local/share/cons/notes.db` (XDG_DATA_HOME)
  - macOS: `~/Library/Application Support/cons/notes.db`
  - Windows: `C:\Users\<user>\AppData\Roaming\cons\notes.db`
- Error handling: user-friendly messages without stack traces, following project error-handling standards
- Tags passed as-is to NoteService (no normalization in this spec - deferred to roadmap item #11)
- NoteService already handles tag deduplication (case-insensitive) internally

### Scope Boundaries

**In Scope:**
- `cons add "<content>"` command implementation
- `--tags` flag with comma-separated values
- Empty content validation
- Success output with note ID and tags
- User-friendly error messages
- Cross-platform database path resolution
- Auto-creation of database directory/file
- CLI help text via clap

**Out of Scope:**
- Stdin input (reading from pipe or editor)
- `--quiet` flag or other output control flags
- `--database` flag for custom database location
- Tag normalization (roadmap item #11)
- Auto-tagging with LLM (roadmap item #9)
- Multi-line note special handling
- Reading content from files
- Other commands (list, search, edit)

### Dependencies

- Add `clap` with derive feature to Cargo.toml
- Add `dirs` crate for cross-platform directory resolution
- Existing: NoteService, Database, Note types from cons crate
