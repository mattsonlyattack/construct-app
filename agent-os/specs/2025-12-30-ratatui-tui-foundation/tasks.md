# Task Breakdown: Ratatui TUI Foundation

## Overview
Total Tasks: 28

This feature adds a terminal user interface to the cons PKM tool using ratatui. The TUI provides a three-panel layout (note list, detail view, search input) with Vim-style navigation, connecting to the existing NoteService layer for real database operations.

## Task List

### Application Foundation

#### Task Group 1: Module Structure and Application State
**Dependencies:** None

- [x] 1.0 Complete TUI module structure and App state
  - [x] 1.1 Write 4-6 focused tests for App state management
    - Test App initialization with default focus (SearchInput)
    - Test focus cycling (Tab key navigation between panels)
    - Test note selection state changes
    - Test notes list population
  - [x] 1.2 Create `src/tui/mod.rs` module exports
    - Export App, Focus enum, and run function
    - Re-export necessary types for external use
  - [x] 1.3 Create `src/tui/app.rs` with App struct
    - Fields: notes (Vec<Note>), selected_index (Option<usize>), search_input (String), focus (Focus)
    - Focus enum: SearchInput, NoteList, DetailView
    - Default focus on SearchInput per requirements
  - [x] 1.4 Implement App methods for state management
    - `new()` - initialize with default state
    - `next_focus()` - cycle focus Tab order
    - `select_next()` / `select_previous()` - j/k navigation
    - `selected_note()` - get currently selected note
  - [x] 1.5 Add `pub mod tui;` to `src/lib.rs`
  - [x] 1.6 Ensure App state tests pass
    - Run ONLY the 4-6 tests written in 1.1
    - Verify state transitions work correctly

**Acceptance Criteria:**
- The 4-6 tests written in 1.1 pass
- App struct compiles with all required fields
- Focus enum has three variants
- State management methods work correctly

---

### Terminal Lifecycle

#### Task Group 2: Terminal Setup and Event Loop
**Dependencies:** Task Group 1

- [x] 2.0 Complete terminal lifecycle management
  - [x] 2.1 Write 3-5 focused tests for terminal setup and event handling
    - Test that quit key (q) triggers shutdown
    - Test that focus keys (Tab) cycle focus state
    - Test that navigation keys (j/k) update selection
  - [x] 2.2 Create `src/tui/event.rs` with keyboard event handling
    - Handle `q` for quit
    - Handle `Tab` for focus cycling
    - Handle `j`/`k` for list navigation (when NoteList focused)
    - Handle `Enter` for note selection
    - Handle `Esc` for returning to search input
    - Handle character input for search buffer (when SearchInput focused)
  - [x] 2.3 Implement terminal initialization in `src/tui/mod.rs`
    - Enable raw mode with crossterm
    - Enter alternate screen
    - Create Terminal with CrosstermBackend
  - [x] 2.4 Implement clean shutdown
    - Restore terminal state (disable raw mode, leave alternate screen)
    - Handle errors gracefully with terminal restoration
  - [x] 2.5 Create main event loop structure
    - Poll for key events using crossterm
    - Call event handlers to update App state
    - Re-render on state change
    - Exit loop on quit signal
  - [x] 2.6 Ensure event handling tests pass
    - Run ONLY the 3-5 tests written in 2.1
    - Verify event dispatch works correctly

**Acceptance Criteria:**
- The 3-5 tests written in 2.1 pass
- Terminal enters and exits cleanly
- Key events are handled correctly
- No terminal corruption on error exit

---

### Data Layer Integration

#### Task Group 3: NoteService Connection and Data Loading
**Dependencies:** Task Group 1

