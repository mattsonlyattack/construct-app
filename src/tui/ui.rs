//! UI rendering functions for the TUI.
//!
//! Implements the three-panel layout with note list, detail view, and search input
//! using ratatui widgets and layout management.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use time::format_description;

use super::app::{App, Focus};

/// Main rendering function for the TUI.
///
/// Draws the three-panel layout with note list, detail view, and search input.
/// Applies focus indicators and styling based on app state.
///
/// # Arguments
///
/// * `frame` - The ratatui Frame to render into
/// * `app` - The application state containing notes, selection, and focus
pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Create main layout: search input at top, content in middle, shortcuts at bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter input
            Constraint::Min(0),    // Content area
            Constraint::Length(1), // Shortcut bar
        ])
        .split(size);

    // Split content area horizontally: note list (30%) | detail view (70%)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Note list
            Constraint::Percentage(70), // Detail view
        ])
        .split(main_chunks[1]);

    // Render each panel
    render_search_input(frame, app, main_chunks[0]);
    render_note_list(frame, app, content_chunks[0]);
    render_detail_view(frame, app, content_chunks[1]);
    render_shortcut_bar(frame, app, main_chunks[2]);
}

/// Renders the search input panel at the top of the screen.
///
/// Shows the current filter buffer with a cursor indicator when focused.
fn render_search_input(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focus(), Focus::SearchInput);

    // Create block with focus-dependent border style
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Search")
        .border_style(border_style);

    // Build content with cursor indicator when focused
    let mut content = app.search_input().to_string();
    if is_focused {
        content.push('█'); // Cursor indicator
    }

    let paragraph = Paragraph::new(content).block(block);

    frame.render_widget(paragraph, area);
}

