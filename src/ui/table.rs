use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::app::{App, ViewMode};
use crate::health::format_git_label;
use crate::project::{DirtyStatus, HealthLevel, ProjectStatus};
use crate::ui::theme::Theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    if app.filtered_indices.is_empty() {
        let message = if app.total_projects == 0 {
            "No projects found.\nAdd a root with: ds add <path>\nThen press r to scan again."
        } else {
            "No matching projects.\nPress Esc to clear search; f changes the active filter."
        };
        frame.render_widget(
            ratatui::widgets::Paragraph::new(message)
                .wrap(ratatui::widgets::Wrap { trim: false })
                .style(theme.muted)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(theme.border)
                        .title(build_title(0, 0, 0, 0, app, area.width)),
                ),
            area,
        );
        return;
    }
    let inner_w = area.width.saturating_sub(2).max(1);
    let layout = resolve_layout(area, app.view_mode);
    let resolved_widths = layout.resolved_widths(inner_w);

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
            let git_style = match project.git.as_ref().map(|g| g.dirty_status) {
                Some(DirtyStatus::Dirty) => theme.dirty,
                Some(DirtyStatus::Clean) => theme.clean,
                Some(DirtyStatus::Error) => theme.health_bad,
                _ => theme.dim,
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
            let health_symbol = if app.config.ui.show_icons {
                health_symbol.to_string()
            } else if project.health.level == HealthLevel::Unknown {
                "n/a".to_string()
            } else {
                project.health.level.as_str().to_string()
            };

            let name = truncate_end(&project.name, layout.cell_width(&resolved_widths, 0));
            let stack_str = project.stack.join(" + ");
            let stack = truncate_end(&stack_str, layout.cell_width(&resolved_widths, 1));

            let name_cell = Cell::from(Line::from(Span::styled(
                name,
                if is_selected { row_style } else { theme.text },
            )));
            let stack_cell = Cell::from(Line::from(if is_selected {
                vec![Span::styled(stack.clone(), row_style)]
            } else {
                if app.config.ui.theme.eq_ignore_ascii_case("light") {
                    vec![Span::styled(stack.clone(), theme.stack)]
                } else {
                    colorize_stack(&stack, theme.stack)
                }
            }));
            let health_cell = Cell::from(Line::from(Span::styled(
                health_symbol.to_string(),
                health_style,
            )));

            let row = match layout.kind {
                LayoutKind::CompactNarrow => {
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.cell_width(&resolved_widths, 3)),
                        git_style,
                    )));
                    let activity = project.activity.relative_time();
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.cell_width(&resolved_widths, 2)),
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
                }
                LayoutKind::CompactMedium => {
                    let activity = project.activity.relative_time();
                    let ports = format_ports(project);
                    let ports_width = layout.cell_width(&resolved_widths, 4);
                    let ports_style = if project.ports.is_empty() {
                        theme.dim
                    } else {
                        theme.command
                    };
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.cell_width(&resolved_widths, 2)),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.cell_width(&resolved_widths, 3)),
                        git_style,
                    )));
                    let ports_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&ports, ports_width),
                        ports_style,
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
                    let ports_width = layout.cell_width(&resolved_widths, 4);
                    let stack_width = layout.cell_width(&resolved_widths, 1);
                    let note_width = layout.cell_width(&resolved_widths, 5);
                    let note = project
                        .note
                        .as_deref()
                        .map(|value| truncate_end(value, note_width))
                        .unwrap_or_default();
                    let ports_style = if project.ports.is_empty() {
                        theme.dim
                    } else {
                        theme.command
                    };
                    let stack_cell = Cell::from(Line::from(if is_selected {
                        vec![Span::styled(
                            truncate_end(&stack_str, stack_width),
                            row_style,
                        )]
                    } else {
                        if app.config.ui.theme.eq_ignore_ascii_case("light") {
                            vec![Span::styled(
                                truncate_end(&stack_str, stack_width),
                                theme.stack,
                            )]
                        } else {
                            colorize_stack(&truncate_end(&stack_str, stack_width), theme.stack)
                        }
                    }));
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.cell_width(&resolved_widths, 2)),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.cell_width(&resolved_widths, 3)),
                        git_style,
                    )));
                    let ports_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&ports, ports_width),
                        ports_style,
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
                        truncate_end(&activity, layout.cell_width(&resolved_widths, 2)),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.cell_width(&resolved_widths, 3)),
                        git_style,
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
                LayoutKind::DetailedNarrow => {
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.cell_width(&resolved_widths, 2)),
                        git_style,
                    )));
                    Row::new(vec![name_cell, stack_cell, git_cell, health_cell]).style(row_style)
                }
                LayoutKind::DetailedWide => {
                    let activity = project.activity.relative_time();
                    let note = project
                        .note
                        .as_deref()
                        .map(|value| truncate_end(value, layout.cell_width(&resolved_widths, 5)))
                        .unwrap_or_default();
                    let status_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(
                            project.status.as_str(),
                            layout.cell_width(&resolved_widths, 3),
                        ),
                        status_style,
                    )));
                    let activity_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&activity, layout.cell_width(&resolved_widths, 2)),
                        if is_selected { row_style } else { theme.dim },
                    )));
                    let git_cell = Cell::from(Line::from(Span::styled(
                        truncate_end(&git_label, layout.cell_width(&resolved_widths, 4)),
                        git_style,
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

    let title = build_title(total, scroll, rows.len(), visible_rows, app, area.width);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if !app.details_focus {
            theme.title
        } else {
            theme.border
        })
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
    DetailedNarrow,
    DetailedMedium,
    DetailedWide,
}

struct TableLayout {
    kind: LayoutKind,
    headers: &'static [&'static str],
    widths: Vec<Constraint>,
}

