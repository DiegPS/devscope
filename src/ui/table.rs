use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::{App, ViewMode};
use crate::health::format_git_label;
use crate::project::{HealthLevel, ProjectStatus};
use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let is_compact = matches!(app.view_mode, ViewMode::Compact);
    let inner_w = area.width.saturating_sub(2) as usize;

    let (header_labels, widths) = if is_compact {
        (vec!["Name", "Stack", "Git", "H"], compact_widths(inner_w))
    } else {
        (
            vec!["Name", "Stack", "Activity", "Status", "Git", "Note", "H"],
            detailed_widths(inner_w),
        )
    };

    let header = Row::new(header_labels.iter().map(|c| Cell::from(*c)))
        .style(theme.table_header)
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

        let sel_style = if is_selected {
            theme.selected
        } else {
            Style::default()
        };

        let row_style = sel_style;

        let name_max = if is_compact {
            inner_w.saturating_sub(38)
        } else {
            inner_w.saturating_sub(64)
        };
        let name = truncate_end(&project.name, name_max);

        let stack_str = project.stack.join(" + ");
        let stack = truncate_end(&stack_str, 15);

        let status = project.status.as_str();
        let status_style = match project.status {
            ProjectStatus::Active => theme.active,
            ProjectStatus::Paused => theme.paused,
            ProjectStatus::Stale => theme.stale,
            ProjectStatus::Archived => theme.archived,
            ProjectStatus::Unknown => theme.dim,
        };

        let git_label = match &project.git {
            Some(g) => format_git_label(g),
            None => "\u{2014}".to_string(),
        };
        let git_style = if project.git.as_ref().is_some_and(|g| g.is_dirty) {
            theme.dirty
        } else {
            theme.clean
        };

        let health_style = match project.health.level {
            HealthLevel::Good => theme.health_good,
            HealthLevel::Warn => theme.health_warn,
            HealthLevel::Bad => theme.health_bad,
            HealthLevel::Unknown => theme.dim,
        };
        let health_symbol = match project.health.level {
            HealthLevel::Good => "\u{2713}",
            HealthLevel::Warn => "!",
            HealthLevel::Bad => "\u{2717}",
            HealthLevel::Unknown => "\u{2014}",
        };

        let name_cell = Cell::from(Line::from(Span::styled(
            name,
            if is_selected { sel_style } else { theme.text },
        )));
        let stack_cell = Cell::from(Line::from(Span::styled(
            stack,
            if is_selected { sel_style } else { theme.stack },
        )));
        let health_cell = Cell::from(Line::from(Span::styled(
            health_symbol.to_string(),
            if is_selected { sel_style } else { health_style },
        )));

        if is_compact {
            let git_cell = Cell::from(Line::from(Span::styled(
                truncate_end(&git_label, 15),
                if is_selected { sel_style } else { git_style },
            )));
            let row = Row::new(vec![name_cell, stack_cell, git_cell, health_cell]).style(row_style);
            rows.push(row);
        } else {
            let activity = project.activity.relative_time.clone();
            let note = project
                .note
                .as_deref()
                .map(|n| truncate_end(n, 14))
                .unwrap_or_default();

            let git_cell = Cell::from(Line::from(Span::styled(
                truncate_end(&git_label, 14),
                if is_selected { sel_style } else { git_style },
            )));
            let activity_cell = Cell::from(Line::from(Span::styled(
                truncate_end(&activity, 7),
                if is_selected { sel_style } else { theme.dim },
            )));
            let status_cell = Cell::from(Line::from(Span::styled(
                truncate_end(status, 7),
                if is_selected { sel_style } else { status_style },
            )));
            let note_cell = Cell::from(Line::from(Span::styled(
                note,
                if is_selected { sel_style } else { theme.note },
            )));

            let row = Row::new(vec![
                name_cell,
                stack_cell,
                activity_cell,
                status_cell,
                git_cell,
                note_cell,
                health_cell,
            ])
            .style(row_style);
            rows.push(row);
        }
    }

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

fn detailed_widths(inner_w: usize) -> Vec<Constraint> {
    let name_w = inner_w.saturating_sub(64).max(10);
    vec![
        Constraint::Min(name_w as u16),
        Constraint::Length(16),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(15),
        Constraint::Length(13),
        Constraint::Length(4),
    ]
}

fn compact_widths(inner_w: usize) -> Vec<Constraint> {
    let name_w = inner_w.saturating_sub(38).max(10);
    vec![
        Constraint::Min(name_w as u16),
        Constraint::Length(16),
        Constraint::Length(18),
        Constraint::Length(4),
    ]
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

pub fn truncate_end(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "\u{2026}".to_string();
    }
    let limit = max_width.saturating_sub(1);
    let mut result = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw > limit {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result.push('\u{2026}');
    result
}
