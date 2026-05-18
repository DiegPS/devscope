use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::{App, ViewMode};
use crate::health::format_git_label;
use crate::project::{DirtyStatus, HealthLevel, ProjectStatus};
use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let inner_w = area.width.saturating_sub(2).max(1) as usize;
    let layout = resolve_layout(area, app.view_mode);

    let header = Row::new(layout.headers.iter().map(|label| Cell::from(*label)))
        .style(theme.table_header)
        .height(1);

    let visible_rows = area.height.saturating_sub(3).max(1) as usize;
    let total = app.filtered_indices.len();
    let sel = app.selected;
    let scroll = calc_scroll(sel, visible_rows, total);

    let mut rows: Vec<Row> = Vec::new();

    if total == 0 {
        rows.push(
            Row::new(vec![Cell::from(Line::from(Span::styled(
                "No projects match the current filter/search",
                theme.dim,
            )))])
            .style(Style::default()),
        );
    } else {
        for (display_idx, &proj_idx) in app
            .filtered_indices
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_rows)
        {
            let project = &app.projects[proj_idx];
            let is_selected = display_idx == sel;
            let row_style = if is_selected {
                theme.selected
            } else {
                Style::default()
            };

            let status_style = match project.status {
                ProjectStatus::Active => theme.active,
                ProjectStatus::Paused => theme.paused,
                ProjectStatus::Stale => theme.stale,
                ProjectStatus::Archived => theme.archived,
                ProjectStatus::Unknown => theme.dim,
            };

            let git_label = match &project.git {
                Some(git) => format_git_label(git),
                None => "\u{2014}".to_string(),
            };
            let git_style = if project
                .git
                .as_ref()
                .is_some_and(|git| git.dirty_status == DirtyStatus::Dirty)
            {
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

            let name = truncate_end(&project.name, layout.name_width(inner_w));
            let stack_str = project.stack.join(" + ");
            let stack = truncate_end(&stack_str, layout.stack_max);

            let name_cell = Cell::from(Line::from(Span::styled(
                name,
                if is_selected { row_style } else { theme.text },
            )));
            let stack_cell = Cell::from(Line::from(if is_selected {
                vec![Span::styled(stack.clone(), row_style)]
            } else {
                colorize_stack(&stack, theme.stack)
            }));
            let health_cell = Cell::from(Line::from(Span::styled(
                health_symbol.to_string(),
                if is_selected { row_style } else { health_style },
            )));

            let row = match layout.kind {
                LayoutKind::CompactNarrow => {
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.git_max),
                        if is_selected { row_style } else { git_style },
                    )));
                    if layout.activity_max > 0 {
                        let activity = project.activity.relative_time();
                        let activity_cell = Cell::from(Line::from(Span::styled(
                            truncate_end(&activity, layout.activity_max),
                            if is_selected { row_style } else { theme.dim },
                        )));
                        Row::new(vec![
                            name_cell,
                            stack_cell,
                            activity_cell,
                            git_cell,
                            health_cell,
                        ])
                        .style(row_style)
                    } else {
                        Row::new(vec![name_cell, stack_cell, git_cell, health_cell])
                            .style(row_style)
                    }
                }
                LayoutKind::CompactMedium => {
                    let activity = project.activity.relative_time();
                    let ports = format_ports(project);
                    let ports_width = layout.column_width(4, inner_w).saturating_sub(1);
                    let ports_style = if project.ports.is_empty() {
                        theme.dim
                    } else {
                        theme.command
                    };
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.activity_max),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.git_max),
                        if is_selected { row_style } else { git_style },
                    )));
                    let ports_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&ports, ports_width),
                        if is_selected { row_style } else { ports_style },
                    )));
                    Row::new(vec![
                        name_cell,
                        stack_cell,
                        activity_cell,
                        git_cell,
                        ports_cell,
                        health_cell,
                    ])
                    .style(row_style)
                }
                LayoutKind::CompactWide => {
                    let activity = project.activity.relative_time();
                    let ports = format_ports(project);
                    let ports_width = layout.column_width(4, inner_w).saturating_sub(1);
                    let note = project
                        .note
                        .as_deref()
                        .map(|value| truncate_end(value, layout.note_max))
                        .unwrap_or_default();
                    let ports_style = if project.ports.is_empty() {
                        theme.dim
                    } else {
                        theme.command
                    };
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.activity_max),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.git_max),
                        if is_selected { row_style } else { git_style },
                    )));
                    let ports_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&ports, ports_width),
                        if is_selected { row_style } else { ports_style },
                    )));
                    let note_cell = Cell::from(Line::from(Span::styled(
                        note,
                        if is_selected { row_style } else { theme.note },
                    )));
                    Row::new(vec![
                        name_cell,
                        stack_cell,
                        activity_cell,
                        git_cell,
                        ports_cell,
                        note_cell,
                        health_cell,
                    ])
                    .style(row_style)
                }
                LayoutKind::DetailedMedium => {
                    let activity = project.activity.relative_time();
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.activity_max),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.git_max),
                        if is_selected { row_style } else { git_style },
                    )));
                    Row::new(vec![
                        name_cell,
                        stack_cell,
                        activity_cell,
                        git_cell,
                        health_cell,
                    ])
                    .style(row_style)
                }
                LayoutKind::DetailedWide => {
                    let activity = project.activity.relative_time();
                    let note = project
                        .note
                        .as_deref()
                        .map(|value| truncate_end(value, layout.note_max))
                        .unwrap_or_default();
                    let status_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(project.status.as_str(), layout.status_max),
                        if is_selected { row_style } else { status_style },
                    )));
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.activity_max),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.git_max),
                        if is_selected { row_style } else { git_style },
                    )));
                    let note_cell = Cell::from(Line::from(Span::styled(
                        note,
                        if is_selected { row_style } else { theme.note },
                    )));
                    Row::new(vec![
                        name_cell,
                        stack_cell,
                        activity_cell,
                        status_cell,
                        git_cell,
                        note_cell,
                        health_cell,
                    ])
                    .style(row_style)
                }
            };

            rows.push(row);
        }
    }

    let title = build_title(total, scroll, rows.len(), visible_rows, app);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Span::styled(title, theme.header));

    let table = Table::new(rows, layout.widths)
        .header(header)
        .block(block)
        .column_spacing(1);

    frame.render_widget(table, area);
}