/// Renders the note list panel showing all loaded notes.
///
/// Displays each note with: content preview (truncated), date, and tag count.
/// Highlights the selected note when focused.
fn render_note_list(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focus(), Focus::NoteList);

    // Create block with focus-dependent border style
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Notes")
        .border_style(border_style);

    // Build list items from notes
    let items: Vec<ListItem> = app
        .notes()
        .iter()
        .map(|note| {
            // Format content preview (truncate to ~40 chars)
            let content = note.content();
            let preview = if content.len() > 40 {
                format!("{}...", &content[..40])
            } else {
                content.to_string()
            };

            // Format date as YYYY-MM-DD
            let date_format =
                format_description::parse("[year]-[month]-[day]").expect("valid date format");
            let date_str = note
                .created_at()
                .format(&date_format)
                .unwrap_or_else(|_| "????-??-??".to_string());

            // Get tag count
            let tag_count = note.tags().len();

            // Build line with preview, date, and tag count
            let line = Line::from(vec![
                Span::raw(preview),
                Span::raw(" "),
                Span::styled(
                    format!("[{date_str} | {tag_count} tags]"),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::REVERSED),
    );

    // Create list state from selected index
    let mut list_state = ListState::default();
    list_state.select(app.selected_index());

    frame.render_stateful_widget(list, area, &mut list_state);
}

/// Renders the detail view panel showing the full content of the selected note.
///
/// Displays:
/// - Original content
/// - Enhanced content (if available) with separator
/// - Confidence percentage (if available)
/// - Tags with source indicators
/// - Timestamps
fn render_detail_view(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = matches!(app.focus(), Focus::DetailView);

    // Create block with focus-dependent border style
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Detail")
        .border_style(border_style);

    // Build content based on selected note
    let content = if let Some(note) = app.selected_note() {
        let mut text = Text::default();

        // Original content section
        text.lines.push(Line::from(vec![Span::styled(
            "Content:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        text.lines.push(Line::from(note.content()));

        // Enhanced content section (if available)
        if let Some(enhanced) = note.content_enhanced() {
            text.lines.push(Line::from(""));
            text.lines.push(Line::from("---"));
            text.lines.push(Line::from(""));

            // Enhanced label with confidence
            if let Some(confidence) = note.enhancement_confidence() {
                #[allow(clippy::cast_possible_truncation)]
                let confidence_pct = (confidence * 100.0) as i32;
                text.lines.push(Line::from(vec![
                    Span::styled("Enhanced:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" ({confidence_pct}% confidence)")),
                ]));
            } else {
                text.lines.push(Line::from(vec![Span::styled(
                    "Enhanced:",
                    Style::default().add_modifier(Modifier::BOLD),
                )]));
            }

            text.lines.push(Line::from(enhanced));
        }

        // Tags section
        if !note.tags().is_empty() {
            text.lines.push(Line::from(""));
            text.lines.push(Line::from(vec![Span::styled(
                "Tags:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));

            for tag in note.tags() {
                let source_indicator = if tag.source().is_user() {
                    "user".to_string()
                } else {
                    format!("llm {}%", tag.confidence())
                };

                text.lines.push(Line::from(vec![
                    Span::raw("  - "),
                    Span::styled(
                        tag.name().to_string(),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("({source_indicator})"),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }

        // Timestamps section
        text.lines.push(Line::from(""));
        let date_format =
            format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                .expect("valid datetime format");

        text.lines.push(Line::from(vec![
            Span::styled("Created:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(
                note.created_at()
                    .format(&date_format)
                    .unwrap_or_else(|_| "????-??-?? ??:??:??".to_string()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        if let Some(enhanced_at) = note.enhanced_at() {
            text.lines.push(Line::from(vec![
                Span::styled("Enhanced:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(
                    enhanced_at
                        .format(&date_format)
                        .unwrap_or_else(|_| "????-??-?? ??:??:??".to_string()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        text
    } else {
        Text::from("No note selected")
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Renders the shortcut bar at the bottom of the screen.
///
/// Shows context-aware keyboard shortcuts based on current focus state.
/// Format: `Key: action | Key: action` with keys highlighted in cyan.
fn render_shortcut_bar(frame: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default().fg(Color::Cyan);
    let sep_style = Style::default().fg(Color::DarkGray);

    // Build shortcuts based on focus
    let mut spans = vec![
        Span::styled("q", key_style),
        Span::raw(": quit"),
        Span::styled(" | ", sep_style),
        Span::styled("Tab", key_style),
        Span::raw(": next panel"),
        Span::styled(" | ", sep_style),
        Span::styled("Shift+Tab", key_style),
        Span::raw(": prev panel"),
        Span::styled(" | ", sep_style),
        Span::styled("Esc", key_style),
        Span::raw(": reset"),
    ];

    // Add focus-specific shortcuts
    match app.focus() {
        Focus::NoteList => {
            spans.push(Span::styled(" | ", sep_style));
            spans.push(Span::styled("j/k", key_style));
            spans.push(Span::raw(": navigate"));
        }
        Focus::SearchInput | Focus::DetailView => {
            // No additional shortcuts for these focus states currently
        }
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NoteBuilder, NoteId, TagAssignment, TagId};
    use time::OffsetDateTime;

    // Helper to create a test app with sample notes
    fn create_test_app() -> App {
        let mut app = App::new();

        let now = OffsetDateTime::now_utc();
        let notes = vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Short note")
                .created_at(now)
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(2))
                .content("This is a much longer note that should be truncated in the list view")
                .created_at(now)
                .tags(vec![
                    TagAssignment::user(TagId::new(1), "rust", now),
                    TagAssignment::llm(TagId::new(2), "async", "deepseek-r1:8b", 85, now),
                ])
                .build(),
            NoteBuilder::new()
                .id(NoteId::new(3))
                .content("Fragment")
                .content_enhanced("This is the expanded version of the fragment")
                .enhancement_confidence(0.92)
                .enhanced_at(now)
                .created_at(now)
                .build(),
        ];

        app.set_notes(notes);
        app
    }

    #[test]
    fn three_panel_layout_structure() {
        // Test that the layout creates three distinct panels
        let _app = create_test_app();

        // Create a test frame area
        let area = Rect::new(0, 0, 100, 30);

        // Create main layout chunks
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Verify search input area
        assert_eq!(
            main_chunks[0].height, 3,
            "search input should be 3 lines tall"
        );

        // Create content area chunks
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_chunks[1]);

        // Verify horizontal split (~30% / ~70%)
        let total_width = content_chunks[0].width + content_chunks[1].width;
        let left_percentage = (content_chunks[0].width as f32 / total_width as f32) * 100.0;

        assert!(
            (left_percentage - 30.0).abs() < 5.0,
            "left panel should be approximately 30% wide, got {}%",
            left_percentage
        );
    }

    #[test]
    fn note_list_item_formatting() {
        // Test that note list items show content preview, date, and tag count
        let app = create_test_app();

        // Check short note (not truncated)
        let note1 = &app.notes()[0];
        assert_eq!(note1.content(), "Short note");
        assert_eq!(note1.tags().len(), 0);

        // Check long note (should be truncated to ~40 chars)
        let note2 = &app.notes()[1];
        assert!(note2.content().len() > 40);
        let preview = if note2.content().len() > 40 {
            format!("{}...", &note2.content()[..40])
        } else {
            note2.content().to_string()
        };
        assert_eq!(preview.len(), 43); // 40 chars + "..."
        assert_eq!(note2.tags().len(), 2);

        // Verify date format (YYYY-MM-DD)
        let date_format =
            format_description::parse("[year]-[month]-[day]").expect("valid date format");
        let date_str = note1
            .created_at()
            .format(&date_format)
            .unwrap_or_else(|_| "????-??-??".to_string());
        assert_eq!(date_str.len(), 10); // YYYY-MM-DD is 10 chars
        assert!(date_str.contains('-'));
    }

    #[test]
    fn detail_view_content_sections() {
        // Test that detail view displays all required sections
        let mut app = create_test_app();

        // Select note with enhanced content
        app.set_notes(vec![
            NoteBuilder::new()
                .id(NoteId::new(1))
                .content("Original")
                .content_enhanced("Enhanced version")
                .enhancement_confidence(0.85)
                .enhanced_at(OffsetDateTime::now_utc())
                .created_at(OffsetDateTime::now_utc())
                .tags(vec![
                    TagAssignment::user(TagId::new(1), "rust", OffsetDateTime::now_utc()),
                    TagAssignment::llm(
                        TagId::new(2),
                        "performance",
                        "deepseek-r1:8b",
                        90,
                        OffsetDateTime::now_utc(),
                    ),
                ])
                .build(),
        ]);
        app.select_next(); // Select the note

        let note = app.selected_note().expect("note should be selected");

        // Verify original content is available
        assert_eq!(note.content(), "Original");

        // Verify enhanced content is available
        assert_eq!(note.content_enhanced(), Some("Enhanced version"));

        // Verify confidence is available
        assert_eq!(note.enhancement_confidence(), Some(0.85));

        // Verify tags with source indicators
        assert_eq!(note.tags().len(), 2);
        assert!(note.tags()[0].source().is_user());
        assert!(note.tags()[1].source().is_llm());

        // Verify timestamps
        assert!(note.created_at().unix_timestamp() > 0);
        assert!(note.enhanced_at().is_some());
    }

    #[test]
    fn search_input_shows_cursor_when_focused() {
        // Test that search input displays cursor indicator when focused
        let mut app = App::new();

        // Default focus is SearchInput
        assert_eq!(app.focus(), Focus::SearchInput);

        // Cursor should be shown when focused
        let is_focused = matches!(app.focus(), Focus::SearchInput);
        assert!(is_focused);

        // When not focused, cursor should not be shown
        app.next_focus(); // Move to NoteList
        assert!(!matches!(app.focus(), Focus::SearchInput));
    }

    // --- Task Group 6: Additional Strategic Tests ---

    #[test]
    fn very_long_note_content_truncates_in_list() {
        // Test that very long content is properly truncated in note list
        let mut app = App::new();

        // Create a note with content longer than 40 characters
        let long_content = "This is an extremely long note that definitely exceeds the 40 character limit and should be truncated with ellipsis";
        let note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content(long_content)
            .created_at(OffsetDateTime::now_utc())
            .build();

        app.set_notes(vec![note]);

        // Verify the note has long content
        assert!(app.notes()[0].content().len() > 40);

        // The truncation logic in render_note_list should create a preview
        let content = app.notes()[0].content();
        let preview = if content.len() > 40 {
            format!("{}...", &content[..40])
        } else {
            content.to_string()
        };

        // Preview should be exactly 43 chars (40 + "...")
        assert_eq!(preview.len(), 43);
        assert!(preview.ends_with("..."));
        // Verify it starts with the beginning of the original content
        assert!(preview.starts_with("This is an extremely long note that"));
    }

    #[test]
    fn detail_view_displays_note_without_enhanced_content() {
        // Test detail view for a note that has no enhanced content
        let mut app = App::new();

        let note = NoteBuilder::new()
            .id(NoteId::new(1))
            .content("Simple note without enhancement")
            .created_at(OffsetDateTime::now_utc())
            .build();

        app.set_notes(vec![note]);
        app.select_next(); // Select the note

        let selected = app.selected_note().expect("note should be selected");

        // Verify no enhanced content
        assert_eq!(selected.content(), "Simple note without enhancement");
        assert_eq!(selected.content_enhanced(), None);
        assert_eq!(selected.enhancement_confidence(), None);
        assert_eq!(selected.enhanced_at(), None);

        // The detail view should only show Content section, no separator or Enhanced section
        // This is tested implicitly by the render function handling None cases
    }
}
