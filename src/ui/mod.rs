pub mod details;
pub mod footer;
pub mod layout;
pub mod table;
pub mod theme;

use ratatui::Frame;

use crate::app::App;

/// Main draw function for the TUI.
pub fn draw(frame: &mut Frame, app: &App) {
    let root = frame.area();

    let theme = theme::Theme::default();

    // Vertical split: header, main, footer
    let vertical = layout::create_main_layout(root);

    // Render header
    render_header(frame, vertical[0], app, &theme);

    // Horizontal split for main area: left table, right details
    let main_area = layout::create_content_layout(vertical[1]);

    // Render project table
    table::render(frame, main_area[0], app, &theme);

    // Render details panel
    details::render(frame, main_area[1], app, &theme);

    // Render footer (mode-dependent)
    match app.mode {
        crate::app::Mode::Search => footer::render_search(frame, vertical[2], app, &theme),
        crate::app::Mode::EditingNote => footer::render_note_edit(frame, vertical[2], app, &theme),
        crate::app::Mode::ChangingStatus => {
            footer::render_status_change(frame, vertical[2], app, &theme)
        }
        crate::app::Mode::Help => {
            render_help_overlay(frame, root, app, &theme);
            footer::render_normal(frame, vertical[2], app, &theme);
        }
        crate::app::Mode::Normal => footer::render_normal(frame, vertical[2], app, &theme),
    }
}

fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, theme: &theme::Theme) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(45)])
        .split(area);

    let title = if let Some(ref msg) = app.status_message {
        Line::from(vec![
            Span::styled(" ", theme.title),
            Span::styled("devscope", theme.title),
            Span::styled("  ", theme.dim),
            Span::styled(msg.as_str(), theme.active),
        ])
    } else {
        Line::from(vec![
            Span::styled(" ", theme.title),
            Span::styled("devscope", theme.title),
            Span::styled("  ", theme.dim),
            Span::styled(format!("{}", app.total_projects), theme.count),
            Span::styled(" projects", theme.muted),
            Span::styled("  ", theme.dim),
            Span::styled(format_scan_time(app.scan_duration_ms), theme.muted),
        ])
    };
    frame.render_widget(Paragraph::new(title), chunks[0]);

    let info = Line::from(vec![
        Span::styled("filter ", theme.muted),
        Span::styled(app.filter.as_str(), theme.filter),
        Span::styled("  ", theme.dim),
        Span::styled(
            format!("{}/{}", app.filtered_count(), app.total_projects),
            theme.count,
        ),
    ]);
    frame.render_widget(
        Paragraph::new(info).alignment(ratatui::layout::Alignment::Right),
        chunks[1],
    );
}

fn format_scan_time(ms: u128) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

fn render_help_overlay(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    _app: &App,
    theme: &theme::Theme,
) {
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let help_text = vec![
        Line::from(Span::styled("  Keyboard Shortcuts", theme.title)),
        Line::from(""),
        Line::from(Span::styled("  Navigation", theme.filter)),
        Line::from("    ↑ / k       Move up"),
        Line::from("    ↓ / j       Move down"),
        Line::from("    PageUp      Move up 10"),
        Line::from("    PageDown    Move down 10"),
        Line::from("    Home        First project"),
        Line::from("    End         Last project"),
        Line::from(""),
        Line::from(Span::styled("  Actions", theme.filter)),
        Line::from("    /           Search"),
        Line::from("    f           Cycle filter"),
        Line::from("    s           Cycle sort"),
        Line::from("    r           Reload scan"),
        Line::from("    n           Edit note"),
        Line::from("    m           Change status"),
        Line::from("    o           Open / print path"),
        Line::from("    Enter       Toggle details"),
        Line::from(""),
        Line::from(Span::styled("  General", theme.filter)),
        Line::from("    ?           This help"),
        Line::from("    Esc         Cancel / back"),
        Line::from("    q / Q       Quit"),
        Line::from(""),
        Line::from(Span::styled("  Press Esc or ? to close", theme.dim)),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(theme.border);

    let paragraph = Paragraph::new(help_text).block(block);

    // Center the help window
    let area = centered_rect(60, 70, area);
    frame.render_widget(Clear, area);
    frame.render_widget(paragraph, area);
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout};

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);

    horizontal[1]
}
