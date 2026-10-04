use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode, PendingOpenAction, ViewMode};

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    app.status_message = None;

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Search => handle_search_mode(app, key),
        Mode::EditingNote => handle_note_mode(app, key),
        Mode::ChangingStatus => handle_status_mode(app, key),
        Mode::Help => handle_help_mode(app, key),
        Mode::OpenMenu => handle_open_menu(app, key),
        Mode::ConfigMenu => handle_config_menu(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('Q') => app.should_quit = true,
        KeyCode::Esc => {
            app.search_query.clear();
            app.apply_filter_and_sort();
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::PageUp => app.move_page_up(),
        KeyCode::PageDown => app.move_page_down(),
        KeyCode::Home => app.move_home(),
        KeyCode::End => app.move_end(),
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
        }
        KeyCode::Char('f') => app.next_filter(),
        KeyCode::Char('s') => app.next_sort(),
        KeyCode::Char('r') => {
            app.needs_reload = true;
            app.reload();
        }
        KeyCode::Char('n') if app.selected_project().is_some() => {
            app.editing_project_id = app.selected_project().map(|p| p.id.clone());
            app.mode = Mode::EditingNote;
            app.note_input = app
                .selected_project()
                .and_then(|p| p.note.clone())
                .unwrap_or_default();
        }
        KeyCode::Char('m') if app.selected_project().is_some() => {
            app.editing_project_id = app.selected_project().map(|p| p.id.clone());
            app.mode = Mode::ChangingStatus;
            if let Some(project) = app.selected_project() {
                app.status_selected = app
                    .status_options
                    .iter()
                    .position(|s| *s == project.status)
                    .unwrap_or(0);
            }
        }
        KeyCode::Char('o') => {
            if app.config.open.actions.is_empty() {
                app.status_message = Some("No open actions configured".to_string());
            } else {
                app.mode = Mode::OpenMenu;
            }
        }
        KeyCode::Char(',') => {
            if app.config.open.actions.is_empty() {
                app.status_message = Some("No open actions configured".to_string());
            } else {
                app.mode = Mode::ConfigMenu;
            }
        }
        KeyCode::Char('D') => {
            app.toggle_view();
            app.status_message = Some(match app.view_mode {
                ViewMode::Compact => "Compact view".to_string(),
                ViewMode::Detailed => "Detailed view".to_string(),
            });
        }
        KeyCode::Char('?') => {
            app.mode = Mode::Help;
            app.help_scroll = 0;
        }
        KeyCode::Enter => {
            let project_id = app.selected_project().map(|p| p.id.clone());
            if let Some(ref id) = project_id {
                let mut updated = app.config.clone();
                crate::config::record_visit(&mut updated, id);
                if app.persist_config(updated) {
                    app.apply_filter_and_sort();
                }
            }
        }
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_query.clear();
            app.apply_filter_and_sort();
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter_and_sort();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter_and_sort();
        }
        _ => {}
    }
}

fn handle_note_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.note_input.clear();
            app.editing_project_id = None;
        }
        KeyCode::Enter => {
            let note_val = app.note_input.clone();
            if let Some(index) = editing_project_index(app) {
                let path_str = app.projects[index].path.to_string_lossy().to_string();
                let mut updated = app.config.clone();
                if note_val.is_empty() {
                    updated.notes.remove(&path_str);
                } else {
                    crate::config::set_note(&mut updated, &path_str, note_val.clone());
                }
                if !app.persist_config(updated) {
                    return;
                }
                app.projects[index].note = (!note_val.is_empty()).then_some(note_val);
                app.apply_filter_and_sort();
            }
            app.mode = Mode::Normal;
            app.note_input.clear();
            app.editing_project_id = None;
        }
        KeyCode::Backspace => {
            app.note_input.pop();
        }
        KeyCode::Char(c) => {
            app.note_input.push(c);
        }
        _ => {}
    }
}

fn handle_status_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.editing_project_id = None;
        }
        KeyCode::Up | KeyCode::Char('k') if app.status_selected > 0 => {
            app.status_selected -= 1;
        }
        KeyCode::Down | KeyCode::Char('j')
            if app.status_selected + 1 < app.status_options.len() =>
        {
            app.status_selected += 1;
        }
        KeyCode::Enter => {
            if let Some(index) = editing_project_index(app) {
                let Some(new_status) = app.status_options.get(app.status_selected).cloned() else {
                    return;
                };
                let path_str = app.projects[index].path.to_string_lossy().to_string();
                let mut updated = app.config.clone();
                crate::config::set_project_status(&mut updated, &path_str, new_status.clone());
                if !app.persist_config(updated) {
                    return;
                }
                app.projects[index].status = new_status;
                app.apply_filter_and_sort();
            }
            app.mode = Mode::Normal;
            app.editing_project_id = None;
        }
        _ => {}
    }
}

fn editing_project_index(app: &mut App) -> Option<usize> {
    let index = app
        .editing_project_id
        .as_ref()
        .and_then(|id| app.projects.iter().position(|project| &project.id == id));
    if index.is_none() {
        app.status_message = Some("The project being edited is no longer available".to_string());
    }
    index
}

fn handle_help_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
            app.mode = Mode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.help_scroll = app.help_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.help_scroll += 1;
        }
        _ => {}
    }
}

