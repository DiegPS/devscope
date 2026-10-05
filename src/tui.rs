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
    let mut app = App::new(config);
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    run_loop(&mut terminal, &mut app)
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Each restoration is attempted even when another operation fails.
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    }
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut redraw = true;
    let mut first_frame = true;
    loop {
        if app.should_quit {
            break;
        }

        if let Some(ref rx) = app.ports_rx {
            match rx.try_recv() {
                Ok(port_map) => {
                    for project in &mut app.projects {
                        let path_str = project.path.to_string_lossy().to_string();
                        if let Some(ports) = port_map.get(&path_str) {
                            project.ports = ports.clone();
                        }
                    }
                    app.ports_rx = None;
                    redraw = true;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => app.ports_rx = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        if app.needs_reload {
            app.start_reload();
            redraw = true;
        }

        if app.should_quit {
            break;
        }

        if let Some(pending) = app.pending_action.take() {
            execute_open_action(&pending, app);
            terminal.clear()?;
            redraw = true;
        }

        redraw |= app.poll_reload();
        redraw |= app.poll_hydration_results();
        if redraw {
            let size = terminal.size()?;
            app.viewport = (size.width, size.height);
            terminal.draw(|frame| ui::draw(frame, app))?;
            redraw = false;
            if first_frame {
                app.start_background_jobs();
                first_frame = false;
            }
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            if matches!(event, Event::Resize(_, _)) {
                redraw = true;
            }
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                input::handle_key_event(app, key);
                redraw = true;
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
                        app.set_message(
                            format!("Opened folder: {}", name),
                            crate::app::MessageLevel::Info,
                        );
                        record_open(app, path);
                    }
                    Err(e) => {
                        app.set_message(
                            format!("Could not open folder. {}", e),
                            crate::app::MessageLevel::Error,
                        );
                    }
                }
                return;
            }
            OpenActionKind::BuildOutput => {
                let artifact = artifacts
                    .iter()
                    .find(|a| a.exists && a.kind != crate::project::ArtifactKind::Executable)
                    .or_else(|| artifacts.iter().find(|a| a.exists));
                let target = artifact.map(|a| {
                    if a.path.is_dir() {
                        a.path.as_path()
                    } else {
                        a.path.parent().unwrap_or(&a.path)
                    }
                });
                match target {
                    Some(t) if t.exists() => {
                        if let Err(e) = open::that(t) {
                            app.set_message(
                                format!("Could not open build output. {}", e),
                                crate::app::MessageLevel::Error,
                            );
                        } else {
                            app.set_message(
                                format!("Opened build output: {}", name),
                                crate::app::MessageLevel::Info,
                            );
                            record_open(app, path);
                        }
                    }
                    Some(_) => {
                        app.set_message(
                            "Build output not found. Run a build first.".to_string(),
                            crate::app::MessageLevel::Warning,
                        );
                    }
                    None => {
                        app.set_message(
                            "No artifacts detected. Run a build first.".to_string(),
                            crate::app::MessageLevel::Info,
                        );
                    }
                }
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
                            app.set_message(
                                format!("Could not open executable. {}", e),
                                crate::app::MessageLevel::Error,
                            );
                        } else {
                            app.set_message(
                                format!("Opened executable: {}", name),
                                crate::app::MessageLevel::Info,
                            );
                            record_open(app, path);
                        }
                    }
                    None => {
                        app.set_message(
                            "No executable found. Run a build first.".to_string(),
                            crate::app::MessageLevel::Warning,
                        );
                    }
                }
                return;
            }
            _ => {}
        }
    }

    let Some(command) = &action.command else {
        app.set_message(
            format!("No command configured for '{}'", action.name),
            crate::app::MessageLevel::Error,
        );
        return;
    };

    let resolved = resolve_command(command);
    let args = action.resolve_args(path, name);

    let opened = if action.terminal_mode {
        match suspend_and_run(&resolved, &args, action.current_dir, path, &action.env) {
            Ok(status) if status.success() => true,
            Ok(status) => {
                app.set_message(
                    format!("{} exited with {}", action.name, status),
                    crate::app::MessageLevel::Error,
                );
                false
            }
            Err(error) => {
                app.set_message(
                    format!("Could not open {}: {error:#}", action.name),
                    crate::app::MessageLevel::Error,
                );
                false
            }
        }
        // Force full redraw by telling terminal to clear on next frame if possible,
        // but crossterm clear in suspend_and_run handles it.
    } else {
        let mut cmd = Command::new(&resolved);
        cmd.args(&args);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if action.current_dir {
            cmd.current_dir(path);
        }
        for (k, v) in &action.env {
            cmd.env(k, v);
        }
        match cmd.spawn() {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                app.set_message(
                    format!("Opened {}: {}", action.name, name),
                    crate::app::MessageLevel::Info,
                );
                true
            }
            Err(e) => {
                app.set_message(
                    format!(
                        "Could not open {}. Check config or PATH. ({})",
                        action.name, e
                    ),
                    crate::app::MessageLevel::Error,
                );
                false
            }
        }
    };

    if opened {
        record_open(app, path);
    }
}

