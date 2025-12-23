# Raw Idea

**CLI: add command** - Implement `cons add "thought"` for instant note capture with optional manual tags via `--tags` flag

## Roadmap Context

- Item #4 on the roadmap
- Size: S (2-3 days)

## Project Context

- This is a Rust CLI tool called "cons" for personal knowledge management
- Uses clap for CLI argument parsing
- Has a NoteService layer that handles business logic
- SQLite database with notes, tags, and note_tags tables
- The add command should call NoteService to create notes
