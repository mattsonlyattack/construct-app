//! Terminal User Interface module for cons.
//!
//! Provides a three-panel TUI with note list, detail view, and search input
//! using ratatui for rendering and crossterm for terminal management.

use std::io;
use std::panic;

use anyhow::{Context, Result};
use crossterm::{
    event::{self as crossterm_event, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
pub mod event;
mod ui;

pub use app::{App, Focus};

/// Initializes the terminal for TUI rendering.
///
/// Enables raw mode and enters the alternate screen.
/// Returns a configured Terminal instance.
///
/// # Errors
///
/// Returns an error if terminal initialization fails.
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("failed to create terminal")?;
    Ok(terminal)
}

/// Restores the terminal to its original state.
///
/// Disables raw mode and leaves the alternate screen.
/// This should always be called before exiting the TUI,
/// even in error cases, to prevent terminal corruption.
///
/// # Errors
///
/// Returns an error if terminal restoration fails.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;
    Ok(())
}

/// Minimal terminal restoration for panic handler.
///
/// Does not require a Terminal reference, making it safe to call
/// from a panic hook where we may not have access to the Terminal.
/// Ignores errors since we're likely already in a bad state.
fn restore_terminal_panic() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Initializes a panic hook that restores the terminal before panicking.
///
/// This ensures the terminal is restored even if a panic occurs anywhere
/// in the application, not just in the event loop. The original panic
/// hook is preserved and called after terminal restoration.
fn init_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal_panic();
        original_hook(panic_info);
    }));
}

/// Runs the main event loop for the TUI.
///
/// Polls for keyboard events, updates app state, and re-renders.
/// Exits when the user presses 'q' or an error occurs.
///
/// # Errors
///
/// Returns an error if event polling, rendering, or terminal operations fail.
/// Terminal state is always restored, even on error.
pub fn run_event_loop(app: &mut App) -> Result<()> {
    let mut terminal = init_terminal()?;

    // Ensure terminal is restored even if we panic or error
    let result = run_event_loop_internal(app, &mut terminal);

    // Always restore terminal state
    if let Err(e) = restore_terminal(&mut terminal) {
        eprintln!("Error restoring terminal: {e}");
    }

    result
}

/// Internal event loop implementation.
///
/// Separated from `run_event_loop` to ensure terminal restoration happens
/// in the outer function.
fn run_event_loop_internal(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    loop {
        // Render the current state
        terminal.draw(|frame| {
            ui::draw(frame, app);
        })?;

        // Poll for events
        if crossterm_event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = crossterm_event::read()?
        {
            // Handle the key event
            let should_quit = event::handle_key_event(app, key);
            if should_quit {
                break;
            }
        }
    }

    Ok(())
}

/// Loads recent notes from the database into the App.
///
/// Uses `NoteService::list_notes()` with:
/// - limit: Some(50)
/// - order: SortOrder::Descending
/// - tags: None
///
/// Reverses the list for oldest-first display within the view.
///
/// # Errors
///
/// Returns an error if note loading fails.
fn load_notes(app: &mut App, service: &crate::service::NoteService) -> Result<()> {
    use crate::service::{ListNotesOptions, SortOrder};

    // Load recent notes with descending order (newest first)
    let options = ListNotesOptions {
        limit: Some(50),
        order: SortOrder::Descending,
        tags: None,
    };

    let mut notes = service
        .list_notes(options)
        .context("Failed to load notes")?;

    // Reverse for oldest-first display within view
    notes.reverse();

    // Store loaded notes in App state
    app.set_notes(notes);

    Ok(())
}

