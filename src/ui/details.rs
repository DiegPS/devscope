use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

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

    let mut lines: Vec<Line> = Vec::new();

    // Project name
    lines.push(Line::from(Span::styled(project.name.clone(), theme.title)));
    lines.push(Line::from(""));

    // Path
    let path_str = project.path.display().to_string();
    lines.push(label_value("Path", path_str, theme));

    // Stack
    if !project.stack.is_empty() {
        let stack_str = project.stack.join(", ");
        lines.push(label_value("Stack", stack_str, theme));
    }

    // Manager
    if let Some(mgr) = &project.manager {
        lines.push(label_value("Manager", mgr.clone(), theme));
    }

    // Scripts
    if !project.scripts.is_empty() {
        let scripts_str = project.scripts.join(", ");
        lines.push(label_value("Scripts", scripts_str, theme));
    }

    lines.push(Line::from(""));

    // Git info
    if let Some(git) = &project.git {
        lines.push(Line::from(Span::styled("  Git", theme.filter)));
        lines.push(label_value("Branch", git.branch.clone(), theme));

        let dirty_str = if git.is_dirty {
            format!(
                "dirty, {} modified, {} untracked",
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
            Span::styled(dirty_str, dirty_style),
        ]));

        let commit_str = format!("{} {}", git.last_commit_hash, git.last_commit_message);
        lines.push(label_value("Commit", commit_str, theme));

        if let Some(remote) = &git.remote_url {
            lines.push(label_value("Remote", remote.clone(), theme));
        }

        lines.push(Line::from(""));
    }

    // Activity
    lines.push(Line::from(Span::styled("  Activity", theme.filter)));
    lines.push(label_value(
        "Last active",
        project.activity.relative_time.clone(),
        theme,
    ));

    if let Some(date) = &project.activity.last_git_activity {
        lines.push(label_value("Git date", date.clone(), theme));
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
        lines.push(Line::from(Span::styled(format!("  {}", note), theme.note)));
    }

    // Warnings
    if !project.warnings.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("  Warnings", theme.warning)));
        for warning in &project.warnings {
            lines.push(Line::from(Span::styled(
                format!("  ! {}", warning.as_str()),
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
