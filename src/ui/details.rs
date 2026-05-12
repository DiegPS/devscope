use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::project::ProjectStatus;
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

    // Path (middle truncation)
    let path_str = project.path.display().to_string();
    let path_label_w = 8; // "  Path: "
    let path_max = inner_w.saturating_sub(path_label_w);
    let path_truncated = truncate_middle(&path_str, path_max);
    lines.push(label_value("Path", path_truncated, theme));

    // Folder (always show, useful when path is truncated)
    let folder = project
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&project.name);
    let folder_line_w = 10; // "  Folder: "
    lines.push(Line::from(Span::styled(
        format!(
            "  Folder: {}",
            truncate_end(folder, inner_w.saturating_sub(folder_line_w))
        ),
        theme.dim,
    )));

    // Stack
    if !project.stack.is_empty() {
        let stack_str = project.stack.join(", ");
        let label_w = 9; // "  Stack: "
        lines.push(label_value(
            "Stack",
            truncate_end(&stack_str, inner_w.saturating_sub(label_w)),
            theme,
        ));
    }

    // Manager
    if let Some(mgr) = &project.manager {
        let label_w = 11; // "  Manager: "
        lines.push(label_value(
            "Manager",
            truncate_end(mgr, inner_w.saturating_sub(label_w)),
            theme,
        ));
    }

    // Scripts
    if !project.scripts.is_empty() {
        let scripts_str = project.scripts.join(", ");
        let label_w = 11; // "  Scripts: "
        lines.push(label_value(
            "Scripts",
            truncate_end(&scripts_str, inner_w.saturating_sub(label_w)),
            theme,
        ));
    }

    lines.push(Line::from(""));

    // Git info
    if let Some(git) = &project.git {
        lines.push(Line::from(Span::styled("  Git", theme.filter)));

        // Branch
        let label_w = 10; // "  Branch: "
        lines.push(label_value(
            "Branch",
            truncate_end(&git.branch, inner_w.saturating_sub(label_w)),
            theme,
        ));

        // Dirty
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
        let dirty_label_w = 11; // "  Dirty:  "
        lines.push(Line::from(vec![
            Span::styled("  Dirty:  ", theme.dim),
            Span::styled(
                truncate_end(&dirty_str, inner_w.saturating_sub(dirty_label_w)),
                dirty_style,
            ),
        ]));

        // Commit
        let commit_str = if git.last_commit_message.is_empty() {
            git.last_commit_hash.clone()
        } else {
            format!("{}  {}", git.last_commit_hash, git.last_commit_message)
        };
        let label_w = 10; // "  Commit: "
        lines.push(label_value(
            "Commit",
            truncate_end(&commit_str, inner_w.saturating_sub(label_w)),
            theme,
        ));

        // Remote (middle truncation for URLs)
        if let Some(remote) = &git.remote_url {
            let label_w = 10; // "  Remote: "
            lines.push(label_value(
                "Remote",
                truncate_middle(remote, inner_w.saturating_sub(label_w)),
                theme,
            ));
        }

        lines.push(Line::from(""));
    }

    // Activity
    lines.push(Line::from(Span::styled("  Activity", theme.filter)));

    let label_w = 14; // "  Last active: "
    lines.push(label_value(
        "Last active",
        truncate_end(
            &project.activity.relative_time,
            inner_w.saturating_sub(label_w),
        ),
        theme,
    ));

    if let Some(date) = &project.activity.last_git_activity {
        let label_w = 11; // "  Git date: "
        lines.push(label_value(
            "Git date",
            truncate_end(date, inner_w.saturating_sub(label_w)),
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
        let note_max = inner_w.saturating_sub(2);
        lines.push(Line::from(Span::styled(
            format!("  {}", truncate_end(note, note_max)),
            theme.note,
        )));
    }

    // Warnings
    if !project.warnings.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Warnings", theme.warning)));
        for warning in &project.warnings {
            let warn_max = inner_w.saturating_sub(4); // "  ! "
            lines.push(Line::from(Span::styled(
                format!("  ! {}", truncate_end(warning.as_str(), warn_max)),
                theme.warning,
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

/// Truncate text with a trailing ellipsis (…).
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

/// Truncate text with an ellipsis in the middle (for paths and URLs).
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
