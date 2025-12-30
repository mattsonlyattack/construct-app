# Specification: Ratatui TUI Foundation

## Goal

Build a terminal user interface for the cons PKM tool using ratatui, providing a three-panel layout with note list, detail view, and search input that connects to the existing NoteService layer for real database operations.

## User Stories

- As a user, I want to browse my notes in a terminal interface so that I can quickly navigate and review my knowledge base without leaving the command line
- As a user, I want to see both original and enhanced content in the detail view so that I can understand what I captured and how AI expanded it

## Specific Requirements

**Three-Panel Layout**
- Left panel: scrollable note list with selection indicator
- Main panel: note detail view showing full content
- Top or bottom: search input bar for text entry
- Use ratatui's Layout with Direction::Horizontal for left/main split
- Responsive sizing within terminal dimensions (left panel ~30%, main ~70%)

**Application State Management**
- Create `App` struct holding: notes list, selected index, search input buffer, current focus panel
- Focus enum: `SearchInput`, `NoteList`, `DetailView`
- Default focus on search input (search mode) per requirements
- No modal operation - simple focus-shifting between panels

**Initial Data Loading**
- On launch, load recent notes using `NoteService::list_notes()` with `ListNotesOptions { limit: Some(50), order: SortOrder::Descending, tags: None }`
- Reverse the list for oldest-first display within the view
- Connect to real database using same `get_database_path()` / `Database::open()` pattern as CLI

**Note List Panel Display**
- Minimal display per item: content preview (truncated to ~40 chars), date (YYYY-MM-DD), tag count
- Visual selection indicator (highlight or `>>` marker)
- Use ratatui's List widget with ListState for selection tracking

**Detail View Panel Display**
- Display original content first, labeled "Content:"
- Separator line "---" when enhanced content exists
- Display enhanced content labeled "Enhanced:" with confidence percentage
- Show all tags with source indicators (user vs llm)
- Show timestamps: created, enhanced (if available)

**Vim-Style Keyboard Navigation**
- `j`: move selection down in note list (when list focused)
- `k`: move selection up in note list (when list focused)
- `Enter`: select note to display in detail view
- `Esc`: return focus to search input / clear selection
- `q`: quit application
- `Tab`: cycle focus between panels (SearchInput -> NoteList -> DetailView -> SearchInput)

**Focus-Shifting Model**
- When search input focused: keypresses go to search buffer (except navigation keys)
- When note list focused: j/k navigate, Enter selects
- No explicit insert/normal modes - focus determines behavior

**Search Input Widget**
- Text input field for search queries
- Visual cursor indicator when focused
- Typing updates internal buffer (no live filtering in this scope)
- Search execution deferred to roadmap item #32

**Application Lifecycle**
- Entry point: new CLI subcommand `cons tui` using clap
- Initialize terminal with crossterm backend (enable raw mode, alternate screen)
- Main event loop: poll for key events, update state, render
- Clean shutdown: restore terminal state on quit or error

**Module Organization**
- Create `src/tui/` module directory
- `src/tui/mod.rs`: module exports and App struct
- `src/tui/app.rs`: application state and business logic
- `src/tui/ui.rs`: rendering functions for each panel
- `src/tui/event.rs`: keyboard event handling
- Add `pub mod tui;` to `src/lib.rs`

## Visual Design

No visual assets provided. Use standard ratatui styling:
- Block borders with titles for each panel
- Reverse colors for selection highlighting
- Dim/italic for secondary information (dates, tag counts)

## Existing Code to Leverage

**NoteService (`src/service.rs`)**
- Reuse `NoteService::new(db)` for all data operations - no business logic duplication
- Use `list_notes(ListNotesOptions)` for fetching recent notes with limit and ordering
- `SortOrder::Descending` gets newest notes first

**Database connection pattern (`src/main.rs`)**
- Reuse `get_database_path()` for XDG Base Directory compliant path resolution
- Reuse `ensure_database_directory()` for creating database directory if needed
- Reuse `Database::open(&db_path)` for opening SQLite connection

**Note model (`src/models/note.rs`)**
- Use `Note` struct accessors: `content()`, `content_enhanced()`, `tags()`, `created_at()`, etc.
- Use `enhancement_confidence()` for displaying confidence percentage
- Tag count available via `note.tags().len()`

**CLI command structure (`src/main.rs`)**
- Follow existing clap pattern for adding `Tui` command variant to Commands enum
- Match existing error handling pattern with `is_user_error()` and exit codes

**Tag name resolution (`src/main.rs`)**
- Reuse `get_tag_names(db, tag_assignments)` helper for converting TagAssignment to display names

## Out of Scope

- Full scrollable note browsing with pagination (roadmap item #31)
- Interactive search execution and live filtering (roadmap item #32)
- Tag-based filtering UI (roadmap item #32)
- Architecture proof documentation (roadmap item #33)
- Note creation within TUI
- Note editing within TUI
- Note deletion within TUI
- Configuration options or settings UI
- Mouse support
- Color themes or customization