fn handle_open_menu(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Char(c) => {
            let action = app
                .config
                .open
                .actions
                .iter()
                .find(|a| a.key_char().eq_ignore_ascii_case(&c))
                .cloned();
            match action {
                Some(a) => {
                    let Some(project) = app.selected_project() else {
                        app.mode = Mode::Normal;
                        return;
                    };
                    let project_path = project.path.clone();
                    let project_name = project.name.clone();
                    let artifacts = project.artifacts.clone();
                    app.pending_action = Some(PendingOpenAction {
                        action: a,
                        project_path,
                        project_name,
                        artifacts,
                    });
                    app.mode = Mode::Normal;
                }
                None => {
                    app.status_message =
                        Some(format!("No open action for key '{}'. Esc to cancel.", c));
                    app.mode = Mode::Normal;
                }
            }
        }
        _ => {}
    }
}

fn handle_config_menu(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Char(c) => {
            let action = app
                .config
                .open
                .actions
                .iter()
                .find(|a| a.key_char().eq_ignore_ascii_case(&c))
                .cloned();
            match action {
                Some(a) => {
                    let config_dir = match crate::config::config_dir() {
                        Ok(d) => d,
                        Err(_) => {
                            app.status_message = Some("Could not find config dir".to_string());
                            app.mode = Mode::Normal;
                            return;
                        }
                    };
                    app.pending_action = Some(PendingOpenAction {
                        action: a,
                        project_path: config_dir,
                        project_name: "ds config".to_string(),
                        artifacts: Vec::new(),
                    });
                    app.mode = Mode::Normal;
                }
                None => {
                    app.status_message =
                        Some(format!("No open action for key '{}'. Esc to cancel.", c));
                    app.mode = Mode::Normal;
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{FilterField, SortField},
        config::Config,
        project::DirtyStatus,
    };
    use crossterm::event::KeyModifiers;

    fn fixture() -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        for name in ["alpha", "beta"] {
            let path = dir.path().join(name);
            std::fs::create_dir(&path).unwrap();
            std::fs::write(
                path.join("Cargo.toml"),
                "[package]\nname='test'\nversion='0.1.0'\n",
            )
            .unwrap();
            git2::Repository::init(&path).unwrap();
        }
        let config = Config {
            roots: vec![dir.path().to_string_lossy().into()],
            ..Config::default()
        };
        let mut app = App::new(Config::default());
        app.projects = crate::scanner::scan_roots(&config).unwrap().projects;
        app.config = config;
        app.config_save_path = Some(dir.path().join("config.toml"));
        app.sort = SortField::Name;
        app.apply_filter_and_sort();
        (dir, app)
    }

    fn key(app: &mut App, code: KeyCode) {
        handle_key_event(app, KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn note_keeps_original_target_after_reordering_and_selection_changes() {
        let (dir, mut app) = fixture();
        let alpha = app.selected_project().unwrap().id.clone();
        key(&mut app, KeyCode::Char('n'));
        app.note_input = "alpha note".into();
        app.sort = SortField::DirtyFirst;
        for project in &mut app.projects {
            project.git.as_mut().unwrap().dirty_status = if project.name == "beta" {
                DirtyStatus::Dirty
            } else {
                DirtyStatus::Clean
            };
        }
        app.apply_filter_and_sort();
        assert_eq!(app.selected_project().unwrap().id, alpha);
        app.selected = 0; // Even a later selection change must not redirect the edit.
        assert_eq!(app.selected_project().unwrap().name, "beta");
        key(&mut app, KeyCode::Enter);
        let saved: Config =
            toml::from_str(&std::fs::read_to_string(dir.path().join("config.toml")).unwrap())
                .unwrap();
        assert_eq!(
            saved.notes.get(&alpha).map(String::as_str),
            Some("alpha note")
        );
        assert_eq!(saved.notes.len(), 1);
        assert_eq!(
            app.projects
                .iter()
                .find(|p| p.name == "alpha")
                .unwrap()
                .note
                .as_deref(),
            Some("alpha note")
        );
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn failed_note_save_preserves_draft_and_previous_state() {
        let (dir, mut app) = fixture();
        let blocked = dir.path().join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        app.config_save_path = Some(blocked);
        key(&mut app, KeyCode::Char('n'));
        app.note_input = "unsaved note".into();
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::EditingNote);
        assert_eq!(app.note_input, "unsaved note");
        assert!(app.config.notes.is_empty());
        assert!(app.selected_project().unwrap().note.is_none());
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("Could not save"));
    }

    #[test]
    fn missing_edit_target_cannot_save_to_another_project() {
        let (dir, mut app) = fixture();
        key(&mut app, KeyCode::Char('n'));
        let target = app.editing_project_id.clone().unwrap();
        app.projects.retain(|p| p.id != target);
        app.apply_filter_and_sort();
        app.note_input = "do not redirect".into();
        key(&mut app, KeyCode::Enter);
        assert!(!dir.path().join("config.toml").exists());
        assert!(app.config.notes.is_empty());
    }

    #[test]
    fn status_change_updates_the_active_filter() {
        let (_dir, mut app) = fixture();
        for project in &mut app.projects {
            project.status = crate::project::ProjectStatus::Active;
        }
        app.filter = FilterField::Active;
        app.apply_filter_and_sort();
        key(&mut app, KeyCode::Char('m'));
        app.status_selected = 3;
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.filtered_count(), 1);
        assert_eq!(app.selected_project().unwrap().name, "beta");
        assert!(app
            .projects
            .iter()
            .any(|p| p.name == "alpha" && p.status == crate::project::ProjectStatus::Archived));
    }

    #[test]
    fn deleting_a_note_updates_the_notes_filter() {
        let (_dir, mut app) = fixture();
        app.projects[0].note = Some("old".into());
        app.filter = FilterField::WithNotes;
        app.apply_filter_and_sort();
        key(&mut app, KeyCode::Char('n'));
        app.note_input.clear();
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.filtered_count(), 0);
        assert_eq!(app.selected, 0);
    }
}
