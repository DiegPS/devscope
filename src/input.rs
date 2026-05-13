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
            app.mode = Mode::EditingNote;
            app.note_input = app
                .selected_project()
                .and_then(|p| p.note.clone())
                .unwrap_or_default();
        }
        KeyCode::Char('m') => {
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
                crate::config::record_visit(&mut app.config, id);
                let _ = crate::config::save_config(&app.config);
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
        }
        KeyCode::Enter => {
            let path_str = app.selected_path_str();
            let note_val = app.note_input.clone();
            if let Some(path_str) = path_str {
                if note_val.is_empty() {
                    app.config.notes.remove(&path_str);
                } else {
                    crate::config::set_note(&mut app.config, &path_str, note_val.clone());
                }
                let _ = crate::config::save_config(&app.config);
                if let Some(p) = app.selected_project_mut() {
                    p.note = if note_val.is_empty() {
                        None
                    } else {
                        Some(note_val)
                    };
                }
            }
            app.mode = Mode::Normal;
            app.note_input.clear();
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
            let path_str = app.selected_path_str();
            if let Some(path_str) = path_str {
                let new_status = app.status_options[app.status_selected].clone();
                crate::config::set_project_status(&mut app.config, &path_str, new_status.clone());
                let _ = crate::config::save_config(&app.config);
                if let Some(p) = app.selected_project_mut() {
                    p.status = new_status;
                }
            }
            app.mode = Mode::Normal;
        }
        _ => {}
    }
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
