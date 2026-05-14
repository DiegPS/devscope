use std::io::{self, Write};
use std::process::{Command, Stdio};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::App;
use crate::config::{Config, OpenActionKind};
use crate::input;
use crate::ui;

pub fn run_tui(config: Config) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);

    let result = run_loop(&mut terminal, &mut app);

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    disable_raw_mode()?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(ref rx) = app.ports_rx {
            if let Ok(port_map) = rx.try_recv() {
                for project in &mut app.projects {
                    let path_str = project.path.to_string_lossy().to_string();
                    if let Some(ports) = port_map.get(&path_str) {
                        project.ports = ports.clone();
                    }
                }
                app.ports_rx = None;
            }
        }

        if app.needs_reload {
            app.reload();
        }

        if app.should_quit {
            break;
        }

        if let Some(pending) = app.pending_action.take() {
            execute_open_action(&pending, app);
            terminal.clear()?;
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                input::handle_key_event(app, key);
            }
        }
    }

    Ok(())
}

fn execute_open_action(pending: &crate::app::PendingOpenAction, app: &mut App) {
    let action = &pending.action;
    let path = &pending.project_path;
    let name = &pending.project_name;
    let artifacts = &pending.artifacts;

    if let Some(kind) = &action.kind {
        match kind {
            OpenActionKind::FileManager => {
                match open::that(path) {
                    Ok(()) => {
                        app.status_message = Some(format!("Opened folder: {}", name));
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Could not open folder. {}", e));
                    }
                }
                record_open(app, path);
                return;
            }
            OpenActionKind::BuildOutput => {
                let artifact = artifacts
                    .iter()
                    .find(|a| a.exists && a.kind != crate::project::ArtifactKind::Executable)
                    .or_else(|| artifacts.iter().find(|a| a.exists));
                let target = artifact.map(|a| a.path.parent().unwrap_or(&a.path));
                match target {
                    Some(t) if t.exists() => {
                        if let Err(e) = open::that(t) {
                            app.status_message =
                                Some(format!("Could not open build output. {}", e));
                        } else {
                            app.status_message = Some(format!("Opened build output: {}", name));
                        }
                    }
                    Some(_) => {
                        app.status_message =
                            Some("Build output not found. Run a build first.".to_string());
                    }
                    None => {
                        app.status_message =
                            Some("No artifacts detected. Run a build first.".to_string());
                    }
                }
                record_open(app, path);
                return;
            }
            OpenActionKind::Executable => {
                let exe = artifacts.iter().find(|a| {
                    a.exists
                        && matches!(
                            a.kind,
                            crate::project::ArtifactKind::Executable
                                | crate::project::ArtifactKind::Apk
                        )
                });
                match exe {
                    Some(a) => {
                        if let Err(e) = open::that(&a.path) {
                            app.status_message = Some(format!("Could not open executable. {}", e));
                        } else {
                            app.status_message = Some(format!("Opened executable: {}", name));
                        }
                    }
                    None => {
                        app.status_message =
                            Some("No executable found. Run a build first.".to_string());
                    }
                }
                record_open(app, path);
                return;
            }
            _ => {}
        }
    }

    let Some(command) = &action.command else {
        app.status_message = Some(format!("No command configured for '{}'", action.name));
        return;
    };

    let resolved = resolve_command(command);
    let args = action.resolve_args(path, name);

    if action.terminal_mode {
        record_open(app, path);
        suspend_and_run(&resolved, &args, action.current_dir, path, &action.env);
        // Force full redraw by telling terminal to clear on next frame if possible,
        // but crossterm clear in suspend_and_run handles it.
    } else {
        let mut cmd = Command::new(&resolved);
        cmd.args(&args);
        if action.current_dir {
            cmd.current_dir(path);
        }
        for (k, v) in &action.env {
            cmd.env(k, v);
        }
        match cmd.spawn() {
            Ok(_child) => {
                app.status_message = Some(format!("Opened {}: {}", action.name, name));
            }
            Err(e) => {
                app.status_message = Some(format!(
                    "Could not open {}. Check config or PATH. ({})",
                    action.name, e
                ));
            }
        }
    }

    record_open(app, path);
}

