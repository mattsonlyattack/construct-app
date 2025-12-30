use crate::models::Note;

/// Application state for the TUI.
///
/// Manages notes list, selection state, search input, and panel focus.
#[derive(Debug, Clone)]
pub struct App {
    /// List of loaded notes
    notes: Vec<Note>,
    /// Currently selected note index (None if no selection)
    selected_index: Option<usize>,
    /// Search input buffer
    search_input: String,
    /// Currently focused panel
    focus: Focus,
}

/// Panel focus state for keyboard navigation.
///
/// Determines which panel receives keyboard input and how keys are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Search input bar is focused (typing updates search buffer)
    SearchInput,
    /// Note list panel is focused (j/k navigation, Enter to select)
    NoteList,
    /// Detail view panel is focused (for future scrolling support)
    DetailView,
}

impl App {
    /// Creates a new App with default state.
    ///
    /// Default focus is `SearchInput` per requirements.
    /// Notes list is empty, selection is None, search input is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::tui::App;
    ///
    /// let app = App::new();
    /// assert!(app.notes().is_empty());
    /// assert_eq!(app.selected_index(), None);
    /// ```
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
            selected_index: None,
            search_input: String::new(),
            focus: Focus::SearchInput,
        }
    }

    /// Returns the list of notes.
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Returns the currently selected note index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Returns the search input buffer.
    pub fn search_input(&self) -> &str {
        &self.search_input
    }

    /// Returns the current focus state.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// Sets the notes list and resets selection to None.
    ///
    /// Used when loading notes from database or after search.
    pub fn set_notes(&mut self, notes: Vec<Note>) {
        self.notes = notes;
        self.selected_index = None;
    }

    /// Returns the currently selected note, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::tui::App;
    /// use cons::{NoteBuilder, NoteId};
    ///
    /// let mut app = App::new();
    /// let note = NoteBuilder::new()
    ///     .id(NoteId::new(1))
    ///     .content("Test note")
    ///     .build();
    /// app.set_notes(vec![note]);
    /// app.select_next();
    ///
    /// assert!(app.selected_note().is_some());
    /// assert_eq!(app.selected_note().unwrap().content(), "Test note");
    /// ```
    pub fn selected_note(&self) -> Option<&Note> {
        self.selected_index.and_then(|i| self.notes.get(i))
    }

    /// Cycles focus to the next panel in Tab order.
    ///
    /// Order: `SearchInput` -> `NoteList` -> `DetailView` -> `SearchInput`
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::tui::{App, Focus};
    ///
    /// let mut app = App::new();
    /// assert_eq!(app.focus(), Focus::SearchInput);
    ///
    /// app.next_focus();
    /// assert_eq!(app.focus(), Focus::NoteList);
    ///
    /// app.next_focus();
    /// assert_eq!(app.focus(), Focus::DetailView);
    ///
    /// app.next_focus();
    /// assert_eq!(app.focus(), Focus::SearchInput);
    /// ```
    pub fn next_focus(&mut self) {
        self.focus = match self.focus {
            Focus::SearchInput => Focus::NoteList,
            Focus::NoteList => Focus::DetailView,
            Focus::DetailView => Focus::SearchInput,
        };
    }

    /// Moves selection down in the notes list (j key navigation).
    ///
    /// If no selection, selects first note.
    /// If at end of list, wraps to beginning.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::tui::App;
    /// use cons::{NoteBuilder, NoteId};
    ///
    /// let mut app = App::new();
    /// let notes = vec![
    ///     NoteBuilder::new().id(NoteId::new(1)).content("Note 1").build(),
    ///     NoteBuilder::new().id(NoteId::new(2)).content("Note 2").build(),
    /// ];
    /// app.set_notes(notes);
    ///
    /// assert_eq!(app.selected_index(), None);
    ///
    /// app.select_next();
    /// assert_eq!(app.selected_index(), Some(0));
    ///
    /// app.select_next();
    /// assert_eq!(app.selected_index(), Some(1));
    ///
    /// app.select_next(); // Wraps to beginning
    /// assert_eq!(app.selected_index(), Some(0));
    /// ```
    pub fn select_next(&mut self) {
        if self.notes.is_empty() {
            self.selected_index = None;
            return;
        }

        self.selected_index = Some(match self.selected_index {
            None => 0,
            Some(i) => {
                if i + 1 >= self.notes.len() {
                    0
                } else {
                    i + 1
                }
            }
        });
    }

    /// Moves selection up in the notes list (k key navigation).
    ///
    /// If no selection, selects last note.
    /// If at beginning of list, wraps to end.
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::tui::App;
    /// use cons::{NoteBuilder, NoteId};
    ///
    /// let mut app = App::new();
    /// let notes = vec![
    ///     NoteBuilder::new().id(NoteId::new(1)).content("Note 1").build(),
    ///     NoteBuilder::new().id(NoteId::new(2)).content("Note 2").build(),
    /// ];
    /// app.set_notes(notes);
    ///
    /// assert_eq!(app.selected_index(), None);
    ///
    /// app.select_previous();
    /// assert_eq!(app.selected_index(), Some(1)); // Selects last
    ///
    /// app.select_previous();
    /// assert_eq!(app.selected_index(), Some(0));
    ///
    /// app.select_previous(); // Wraps to end
    /// assert_eq!(app.selected_index(), Some(1));
    /// ```
    pub fn select_previous(&mut self) {
        if self.notes.is_empty() {
            self.selected_index = None;
            return;
        }

        self.selected_index = Some(match self.selected_index {
            None => self.notes.len() - 1,
            Some(0) => self.notes.len() - 1,
            Some(i) => i - 1,
        });
    }

    /// Adds a character to the search input buffer.
    ///
    /// Used when `SearchInput` is focused and user types.
    pub fn push_search_char(&mut self, c: char) {
        self.search_input.push(c);
    }

    /// Removes the last character from the search input buffer.
    ///
    /// Used for Backspace handling when `SearchInput` is focused.
    pub fn pop_search_char(&mut self) {
        self.search_input.pop();
    }

    /// Clears the selection (Esc key behavior).
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
    }

    /// Returns focus to `SearchInput` (Esc key behavior).
    pub fn reset_focus(&mut self) {
        self.focus = Focus::SearchInput;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use crate::models::{NoteBuilder, NoteId};
    use crate::service::{ListNotesOptions, NoteService, SortOrder};

    #[test]
    fn app_initializes_with_default_state() {
        let app = App::new();
        assert!(app.notes().is_empty());
        assert_eq!(app.selected_index(), None);
        assert_eq!(app.search_input(), "");
        assert_eq!(app.focus(), Focus::SearchInput);
    }

    // --- Task Group 3: Data Loading Tests ---

    #[test]
    fn loading_notes_with_note_service_list_notes() {
        // Test that notes can be loaded using NoteService::list_notes()
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create test notes
        service
            .create_note("First note", None)
            .expect("failed to create note");
        service
            .create_note("Second note", None)
            .expect("failed to create note");
        service
            .create_note("Third note", None)
            .expect("failed to create note");

        // Load notes using NoteService
        let options = ListNotesOptions {
            limit: Some(50),
            order: SortOrder::Descending,
            tags: None,
        };
        let notes = service.list_notes(options).expect("failed to list notes");

        // Verify notes were loaded
        assert_eq!(notes.len(), 3);
    }

    #[test]
    fn list_notes_options_configured_correctly() {
        // Test that ListNotesOptions is configured with limit: 50, order: Descending
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create more than 50 notes to test limit
        for i in 1..=60 {
            service
                .create_note(&format!("Note {}", i), None)
                .expect("failed to create note");
        }

        // Use the same options as TUI will use
        let options = ListNotesOptions {
            limit: Some(50),
            order: SortOrder::Descending,
            tags: None,
        };
        let notes = service.list_notes(options).expect("failed to list notes");

        // Verify limit is enforced
        assert_eq!(notes.len(), 50, "should limit to 50 notes");

        // Verify descending order (newest first)
        // The first note in the result should be the most recently created
        assert!(
            notes[0].content().contains("Note 60"),
            "should be newest note"
        );
    }

    #[test]
    fn note_list_reversal_for_oldest_first_display() {
        // Test that reversing the list gives oldest-first display
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Create notes in order
        service
            .create_note("Oldest note", None)
            .expect("failed to create note");
        service
            .create_note("Middle note", None)
            .expect("failed to create note");
        service
            .create_note("Newest note", None)
            .expect("failed to create note");

        // Load with Descending order (newest first)
        let options = ListNotesOptions {
            limit: Some(50),
            order: SortOrder::Descending,
            tags: None,
        };
        let mut notes = service.list_notes(options).expect("failed to list notes");

        // Before reversal: newest first
        assert!(notes[0].content().contains("Newest"));

        // Reverse for oldest-first display
        notes.reverse();

        // After reversal: oldest first
        assert!(notes[0].content().contains("Oldest"));
        assert!(notes[2].content().contains("Newest"));
    }

    #[test]
    fn loading_notes_with_empty_database() {
        // Test graceful handling of empty database
        let db = Database::in_memory().expect("failed to create in-memory database");
        let service = NoteService::new(db);

        // Load notes from empty database
        let options = ListNotesOptions {
            limit: Some(50),
            order: SortOrder::Descending,
            tags: None,
        };
        let notes = service.list_notes(options).expect("failed to list notes");

        // Should return empty list, not error
        assert_eq!(notes.len(), 0);
    }

    #[test]
    fn app_set_notes_populates_state() {
        // Test that App.set_notes() correctly populates app state
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Loaded note")
                .build(),
        ];

        app.set_notes(notes);

        assert_eq!(app.notes().len(), 1);
        assert_eq!(app.notes()[0].content(), "Loaded note");
    }

    #[test]
    fn focus_cycles_in_tab_order() {
        let mut app = App::new();
        assert_eq!(app.focus(), Focus::SearchInput);

        app.next_focus();
        assert_eq!(app.focus(), Focus::NoteList);

        app.next_focus();
        assert_eq!(app.focus(), Focus::DetailView);

        app.next_focus();
        assert_eq!(app.focus(), Focus::SearchInput);
    }

    #[test]
    fn select_next_moves_down_through_list() {
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

        // No selection initially
        assert_eq!(app.selected_index(), None);

        // First select_next selects index 0
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));

        // Next moves to index 1
        app.select_next();
        assert_eq!(app.selected_index(), Some(1));

        // Next moves to index 2
        app.select_next();
        assert_eq!(app.selected_index(), Some(2));

        // Next wraps to index 0
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));
    }

    #[test]
    fn select_previous_moves_up_through_list() {
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

        // No selection initially
        assert_eq!(app.selected_index(), None);

        // First select_previous selects last index (2)
        app.select_previous();
        assert_eq!(app.selected_index(), Some(2));

        // Previous moves to index 1
        app.select_previous();
        assert_eq!(app.selected_index(), Some(1));

        // Previous moves to index 0
        app.select_previous();
        assert_eq!(app.selected_index(), Some(0));

        // Previous wraps to last index (2)
        app.select_previous();
        assert_eq!(app.selected_index(), Some(2));
    }

    #[test]
    fn selected_note_returns_current_selection() {
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("First note")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Second note")
                .build(),
        ];
        app.set_notes(notes);

        // No selection returns None
        assert!(app.selected_note().is_none());

        // After selecting, returns the correct note
        app.select_next();
        assert!(app.selected_note().is_some());
        assert_eq!(app.selected_note().unwrap().content(), "First note");

        app.select_next();
        assert!(app.selected_note().is_some());
        assert_eq!(app.selected_note().unwrap().content(), "Second note");
    }

    #[test]
    fn set_notes_resets_selection() {
        let mut app = App::new();
        let notes1 = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note 1")
                .build(),
        ];
        app.set_notes(notes1);
        app.select_next();
        assert_eq!(app.selected_index(), Some(0));

        // Setting new notes resets selection
        let notes2 = vec![
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Note 2")
                .build(),
        ];
        app.set_notes(notes2);
        assert_eq!(app.selected_index(), None);
    }

    #[test]
    fn navigation_with_empty_list_does_nothing() {
        let mut app = App::new();
        assert_eq!(app.selected_index(), None);

        app.select_next();
        assert_eq!(app.selected_index(), None);

        app.select_previous();
        assert_eq!(app.selected_index(), None);
    }
}
