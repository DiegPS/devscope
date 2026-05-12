use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;

/// Render the normal mode footer with key hints.
pub fn render_normal(frame: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    let keys = vec![
        key_hint("↑↓", "move", theme),
        key_hint("/", "search", theme),
        key_hint("f", "filter", theme),
        key_hint("s", "sort", theme),
        key_hint("n", "note", theme),
        key_hint("m", "status", theme),
        key_hint("r", "reload", theme),
        key_hint("o", "open", theme),
        key_hint("Enter", "visit", theme),
        key_hint("?", "help", theme),
        key_hint("q", "quit", theme),
    ];

    let line = Line::from(keys);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

/// Render the search mode footer.
pub fn render_search(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled("  Search: ", theme.footer_key),
        Span::styled(&app.search_query, theme.text),
        Span::styled("█", theme.text),
        Span::styled("  (Esc to cancel, Enter to confirm)", theme.dim),
    ]);

    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

/// Render the note editing footer.
pub fn render_note_edit(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled("  Note: ", theme.footer_key),
        Span::styled(&app.note_input, theme.text),
        Span::styled("█", theme.text),
        Span::styled("  (Enter to save, Esc to cancel)", theme.dim),
    ]);

    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

/// Render the status change footer.
pub fn render_status_change(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let options: Vec<Span> = app
        .status_options
        .iter()
        .enumerate()
        .flat_map(|(i, status)| {
            let style = if i == app.status_selected {
                theme.footer_key
            } else {
                theme.dim
            };
            let mut items = vec![Span::styled(format!(" {} ", status.as_str()), style)];
            if i + 1 < app.status_options.len() {
                items.push(Span::styled(" │ ", theme.dim));
            }
            items
        })
        .collect();

    let mut line_items = vec![Span::styled("  Status: ", theme.footer_key)];
    line_items.extend(options);
    line_items.push(Span::styled(
        "  (↑↓ select, Enter confirm, Esc cancel)",
        theme.dim,
    ));

    let line = Line::from(line_items);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

fn key_hint<'a>(key: &'a str, label: &'a str, theme: &Theme) -> Span<'a> {
    Span::styled(format!(" {}:{} ", key, label), theme.footer_key)
}