#[derive(Clone, Copy)]
enum LayoutKind {
    CompactNarrow,
    CompactMedium,
    CompactWide,
    DetailedMedium,
    DetailedWide,
}

struct TableLayout {
    kind: LayoutKind,
    headers: &'static [&'static str],
    widths: Vec<Constraint>,
    stack_max: usize,
    activity_max: usize,
    status_max: usize,
    git_max: usize,
    note_max: usize,
}

impl TableLayout {
    fn name_width(&self, inner_w: usize) -> usize {
        let fixed: usize = self
            .widths
            .iter()
            .skip(1)
            .map(|constraint| match constraint {
                Constraint::Length(value) => *value as usize,
                Constraint::Percentage(value) => inner_w * (*value as usize) / 100,
                Constraint::Min(value) => *value as usize,
                Constraint::Max(value) => *value as usize,
                Constraint::Fill(_) => 0,
                Constraint::Ratio(num, den) => inner_w * (*num as usize) / (*den as usize),
            })
            .sum();
        let spacing = self.widths.len().saturating_sub(1);
        inner_w.saturating_sub(fixed + spacing).max(10)
    }

    fn column_width(&self, index: usize, inner_w: usize) -> usize {
        let Some(constraint) = self.widths.get(index) else {
            return 0;
        };

        match constraint {
            Constraint::Length(value) => *value as usize,
            Constraint::Percentage(value) => inner_w * (*value as usize) / 100,
            Constraint::Min(value) => *value as usize,
            Constraint::Max(value) => *value as usize,
            Constraint::Fill(_) => inner_w,
            Constraint::Ratio(num, den) => inner_w * (*num as usize) / (*den as usize),
        }
    }
}

