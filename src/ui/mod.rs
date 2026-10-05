pub mod details;
pub mod footer;
pub mod layout;
pub mod table;
pub mod text;
pub mod theme;

use crate::app::{App, MessageLevel, Mode, ViewMode};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use theme::Theme;

pub fn draw(frame: &mut Frame, app: &App) {
    let root = frame.area();
    let theme = Theme::named(&app.config.ui.theme);
    let vertical = layout::create_main_layout(root);
    render_header(frame, vertical[0], app, &theme);
    let panels = layout::create_content_layout(vertical[1], app.view_mode);
    table::render(frame, panels[0], app, &theme);
    if app.view_mode == ViewMode::Detailed {
        details::render(frame, panels[1], app, &theme);
    }
    match app.mode {
        Mode::Search => footer::render_search(frame, vertical[2], app, &theme),
        Mode::EditingNote => footer::render_note_edit(frame, vertical[2], app, &theme),
        _ => footer::render_normal(frame, vertical[2], app, &theme),
    }
    draw_message(frame, app, &theme);
    match app.mode {
        Mode::Help => render_help_overlay(frame, root, app, &theme),
        Mode::OpenMenu | Mode::ConfigMenu | Mode::ChangingStatus => {
            footer::render_menu(frame, root, app, &theme)
        }
        _ => {}
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let dirty = app
        .projects
        .iter()
        .filter(|p| {
            p.git
                .as_ref()
                .is_some_and(|g| g.dirty_status == crate::project::DirtyStatus::Dirty)
        })
        .count();
    let top = Line::from(vec![
        Span::styled(" ds ", theme.title),
        Span::styled(
            format!("{}/{} projects", app.filtered_count(), app.total_projects),
            theme.count,
        ),
        Span::styled(
            format!(" · {}", format_scan_time(app.scan_duration_ms)),
            theme.muted,
        ),
        Span::styled(
            format!(" · {} dirty", dirty),
            if dirty > 0 { theme.dirty } else { theme.dim },
        ),
    ]);
    frame.render_widget(Paragraph::new(top), area);
}

fn draw_message(frame: &mut Frame, app: &App, theme: &Theme) {
    let Some(msg) = &app.status_message else {
        return;
    };
    let (label, style) = match app.message_level {
        MessageLevel::Error => ("Error", theme.health_bad),
        MessageLevel::Warning => ("Warning", theme.warning),
        MessageLevel::Info => ("Info", theme.count),
    };
    let root = frame.area();
    let lines = text::wrap(
        vec![Line::from(Span::styled(msg.clone(), style))],
        root.width.saturating_sub(2),
    );
    let height = (lines.len().min(u16::MAX as usize) as u16)
        .saturating_add(2)
        .min(root.height.saturating_sub(3));
    let area = Rect::new(
        root.x,
        root.y + root.height.saturating_sub(height + 1),
        root.width,
        height,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .title(format!(" {} | next action dismisses ", label)),
        ),
        area,
    );
}

fn format_scan_time(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn help_lines(theme: &Theme) -> Vec<Line<'static>> {
    let groups = [
        (
            "Navigation",
            vec![
                "↑ / k, ↓ / j   Select project / scroll focused details",
                "PageUp / PageDown   Move a page",
                "Home / End   First / last item",
                "Tab   Switch list / details focus",
            ],
        ),
        (
            "Actions",
            vec![
                "/   Search projects",
                "f   Cycle filter",
                "s   Cycle sort",
                "r   Reload scan",
                "n   Edit note",
                "m   Change status",
                "o   Open action menu",
                ",   Open config action menu",
                "Enter   Record visit",
                "D   Toggle compact / detailed",
            ],
        ),
        (
            "Modes",
            vec![
                "Search: type, Backspace deletes, Enter keeps filter",
                "Note: type, Backspace deletes, Enter saves",
                "Menus: arrows select, Enter confirms, or action key",
                "Esc   Cancel mode; in list, clear search",
                "Help: arrows / PageUp / PageDown / Home / End scroll",
            ],
        ),
        (
            "General",
            vec![
                "?   Show / close help",
                "q / Q   Quit (q closes help first)",
                "Ctrl+C   Quit",
            ],
        ),
    ];
    let mut lines = Vec::new();
    for (name, items) in groups {
        lines.push(Line::from(Span::styled(
            format!(" {}", name),
            theme.section_title,
        )));
        for item in items {
            lines.push(Line::from(Span::styled(format!("   {}", item), theme.text)));
        }
        lines.push(Line::default());
    }
    lines
}

pub fn help_scroll_limit(app: &App) -> usize {
    let area = text::popup(Rect::new(0, 0, app.viewport.0, app.viewport.1), 82, 36);
    text::wrap(
        help_lines(&Theme::named(&app.config.ui.theme)),
        area.width.saturating_sub(2),
    )
    .len()
    .saturating_sub(area.height.saturating_sub(3) as usize)
}

fn render_help_overlay(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let area = text::popup(area, 82, 36);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.title)
        .title(" Help ")
        .title_bottom(" ↑↓ scroll | PgUp/PgDn | Esc / ? close ");
    let inner = block.inner(area);
    let lines = text::wrap(help_lines(theme), inner.width);
    let visible = inner.height.saturating_sub(1) as usize;
    let scroll = app.help_scroll.min(lines.len().saturating_sub(visible));
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(
            lines
                .iter()
                .skip(scroll)
                .take(visible)
                .cloned()
                .collect::<Vec<_>>(),
        ),
        Rect::new(inner.x, inner.y, inner.width, visible as u16),
    );
    frame.render_widget(
        Paragraph::new(format!(
            " {}-{}/{}",
            if lines.is_empty() { 0 } else { scroll + 1 },
            (scroll + visible).min(lines.len()),
            lines.len()
        ))
        .style(theme.muted),
        Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            inner.height.min(1),
        ),
    );
}
