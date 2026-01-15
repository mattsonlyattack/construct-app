use std::time::Instant;

use crate::models::Note;

/// Application state for the TUI.
///
/// Manages notes list, selection state, filter input, and panel focus.
#[derive(Debug, Clone)]
pub struct App {
    /// All loaded notes (unfiltered, used for fallback when filter is empty)
    all_notes: Vec<Note>,
    /// Currently displayed notes (search results or all notes)
    notes: Vec<Note>,
    /// Currently selected note index (None if no selection)
    selected_index: Option<usize>,
    /// Filter input buffer
    search_input: String,
    /// Currently focused panel
    focus: Focus,
    /// When the filter was last changed (for debouncing search)
    search_changed_at: Option<Instant>,
    /// Whether we need to run a search (filter changed but not yet searched)
    search_pending: bool,
    /// Scroll offset for detail view
    detail_scroll: u16,
}

/// Panel focus state for keyboard navigation.
///
/// Determines which panel receives keyboard input and how keys are interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// Filter input bar is focused (typing updates filter buffer and filters notes)
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
    /// Notes list is empty, selection is None, filter input is empty.
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
            all_notes: Vec::new(),
            notes: Vec::new(),
            selected_index: None,
            search_input: String::new(),
            focus: Focus::SearchInput,
            search_changed_at: None,
            search_pending: false,
            detail_scroll: 0,
        }
    }

    /// Returns the currently displayed (filtered) notes.
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Returns all loaded notes (unfiltered).
    pub fn all_notes(&self) -> &[Note] {
        &self.all_notes
    }

    /// Returns the currently selected note index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Returns the filter input buffer.
    pub fn search_input(&self) -> &str {
        &self.search_input
    }

    /// Returns the current focus state.
    pub fn focus(&self) -> Focus {
        self.focus
    }

    /// Sets the notes list and resets selection to None.
    ///
    /// Used when loading notes from database. Stores notes in both
    /// `all_notes` (for filtering) and `notes` (for display).
    pub fn set_notes(&mut self, notes: Vec<Note>) {
        self.all_notes = notes.clone();
        self.notes = notes;
        self.selected_index = None;
        // Apply current filter if any
        if !self.search_input.is_empty() {
            self.apply_filter();
        }
    }

    /// Applies the current filter to notes.
    ///
    /// Filters `all_notes` based on `search_input` (case-insensitive substring match)
    /// and updates `notes` with the results. Resets selection when filter changes.
    pub fn apply_filter(&mut self) {
        let query = self.search_input.to_lowercase();

        if query.is_empty() {
            // No filter - show all notes
            self.notes = self.all_notes.clone();
        } else {
            // Filter notes by content (case-insensitive)
            self.notes = self
                .all_notes
                .iter()
                .filter(|note| note.content().to_lowercase().contains(&query))
                .cloned()
                .collect();
        }

        // Reset selection when filter changes
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
        self.auto_select_on_note_list_focus();
    }

    /// Cycles focus to the previous panel in reverse Tab order.
    ///
    /// Order: `SearchInput` -> `DetailView` -> `NoteList` -> `SearchInput`
    ///
    /// # Examples
    ///
    /// ```
    /// use cons::tui::{App, Focus};
    ///
    /// let mut app = App::new();
    /// assert_eq!(app.focus(), Focus::SearchInput);
    ///
    /// app.prev_focus();
    /// assert_eq!(app.focus(), Focus::DetailView);
    ///
    /// app.prev_focus();
    /// assert_eq!(app.focus(), Focus::NoteList);
    ///
    /// app.prev_focus();
    /// assert_eq!(app.focus(), Focus::SearchInput);
    /// ```
    pub fn prev_focus(&mut self) {
        self.focus = match self.focus {
            Focus::SearchInput => Focus::DetailView,
            Focus::NoteList => Focus::SearchInput,
            Focus::DetailView => Focus::NoteList,
        };
        self.auto_select_on_note_list_focus();
    }

    /// Auto-selects first note when entering NoteList focus with no selection.
    fn auto_select_on_note_list_focus(&mut self) {
        if self.focus == Focus::NoteList && self.selected_index.is_none() && !self.notes.is_empty()
        {
            self.selected_index = Some(0);
        }
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
        self.detail_scroll = 0;
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
        self.detail_scroll = 0;
    }

    /// Returns the current detail view scroll offset.
    pub fn detail_scroll(&self) -> u16 {
        self.detail_scroll
    }

    /// Scrolls the detail view down by the specified amount.
    pub fn scroll_detail_down(&mut self, amount: u16) {
        self.detail_scroll = self.detail_scroll.saturating_add(amount);
    }

    /// Scrolls the detail view up by the specified amount.
    pub fn scroll_detail_up(&mut self, amount: u16) {
        self.detail_scroll = self.detail_scroll.saturating_sub(amount);
    }

    /// Resets the detail view scroll to the top.
    pub fn reset_detail_scroll(&mut self) {
        self.detail_scroll = 0;
    }

    /// Adds a character to the filter input buffer and marks search as pending.
    ///
    /// Used when `SearchInput` is focused and user types.
    /// Sets `search_changed_at` for debouncing.
    pub fn push_search_char(&mut self, c: char) {
        self.search_input.push(c);
        self.mark_search_changed();
    }

    /// Removes the last character from the filter input buffer and marks search as pending.
    ///
    /// Used for Backspace handling when `SearchInput` is focused.
    pub fn pop_search_char(&mut self) {
        self.search_input.pop();
        self.mark_search_changed();
    }

    /// Marks the filter as changed, triggering a debounced search.
    fn mark_search_changed(&mut self) {
        self.search_changed_at = Some(Instant::now());
        self.search_pending = true;
    }

    /// Returns whether a search is pending and enough time has passed (debounce).
    ///
    /// Returns `true` if filter changed and at least `debounce_ms` milliseconds
    /// have passed since the last change.
    pub fn should_search(&self, debounce_ms: u64) -> bool {
        if !self.search_pending {
            return false;
        }
        match self.search_changed_at {
            Some(changed_at) => changed_at.elapsed().as_millis() >= u128::from(debounce_ms),
            None => false,
        }
    }

    /// Clears the search pending flag after a search is executed.
    pub fn clear_search_pending(&mut self) {
        self.search_pending = false;
    }

    /// Returns whether the filter is empty.
    pub fn search_is_empty(&self) -> bool {
        self.search_input.is_empty()
    }

    /// Sets the filtered/displayed notes without updating all_notes.
    ///
    /// Used when search results come from NoteService.dual_search().
    /// Resets selection when notes change.
    pub fn set_filtered_notes(&mut self, notes: Vec<Note>) {
        self.notes = notes;
        self.selected_index = None;
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
    fn focus_cycles_in_reverse_tab_order() {
        let mut app = App::new();
        assert_eq!(app.focus(), Focus::SearchInput);

        app.prev_focus();
        assert_eq!(app.focus(), Focus::DetailView);

        app.prev_focus();
        assert_eq!(app.focus(), Focus::NoteList);

        app.prev_focus();
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

    // --- Debounced Search Tests ---

    #[test]
    fn push_search_char_marks_search_pending() {
        let mut app = App::new();

        // Initially no search pending
        assert!(!app.should_search(0));

        // After typing, search becomes pending
        app.push_search_char('a');
        assert!(app.should_search(0)); // With 0ms debounce, should be ready immediately
        assert_eq!(app.search_input(), "a");
    }

    #[test]
    fn pop_search_char_marks_search_pending() {
        let mut app = App::new();
        app.push_search_char('a');
        app.push_search_char('b');
        app.clear_search_pending(); // Clear from push

        // After backspace, search becomes pending again
        app.pop_search_char();
        assert!(app.should_search(0));
        assert_eq!(app.search_input(), "a");
    }

    #[test]
    fn clear_search_pending_prevents_should_search() {
        let mut app = App::new();
        app.push_search_char('t');
        assert!(app.should_search(0));

        // After clearing, should_search returns false
        app.clear_search_pending();
        assert!(!app.should_search(0));
    }

    #[test]
    fn debounce_timing_works() {
        let mut app = App::new();
        app.push_search_char('x');

        // With 1000ms debounce, should NOT be ready immediately
        assert!(!app.should_search(1000));

        // But with 0ms debounce, should be ready
        assert!(app.should_search(0));
    }

    #[test]
    fn search_is_empty_returns_correctly() {
        let mut app = App::new();
        assert!(app.search_is_empty());

        app.push_search_char('z');
        assert!(!app.search_is_empty());

        app.pop_search_char();
        assert!(app.search_is_empty());
    }

    #[test]
    fn set_filtered_notes_updates_displayed_notes() {
        let mut app = App::new();

        // Set all notes first
        let all = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note A")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Note B")
                .build(),
        ];
        app.set_notes(all);
        assert_eq!(app.notes().len(), 2);
        assert_eq!(app.all_notes().len(), 2);

        // Simulate search results - only 1 note matches
        let filtered = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Note A")
                .build(),
        ];
        app.set_filtered_notes(filtered);

        // Displayed notes updated, but all_notes unchanged
        assert_eq!(app.notes().len(), 1);
        assert_eq!(app.all_notes().len(), 2);
        assert_eq!(app.notes()[0].content(), "Note A");
    }

    #[test]
    fn apply_filter_restores_all_notes_when_empty() {
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Hello world")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Goodbye world")
                .build(),
        ];
        app.set_notes(notes);

        // Simulate filtered results
        app.set_filtered_notes(vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Hello world")
                .build(),
        ]);
        assert_eq!(app.notes().len(), 1);

        // Apply filter with empty search restores all
        app.apply_filter();
        assert_eq!(app.notes().len(), 2);
    }

    #[test]
    fn apply_filter_filters_by_content() {
        let mut app = App::new();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Hello world")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("Goodbye moon")
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(3))
                .content("Hello again")
                .build(),
        ];
        app.set_notes(notes);

        // Type "hello" (case-insensitive)
        app.push_search_char('h');
        app.push_search_char('e');
        app.push_search_char('l');
        app.push_search_char('l');
        app.push_search_char('o');

        // Apply client-side filter
        app.apply_filter();

        // Should only show notes containing "hello"
        assert_eq!(app.notes().len(), 2);
        assert!(app.notes()[0].content().to_lowercase().contains("hello"));
        assert!(app.notes()[1].content().to_lowercase().contains("hello"));
    }
}