impl TableLayout {
    fn resolved_widths(&self, inner_w: u16) -> Vec<usize> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(self.widths.clone())
            .spacing(1)
            .split(Rect::new(0, 0, inner_w, 1))
            .iter()
            .map(|chunk| chunk.width as usize)
            .collect()
    }

    fn cell_width(&self, widths: &[usize], index: usize) -> usize {
        widths
            .get(index)
            .copied()
            .unwrap_or(1)
            .saturating_sub(1)
            .max(1)
    }
}

fn resolve_layout(area: Rect, view_mode: ViewMode) -> TableLayout {
    let width = area.width;
    if matches!(view_mode, ViewMode::Compact) {
        if width < 96 {
            let name_cap = cap_name_width(width, 18, 26);
            return TableLayout {
                kind: LayoutKind::CompactNarrow,
                headers: &["Name", "Stack", "Active", "Git", "Health"],
                widths: vec![
                    Constraint::Max(name_cap),
                    Constraint::Min(16),
                    Constraint::Length(7),
                    Constraint::Length(14),
                    Constraint::Length(6),
                ],
            };
        }

        if width < 126 {
            let name_cap = cap_name_width(width, 20, 30);
            return TableLayout {
                kind: LayoutKind::CompactMedium,
                headers: &["Name", "Stack", "Active", "Git", "Ports", "Health"],
                widths: vec![
                    Constraint::Max(name_cap),
                    Constraint::Min(18),
                    Constraint::Length(7),
                    Constraint::Length(14),
                    Constraint::Length(8),
                    Constraint::Length(6),
                ],
            };
        }

        let name_cap = cap_name_width(width, 22, 32);
        return TableLayout {
            kind: LayoutKind::CompactWide,
            headers: &["Name", "Stack", "Active", "Git", "Ports", "Note", "Health"],
            widths: vec![
                Constraint::Max(name_cap),
                Constraint::Min(18),
                Constraint::Length(6),
                Constraint::Length(14),
                Constraint::Length(8),
                Constraint::Min(12),
                Constraint::Length(6),
            ],
        };
    }

    if width < 78 {
        let name_cap = cap_name_width(width, 18, 24);
        return TableLayout {
            kind: LayoutKind::DetailedNarrow,
            headers: &["Name", "Stack", "Git", "Health"],
            widths: vec![
                Constraint::Max(name_cap),
                Constraint::Min(16),
                Constraint::Length(14),
                Constraint::Length(6),
            ],
        };
    }

    if width < 104 {
        let name_cap = cap_name_width(width, 20, 30);
        return TableLayout {
            kind: LayoutKind::DetailedMedium,
            headers: &["Name", "Stack", "Active", "Git", "Health"],
            widths: vec![
                Constraint::Max(name_cap),
                Constraint::Min(18),
                Constraint::Length(6),
                Constraint::Length(15),
                Constraint::Length(6),
            ],
        };
    }

    let name_cap = cap_name_width(width, 22, 32);
    TableLayout {
        kind: LayoutKind::DetailedWide,
        headers: &["Name", "Stack", "Active", "Status", "Git", "Note", "Health"],
        widths: vec![
            Constraint::Max(name_cap),
            Constraint::Min(18),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(15),
            Constraint::Min(12),
            Constraint::Length(6),
        ],
    }
}

fn cap_name_width(width: u16, min: u16, max: u16) -> u16 {
    (width / 3).clamp(min, max)
}

fn build_title(
    total: usize,
    scroll: usize,
    shown: usize,
    visible_rows: usize,
    app: &App,
    width: u16,
) -> String {
    use unicode_width::UnicodeWidthStr;
    let prefix = if total > visible_rows {
        format!(
            " Projects {}-{}/{}",
            scroll + 1,
            (scroll + shown).min(total),
            total
        )
    } else {
        format!(" Projects {}", total)
    };
    let max_width = width.saturating_sub(2) as usize;
    let mut title = format!("{} · filter: {}", prefix, app.filter.as_str());
    if !app.search_query.is_empty() {
        let label = " · search: ";
        let hint = " [Esc clear]";
        let budget = max_width.saturating_sub(title.width() + label.width() + hint.width() + 1);
        title.push_str(label);
        title.push_str(&truncate_end(
            &app.search_query.replace(['\n', '\r', '\t'], " "),
            budget,
        ));
        title.push_str(hint);
    }
    for extra in [
        format!(" · sort: {}", app.sort.as_str()),
        format!(" · {}", app.view_mode.as_str()),
    ] {
        if title.width() + extra.width() < max_width {
            title.push_str(&extra);
        }
    }
    title.push(' ');
    truncate_end(&title, max_width)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detailed_medium_caps_name_and_gives_extra_width_to_stack() {
        let narrow = resolve_layout(Rect::new(0, 0, 80, 1), ViewMode::Detailed);
        let narrow_widths = narrow.resolved_widths(78);
        let wide = resolve_layout(Rect::new(0, 0, 100, 1), ViewMode::Detailed);
        let wide_widths = wide.resolved_widths(98);

        assert!(wide_widths[0] <= 30);
        assert!(wide_widths[0] <= narrow_widths[0] + 4);
        assert!(wide_widths[1] > narrow_widths[1]);
    }

    #[test]
    fn detailed_wide_keeps_name_bounded() {
        let layout = resolve_layout(Rect::new(0, 0, 160, 1), ViewMode::Detailed);
        let widths = layout.resolved_widths(158);

        assert!(widths[0] <= 32);
        assert!(widths[1] >= 18);
        assert!(widths[5] >= 12);
    }
}