- [x] 3.0 Complete database integration
  - [x] 3.1 Write 3-5 focused tests for data loading
    - Test loading notes with NoteService::list_notes()
    - Test ListNotesOptions configuration (limit: 50, order: Descending)
    - Test note list reversal for oldest-first display
  - [x] 3.2 Add database connection to TUI entry point
    - Reuse `get_database_path()` from main.rs
    - Reuse `ensure_database_directory()` from main.rs
    - Reuse `Database::open()` pattern
  - [x] 3.3 Implement note loading in App
    - Call `NoteService::list_notes()` with `ListNotesOptions { limit: Some(50), order: SortOrder::Descending, tags: None }`
    - Reverse list for oldest-first display within view
    - Store loaded notes in App state
  - [x] 3.4 Add `get_tag_names()` helper for tag display
    - Reuse or import existing helper from main.rs
    - Batch query tag names for efficiency
  - [x] 3.5 Ensure data loading tests pass
    - Run ONLY the 3-5 tests written in 3.1
    - Verify notes load correctly from database

**Acceptance Criteria:**
- [x] The 3-5 tests written in 3.1 pass
- [x] Notes load from real SQLite database
- [x] NoteService is reused (no business logic duplication)
- [x] Error handling for database failures

---

### UI Rendering

#### Task Group 4: Panel Layout and Rendering
**Dependencies:** Task Groups 1, 2, 3

- [x] 4.0 Complete UI rendering
  - [x] 4.1 Write 2-4 focused tests for UI layout and rendering
    - Test three-panel layout structure (left ~30%, main ~70%)
    - Test note list item formatting (content preview, date, tag count)
    - Test detail view content sections (original, enhanced, tags)
  - [x] 4.2 Create `src/tui/ui.rs` with rendering functions
    - `draw()` - main rendering function accepting Frame and App
    - Use ratatui Layout with Direction::Horizontal for left/main split
    - Create Block widgets with titles for each panel
  - [x] 4.3 Implement note list panel rendering
    - Use ratatui List widget with ListState
    - Display: content preview (truncated ~40 chars), date (YYYY-MM-DD), tag count
    - Visual selection indicator (highlight with reverse colors)
    - Show focused state with different border style
  - [x] 4.4 Implement detail view panel rendering
    - Display "Content:" label with original content
    - Display "---" separator when enhanced content exists
    - Display "Enhanced:" label with enhanced content
    - Show confidence percentage when available
    - Display tags with source indicators (user vs llm)
    - Show timestamps (created, enhanced if available)
  - [x] 4.5 Implement search input rendering
    - Text input field with current buffer content
    - Visual cursor indicator when focused
    - Block border with "Search" title
  - [x] 4.6 Apply visual styling
    - Block borders with titles for each panel
    - Reverse colors for selection highlighting
    - Dim/italic for secondary information (dates, tag counts)
    - Different border style for focused panel
  - [x] 4.7 Ensure UI rendering tests pass
    - Run ONLY the 2-4 tests written in 4.1
    - Verify layout renders without panic

**Acceptance Criteria:**
- [x] The 2-4 tests written in 4.1 pass
- [x] Three-panel layout renders correctly
- [x] Note list shows items with proper formatting
- [x] Detail view displays all note information
- [x] Search input shows with cursor when focused

---

### CLI Integration

#### Task Group 5: CLI Subcommand and Entry Point
**Dependencies:** Task Groups 1, 2, 3, 4

- [x] 5.0 Complete CLI integration
  - [x] 5.1 Write 2-3 focused tests for CLI subcommand
    - Test `cons tui` command parsing with clap
    - Test TUI entry point initialization
  - [x] 5.2 Add `Tui` variant to Commands enum in `src/main.rs`
    - Follow existing clap pattern for subcommands
    - Add help text: "Launch interactive terminal UI"
  - [x] 5.3 Add `handle_tui()` command handler in `src/main.rs`
    - Call `tui::run()` function
    - Match existing error handling pattern with `is_user_error()`
  - [x] 5.4 Create `pub fn run()` entry point in `src/tui/mod.rs`
    - Initialize database connection
    - Create NoteService and load notes
    - Initialize terminal
    - Create App with loaded notes
    - Start event loop
    - Restore terminal on exit
  - [x] 5.5 Handle TUI-specific errors gracefully
    - Terminal initialization failures
    - Database connection errors
    - User-friendly error messages
  - [x] 5.6 Ensure CLI integration tests pass
    - Run ONLY the 2-3 tests written in 5.1
    - Verify command parses correctly