fn suspend_and_run(
    resolved: &str,
    args: &[String],
    use_current_dir: bool,
    path: &std::path::Path,
    env: &std::collections::HashMap<String, String>,
) -> Result<std::process::ExitStatus> {
    // Release any stdout lock
    drop(io::stdout().lock());

    let mut stdout = io::stdout();

    // 1. Suspend TUI
    let mut resume = ResumeTerminal(true);
    execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show)?;
    stdout.flush()?;
    disable_raw_mode()?;

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

    let result = cmd.status();

    // 3. Resume TUI
    resume.restore()?;
    Ok(result?)
}

struct ResumeTerminal(bool);

impl ResumeTerminal {
    fn restore(&mut self) -> Result<()> {
        let raw = enable_raw_mode();
        let screen = execute!(io::stdout(), EnterAlternateScreen, Clear(ClearType::All));
        let flush = io::stdout().flush();
        raw?;
        screen?;
        flush?;
        self.0 = false;
        Ok(())
    }
}

impl Drop for ResumeTerminal {
    fn drop(&mut self) {
        if self.0 {
            let _ = self.restore();
        }
    }
}

fn resolve_command(name: &str) -> String {
    if name.contains('\\') || name.contains('/') {
        return name.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(paths) = std::env::var_os("PATH") {
            return resolve_command_from_paths(
                name,
                &std::env::split_paths(&paths).collect::<Vec<_>>(),
            );
        }
    }

    name.to_string()
}

#[cfg(windows)]
fn resolve_command_from_paths(name: &str, paths: &[std::path::PathBuf]) -> String {
    if name.contains('\\') || name.contains('/') {
        return name.to_string();
    }
    let has_ext = std::path::Path::new(name).extension().is_some();
    for dir in paths {
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
    name.to_string()
}

fn record_open(app: &mut App, path: &std::path::Path) {
    let path_str = path.to_string_lossy().to_string();
    let mut updated = app.config.clone();
    crate::config::record_open(&mut updated, &path_str);
    if app.persist_config(updated) {
        app.apply_filter_and_sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a real PTY; run python scripts/test_tui.py"]
    fn real_terminal_child() {
        let root = std::env::var("DS_TEST_TUI_ROOT").expect("PTY fixture root required");
        let path = std::path::PathBuf::from(&root).join("test-config.toml");
        crate::config::with_test_config_path(path, || {
            run_tui(Config {
                roots: vec![root],
                ..Config::default()
            })
            .unwrap();
        });
    }
    #[cfg(target_os = "windows")]
    use std::fs;
    #[cfg(target_os = "windows")]
    use tempfile::TempDir;

    #[test]
    #[cfg(target_os = "windows")]
    fn resolve_cmd_over_bare_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Create both opencode (bare) and opencode.cmd
        fs::write(dir.join("opencode"), "").unwrap();
        fs::write(dir.join("opencode.cmd"), "").unwrap();

        let result = resolve_command_from_paths("opencode", &[dir.to_path_buf()]);
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
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn never_return_bare_file_on_windows() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Only bare file, no extension variants
        fs::write(dir.join("mytool"), "").unwrap();

        let result = resolve_command_from_paths("mytool", &[dir.to_path_buf()]);
        assert_eq!(
            result, "mytool",
            "should not return bare file without extension"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn resolve_with_existing_extension() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("tool.cmd"), "").unwrap();

        let result = resolve_command_from_paths("tool.cmd", &[dir.to_path_buf()]);
        assert!(
            result.ends_with("tool.cmd"),
            "should resolve tool.cmd as-is, got: {}",
            result
        );
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
        #[cfg(windows)]
        let result = resolve_command_from_paths("nonexistent-tool-xyz", &[]);
        #[cfg(not(windows))]
        let result = resolve_command("nonexistent-tool-xyz");
        assert_eq!(result, "nonexistent-tool-xyz");
    }
}