fn suspend_and_run(
    resolved: &str,
    args: &[String],
    use_current_dir: bool,
    path: &std::path::Path,
    env: &std::collections::HashMap<String, String>,
) {
    // Release any stdout lock
    drop(io::stdout().lock());

    let mut stdout = io::stdout();

    // 1. Suspend TUI
    let _ = execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show,);
    let _ = stdout.flush();
    let _ = disable_raw_mode();

    // 2. Run the command synchronously
    let mut cmd = Command::new(resolved);
    cmd.args(args);
    if use_current_dir {
        cmd.current_dir(path);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let _ = cmd.status();

    // 3. Resume TUI
    let _ = enable_raw_mode();
    let _ = execute!(stdout, EnterAlternateScreen, Clear(ClearType::All));
    let _ = stdout.flush();
}

fn resolve_command(name: &str) -> String {
    let path = std::path::Path::new(name);

    if name.contains('\\') || name.contains('/') {
        return name.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        let has_ext = path.extension().is_some();

        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                if has_ext {
                    let candidate = dir.join(name);
                    if candidate.is_file() {
                        return candidate.to_string_lossy().to_string();
                    }
                } else {
                    for ext in [".exe", ".com", ".cmd", ".bat"] {
                        let candidate = dir.join(format!("{}{}", name, ext));
                        if candidate.is_file() {
                            return candidate.to_string_lossy().to_string();
                        }
                    }
                }
            }
        }
    }

    name.to_string()
}

fn record_open(app: &mut App, path: &std::path::Path) {
    let path_str = path.to_string_lossy().to_string();
    crate::config::record_open(&mut app.config, &path_str);
    let _ = crate::config::save_config(&app.config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[cfg(target_os = "windows")]
    fn resolve_cmd_over_bare_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Create both opencode (bare) and opencode.cmd
        fs::write(dir.join("opencode"), "").unwrap();
        fs::write(dir.join("opencode.cmd"), "").unwrap();

        let old_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", format!("{};", dir.display()));

        let result = resolve_command("opencode");
        let resolved = std::path::Path::new(&result);
        assert!(
            resolved.extension().is_some(),
            "should resolve opencode.cmd (has ext), got: {}",
            result
        );
        assert!(
            resolved
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".cmd"),
            "should resolve .cmd, got: {}",
            result
        );

        if let Some(p) = old_path {
            std::env::set_var("PATH", p);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn never_return_bare_file_on_windows() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Only bare file, no extension variants
        fs::write(dir.join("mytool"), "").unwrap();

        let old_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", format!("{};", dir.display()));

        let result = resolve_command("mytool");
        assert_eq!(
            result, "mytool",
            "should not return bare file without extension"
        );

        if let Some(p) = old_path {
            std::env::set_var("PATH", p);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn resolve_with_existing_extension() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("tool.cmd"), "").unwrap();

        let old_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", format!("{};", dir.display()));

        let result = resolve_command("tool.cmd");
        assert!(
            result.ends_with("tool.cmd"),
            "should resolve tool.cmd as-is, got: {}",
            result
        );

        if let Some(p) = old_path {
            std::env::set_var("PATH", p);
        }
    }

    #[test]
    fn path_with_separator_returned_as_is() {
        let result = resolve_command("C:\\tools\\myapp.exe");
        assert_eq!(result, "C:\\tools\\myapp.exe");

        let result = resolve_command("/usr/bin/myapp");
        assert_eq!(result, "/usr/bin/myapp");
    }

    #[test]
    fn bare_name_fallback() {
        // When nothing is found in PATH, return original name
        let old_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", "");

        let result = resolve_command("nonexistent-tool-xyz");
        assert_eq!(result, "nonexistent-tool-xyz");

        if let Some(p) = old_path {
            std::env::set_var("PATH", p);
        }
    }
}