**Acceptance Criteria:**
- [x] The 2-3 tests written in 5.1 pass
- [x] `cons tui` launches the TUI
- [x] Clean error handling and exit
- [x] Terminal always restored on exit

---

### Testing

#### Task Group 6: Test Review and Gap Analysis
**Dependencies:** Task Groups 1-5

- [x] 6.0 Review existing tests and fill critical gaps only
  - [x] 6.1 Review tests from Task Groups 1-5
    - Review the 4-6 tests written by Task Group 1 (App state)
    - Review the 3-5 tests written by Task Group 2 (terminal/events)
    - Review the 3-5 tests written by Task Group 3 (data loading)
    - Review the 2-4 tests written by Task Group 4 (UI rendering)
    - Review the 2-3 tests written by Task Group 5 (CLI integration)
    - Total existing tests: approximately 14-23 tests
  - [x] 6.2 Analyze test coverage gaps for TUI feature only
    - Identify critical user workflows that lack test coverage
    - Focus ONLY on gaps related to this spec's feature requirements
    - Prioritize end-to-end workflows: launch -> navigate -> view -> quit
    - Do NOT assess entire application test coverage
  - [x] 6.3 Write up to 8 additional strategic tests maximum
    - Add maximum of 8 new tests to fill identified critical gaps
    - Focus on integration points: App + NoteService, Event + State
    - Test focus-shifting model edge cases
    - Test empty database graceful handling
    - Do NOT write comprehensive coverage for all scenarios
  - [x] 6.4 Run TUI feature-specific tests only
    - Run ONLY tests related to this spec's feature (tests from 1.1, 2.1, 3.1, 4.1, 5.1, and 6.3)
    - Expected total: approximately 22-31 tests maximum
    - Do NOT run the entire application test suite
    - Verify critical workflows pass

**Acceptance Criteria:**
- [x] All TUI feature-specific tests pass (approximately 22-31 tests total)
- [x] Critical user workflows for TUI are covered
- [x] No more than 8 additional tests added when filling gaps
- [x] Testing focused exclusively on this spec's feature requirements

---

## Execution Order

Recommended implementation sequence:

1. **Task Group 1: Module Structure and Application State**
   - Foundation for all other groups
   - No dependencies
   - Specialist: Backend/Rust engineer

2. **Task Group 2: Terminal Lifecycle**
   - Depends on Task Group 1 (App state)
   - Sets up crossterm terminal management
   - Specialist: Backend/Rust engineer

3. **Task Group 3: NoteService Connection**
   - Depends on Task Group 1 (App state)
   - Can be done in parallel with Task Group 2
   - Specialist: Backend/Rust engineer

4. **Task Group 4: Panel Layout and Rendering**
   - Depends on Task Groups 1, 2, 3
   - Main UI implementation
   - Specialist: UI/Frontend engineer with ratatui experience

5. **Task Group 5: CLI Integration**
   - Depends on Task Groups 1, 2, 3, 4
   - Ties everything together
   - Specialist: Backend/Rust engineer

6. **Task Group 6: Test Review and Gap Analysis**
   - Depends on Task Groups 1-5
   - Final verification
   - Specialist: QA/Testing engineer

---

## Technical Notes

### Dependencies to Add
```toml
# Cargo.toml additions
ratatui = "0.28"
crossterm = "0.28"
```

### Module Structure
```
src/
  tui/
    mod.rs      # Module exports, run() entry point
    app.rs      # App struct, Focus enum, state management
    ui.rs       # Rendering functions for each panel
    event.rs    # Keyboard event handling
```

### Key Design Decisions
- **No modes**: Focus-shifting model, not Vim-like modal editing
- **Reuse NoteService**: All data operations through existing service layer
- **Fail-safe terminal**: Always restore terminal state, even on errors
- **Sync rendering**: TUI is synchronous, only async for future features
