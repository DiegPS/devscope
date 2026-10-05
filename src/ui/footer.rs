use crate::{
    app::{App, Mode},
    ui::{text, theme::Theme},
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub fn render_normal(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let primary = [
        ("↑↓", if app.details_focus { "scroll" } else { "nav" }),
        ("/", "search"),
        ("o", "open"),
    ];
    let tail = [("?", "help"), ("q", "quit")];
    let mut pairs = primary.to_vec();
    for pair in [
        ("Tab", "focus"),
        ("f", "filter"),
        ("s", "sort"),
        ("n", "note"),
        ("m", "status"),
        ("r", "reload"),
        ("D", "view"),
        (",", "config"),
        ("Enter", "visit"),
    ] {
        let mut trial = pairs.clone();
        trial.push(pair);
        trial.extend(tail);
        if footer_width(&trial) <= area.width as usize {
            pairs.push(pair);
        }
    }
    pairs.extend(tail);
    let mut spans = Vec::new();
    for (i, (key, label)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", theme.footer_sep));
        }
        spans.push(Span::styled(key.to_string(), theme.footer_key));
        spans.push(Span::styled(format!(" {}", label), theme.footer_hint));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn footer_width(pairs: &[(&str, &str)]) -> usize {
    pairs
        .iter()
        .map(|(k, v)| k.width() + v.width() + 1)
        .sum::<usize>()
        + pairs.len().saturating_sub(1) * 3
}

fn render_editor(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    hint: &str,
    theme: &Theme,
) {
    let hint = if area.width >= 50 { hint } else { " ↵ / Esc" };
    let budget = (area.width as usize).saturating_sub(label.width() + hint.width() + 2);
    let value = text::tail(value, budget);
    let line = Line::from(vec![
        Span::styled(format!("{} ", label), theme.footer_key),
        Span::styled(value, theme.text),
        Span::styled("█", theme.text),
        Span::styled(hint.to_string(), theme.muted),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

pub fn render_search(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    render_editor(
        frame,
        area,
        "Search",
        &app.search_query,
        " · Enter keep / Esc clear",
        theme,
    );
}
pub fn render_note_edit(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    render_editor(
        frame,
        area,
        "Note",
        &app.note_input,
        " · Enter save / Esc cancel",
        theme,
    );
}

pub fn render_menu(frame: &mut Frame, root: Rect, app: &App, theme: &Theme) {
    let status = app.mode == Mode::ChangingStatus;
    let selected = if status {
        app.status_selected
    } else {
        app.menu_selected
    };
    let items: Vec<String> = if status {
        app.status_options
            .iter()
            .map(|s| s.as_str().to_string())
            .collect()
    } else {
        app.config
            .open
            .actions
            .iter()
            .map(|a| format!("[{}] {}", a.key_char(), a.name))
            .collect()
    };
    let title = match app.mode {
        Mode::ChangingStatus => " Status ",
        Mode::ConfigMenu => " Open config ",
        _ => " Open project ",
    };
    let area = text::popup(
        root,
        72,
        (items.len().saturating_add(5).min(u16::MAX as usize) as u16).max(6),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.title)
        .title(title)
        .title_bottom(" ↑↓ select | Enter / key | Esc cancel ");
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    let context = if status {
        app.selected_project()
            .map(|p| p.name.clone())
            .unwrap_or_default()
    } else if app.mode == Mode::ConfigMenu {
        "Configuration directory".into()
    } else {
        app.selected_project()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "No project selected".into())
    };
    frame.render_widget(
        Paragraph::new(crate::ui::table::truncate_end(
            &context,
            inner.width.saturating_sub(2) as usize,
        ))
        .style(theme.title),
        Rect::new(inner.x, inner.y, inner.width, inner.height.min(1)),
    );
    let mut lines = Vec::new();
    let mut selected_start = 0;
    for (i, item) in items.iter().enumerate() {
        if i == selected {
            selected_start = lines.len();
        }
        lines.extend(text::wrap(
            vec![Line::from(Span::styled(
                format!("{} {}", if i == selected { ">" } else { " " }, item),
                if i == selected {
                    theme.selected
                } else {
                    theme.text
                },
            ))],
            inner.width,
        ));
    }
    let visible = inner.height.saturating_sub(2) as usize;
    let scroll = selected_start
        .saturating_sub(visible.saturating_sub(1))
        .min(lines.len().saturating_sub(visible));
    frame.render_widget(
        Paragraph::new(
            lines
                .into_iter()
                .skip(scroll)
                .take(visible)
                .collect::<Vec<_>>(),
        ),
        Rect::new(
            inner.x,
            inner.y + inner.height.min(1),
            inner.width,
            visible as u16,
        ),
    );
    let footer = if let Some(msg) = &app.status_message {
        crate::ui::table::truncate_end(msg, inner.width as usize)
    } else {
        format!(
            " {}/{} options",
            if items.is_empty() { 0 } else { selected + 1 },
            items.len()
        )
    };
    frame.render_widget(
        Paragraph::new(footer).style(theme.muted),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            inner.height.min(1),
        ),
    );
}
