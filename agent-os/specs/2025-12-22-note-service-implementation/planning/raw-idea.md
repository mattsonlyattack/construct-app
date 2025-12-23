# Raw Idea: NoteService Implementation

**Feature Description:** Build the core business logic layer independent of any UI, handling note CRUD operations and tag management. This is item #3 from the product roadmap.

## Context from the Project

- This is a Rust CLI app called "cons" - a structure-last personal knowledge management tool
- Users capture thoughts freely; AI handles tagging automatically
- Local-first (SQLite + Ollama), privacy-focused, single Rust binary
- Architecture: NoteService sits between CLI/TUI and the database layer
- Domain types already exist: Note, Tag, NoteId, TagId, TagSource, TagAssignment
- Database layer exists with SQLite schema for notes, tags, note_tags tables

## Requirements

The NoteService should be UI-independent and reusable across CLI/TUI/GUI interfaces.
