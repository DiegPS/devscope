use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::ui::theme::Theme;

pub fn render_normal(frame: &mut Frame, area: Rect, _app: &App, theme: &Theme) {
    let line = build_footer(area.width, theme);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

pub fn render_search(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled("  ", theme.footer_key),
        Span::styled("Search ", theme.footer_key),
        Span::styled(&app.search_query, theme.text),
        Span::styled("\u{2588}", theme.text),
        Span::styled("  (Esc cancel, Enter done)", theme.dim),
    ]);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

pub fn render_note_edit(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let line = Line::from(vec![
        Span::styled("  ", theme.footer_key),
        Span::styled("Note ", theme.footer_key),
        Span::styled(&app.note_input, theme.text),
        Span::styled("\u{2588}", theme.text),
        Span::styled("  (Enter save, Esc cancel)", theme.dim),
    ]);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

pub fn render_open_menu(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let sep = Span::styled(" \u{00B7} ", theme.footer_sep);
    let mut spans: Vec<Span> = Vec::new();

    spans.push(Span::styled("  OPEN", theme.footer_key));
    spans.push(sep.clone());

    for (i, action) in app.config.open.actions.iter().enumerate() {
        if i > 0 {
            spans.push(sep.clone());
        }
        spans.push(Span::styled(
            format!("{}", action.key_char()),
            theme.footer_key,
        ));
        spans.push(Span::styled(format!(" {}", action.name), theme.footer_hint));
    }

    spans.push(sep);
    spans.push(Span::styled("Esc cancel", theme.dim));

    let line = Line::from(spans);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

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
                items.push(Span::styled(" \u{2502} ", theme.dim));
            }
            items
        })
        .collect();

    let mut line_items = vec![
        Span::styled("  ", theme.footer_key),
        Span::styled("Status ", theme.footer_key),
    ];
    line_items.extend(options);
    line_items.push(Span::styled(
        "  (arrows select, Enter confirm, Esc cancel)",
        theme.dim,
    ));

    let line = Line::from(line_items);
    let footer = Paragraph::new(line).style(theme.footer);
    frame.render_widget(footer, area);
}

fn build_footer(width: u16, theme: &Theme) -> Line<'static> {
    let short = width < 90;
    let sep = Span::styled(" \u{00B7} ", theme.footer_sep);

    let pairs: Vec<(&str, &str)> = if short {
        vec![
            ("\u{2191}\u{2193}", "nav"),
            ("/", "search"),
            ("f", "filter"),
            ("D", "view"),
            ("?", "help"),
            ("q", "quit"),
        ]
    } else {
        vec![
            ("\u{2191}\u{2193}", "nav"),
            ("/", "search"),
            ("f", "filter"),
            ("s", "sort"),
            ("n", "note"),
            ("m", "status"),
            ("r", "reload"),
            ("o", "open"),
            ("D", "view"),
            ("?", "help"),
            ("q", "quit"),
        ]
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(sep.clone());
        }
        spans.push(Span::styled(key.to_string(), theme.footer_key));
        spans.push(Span::styled(format!(" {}", label), theme.footer_hint));
    }

    let mut line_spans = vec![Span::styled("  ", theme.footer_sep)];
    line_spans.extend(spans);

    Line::from(line_spans)
}
