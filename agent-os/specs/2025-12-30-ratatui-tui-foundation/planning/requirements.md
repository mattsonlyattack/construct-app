# Spec Requirements: Ratatui TUI Foundation

## Initial Description

**Roadmap Item #30: Ratatui TUI foundation**

Description from roadmap: "Build terminal UI scaffold using ratatui with basic layout (note list, detail view, search input)"

This is part of a Rust CLI application called "cons" - a structure-last personal knowledge management tool. The TUI will be an alternative interface to the existing CLI, reusing the same NoteService business logic layer.

## Requirements Discussion

### First Round Questions

**Q1:** I assume a three-panel layout is desired: note list on the left (or top), detail view for selected note (main area), and a search/filter input bar (bottom or top). Is that correct, or would you prefer a different arrangement like a single-column stacked layout?
**Answer:** Yes, three-panel layout with notes list on the LEFT.

**Q2:** I'm thinking the initial view should show recent notes by default (similar to `cons list`), with search/filter accessible but not the primary focus. Should the TUI launch into browse mode first, or should it start with an empty search field focused for immediate querying?
**Answer:** Show recent notes like `cons list`, but DEFAULT INTO SEARCH MODE (focus on search input).

**Q3:** For the "foundation" scope, I assume we should build the rendering and navigation structure without implementing full note browsing (#31) or search functionality (#32) - essentially creating the scaffold and layout with placeholder content. Is that correct, or should this foundation include basic note listing and selection?
**Answer:** Include basic note listing and selection (not just placeholder scaffold).

**Q4:** I assume Vim-style keybindings for navigation (j/k for list, Enter to select, Esc to go back, q to quit) as the primary input method. Is that correct, or would you prefer arrow-key navigation as primary with Vim keys as alternative?
**Answer:** Vim-style (j/k, Enter, Esc, q).

**Q5:** Should the TUI support modal operation (Normal mode for navigation, Insert mode for search input) similar to Vim, or would you prefer a simpler model where focus shifts between panels without explicit modes?
**Answer:** NO modes - simpler focus-shifting model between panels.

**Q6:** For the note list panel, I assume showing note content preview (truncated), creation date, and tag count per item. Should we also display the AI-enhanced content indicator or tag names inline, or keep the list minimal?
**Answer:** MINIMAL (content preview, date, tag count).

**Q7:** For the detail view, should we display both original content and enhanced content when available, or just show the original with an option to toggle?
**Answer:** Show EVERYTHING (both original and enhanced content).

**Q8:** Is there anything specific you want excluded from this foundation that should be deferred to later roadmap items (31-33)? For example, should actual database connectivity be in scope, or should we use mock data for the scaffold?
**Answer:** NO exclusions - database connectivity should be in scope.

### Existing Code to Reference

No similar existing features identified for reference.

The TUI will integrate with existing codebase components:
- `NoteService` in `src/service.rs` - core business logic layer
- `Note` model in `src/models/note.rs` - note data structure with content, tags, enhanced content
- `Database` in `src/db.rs` - SQLite connection handling
- CLI implementation in `src/main.rs` - reference for how NoteService is instantiated and used

### Follow-up Questions

No follow-up questions needed - user provided clear, comprehensive answers.

## Visual Assets

### Files Provided:
No visual assets provided.

### Visual Insights:
N/A

## Requirements Summary

### Functional Requirements

**Layout:**
- Three-panel layout:
  - LEFT panel: Note list (scrollable)
  - MAIN panel: Note detail view
  - Search input bar (position TBD - likely top or bottom)
- Responsive panel sizing within terminal dimensions

**Initial State:**
- On launch, display recent notes (like `cons list`)
- Focus defaults to search input (search mode)
- Note list populated from database immediately

**Note List Panel:**
- Display minimal information per note:
  - Content preview (truncated to fit)
  - Creation date
  - Tag count
- Scrollable list with visual selection indicator
- Vim-style navigation (j/k to move, Enter to select)

**Detail View Panel:**
- Display complete note information:
  - Original content (full)
  - Enhanced content (when available)
  - All tags with their sources
  - Timestamps (created, updated, enhanced)
  - Enhancement metadata (model, confidence)

**Navigation & Input:**
- Vim-style keybindings:
  - `j` / `k`: Move selection down/up in note list
  - `Enter`: Select note (show in detail view)
  - `Esc`: Return focus to search input / clear selection
  - `q`: Quit application
- Focus-shifting model (NO explicit modes):
  - Tab or similar to shift focus between panels
  - When search input focused, typing filters/searches
  - When list focused, j/k navigates

**Database Integration:**
- Connect to actual SQLite database (not mock data)
- Reuse existing NoteService for all data operations
- Use same database path resolution as CLI

### Reusability Opportunities

- Reuse `NoteService` directly - no business logic duplication
- Reuse `Database::open()` for connection management
- Reuse `ListNotesOptions` for note retrieval configuration
- Follow same XDG Base Directory conventions as CLI for database location

### Scope Boundaries

**In Scope:**
- Three-panel TUI layout with ratatui
- Note list with basic display and selection
- Detail view showing all note information
- Vim-style keyboard navigation
- Focus shifting between panels (no modes)
- Real database connectivity via NoteService
- Search input widget (text entry capability)
- Basic application lifecycle (startup, run loop, quit)

**Out of Scope (deferred to roadmap items 31-33):**
- Full scrollable note browsing with pagination (#31)
- Interactive search execution and filtering (#32)
- Tag-based filtering UI (#32)
- Architecture proof documentation (#33)
- Note creation/editing within TUI
- Configuration options or settings UI

### Technical Considerations

- **Framework**: ratatui (modern tui-rs fork)
- **Async**: May need tokio runtime for compatibility with existing async components, but TUI rendering itself is synchronous
- **State Management**: Application state struct managing:
  - Current focus (which panel)
  - Selected note index
  - Search input buffer
  - Loaded notes list
- **Error Handling**: Graceful degradation if database unavailable; user-friendly error display
- **Module Organization**: New `src/tui/` module or `src/tui.rs` with submodules for:
  - App state
  - UI rendering
  - Event handling
  - Widgets (if custom)
- **Entry Point**: New CLI subcommand `cons tui` or similar to launch TUI mode
