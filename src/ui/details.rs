use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::project::{HealthLevel, ProjectStatus};
use crate::ui::theme::Theme;

const LABEL_WIDTH: usize = 12;

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border)
        .title(Span::styled(" Details ", theme.header));

    let Some(project) = app.selected_project() else {
        let empty = Paragraph::new(Line::from(Span::styled("  No project selected", theme.dim)))
            .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let inner_w = area.width.saturating_sub(4).max(20) as usize;

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        format!("  {}", truncate_end(&project.name, inner_w)),
        theme.title,
    )));
    lines.push(Line::from(""));

    let path_str = project.path.display().to_string();
    lines.push(aligned_line(
        "Path",
        &truncate_middle(&path_str, inner_w.saturating_sub(LABEL_WIDTH + 2)),
        theme,
    ));

    let folder = project
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&project.name);
    lines.push(aligned_line(
        "Folder",
        &truncate_end(folder, inner_w.saturating_sub(LABEL_WIDTH + 2)),
        theme,
    ));

    let stack_str = if project.stack.is_empty() {
        "none".to_string()
    } else {
        project.stack.join(", ")
    };
    lines.push(aligned_line("Stack", &stack_str, theme));

    let manager_str = project.manager.as_deref().unwrap_or("none");
    lines.push(aligned_line("Manager", manager_str, theme));

    lines.push(line_separator(inner_w, theme));

    let health_style = match project.health.level {
        HealthLevel::Good => theme.health_good,
        HealthLevel::Warn => theme.health_warn,
        HealthLevel::Bad => theme.health_bad,
        HealthLevel::Unknown => theme.dim,
    };
    lines.push(Line::from(vec![
        Span::styled("  Health ", theme.section_title),
        Span::styled(
            format!(
                " {} {}/100",
                project.health.level.as_str(),
                project.health.score
            ),
            health_style,
        ),
    ]));

    let max_items = 6usize;
    let mut shown = 0usize;
    for pos in &project.health.positives {
        if shown >= max_items {
            break;
        }
        lines.push(Line::from(Span::styled(
            format!(
                "    \u{2713} {}",
                truncate_end(pos, inner_w.saturating_sub(6))
            ),
            theme.clean,
        )));
        shown += 1;
    }

    for w in &project.warnings {
        if shown >= max_items {
            break;
        }
        let text = truncate_end(&w.as_str(), inner_w.saturating_sub(6));
        lines.push(Line::from(Span::styled(
            format!("    ! {}", text),
            theme.warning,
        )));
        shown += 1;
    }

    let total_items = project.health.positives.len() + project.warnings.len();
    if total_items > max_items {
        lines.push(Line::from(Span::styled(
            format!("    \u{2026} and {} more", total_items - max_items),
            theme.dim,
        )));
    } else if project.health.positives.is_empty() && project.warnings.is_empty() {
        lines.push(Line::from(Span::styled("    no health data", theme.dim)));
    }

    if let Some(git) = &project.git {
        lines.push(line_separator(inner_w, theme));
        lines.push(Line::from(Span::styled("  Git", theme.section_title)));

        lines.push(aligned_line(
            "Branch",
            &truncate_end(&git.branch, inner_w.saturating_sub(LABEL_WIDTH + 2)),
            theme,
        ));

        let dirty_str = if git.is_dirty {
            format!(
                "{} mod, {} untracked",
                git.modified_count, git.untracked_count
            )
        } else {
            "clean".to_string()
        };
        let dirty_style = if git.is_dirty {
            theme.dirty
        } else {
            theme.clean
        };
        lines.push(Line::from(vec![
            Span::styled(pad_label("Dirty"), theme.dim),
            Span::styled(dirty_str, dirty_style),
        ]));

        let commit_str = if git.last_commit_message.is_empty() {
            git.last_commit_hash.clone()
        } else {
            format!(
                "{} {}",
                truncate_end(&git.last_commit_hash, 7),
                truncate_end(
                    &git.last_commit_message,
                    inner_w.saturating_sub(LABEL_WIDTH + 10)
                )
            )
        };
        lines.push(aligned_line("Commit", &commit_str, theme));

        match (&git.remote_url, &git.upstream) {
            (Some(remote), Some(upstream)) => {
                lines.push(aligned_line(
                    "Remote",
                    &truncate_middle(remote, inner_w.saturating_sub(LABEL_WIDTH + 2)),
                    theme,
                ));
                lines.push(aligned_line("Upstream", upstream, theme));
            }
            (Some(remote), None) => {
                lines.push(aligned_line(
                    "Remote",
                    &truncate_middle(remote, inner_w.saturating_sub(LABEL_WIDTH + 2)),
                    theme,
                ));
                if git.has_remote {
                    lines.push(aligned_line("Upstream", "none", theme));
                }
            }
            (None, _) => {
                lines.push(aligned_line("Remote", "none", theme));
            }
        }

        match (git.ahead, git.behind) {
            (Some(a), Some(b)) if a > 0 || b > 0 => {
                let mut parts = Vec::new();
                if a > 0 {
                    parts.push(format!("\u{2191}{}", a));
                }
                if b > 0 {
                    parts.push(format!("\u{2193}{}", b));
                }
                lines.push(Line::from(Span::styled(
                    format!("  Ahead/behind  {}", parts.join(" ")),
                    theme.ahead_behind,
                )));
            }
            _ => {}
        }
    } else {
        lines.push(line_separator(inner_w, theme));
        lines.push(Line::from(Span::styled("  Git", theme.section_title)));
        lines.push(Line::from(Span::styled("    no git repository", theme.dim)));
    }

    lines.push(line_separator(inner_w, theme));
    lines.push(Line::from(Span::styled("  Activity", theme.section_title)));

    lines.push(aligned_line(
        "Last active",
        &project.activity.relative_time,
        theme,
    ));

    let git_date = project
        .activity
        .last_git_activity
        .as_deref()
        .unwrap_or("none");
    lines.push(aligned_line("Git date", git_date, theme));

    lines.push(line_separator(inner_w, theme));

    let status_style = match project.status {
        ProjectStatus::Active => theme.active,
        ProjectStatus::Paused => theme.paused,
        ProjectStatus::Stale => theme.stale,
        ProjectStatus::Archived => theme.archived,
        ProjectStatus::Unknown => theme.dim,
    };
    lines.push(Line::from(vec![
        Span::styled(pad_label("Status"), theme.dim),
        Span::styled(project.status.as_str(), status_style),
    ]));

    let note_text = project.note.as_deref().unwrap_or("none");
    lines.push(aligned_line("Note", note_text, theme));

    lines.push(line_separator(inner_w, theme));
    lines.push(Line::from(Span::styled("  Commands", theme.section_title)));

    if project.commands.is_empty() {
        lines.push(Line::from(Span::styled("    none detected", theme.dim)));
    } else {
        let cmd_max = 6usize.min(project.commands.len());
        for cmd in project.commands.iter().take(cmd_max) {
            lines.push(Line::from(Span::styled(
                format!(
                    "    {}",
                    truncate_end(&cmd.command, inner_w.saturating_sub(4))
                ),
                theme.command,
            )));
        }
        if project.commands.len() > cmd_max {
            lines.push(Line::from(Span::styled(
                format!("    \u{2026} and {} more", project.commands.len() - cmd_max),
                theme.dim,
            )));
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn aligned_line(label: &str, value: &str, theme: &Theme) -> Line<'static> {
    let value_style = if value == "none" || value == "not available" {
        theme.dim
    } else {
        theme.text
    };
    Line::from(vec![
        Span::styled(pad_label(label), theme.dim),
        Span::styled(truncate_end(value, 200), value_style),
    ])
}

fn pad_label(label: &str) -> String {
    let w = label.width();
    let padding = LABEL_WIDTH.saturating_sub(w);
    format!("  {}{}", label, " ".repeat(padding))
}

fn line_separator(inner_w: usize, theme: &Theme) -> Line<'static> {
    let sep_w = inner_w.min(60);
    let sep_str = "\u{2500}".repeat(sep_w);
    Line::from(Span::styled(sep_str, theme.dim))
}

fn truncate_end(text: &str, max_width: usize) -> String {
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

fn truncate_middle(text: &str, max_width: usize) -> String {
    if max_width <= 1 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width <= 3 {
        return "\u{2026}".to_string();
    }
    let ellipsis = "\u{2026}";
    let left_w = max_width.saturating_sub(1) * 2 / 5;
    let right_w = max_width.saturating_sub(left_w).saturating_sub(1);
    let left = take_width(text, left_w);
    let right = take_width_from_end(text, right_w);
    format!("{}{}{}", left, ellipsis, right)
}

fn take_width(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw > max_width {
            break;
        }
        result.push(ch);
        w += cw;
    }
    result
}

fn take_width_from_end(text: &str, max_width: usize) -> String {
    let mut chars: Vec<char> = Vec::new();
    let mut w = 0;
    for ch in text.chars().rev() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw > max_width {
            break;
        }
        chars.push(ch);
        w += cw;
    }
    chars.iter().rev().collect()
}
