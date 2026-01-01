//! Keyboard event handling for the TUI.
//!
//! Maps crossterm keyboard events to application state changes.
//! Handles focus-shifting model where key behavior depends on current focus.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, Focus};

/// Handles a keyboard event and updates the app state accordingly.
///
/// Returns `true` if the application should quit, `false` otherwise.
///
/// # Event Handling
///
/// - `q`: Quit application (from any focus state)
/// - `Tab`: Cycle focus between panels
/// - `Esc`: Return to search input focus
/// - When `SearchInput` focused: character input updates filter buffer
/// - When `NoteList` focused: j/k navigation, Enter to select
///
/// # Examples
///
/// ```
/// use cons::tui::{App, event::handle_key_event};
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
///
/// let mut app = App::new();
/// let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
/// let should_quit = handle_key_event(&mut app, key);
/// assert!(should_quit);
/// ```
pub fn handle_key_event(app: &mut App, key: KeyEvent) -> bool {
    // Global quit key - works from any focus state
    if key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
        return true;
    }

    // Global focus cycling with Tab / Shift+Tab (BackTab)
    if key.code == KeyCode::Tab {
        app.next_focus();
        return false;
    }
    if key.code == KeyCode::BackTab {
        app.prev_focus();
        return false;
    }

    // Global Esc - return to search input
    if key.code == KeyCode::Esc {
        app.reset_focus();
        app.clear_selection();
        return false;
    }

    // Focus-specific handling
    match app.focus() {
        Focus::SearchInput => handle_search_input(app, key),
        Focus::NoteList => handle_note_list(app, key),
        Focus::DetailView => {
            handle_detail_view(app, key);
        }
    }

    false
}

/// Handles keyboard input when search input is focused.
///
/// Accepts character input and backspace for editing the filter buffer.
/// Changes mark the filter as pending search (debounced in event loop).
fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            app.push_search_char(c);
        }
        KeyCode::Backspace => {
            app.pop_search_char();
        }
        _ => {
            // Ignore other keys when in search input
        }
    }
}

/// Handles keyboard input when note list is focused.
///
/// Supports Vim-style navigation (j/k) and Enter to select.
fn handle_note_list(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') if key.modifiers.is_empty() => {
            app.select_next();
        }
        KeyCode::Char('k') if key.modifiers.is_empty() => {
            app.select_previous();
        }
        KeyCode::Enter => {
            // Enter in note list maintains current selection
            // (selection is already set by j/k navigation)
            // Future: could switch focus to detail view
        }
        _ => {
            // Ignore other keys when in note list
        }
    }
}

