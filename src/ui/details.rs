use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::project::{ArtifactKind, DirtyStatus, HealthLevel, ProjectStatus};
use crate::ui::theme::Theme;

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
    let label_width = detail_label_width(inner_w);
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        format!("  {}", truncate_end(&project.name, inner_w)),
        theme.title,
    )));
    lines.push(build_summary_line(project, inner_w, label_width, theme));
    lines.push(Line::from(""));

    push_section(&mut lines, "Overview", theme);
    let path_str = project.path.display().to_string();
    let folder = project
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&project.name);
    let stack = if project.stack.is_empty() {
        "none".to_string()
    } else {
        project.stack.join(", ")
    };
    let value_width = inner_w.saturating_sub(label_width + 2);

    lines.push(aligned_line(
        "Path",
        &truncate_middle(&path_str, value_width),
        label_width,
        value_width,
        theme,
    ));
    lines.push(aligned_line(
        "Folder",
        &truncate_end(folder, value_width),
        label_width,
        value_width,
        theme,
    ));
    lines.push(aligned_line(
        "Stack",
        &stack,
        label_width,
        value_width,
        theme,
    ));
    lines.push(aligned_line(
        "Manager",
        project.manager.as_deref().unwrap_or("none"),
        label_width,
        value_width,
        theme,
    ));
    lines.push(aligned_line(
        "Last active",
        &project.activity.relative_time(),
        label_width,
        value_width,
        theme,
    ));
    lines.push(aligned_line(
        "Git date",
        &project
            .activity
            .last_git_activity_display()
            .unwrap_or_else(|| "none".to_string()),
        label_width,
        value_width,
        theme,
    ));
    lines.push(aligned_line(
        "Note",
        project.note.as_deref().unwrap_or("none"),
        label_width,
        value_width,
        theme,
    ));

    lines.push(section_separator(inner_w, theme));
    push_section(&mut lines, "Health", theme);

    let health_style = match project.health.level {
        HealthLevel::Good => theme.health_good,
        HealthLevel::Warn => theme.health_warn,
        HealthLevel::Bad => theme.health_bad,
        HealthLevel::Unknown => theme.dim,
    };
    let bar_width = inner_w.clamp(8, 16);
    lines.push(Line::from(vec![
        Span::styled(pad_label("Score", label_width), theme.dim),
        Span::styled(health_bar(project.health.score, bar_width), health_style),
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
    for warning in &project.warnings {
        if shown >= max_items {
            break;
        }
        lines.push(Line::from(Span::styled(
            format!(
                "    ! {}",
                truncate_end(&warning.as_str(), inner_w.saturating_sub(6))
            ),
            theme.warning,
        )));
        shown += 1;
    }

    for positive in &project.health.positives {
        if shown >= max_items {
            break;
        }
        lines.push(Line::from(Span::styled(
            format!(
                "    \u{2713} {}",
                truncate_end(positive, inner_w.saturating_sub(6))
            ),
            theme.clean,
        )));
        shown += 1;
    }

    let total_items = project.health.positives.len() + project.warnings.len();
    if total_items > max_items {
        lines.push(Line::from(Span::styled(
            format!("    \u{2026} and {} more", total_items - max_items),
            theme.dim,
        )));
    } else if total_items == 0 {
        lines.push(Line::from(Span::styled("    no health data", theme.dim)));
    }

    lines.push(section_separator(inner_w, theme));
    push_section(&mut lines, "Git", theme);

    if let Some(git) = &project.git {
        lines.push(aligned_line(
            "Branch",
            &truncate_end(&git.branch, value_width),
            label_width,
            value_width,
            theme,
        ));

        let dirty_text = match git.dirty_status {
            DirtyStatus::Dirty => format!(
                "{} mod, {} untracked",
                git.modified_count.unwrap_or(0),
                git.untracked_count.unwrap_or(0)
            ),
            DirtyStatus::Checking => "checking\u{2807}".to_string(),
            DirtyStatus::Error => "error".to_string(),
            DirtyStatus::Clean => "clean".to_string(),
            DirtyStatus::Queued => "queued".to_string(),
            DirtyStatus::Unknown => "unknown".to_string(),
        };
        let dirty_style = match git.dirty_status {
            DirtyStatus::Dirty => theme.dirty,
            DirtyStatus::Clean => theme.clean,
            DirtyStatus::Checking | DirtyStatus::Queued | DirtyStatus::Unknown => theme.dim,
            DirtyStatus::Error => theme.health_bad,
        };
        lines.push(Line::from(vec![
            Span::styled(pad_label("Dirty", label_width), theme.dim),
            Span::styled(dirty_text, dirty_style),
        ]));

        let commit_text = if git.last_commit_message.is_empty() {
            git.last_commit_hash.clone()
        } else {
            format!(
                "{} {}",
                truncate_end(&git.last_commit_hash, 7),
                truncate_end(
                    &git.last_commit_message,
                    inner_w.saturating_sub(label_width + 10)
                )
            )
        };
        lines.push(aligned_line(
            "Commit",
            &commit_text,
            label_width,
            value_width,
            theme,
        ));

        let remote = git.remote_url.as_deref().unwrap_or("none");
        lines.push(aligned_line(
            "Remote",
            &truncate_middle(remote, value_width),
            label_width,
            value_width,
            theme,
        ));
        if git.has_remote {
            lines.push(aligned_line(
                "Upstream",
                git.upstream.as_deref().unwrap_or("none"),
                label_width,
                value_width,
                theme,
            ));
        }

        if let (Some(ahead), Some(behind)) = (git.ahead, git.behind) {
            if ahead > 0 || behind > 0 {
                let mut parts = Vec::new();
                if ahead > 0 {
                    parts.push(format!("\u{2191}{}", ahead));
                }
                if behind > 0 {
                    parts.push(format!("\u{2193}{}", behind));
                }
                lines.push(Line::from(vec![
                    Span::styled(pad_label("Sync", label_width), theme.dim),
                    Span::styled(parts.join(" "), theme.ahead_behind),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled("    no git repository", theme.dim)));
    }

    lines.push(section_separator(inner_w, theme));
    push_section(&mut lines, "Commands", theme);
    if project.commands.is_empty() {
        lines.push(Line::from(Span::styled("    none detected", theme.dim)));
    } else {
        let shown_commands = 6usize.min(project.commands.len());
        for command in project.commands.iter().take(shown_commands) {
            lines.push(Line::from(Span::styled(
                format!(
                    "    {}",
                    truncate_end(&command.command, inner_w.saturating_sub(4))
                ),
                theme.command,
            )));
        }
        if project.commands.len() > shown_commands {
            lines.push(Line::from(Span::styled(
                format!(
                    "    \u{2026} and {} more",
                    project.commands.len() - shown_commands
                ),
                theme.dim,
            )));
        }
    }

    if !project.artifacts.is_empty() {
        lines.push(section_separator(inner_w, theme));
        push_section(&mut lines, "Artifacts", theme);
        let shown_artifacts = 6usize.min(project.artifacts.len());
        for artifact in project.artifacts.iter().take(shown_artifacts) {
            let icon = match artifact.kind {
                ArtifactKind::Executable | ArtifactKind::Apk => "\u{25B6}",
                ArtifactKind::Folder
                | ArtifactKind::Web
                | ArtifactKind::Bundle
                | ArtifactKind::Other => "\u{25A1}",
            };
            let (icon_style, label_style) = if artifact.exists {
                (theme.clean, theme.text)
            } else {
                (theme.dim, theme.dim)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("    {} ", icon), icon_style),
                Span::styled(
                    truncate_end(&artifact.label, inner_w.saturating_sub(6)),
                    label_style,
                ),
            ]));
        }
        if project.artifacts.len() > shown_artifacts {
            lines.push(Line::from(Span::styled(
                format!(
                    "    \u{2026} and {} more",
                    project.artifacts.len() - shown_artifacts
                ),
                theme.dim,
            )));
        }
    }

    if !project.ports.is_empty() {
        lines.push(section_separator(inner_w, theme));
        push_section(&mut lines, "Ports", theme);
        let ports = project
            .ports
            .iter()
            .map(|port| port.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(Span::styled(
            format!("    \u{2192} {}", ports),
            theme.command,
        )));
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn build_summary_line(
    project: &crate::project::Project,
    inner_w: usize,
    label_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let status_style = match project.status {
        ProjectStatus::Active => theme.active,
        ProjectStatus::Paused => theme.paused,
        ProjectStatus::Stale => theme.stale,
        ProjectStatus::Archived => theme.archived,
        ProjectStatus::Unknown => theme.dim,
    };
    let health_style = match project.health.level {
        HealthLevel::Good => theme.health_good,
        HealthLevel::Warn => theme.health_warn,
        HealthLevel::Bad => theme.health_bad,
        HealthLevel::Unknown => theme.dim,
    };
    let git_text = match project.git.as_ref().map(|git| git.dirty_status) {
        Some(DirtyStatus::Clean) => "git clean",
        Some(DirtyStatus::Dirty) => "git dirty",
        Some(DirtyStatus::Checking) => "git checking",
        Some(DirtyStatus::Queued) => "git queued",
        Some(DirtyStatus::Error) => "git error",
        Some(DirtyStatus::Unknown) => "git unknown",
        None => "no git",
    };
    let git_style = match project.git.as_ref().map(|git| git.dirty_status) {
        Some(DirtyStatus::Clean) => theme.clean,
        Some(DirtyStatus::Dirty) => theme.dirty,
        Some(DirtyStatus::Error) => theme.health_bad,
        _ => theme.dim,
    };

    let summary = format!(
        "{} \u{00B7} {} {}/100 \u{00B7} {}",
        project.status.as_str(),
        project.health.level.as_str(),
        project.health.score,
        git_text
    );

    if summary.width() > inner_w {
        return Line::from(vec![
            Span::styled(pad_label("Status", label_width), theme.dim),
            Span::styled(project.status.as_str().to_string(), status_style),
        ]);
    }

    Line::from(vec![
        Span::styled("  ", theme.dim),
        Span::styled(project.status.as_str().to_string(), status_style),
        Span::styled(" \u{00B7} ", theme.dim),
        Span::styled(
            format!(
                "{} {}/100",
                project.health.level.as_str(),
                project.health.score
            ),
            health_style,
        ),
        Span::styled(" \u{00B7} ", theme.dim),
        Span::styled(git_text.to_string(), git_style),
    ])
}

fn push_section(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    lines.push(Line::from(Span::styled(
        format!("  {}", title),
        theme.section_title,
    )));
}

fn aligned_line(
    label: &str,
    value: &str,
    label_width: usize,
    value_width: usize,
    theme: &Theme,
) -> Line<'static> {
    let value_style = if value == "none" || value == "not available" || value == "unknown" {
        theme.dim
    } else {
        theme.text
    };
    Line::from(vec![
        Span::styled(pad_label(label, label_width), theme.dim),
        Span::styled(truncate_end(value, value_width), value_style),
    ])
}

fn pad_label(label: &str, label_width: usize) -> String {
    let width = label.width();
    let padding = label_width.saturating_sub(width);
    format!("  {}{}", label, " ".repeat(padding))
}

fn detail_label_width(inner_w: usize) -> usize {
    if inner_w < 34 {
        9
    } else if inner_w < 48 {
        10
    } else {
        12
    }
}

fn section_separator(inner_w: usize, theme: &Theme) -> Line<'static> {
    let separator_width = inner_w.saturating_add(2);
    Line::from(Span::styled(
        "\u{2500}".repeat(separator_width),
        theme.border,
    ))
}

fn health_bar(score: u8, width: usize) -> String {
    let filled = (score as usize * width).div_ceil(100);
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}]",
        "\u{25A0}".repeat(filled),
        "\u{25A1}".repeat(empty)
    )
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
    let mut width = 0;
    for ch in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + char_width > limit {
            break;
        }
        result.push(ch);
        width += char_width;
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

    let left_width = max_width.saturating_sub(1) * 2 / 5;
    let right_width = max_width.saturating_sub(left_width).saturating_sub(1);
    let left = take_width(text, left_width);
    let right = take_width_from_end(text, right_width);
    format!("{}\u{2026}{}", left, right)
}

fn take_width(text: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + char_width > max_width {
            break;
        }
        result.push(ch);
        width += char_width;
    }
    result
}

fn take_width_from_end(text: &str, max_width: usize) -> String {
    let mut chars = Vec::new();
    let mut width = 0;
    for ch in text.chars().rev() {
        let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + char_width > max_width {
            break;
        }
        chars.push(ch);
        width += char_width;
    }
    chars.iter().rev().collect()
}
