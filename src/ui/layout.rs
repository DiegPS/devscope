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
    let use_vertical = area.width < 125;

    let (direction, constraints) = if use_vertical {
        (
            Direction::Vertical,
            vec![Constraint::Percentage(56), Constraint::Percentage(44)],
        )
    } else {
        (
            Direction::Horizontal,
            vec![Constraint::Percentage(60), Constraint::Percentage(40)],
        )
    };

    Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area)
        .to_vec()
}
