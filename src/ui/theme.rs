use ratatui::style::Color;
use ratatui::style::Style;

pub struct Theme {
    pub title: Style,
    pub text: Style,
    pub dim: Style,
    pub muted: Style,
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
    pub footer_hint: Style,
    pub footer_sep: Style,
    pub health_good: Style,
    pub health_warn: Style,
    pub health_bad: Style,
    pub ahead_behind: Style,
    pub section_title: Style,
    pub table_header: Style,
    pub command: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            title: Style::default()
                .fg(Color::Rgb(133, 221, 212))
                .add_modifier(ratatui::style::Modifier::BOLD),
            text: Style::default().fg(Color::Rgb(222, 226, 230)),
            dim: Style::default().fg(Color::Rgb(107, 114, 128)),
            muted: Style::default().fg(Color::Rgb(148, 163, 184)),
            header: Style::default()
                .fg(Color::Rgb(255, 214, 102))
                .add_modifier(ratatui::style::Modifier::BOLD),
            selected: Style::default()
                .bg(Color::Rgb(33, 61, 72))
                .fg(Color::Rgb(248, 250, 252))
                .add_modifier(ratatui::style::Modifier::BOLD),
            active: Style::default().fg(Color::Rgb(122, 222, 122)),
            paused: Style::default().fg(Color::Rgb(255, 199, 95)),
            stale: Style::default().fg(Color::Rgb(148, 163, 184)),
            archived: Style::default().fg(Color::Rgb(255, 117, 117)),
            dirty: Style::default().fg(Color::Rgb(255, 135, 107)),
            clean: Style::default().fg(Color::Rgb(122, 222, 122)),
            warning: Style::default().fg(Color::Rgb(255, 199, 95)),
            filter: Style::default().fg(Color::Rgb(255, 214, 102)),
            count: Style::default().fg(Color::Rgb(133, 221, 212)),
            border: Style::default().fg(Color::Rgb(71, 85, 105)),
            note: Style::default().fg(Color::Rgb(255, 214, 153)),
            stack: Style::default().fg(Color::Rgb(144, 205, 244)),
            footer: Style::default().fg(Color::Rgb(148, 163, 184)),
            footer_key: Style::default()
                .fg(Color::Rgb(133, 221, 212))
                .add_modifier(ratatui::style::Modifier::BOLD),
            footer_hint: Style::default().fg(Color::Rgb(203, 213, 225)),
            footer_sep: Style::default().fg(Color::Rgb(71, 85, 105)),
            health_good: Style::default().fg(Color::Rgb(122, 222, 122)),
            health_warn: Style::default().fg(Color::Rgb(255, 199, 95)),
            health_bad: Style::default().fg(Color::Rgb(255, 117, 117)),
            ahead_behind: Style::default().fg(Color::Rgb(133, 221, 212)),
            section_title: Style::default()
                .fg(Color::Rgb(255, 214, 102))
                .add_modifier(ratatui::style::Modifier::BOLD),
            table_header: Style::default()
                .fg(Color::Rgb(160, 231, 224))
                .add_modifier(ratatui::style::Modifier::BOLD),
            command: Style::default().fg(Color::Rgb(133, 221, 212)),
        }
    }
}
