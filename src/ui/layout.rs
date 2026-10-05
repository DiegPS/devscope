use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::ViewMode;

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
pub fn create_content_layout(area: Rect, view_mode: ViewMode) -> Vec<Rect> {
    if matches!(view_mode, ViewMode::Compact) {
        return vec![area];
    }

    // Preserve the original list density: side-by-side from 125 columns.
    // Short windows also benefit from full-height panels at 110 columns.
    let use_vertical = area.width < 125 && !(area.width >= 110 && area.height < 22);

    let (direction, constraints) = if use_vertical {
        (
            Direction::Vertical,
            vec![Constraint::Percentage(56), Constraint::Percentage(44)],
        )
    } else {
        (
            Direction::Horizontal,
            vec![
                Constraint::Min(48),
                Constraint::Length((area.width / 3).max(48)),
            ],
        )
    };

    Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area)
        .to_vec()
}