/// Entry point for the TUI application.
///
/// Initializes the database connection, loads notes, and starts the event loop.
///
/// # Errors
///
/// Returns an error if:
/// - Database path cannot be determined
/// - Database directory creation fails
/// - Database connection fails
/// - Note loading fails
/// - Terminal initialization or event loop fails
pub fn run() -> Result<()> {
    // Install panic hook to restore terminal on panic
    init_panic_hook();

    // Get database path and ensure directory exists (reusing shared utilities)
    let db_path = crate::utils::get_database_path().context("Failed to get database path")?;
    crate::utils::ensure_database_directory(&db_path)
        .context("Failed to ensure database directory")?;

    // Open database connection
    let db = crate::Database::open(&db_path).context("Failed to open database")?;

    // Create NoteService
    let service = crate::service::NoteService::new(db);

    // Create App and load notes
    let mut app = App::new();
    load_notes(&mut app, &service).context("Failed to load notes from database")?;

    // Start the TUI event loop
    run_event_loop(&mut app).context("TUI event loop failed")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Terminal initialization tests are difficult to write in unit tests
    // because they require actual terminal capabilities. These are better tested
    // manually or with integration tests.

    #[test]
    fn event_loop_can_be_created() {
        // This test just verifies the function signatures compile correctly
        // Actual event loop testing requires a terminal, which we can't mock easily
        let mut _app = App::new();
        // We can't actually run the event loop in tests without a terminal
    }

    #[test]
    fn load_notes_populates_app_state() {
        // Test that load_notes correctly populates the app with database notes
        use crate::service::NoteService;

        let db = crate::Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create test notes
        service
            .create_note("First note", None)
            .expect("failed to create note");
        service
            .create_note("Second note", None)
            .expect("failed to create note");

        // Load notes into app
        let mut app = App::new();
        load_notes(&mut app, &service).expect("failed to load notes");

        // Verify notes were loaded
        assert_eq!(app.notes().len(), 2);
        // Verify oldest-first order (after reversal)
        assert!(app.notes()[0].content().contains("First"));
        assert!(app.notes()[1].content().contains("Second"));
    }

    #[test]
    fn load_notes_with_empty_database() {
        // Test that load_notes handles empty database gracefully
        let db = crate::Database::in_memory().expect("failed to create in-memory database");
        let service = crate::service::NoteService::new(db);
        let mut app = App::new();

        let result = load_notes(&mut app, &service);
        assert!(result.is_ok(), "should handle empty database gracefully");
        assert_eq!(app.notes().len(), 0);
    }

    #[test]
    fn load_notes_enforces_limit_50() {
        // Test that load_notes respects the 50-note limit
        use crate::service::NoteService;

        let db = crate::Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create more than 50 notes
        for i in 1..=60 {
            service
                .create_note(&format!("Note {}", i), None)
                .expect("failed to create note");
        }

        let mut app = App::new();
        load_notes(&mut app, &service).expect("failed to load notes");

        // Should only load 50 notes
        assert_eq!(app.notes().len(), 50);
    }

    // --- Task Group 6: Additional Strategic Tests ---

    #[test]
    fn integration_workflow_load_navigate_select_view() {
        // Test the complete user workflow: load notes -> navigate -> select -> view details
        use crate::service::NoteService;

        let db = crate::Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create test notes
        service
            .create_note("First note content", None)
            .expect("failed to create note");
        service
            .create_note("Second note content", None)
            .expect("failed to create note");
        service
            .create_note("Third note content", None)
            .expect("failed to create note");

        // Load notes into app
        let mut app = App::new();
        load_notes(&mut app, &service).expect("failed to load notes");

        // Verify notes loaded
        assert_eq!(app.notes().len(), 3);
        assert_eq!(app.focus(), Focus::SearchInput);
        assert_eq!(app.selected_index(), None);

        // Navigate to note list
        app.next_focus();
        assert_eq!(app.focus(), Focus::NoteList);

        // Select first note
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));
        assert!(app.selected_note().is_some());
        assert_eq!(app.selected_note().unwrap().content(), "First note content");

        // Navigate to detail view
        app.next_focus();
        assert_eq!(app.focus(), Focus::DetailView);
        // Selection should persist
        assert_eq!(app.selected_index(), Some(0));

        // Navigate down in list (switch back to list first)
        app.next_focus(); // -> SearchInput
        app.next_focus(); // -> NoteList
        app.select_next(); // -> Second note
        assert_eq!(app.selected_index(), Some(1));
        assert_eq!(
            app.selected_note().unwrap().content(),
            "Second note content"
        );
    }

    #[test]
    fn integration_workflow_search_input_then_navigate() {
        // Test workflow: type in search -> tab to list -> navigate
        use crate::service::NoteService;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let db = crate::Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        service
            .create_note("Test note", None)
            .expect("failed to create note");

        let mut app = App::new();
        load_notes(&mut app, &service).expect("failed to load notes");

        // Start in search input (default)
        assert_eq!(app.focus(), Focus::SearchInput);

        // Type some text
        let key_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        event::handle_key_event(&mut app, key_h);
        assert_eq!(app.search_input(), "h");

        let key_i = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        event::handle_key_event(&mut app, key_i);
        assert_eq!(app.search_input(), "hi");

        // Tab to note list
        let key_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        event::handle_key_event(&mut app, key_tab);
        assert_eq!(app.focus(), Focus::NoteList);

        // Navigate in list
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        event::handle_key_event(&mut app, key_j);
        assert_eq!(app.selected_index(), Some(0));

        // Search input should still contain "hi"
        assert_eq!(app.search_input(), "hi");
    }

    #[test]
    fn single_note_navigation_no_wrapping() {
        // Test navigation with exactly 1 note (edge case for wrapping logic)
        use crate::service::NoteService;

        let db = crate::Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        service
            .create_note("Only note", None)
            .expect("failed to create note");

        let mut app = App::new();
        load_notes(&mut app, &service).expect("failed to load notes");

        assert_eq!(app.notes().len(), 1);

        // Select the only note
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));

        // Try to go down - should stay at index 0 (wrap to self)
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));

        // Try to go up - should stay at index 0 (wrap to self)
        app.select_previous();
        assert_eq!(app.selected_index(), Some(0));
    }
}
