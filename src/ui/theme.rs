use ratatui::style::Color;
use ratatui::style::Style;

pub struct Theme {
    pub title: Style,
    pub text: Style,
    pub dim: Style,
    pub header: Style,
    pub selected: Style,
    pub active: Style,
    pub paused: Style,
    pub stale: Style,
    pub archived: Style,
    pub dirty: Style,
    pub clean: Style,
    pub warning: Style,
    pub filter: Style,
    pub count: Style,
    pub border: Style,
    pub note: Style,
    pub stack: Style,
    pub footer: Style,
    pub footer_key: Style,
    pub health_good: Style,
    pub health_warn: Style,
    pub health_bad: Style,
    pub ahead_behind: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
            text: Style::default().fg(Color::White),
            dim: Style::default().fg(Color::DarkGray),
            header: Style::default()
                .fg(Color::LightCyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
            selected: Style::default().bg(Color::Blue).fg(Color::White),
            active: Style::default().fg(Color::Green),
            paused: Style::default().fg(Color::Yellow),
            stale: Style::default().fg(Color::DarkGray),
            archived: Style::default().fg(Color::Red),
            dirty: Style::default().fg(Color::LightRed),
            clean: Style::default().fg(Color::Green),
            warning: Style::default().fg(Color::Yellow),
            filter: Style::default().fg(Color::LightMagenta),
            count: Style::default().fg(Color::Cyan),
            border: Style::default().fg(Color::DarkGray),
            note: Style::default().fg(Color::LightYellow),
            stack: Style::default().fg(Color::LightBlue),
            footer: Style::default().bg(Color::DarkGray).fg(Color::White),
            footer_key: Style::default()
                .bg(Color::DarkGray)
                .fg(Color::LightCyan)
                .add_modifier(ratatui::style::Modifier::BOLD),
            health_good: Style::default().fg(Color::Green),
            health_warn: Style::default().fg(Color::Yellow),
            health_bad: Style::default().fg(Color::Red),
            ahead_behind: Style::default().fg(Color::Cyan),
        }
    }
}
