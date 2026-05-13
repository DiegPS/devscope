use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::project::{HealthLevel, ProjectStatus};
use crate::ui::theme::Theme;

/// Render the details panel on the right side.
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

    let inner_w = area.width.saturating_sub(2).max(10) as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Project name
    lines.push(Line::from(Span::styled(
        format!("  {}", truncate_end(&project.name, inner_w - 2)),
        theme.title,
    )));
    lines.push(Line::from(""));

    // Path
    let path_str = project.path.display().to_string();
    let path_label_w = 8;
    let path_truncated = truncate_middle(&path_str, inner_w.saturating_sub(path_label_w));
    lines.push(label_value("Path", path_truncated, theme));

    // Folder
    let folder = project
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&project.name);
    lines.push(Line::from(Span::styled(
        format!(
            "  Folder: {}",
            truncate_end(folder, inner_w.saturating_sub(10))
        ),
        theme.dim,
    )));

    // Stack
    if !project.stack.is_empty() {
        let stack_str = project.stack.join(", ");
        lines.push(label_value(
            "Stack",
            truncate_end(&stack_str, inner_w.saturating_sub(9)),
            theme,
        ));
    }

    // Manager
    if let Some(mgr) = &project.manager {
        lines.push(label_value(
            "Manager",
            truncate_end(mgr, inner_w.saturating_sub(11)),
            theme,
        ));
    }

    lines.push(Line::from(""));

    // ── Health ────────────────────────────────────────────────────
    let health_style = match project.health.level {
        HealthLevel::Good => theme.health_good,
        HealthLevel::Warn => theme.health_warn,
        HealthLevel::Bad => theme.health_bad,
        HealthLevel::Unknown => theme.dim,
    };
    lines.push(Line::from(vec![
        Span::styled("  Health: ", theme.dim),
        Span::styled(
            format!(
                "{} {}/100",
                project.health.level.as_str(),
                project.health.score
            ),
            health_style,
        ),
    ]));

    // Positives
    if !project.health.positives.is_empty() {
        let pos_str = project.health.positives.join(", ");
        lines.push(Line::from(Span::styled(
            format!(
                "    + {}",
                truncate_end(&pos_str, inner_w.saturating_sub(6))
            ),
            theme.clean,
        )));
    }

    // Warnings (health + project warnings combined)
    let all_warnings = &project.warnings;
    if !all_warnings.is_empty() {
        let max_show = 6usize;
        for (i, w) in all_warnings.iter().take(max_show).enumerate() {
            let warn_max = inner_w.saturating_sub(6);
            let text = truncate_end(&w.as_str(), warn_max);
            if i < max_show {
                lines.push(Line::from(Span::styled(
                    format!("    ! {}", text),
                    theme.warning,
                )));
            }
        }
        if all_warnings.len() > max_show {
            lines.push(Line::from(Span::styled(
                format!("    ... and {} more", all_warnings.len() - max_show),
                theme.dim,
            )));
        }
    }

    lines.push(Line::from(""));

    // ── Git info ──────────────────────────────────────────────────
    if let Some(git) = &project.git {
        lines.push(Line::from(Span::styled("  Git", theme.filter)));

        lines.push(label_value(
            "Branch",
            truncate_end(&git.branch, inner_w.saturating_sub(10)),
            theme,
        ));

        let dirty_str = if git.is_dirty {
            format!(
                "dirty, {} mod, {} untracked",
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
            Span::styled("  Dirty:  ", theme.dim),
            Span::styled(
                truncate_end(&dirty_str, inner_w.saturating_sub(11)),
                dirty_style,
            ),
        ]));

        let commit_str = if git.last_commit_message.is_empty() {
            git.last_commit_hash.clone()
        } else {
            format!("{}  {}", git.last_commit_hash, git.last_commit_message)
        };
        lines.push(label_value(
            "Commit",
            truncate_end(&commit_str, inner_w.saturating_sub(10)),
            theme,
        ));

        if let Some(remote) = &git.remote_url {
            lines.push(label_value(
                "Remote",
                truncate_middle(remote, inner_w.saturating_sub(10)),
                theme,
            ));
        }

        if let Some(upstream) = &git.upstream {
            lines.push(label_value(
                "Upstream",
                truncate_end(upstream, inner_w.saturating_sub(12)),
                theme,
            ));
        } else if git.has_remote {
            lines.push(Line::from(Span::styled("  Upstream: none", theme.dim)));
        }

        match (git.ahead, git.behind) {
            (Some(a), Some(b)) if a > 0 || b > 0 => {
                let mut parts = Vec::new();
                if a > 0 {
                    parts.push(format!("ahead {}", a));
                }
                if b > 0 {
                    parts.push(format!("behind {}", b));
                }
                lines.push(Line::from(Span::styled(
                    format!("  Ahead/Behind: {}", parts.join(", ")),
                    theme.ahead_behind,
                )));
            }
            _ => {}
        }

        lines.push(Line::from(""));
    }

    // Activity
    lines.push(Line::from(Span::styled("  Activity", theme.filter)));
    lines.push(label_value(
        "Last active",
        truncate_end(&project.activity.relative_time, inner_w.saturating_sub(14)),
        theme,
    ));
    if let Some(date) = &project.activity.last_git_activity {
        lines.push(label_value(
            "Git date",
            truncate_end(date, inner_w.saturating_sub(11)),
            theme,
        ));
    }
    lines.push(Line::from(""));

    // Status
    let status_style = match project.status {
        ProjectStatus::Active => theme.active,
        ProjectStatus::Paused => theme.paused,
        ProjectStatus::Stale => theme.stale,
        ProjectStatus::Archived => theme.archived,
        ProjectStatus::Unknown => theme.dim,
    };
    lines.push(Line::from(vec![
        Span::styled("  Status: ", theme.dim),
        Span::styled(project.status.as_str(), status_style),
    ]));

    // Note
    if let Some(note) = &project.note {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Note", theme.filter)));
        lines.push(Line::from(Span::styled(
            format!("  {}", truncate_end(note, inner_w.saturating_sub(2))),
            theme.note,
        )));
    }

    // Commands
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Commands", theme.filter)));
    if project.commands.is_empty() {
        lines.push(Line::from(Span::styled("    none detected", theme.dim)));
    } else {
        for cmd in &project.commands {
            let cmd_max = inner_w.saturating_sub(4);
            lines.push(Line::from(Span::styled(
                format!("    {}", truncate_end(&cmd.command, cmd_max)),
                theme.stack,
            )));
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn label_value(label: &str, value: String, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {}: ", label), theme.dim),
        Span::styled(value, theme.text),
    ])
}

fn truncate_end(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
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
    result.push('…');
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
        return "…".to_string();
    }
    let ellipsis = "…";
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