fn resolve_layout(area: Rect, view_mode: ViewMode) -> TableLayout {
    let width = area.width;
    if matches!(view_mode, ViewMode::Compact) {
        if width < 96 {
            return TableLayout {
                kind: LayoutKind::CompactNarrow,
                headers: &["Name", "Stack", "Act", "Git", "H"],
                widths: vec![
                    Constraint::Percentage(38),
                    Constraint::Percentage(26),
                    Constraint::Percentage(8),
                    Constraint::Percentage(20),
                    Constraint::Percentage(8),
                ],
                stack_max: 22,
                activity_max: 6,
                status_max: 0,
                git_max: 16,
                note_max: 0,
            };
        }

        if width < 126 {
            return TableLayout {
                kind: LayoutKind::CompactMedium,
                headers: &["Name", "Stack", "Act", "Git", "Ports", "H"],
                widths: vec![
                    Constraint::Percentage(38),
                    Constraint::Percentage(24),
                    Constraint::Percentage(8),
                    Constraint::Percentage(14),
                    Constraint::Percentage(10),
                    Constraint::Percentage(6),
                ],
                stack_max: 22,
                activity_max: 6,
                status_max: 0,
                git_max: 15,
                note_max: 0,
            };
        }

        return TableLayout {
            kind: LayoutKind::CompactWide,
            headers: &["Name", "Stack", "Act", "Git", "Ports", "Note", "H"],
            widths: vec![
                Constraint::Percentage(28),
                Constraint::Percentage(21),
                Constraint::Percentage(7),
                Constraint::Percentage(13),
                Constraint::Percentage(9),
                Constraint::Percentage(17),
                Constraint::Percentage(5),
            ],
            stack_max: 24,
            activity_max: 6,
            status_max: 0,
            git_max: 15,
            note_max: 18,
        };
    }

    if width < 78 {
        return TableLayout {
            kind: LayoutKind::CompactNarrow,
            headers: &["Name", "Stack", "Git", "H"],
            widths: vec![
                Constraint::Percentage(43),
                Constraint::Percentage(27),
                Constraint::Percentage(22),
                Constraint::Percentage(8),
            ],
            stack_max: 20,
            activity_max: 0,
            status_max: 0,
            git_max: 14,
            note_max: 0,
        };
    }

    if width < 104 {
        return TableLayout {
            kind: LayoutKind::DetailedMedium,
            headers: &["Name", "Stack", "Act", "Git", "H"],
            widths: vec![
                Constraint::Min(12),
                Constraint::Length(22),
                Constraint::Length(6),
                Constraint::Length(15),
                Constraint::Length(3),
            ],
            stack_max: 22,
            activity_max: 6,
            status_max: 0,
            git_max: 15,
            note_max: 0,
        };
    }

    TableLayout {
        kind: LayoutKind::DetailedWide,
        headers: &["Name", "Stack", "Act", "Status", "Git", "Note", "H"],
        widths: vec![
            Constraint::Min(14),
            Constraint::Length(24),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Length(14),
            Constraint::Length(3),
        ],
        stack_max: 24,
        activity_max: 6,
        status_max: 8,
        git_max: 15,
        note_max: 14,
    }
}

fn build_title(total: usize, scroll: usize, shown: usize, visible_rows: usize, app: &App) -> String {
    let prefix = if total > visible_rows {
        format!(
            " Projects {}-{} / {} ",
            scroll + 1,
            (scroll + shown).min(total),
            total
        )
    } else {
        format!(" Projects {} ", total)
    };

    format!(
        "{}\u{00B7} {} \u{00B7} {} ",
        prefix,
        app.sort.as_str(),
        app.view_mode.as_str()
    )
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

fn colorize_stack(stack: &str, default_style: Style) -> Vec<Span<'static>> {
    use ratatui::style::Color;
    let mut spans = Vec::new();
    let parts: Vec<&str> = stack.split(" + ").collect();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" + ", default_style));
        }

        let lower = part.to_lowercase();
        let fg_color = if lower.contains("rust") || lower.contains("cargo") {
            Some(Color::Rgb(250, 130, 49))
        } else if lower.contains("node")
            || lower.contains("npm")
            || lower.contains("yarn")
            || lower.contains("pnpm")
        {
            Some(Color::Rgb(136, 192, 87))
        } else if lower.contains("flutter") || lower.contains("dart") {
            Some(Color::Rgb(84, 197, 248))
        } else if lower.contains("go") {
            Some(Color::Rgb(0, 200, 255))
        } else if lower.contains("python") {
            Some(Color::Rgb(255, 224, 90))
        } else if lower.contains("react") {
            Some(Color::Rgb(97, 218, 251))
        } else if lower.contains("vite") {
            Some(Color::Rgb(173, 108, 255))
        } else if lower.contains("typescript") {
            Some(Color::Rgb(97, 175, 239))
        } else if lower.contains("electron") {
            Some(Color::Rgb(159, 234, 249))
        } else if lower.contains("docker") || lower.contains("compose") {
            Some(Color::Rgb(36, 150, 237))
        } else {
            None
        };

        let style = if let Some(color) = fg_color {
            default_style.fg(color)
        } else {
            default_style
        };

        spans.push(Span::styled(part.to_string(), style));
    }

    spans
}

fn format_ports(project: &crate::project::Project) -> String {
    if project.ports.is_empty() {
        return "\u{2014}".to_string();
    }

    project
        .ports
        .iter()
        .map(|port| port.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
