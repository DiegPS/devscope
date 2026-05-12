use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::project::ProjectStatus;
use crate::ui::theme::Theme;

/// Render the main project table in the left panel.
pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Stack"),
        Cell::from("Activity"),
        Cell::from("Status"),
        Cell::from("Git"),
        Cell::from("Note"),
    ])
    .style(theme.header)
    .height(1);

    let visible_rows = area.height.saturating_sub(3).max(1) as usize;
    let total = app.filtered_indices.len();
    let sel = app.selected;

    let scroll = calc_scroll(sel, visible_rows, total);

    let mut rows: Vec<Row> = Vec::new();

    for (display_idx, &proj_idx) in app
        .filtered_indices
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_rows)
    {
        let project = &app.projects[proj_idx];
        let is_selected = display_idx == sel;

        let name = truncate_str(&project.name, 22);

        let stack_str = project.stack.join("+");
        let stack = truncate_str(&stack_str, 20);

        let activity = project.activity.relative_time.clone();

        let status = project.status.as_str();

        let git_info = match &project.git {
            Some(g) => {
                let dirty_marker = if g.is_dirty { "*" } else { "" };
                format!("{}{}", g.branch, dirty_marker)
            }
            None => "-".to_string(),
        };

        let note = project
            .note
            .as_deref()
            .map(|n| truncate_str(n, 20))
            .unwrap_or_default();

        let status_style = match project.status {
            ProjectStatus::Active => theme.active,
            ProjectStatus::Paused => theme.paused,
            ProjectStatus::Stale => theme.stale,
            ProjectStatus::Archived => theme.archived,
            ProjectStatus::Unknown => theme.dim,
        };

        let git_style = if project.git.as_ref().is_some_and(|g| g.is_dirty) {
            theme.dirty
        } else {
            theme.clean
        };

        let row_style = if is_selected {
            theme.selected
        } else {
            Style::default()
        };

        let row = Row::new(vec![
            Cell::from(Line::from(Span::styled(
                name,
                if is_selected {
                    theme.selected
                } else {
                    theme.text
                },
            ))),
            Cell::from(Line::from(Span::styled(
                stack,
                if is_selected {
                    theme.selected
                } else {
                    theme.stack
                },
            ))),
            Cell::from(Line::from(Span::styled(
                activity,
                if is_selected {
                    theme.selected
                } else {
                    theme.dim
                },
            ))),
            Cell::from(Line::from(Span::styled(
                status.to_string(),
                if is_selected {
                    theme.selected
                } else {
                    status_style
                },
            ))),
            Cell::from(Line::from(Span::styled(
                git_info,
                if is_selected {
                    theme.selected
                } else {
                    git_style
                },
            ))),
            Cell::from(Line::from(Span::styled(
                note,
                if is_selected {
                    theme.selected
                } else {
                    theme.note
                },
            ))),
        ])
        .style(row_style);

        rows.push(row);
    }

    let widths = [
        ratatui::layout::Constraint::Length(22),
        ratatui::layout::Constraint::Length(20),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Length(10),
        ratatui::layout::Constraint::Length(15),
        ratatui::layout::Constraint::Min(10),
    ];

    let title = if total > visible_rows {
        format!(
            " Projects ({}-{}/{}) ",
            scroll + 1,
            (scroll + rows.len()).min(total),
            total
        )
    } else {
        format!(" Projects ({}) ", total)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Span::styled(title, theme.header));

    let table = Table::new(rows, widths).header(header).block(block);

    frame.render_widget(table, area);
}

fn calc_scroll(selected: usize, visible_rows: usize, total: usize) -> usize {
    if total <= visible_rows {
        return 0;
    }
    let half = visible_rows / 2;
    if selected < half {
        0
    } else if selected + half >= total {
        total.saturating_sub(visible_rows)
    } else {
        selected - half
    }
}

fn truncate_str(s: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    if s.width() <= max_width {
        s.to_string()
    } else {
        let mut result = String::new();
        let mut width = 0;
        for ch in s.chars() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
            if width + ch_width + 3 > max_width {
                result.push_str("...");
                break;
            }
            result.push(ch);
            width += ch_width;
        }
        result
    }
}
