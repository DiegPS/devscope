use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Create the main vertical layout: header, content, footer.
pub fn create_main_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area)
        .to_vec()
}

/// Create the horizontal split for content: left table, right details.
pub fn create_content_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area)
        .to_vec()
}