/// Handles keyboard input when detail view is focused.
///
/// Supports Vim-style scrolling (j/k).
fn handle_detail_view(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') if key.modifiers.is_empty() => {
            app.scroll_detail_down(1);
        }
        KeyCode::Char('k') if key.modifiers.is_empty() => {
            app.scroll_detail_up(1);
        }
        _ => {
            // Ignore other keys when in detail view
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NoteBuilder, NoteId};

    #[test]
    fn quit_key_triggers_shutdown() {
        let mut app = App::new();
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);

        let should_quit = handle_key_event(&mut app, key);
        assert!(should_quit);
    }

    #[test]
    fn quit_works_from_any_focus() {
        let mut app = App::new();

        // From SearchInput
        assert_eq!(app.focus(), Focus::SearchInput);
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(handle_key_event(&mut app, key));

        // From NoteList
        app = App::new();
        app.next_focus();
        assert_eq!(app.focus(), Focus::NoteList);
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(handle_key_event(&mut app, key));

        // From DetailView
        app = App::new();
        app.next_focus();
        app.next_focus();
        assert_eq!(app.focus(), Focus::DetailView);
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(handle_key_event(&mut app, key));
    }

    #[test]
    fn tab_key_cycles_focus() {
        let mut app = App::new();
        assert_eq!(app.focus(), Focus::SearchInput);

        // Tab to NoteList
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let should_quit = handle_key_event(&mut app, key);
        assert!(!should_quit);
        assert_eq!(app.focus(), Focus::NoteList);

        // Tab to DetailView
        let should_quit = handle_key_event(&mut app, key);
        assert!(!should_quit);
        assert_eq!(app.focus(), Focus::DetailView);

        // Tab back to SearchInput
        let should_quit = handle_key_event(&mut app, key);
        assert!(!should_quit);
        assert_eq!(app.focus(), Focus::SearchInput);
    }

    #[test]
    fn shift_tab_cycles_focus_backwards() {
        let mut app = App::new();
        assert_eq!(app.focus(), Focus::SearchInput);

        // BackTab (Shift+Tab) to DetailView
        let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        let should_quit = handle_key_event(&mut app, key);
        assert!(!should_quit);
        assert_eq!(app.focus(), Focus::DetailView);

        // BackTab to NoteList
        let should_quit = handle_key_event(&mut app, key);
        assert!(!should_quit);
        assert_eq!(app.focus(), Focus::NoteList);

        // BackTab back to SearchInput
        let should_quit = handle_key_event(&mut app, key);
        assert!(!should_quit);
        assert_eq!(app.focus(), Focus::SearchInput);
    }

    #[test]
    fn navigation_keys_update_selection_when_note_list_focused() {
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note 1")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Note 2")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(3))
                .content("Note 3")
                .build(),
        ];
        app.set_notes(notes);

        // Switch to NoteList focus - auto-selects first note
        app.next_focus();
        assert_eq!(app.focus(), Focus::NoteList);
        assert_eq!(app.selected_index(), Some(0));

        // Press 'j' to move down
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let should_quit = handle_key_event(&mut app, key_j);
        assert!(!should_quit);
        assert_eq!(app.selected_index(), Some(1));

        // Press 'j' again to move down
        let should_quit = handle_key_event(&mut app, key_j);
        assert!(!should_quit);
        assert_eq!(app.selected_index(), Some(2));

        // Press 'k' to move up
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let should_quit = handle_key_event(&mut app, key_k);
        assert!(!should_quit);
        assert_eq!(app.selected_index(), Some(1));
    }

    #[test]
    fn navigation_keys_ignored_when_not_in_note_list() {
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note 1")
                .build(),
        ];
        app.set_notes(notes);

        // In SearchInput focus, j/k should add characters to search
        assert_eq!(app.focus(), Focus::SearchInput);
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        handle_key_event(&mut app, key_j);
        assert_eq!(app.search_input(), "j");

        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        handle_key_event(&mut app, key_k);
        assert_eq!(app.search_input(), "jk");

        // Selection should not change
        assert_eq!(app.selected_index(), None);
    }

    #[test]
    fn esc_returns_to_search_input_and_clears_selection() {
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note 1")
                .build(),
        ];
        app.set_notes(notes);

        // Go to NoteList and select a note
        app.next_focus();
        assert_eq!(app.focus(), Focus::NoteList);
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));

        // Press Esc
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let should_quit = handle_key_event(&mut app, key_esc);
        assert!(!should_quit);

        // Should return to SearchInput and clear selection
        assert_eq!(app.focus(), Focus::SearchInput);
        assert_eq!(app.selected_index(), None);
    }

    #[test]
    fn character_input_updates_search_buffer_when_search_focused() {
        let mut app = App::new();
        assert_eq!(app.focus(), Focus::SearchInput);
        assert_eq!(app.search_input(), "");

        // Type some characters
        let key_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        handle_key_event(&mut app, key_h);
        assert_eq!(app.search_input(), "h");

        let key_i = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        handle_key_event(&mut app, key_i);
        assert_eq!(app.search_input(), "hi");

        // Test backspace
        let key_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        handle_key_event(&mut app, key_backspace);
        assert_eq!(app.search_input(), "h");
    }

    #[test]
    fn shift_modified_characters_work_in_search() {
        let mut app = App::new();
        assert_eq!(app.focus(), Focus::SearchInput);

        // Shift+A should produce 'A'
        let key = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        handle_key_event(&mut app, key);
        assert_eq!(app.search_input(), "A");
    }

    // --- Task Group 6: Additional Strategic Tests ---

    #[test]
    fn backspace_on_empty_search_buffer_is_safe() {
        // Test that backspace on empty search buffer doesn't panic or error
        let mut app = App::new();
        assert_eq!(app.search_input(), "");

        let key_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        handle_key_event(&mut app, key_backspace);

        // Should still be empty, not panic
        assert_eq!(app.search_input(), "");

        // Multiple backspaces should also be safe
        handle_key_event(&mut app, key_backspace);
        handle_key_event(&mut app, key_backspace);
        assert_eq!(app.search_input(), "");
    }

    #[test]
    fn jk_scrolls_in_detail_view() {
        // Test that j/k keys scroll (not navigate) when DetailView is focused
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note 1")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Note 2")
                .build(),
        ];
        app.set_notes(notes);

        // Navigate to DetailView focus (auto-selects first note when entering NoteList)
        app.next_focus(); // -> NoteList (auto-selects index 0)
        app.next_focus(); // -> DetailView

        assert_eq!(app.focus(), Focus::DetailView);
        assert_eq!(app.selected_index(), Some(0));
        assert_eq!(app.detail_scroll(), 0);

        // Press j - should scroll down, not change selection
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        handle_key_event(&mut app, key_j);
        assert_eq!(app.selected_index(), Some(0), "j should not change selection");
        assert_eq!(app.detail_scroll(), 1, "j should scroll down");

        // Press k - should scroll up
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        handle_key_event(&mut app, key_k);
        assert_eq!(app.selected_index(), Some(0), "k should not change selection");
        assert_eq!(app.detail_scroll(), 0, "k should scroll up");

        // Press k again - should not go negative
        handle_key_event(&mut app, key_k);
        assert_eq!(app.detail_scroll(), 0, "scroll should not go below 0");
    }

    #[test]
    fn selection_persists_across_focus_changes() {
        // Test that selection is maintained when cycling focus
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note 1")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Note 2")
                .build(),
        ];
        app.set_notes(notes);

        // Go to NoteList (auto-selects first) and move to second note
        app.next_focus(); // -> NoteList (auto-selects index 0)
        app.select_next(); // Move to second (index 1)
        assert_eq!(app.selected_index(), Some(1));

        // Cycle through focus states
        app.next_focus(); // -> DetailView
        assert_eq!(
            app.selected_index(),
            Some(1),
            "selection should persist in DetailView"
        );

        app.next_focus(); // -> SearchInput
        assert_eq!(
            app.selected_index(),
            Some(1),
            "selection should persist in SearchInput"
        );

        app.next_focus(); // -> NoteList (selection already exists, no auto-select)
        assert_eq!(
            app.selected_index(),
            Some(1),
            "selection should persist back in NoteList"
        );
    }
}
