//! Cross-module contracts: filesystem -> model -> input -> persistence -> UI.
//! All writable state belongs to fixtures; no personal config or global env.
use std::{fs, path::Path};

use clap::Parser;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use crate::{
    app::{App, FilterField, Mode, SortField, ViewMode},
    config::{self, Config},
    project::{ArtifactKind, DirtyStatus, Project, ProjectStatus},
    scanner, scoring,
    snapshot::DirSnapshot,
};

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
#[ignore = "large reproducible performance fixture; run scripts/bench_scan.py"]
fn benchmark_scan_fixture() {
    use std::time::Instant;
    let root =
        std::path::PathBuf::from(std::env::var("DS_BENCH_ROOT").expect("owned benchmark root"));
    if !root.join("ready").exists() {
        for i in 0..1000 {
            let path = root.join(format!("group-{}/project-{i:04}", i % 10));
            fs::create_dir_all(path.join("src/nested")).unwrap();
            let (marker, content) = match i % 4 {
                0 => ("Cargo.toml", "[package]\nname='bench-project'\nversion='0.1.0'\n"),
                1 => ("package.json", "{\"name\":\"bench-project\",\"scripts\":{\"dev\":\"vite\",\"test\":\"vitest\"},\"dependencies\":{\"react\":\"1\"}}"),
                2 => ("pyproject.toml", "[project]\nname='bench-project'\nversion='0.1.0'\n"),
                _ => ("go.mod", "module example.com/bench\ngo 1.22\n"),
            };
            fs::write(path.join(marker), content).unwrap();
            fs::write(path.join("README.md"), "Benchmark fixture").unwrap();
            fs::write(path.join(".gitignore"), "target/\nnode_modules/\n").unwrap();
            for n in 0..16 {
                fs::write(
                    path.join(format!("src/nested/file-{n}.txt")),
                    "source contents\n",
                )
                .unwrap();
            }
            fs::create_dir_all(path.join("node_modules/ignored")).unwrap();
            fs::write(path.join("node_modules/ignored/package.json"), "{}").unwrap();
            if i % 5 == 0 {
                let repo = git2::Repository::init(&path).unwrap();
                let mut index = repo.index().unwrap();
                index.add_path(Path::new("README.md")).unwrap();
                index.write().unwrap();
                let oid = index.write_tree().unwrap();
                let tree = repo.find_tree(oid).unwrap();
                let sig = git2::Signature::new(
                    "Benchmark",
                    "bench@example.invalid",
                    &git2::Time::new(1700000000, 0),
                )
                .unwrap();
                repo.commit(Some("HEAD"), &sig, &sig, "fixture", &tree, &[])
                    .unwrap();
            }
        }
        fs::write(
            root.join("ready"),
            "1000 projects; 200 Git repos; 16000 source files",
        )
        .unwrap();
    }
    let config = Config {
        roots: vec![root.to_string_lossy().into_owned()],
        ..Config::default()
    };
    for _ in 0..3 {
        let mut result = scanner::scan_roots(&config).unwrap();
        scanner::hydrate_git_statuses(&mut result.projects);
    }
    let mut rows = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let mut result = scanner::scan_roots(&config).unwrap();
        let scan = start.elapsed().as_secs_f64() * 1000.;
        let count = result.projects.len();
        assert_eq!(count, 1000);
        scanner::hydrate_git_statuses(&mut result.projects);
        rows.push(serde_json::json!({"scan_ms":scan,"full_scan_ms":start.elapsed().as_secs_f64()*1000.,"projects":count}));
    }
    println!("DS_BENCH_JSON={}", serde_json::to_string(&rows).unwrap());
}

fn fixture() -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().unwrap();
    for name in ["alpha", "beta", "gamma"] {
        write(
            dir.path(),
            &format!("{name}/Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        );
    }
    let config = Config {
        roots: vec![dir.path().to_string_lossy().into_owned()],
        ..Config::default()
    };
    // Construct without starting background network/process or Git workers.
    let mut app = App::new(Config::default());
    app.projects = scanner::scan_roots(&config).unwrap().projects;
    app.projects.sort_by(|a, b| a.name.cmp(&b.name));
    app.total_projects = app.projects.len();
    app.config = config;
    app.config_save_path = Some(dir.path().join("saved.toml"));
    app.sort = SortField::Name;
    app.apply_filter_and_sort();
    (dir, app)
}

fn key(app: &mut App, code: KeyCode) {
    crate::input::handle_key_event(app, KeyEvent::new(code, KeyModifiers::NONE));
}

fn names(app: &App) -> Vec<&str> {
    app.filtered_indices
        .iter()
        .map(|&i| app.projects[i].name.as_str())
        .collect()
}

fn render(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| crate::ui::draw(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn help_scroll_reaches_the_end_and_close_hint_stays_visible() {
    let (_dir, mut app) = fixture();
    app.viewport = (80, 24);
    key(&mut app, KeyCode::Char('?'));
    let first = render(&app, 80, 24);
    assert!(first.contains("Esc / ? close"));
    assert!(!first.contains("Ctrl+C   Quit"));
    key(&mut app, KeyCode::End);
    let last = render(&app, 80, 24);
    assert_ne!(first, last);
    assert!(last.contains("Ctrl+C   Quit"));
    assert!(last.contains("Esc / ? close"));
    assert_eq!(app.help_scroll, crate::ui::help_scroll_limit(&app));
    key(&mut app, KeyCode::Down);
    assert_eq!(app.help_scroll, crate::ui::help_scroll_limit(&app));
    key(&mut app, KeyCode::Home);
    assert_eq!(app.help_scroll, 0);
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn concurrent_config_merges_independent_edits_and_adds_visits_without_lost_updates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    config::save_config_at(&Config::default(), &path).unwrap();
    config::with_test_config_path(path.clone(), || {
        let mut a = config::load_config().unwrap();
        let mut b = config::load_config().unwrap();
        config::set_note(&mut a, "one", "first".into());
        config::set_note(&mut b, "two", "second".into());
        config::record_visit(&mut a, "project");
        config::record_visit(&mut b, "project");
        config::commit_config_at(&a, &path).unwrap();
        config::commit_config_at(&b, &path).unwrap();
        let saved = config::load_config().unwrap();
        assert_eq!(saved.notes.len(), 2);
        assert_eq!(saved.scores["project"].visits, 2);
        let mut a = saved.clone();
        let mut b = saved;
        config::set_note(&mut a, "one", "changed-a".into());
        config::set_note(&mut b, "one", "changed-b".into());
        config::commit_config_at(&a, &path).unwrap();
        assert!(config::commit_config_at(&b, &path).is_err());
        assert_eq!(config::load_config().unwrap().notes["one"], "changed-a");
    });
}

#[test]
fn canonical_project_identity_deduplicates_alias_roots_and_keeps_old_metadata() {
    let (dir, mut app) = fixture();
    let root = dir.path().join("alpha");
    let alias = root.join("..").join("alpha").to_string_lossy().into_owned();
    app.config.roots = vec![root.to_string_lossy().into_owned(), alias.clone()];
    app.config.notes.insert(alias, "legacy-note".into());
    let scanned = scanner::scan_roots(&app.config).unwrap();
    assert_eq!(scanned.projects.len(), 1);
    assert_eq!(scanned.projects[0].id, config::project_key(&root));
    assert_eq!(scanned.projects[0].note.as_deref(), Some("legacy-note"));
}

#[test]
fn compact_header_and_border_context_preserve_nine_project_rows_at_80x24() {
    let (_dir, mut app) = fixture();
    let template = app.projects[0].clone();
    app.projects = (0..24)
        .map(|i| {
            let mut project = template.clone();
            project.name = format!("project-{i:02}");
            project.id = project.name.clone();
            project
        })
        .collect();
    app.total_projects = 24;
    app.apply_filter_and_sort();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| crate::ui::draw(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let row = |y: usize| {
        buffer.content[y * 80..(y + 1) * 80]
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    assert!(row(0).contains("ds"));
    assert!(row(1).contains("Projects 1-9/24"));
    assert!(row(1).contains("filter: all"));
    assert!(row(1).contains("sort: name"));
    for i in 0..9 {
        assert!(row(i + 3).contains(&format!("project-{i:02}")));
    }
    assert!(!row(12).contains("project-09"));
    assert!(!render(&app, 80, 24).contains("Overview"));
}

#[test]
fn details_show_a_long_name_once_without_a_duplicate_name_field() {
    let (_dir, mut app) = fixture();
    let name = "project-with-a-name-that-is-longer-than-a-normal-panel-and-ends-here";
    app.projects[0].name = name.into();
    let screen = render(&app, 80, 24);
    assert!(screen.contains(name));
    // List header Name still exists; details must not add its own Name row.
    let area = crate::ui::layout::create_main_layout(ratatui::layout::Rect::new(0, 0, 80, 24));
    let details = crate::ui::layout::create_content_layout(area[1], app.view_mode)[1];
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|frame| crate::ui::draw(frame, &app)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .chunks(80)
        .skip(details.y as usize)
        .take(details.height as usize)
        .flat_map(|row| row.iter().map(|cell| cell.symbol()))
        .collect::<String>();
    assert_eq!(text.matches(name).count(), 1);
    assert!(!text.contains("Name"));
}

#[test]
fn focused_details_reveal_long_values_commands_ports_and_all_health_items() {
    let (_dir, mut app) = fixture();
    app.viewport = (80, 24);
    app.projects[0].path =
        "/company/very-long-workspace-path/services/project-with-long-name".into();
    app.projects[0].note = Some("note-start: ".to_string() + &"long text ".repeat(30) + "note-end");
    app.projects[0].ports = vec![3000, 8080];
    app.projects[0].health.positives = (0..12).map(|i| format!("positive-{i}")).collect();
    let id = app.selected_project().unwrap().id.clone();
    key(&mut app, KeyCode::Tab);
    assert!(app.details_focus);
    let mut screens = String::new();
    let limit = crate::ui::details::scroll_limit(&app);
    assert!(limit > 0);
    for _ in 0..=limit {
        screens.push_str(&render(&app, 80, 24));
        key(&mut app, KeyCode::Down);
    }
    for value in [
        "note-start",
        "note-end",
        "cargo build",
        "3000, 8080",
        "positive-11",
    ] {
        assert!(screens.contains(value), "missing {value}");
    }
    assert_eq!(app.selected_project().unwrap().id, id);
    assert_eq!(app.details_scroll, limit);
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Down);
    assert_ne!(app.selected_project().unwrap().id, id);
    assert_eq!(app.details_scroll, 0);
}

#[test]
fn confirmed_search_and_empty_states_explain_the_active_filter() {
    let (_dir, mut app) = fixture();
    key(&mut app, KeyCode::Char('/'));
    for ch in "alpha".chars() {
        key(&mut app, KeyCode::Char(ch));
    }
    key(&mut app, KeyCode::Enter);
    let screen = render(&app, 80, 24);
    assert!(screen.contains("search: alpha [Esc clear]"));
    app.search_query = "missing".into();
    app.apply_filter_and_sort();
    let screen = render(&app, 80, 24);
    assert!(screen.contains("No matching projects."));
    assert!(screen.contains("Esc to clear search"));
    app.projects.clear();
    app.total_projects = 0;
    app.apply_filter_and_sort();
    let screen = render(&app, 80, 24);
    assert!(screen.contains("ds add <path>"));
    assert!(!screen.contains("No matching projects."));
}

#[test]
fn long_editor_text_keeps_cursor_end_and_save_cancel_hints_visible() {
    let (_dir, mut app) = fixture();
    key(&mut app, KeyCode::Char('n'));
    app.note_input = "long prefix ".repeat(30) + "VISIBLE-END";
    let screen = render(&app, 80, 24);
    assert!(screen.contains("VISIBLE-END█"));
    assert!(screen.contains("Enter save / Esc cancel"));
    key(&mut app, KeyCode::Esc);
    assert!(!app.config_save_path.as_ref().unwrap().exists());
    app.mode = Mode::Search;
    app.search_query = "日本 ".repeat(30) + "QUERY-END";
    let screen = render(&app, 80, 24);
    assert!(screen.contains("QUERY-END█"));
    assert!(screen.contains("Enter keep / Esc clear"));
}

#[test]
fn menu_navigation_can_reach_and_confirm_the_last_action_in_small_windows() {
    let (_dir, mut app) = fixture();
    app.viewport = (80, 12);
    key(&mut app, KeyCode::Char('o'));
    key(&mut app, KeyCode::End);
    let action = app.config.open.actions.last().unwrap().clone();
    let screen = render(&app, 80, 12);
    assert!(screen.contains(&action.name));
    assert!(screen.contains("Esc cancel"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(
        app.pending_action.as_ref().unwrap().action.name,
        action.name
    );
    assert!(!app.config_save_path.as_ref().unwrap().exists());
}

#[test]
fn errors_use_error_style_and_survive_navigation_while_preserving_note_draft() {
    let (dir, mut app) = fixture();
    app.config_save_path = Some(dir.path().join("alpha")); // existing directory cannot be replaced
    key(&mut app, KeyCode::Char('n'));
    app.note_input = "draft".into();
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.message_level, crate::app::MessageLevel::Error);
    assert_eq!(app.note_input, "draft");
    let screen = render(&app, 80, 24);
    assert!(screen.contains("Error"));
    assert!(screen.contains("Could not save"));
    key(&mut app, KeyCode::Down);
    assert!(app.status_message.is_some());
}

#[test]
fn layout_preserves_original_density_from_125_and_accounts_for_short_windows() {
    use ratatui::layout::Rect;
    let a = crate::ui::layout::create_content_layout(Rect::new(0, 0, 124, 27), ViewMode::Detailed);
    let b = crate::ui::layout::create_content_layout(Rect::new(0, 0, 125, 27), ViewMode::Detailed);
    assert_eq!(a[0].width, 124);
    assert_eq!(b[0].height, 27);
    assert!(b[1].width >= 48);
    let short =
        crate::ui::layout::create_content_layout(Rect::new(0, 0, 125, 15), ViewMode::Detailed);
    assert_eq!(short[0].height, 15);
    assert_eq!(short[1].height, 15);
    let wide =
        crate::ui::layout::create_content_layout(Rect::new(0, 0, 160, 42), ViewMode::Detailed);
    assert!(wide[0].width >= 96);
    assert!(wide[1].width >= 48);
}

#[test]
fn visual_preferences_apply_without_changing_project_data_or_saved_config() {
    let mut config = Config::default();
    config.ui.right_panel = false;
    let app = App::new(config);
    assert_eq!(app.view_mode, ViewMode::Compact);
    let (_dir, mut app) = fixture();
    let projects = serde_json::to_string(&app.projects).unwrap();
    app.config.ui.theme = "light".into();
    app.config.ui.show_icons = false;
    let screen = render(&app, 160, 45);
    assert!(screen.contains("warn"));
    assert!(!screen.contains("✓"));
    assert_eq!(projects, serde_json::to_string(&app.projects).unwrap());
    assert!(!app.config_save_path.as_ref().unwrap().exists());
}

#[test]
fn config_defaults_accept_old_minimal_files_and_unknown_fields() {
    let config: Config = toml::from_str("roots=[]\nfuture_option=true\n").unwrap();
    assert_eq!(config.max_depth, 4);
    assert!(config.notes.is_empty());
    assert!(config.scores.is_empty());
    assert!(!config.open.actions.is_empty());
    assert!(toml::from_str::<Config>("max_depth='wrong'").is_err());
    assert!(toml::from_str::<Config>("notes=[]").is_err());
}

#[test]
fn config_round_trip_preserves_unicode_notes_statuses_scores_and_custom_actions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/config.toml");
    let mut config = Config {
        roots: vec!["/work/Proyecto 日本".into()],
        ..Config::default()
    };
    config::set_note(&mut config, "project", "línea 1\n日本 🦀\n\"quote\"".into());
    config::set_project_status(&mut config, "project", ProjectStatus::Paused);
    config::record_visit(&mut config, "project");
    config::record_open(&mut config, "project");
    config.open.actions[0].args = vec!["{path}".into(), "--name={name}".into()];
    config::save_config_at(&config, &path).unwrap();
    let loaded: Config = toml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(loaded.roots, config.roots);
    assert_eq!(loaded.notes, config.notes);
    assert_eq!(
        config::get_project_status(&loaded, "project"),
        Some(ProjectStatus::Paused)
    );
    assert_eq!(loaded.scores["project"].visits, 1);
    assert_eq!(loaded.scores["project"].opens, 1);
    assert_eq!(loaded.open.actions[0].args, config.open.actions[0].args);
}

#[test]
fn readonly_config_failure_preserves_contents_and_does_not_leave_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "roots=[]\n").unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = config::save_config_at(&Config::default(), &path);
    // Restore before assertions so TempDir can remove the fixture on Windows.
    fs::set_permissions(&path, original_permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "roots=[]\n");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn counters_saturate_independently_without_losing_other_projects() {
    let mut config = Config::default();
    config.scores.insert(
        "full".into(),
        scoring::ScoreEntry {
            visits: u32::MAX,
            opens: u32::MAX,
            last_used: None,
        },
    );
    config::record_visit(&mut config, "full");
    config::record_open(&mut config, "full");
    config::record_open(&mut config, "other");
    assert_eq!(config.scores["full"].visits, u32::MAX);
    assert_eq!(config.scores["full"].opens, u32::MAX);
    assert!(config.scores["full"].last_used.is_some());
    assert_eq!(config.scores["other"].opens, 1);
    assert_eq!(config.scores["other"].visits, 0);
}

#[test]
fn action_arguments_preserve_spaces_quotes_unicode_and_shell_metacharacters() {
    let mut action = Config::default().open.actions.remove(0);
    action.args = vec![
        "{path}".into(),
        "--name={name}".into(),
        "$HOME; literal".into(),
    ];
    let path = Path::new("/work/my project 🦀");
    assert_eq!(
        action.resolve_args(path, "quoted \"name\""),
        vec![
            "/work/my project 🦀",
            "--name=quoted \"name\"",
            "$HOME; literal"
        ]
    );
}

#[test]
fn snapshot_distinguishes_files_directories_and_missing_content() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "README.md", "hello");
    fs::create_dir(dir.path().join("src")).unwrap();
    let snapshot = DirSnapshot::read(dir.path());
    assert!(snapshot.has_any(&["missing", "README.md"]));
    assert!(snapshot
        .entries()
        .iter()
        .any(|e| e.name == "README.md" && e.is_file));
    assert!(snapshot
        .entries()
        .iter()
        .any(|e| e.name == "src" && !e.is_file));
    assert_eq!(
        snapshot.read_to_string("README.md").as_deref(),
        Some("hello")
    );
    assert!(snapshot.read_to_string("missing").is_none());
    assert!(snapshot.read_to_string("src").is_none());
    assert!(DirSnapshot::read(&dir.path().join("absent"))
        .entries()
        .is_empty());
}

#[test]
fn stack_detection_covers_supported_marker_families() {
    for (marker, content, expected) in [
        ("Cargo.toml", "[package]\nname='demo'", "Rust"),
        ("package.json", "{}", "Node"),
        ("go.mod", "module example.test/app", "Go"),
        ("requirements.txt", "", "Python"),
        ("pubspec.yaml", "name: demo", "Flutter/Dart"),
        ("Dockerfile", "FROM scratch", "Docker"),
        ("pom.xml", "<project/>", "Java"),
        ("Gemfile", "", "Ruby"),
        ("Package.swift", "", "Swift"),
        ("deno.json", "{}", "Deno"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), marker, content);
        assert!(
            crate::detect::detect_stack(dir.path())
                .iter()
                .any(|s| s == expected),
            "{marker}: {expected}"
        );
    }
}

#[test]
fn node_frameworks_manager_and_scripts_are_detected_together() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "package.json",
        r#"{"dependencies":{"react":"*","next":"*"},"scripts":{"dev":"next dev","test":"test","custom":"echo custom"}}"#,
    );
    write(dir.path(), "pnpm-lock.yaml", "");
    let stack = crate::detect::detect_stack(dir.path());
    for expected in ["Node", "React", "Next.js"] {
        assert!(stack.iter().any(|s| s == expected));
    }
    assert_eq!(
        crate::detect::detect_manager(dir.path()).as_deref(),
        Some("pnpm")
    );
    let scripts = crate::detect::detect_scripts(dir.path());
    assert_eq!(scripts, vec!["custom", "dev", "test"]);
    let commands = crate::commands::detect_commands(dir.path(), &stack);
    assert!(commands.iter().any(|c| c.command == "pnpm dev"));
    assert!(commands.iter().any(|c| c.command == "pnpm install"));
}

#[test]
fn manager_detection_handles_each_unambiguous_lockfile_family() {
    for (marker, manager) in [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
        ("Cargo.toml", "cargo"),
        ("go.mod", "go"),
        ("pubspec.yaml", "pub"),
        ("Pipfile", "pipenv"),
        ("requirements.txt", "pip"),
        ("Gemfile", "bundler"),
        ("pom.xml", "maven"),
        ("build.gradle.kts", "gradle"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), marker, "");
        assert_eq!(
            crate::detect::detect_manager(dir.path()).as_deref(),
            Some(manager),
            "{marker}"
        );
    }
}

#[test]
fn malformed_and_wrong_shaped_package_scripts_are_safe() {
    let dir = tempfile::tempdir().unwrap();
    for contents in [
        "{",
        "null",
        "[]",
        r#"{"scripts":null}"#,
        r#"{"scripts":[]}"#,
    ] {
        write(dir.path(), "package.json", contents);
        assert!(crate::detect::detect_scripts(dir.path()).is_empty());
        assert!(crate::commands::detect_commands(dir.path(), &[]).is_empty());
    }
}

#[test]
fn scanner_applies_saved_metadata_and_deduplicates_identical_roots() {
    let (_dir, mut app) = fixture();
    let id = app
        .projects
        .iter()
        .find(|p| p.name == "beta")
        .unwrap()
        .id
        .clone();
    app.config.roots.push(app.config.roots[0].clone());
    config::set_note(&mut app.config, &id, "saved 🦀".into());
    config::set_project_status(&mut app.config, &id, ProjectStatus::Archived);
    let result = scanner::scan_roots(&app.config).unwrap();
    assert_eq!(result.projects_found, 3);
    assert_eq!(result.projects.len(), 3);
    let beta = result.projects.iter().find(|p| p.id == id).unwrap();
    assert_eq!(beta.note.as_deref(), Some("saved 🦀"));
    assert_eq!(beta.status, ProjectStatus::Archived);
    assert_eq!(beta.warnings, beta.health.warnings);
}

#[test]
fn scanner_ignores_build_and_vendor_trees_and_unmarked_directories() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "node_modules",
        "target",
        "vendor",
        "dist",
        "build",
        ".hidden",
    ] {
        write(dir.path(), &format!("{name}/nested/package.json"), "{}");
    }
    write(dir.path(), "plain/README.md", "not a project");
    write(dir.path(), "actual/package.json", "{}");
    let result = scanner::scan_roots(&Config {
        roots: vec![dir.path().to_string_lossy().into_owned()],
        ..Config::default()
    })
    .unwrap();
    assert_eq!(result.projects.len(), 1);
    assert_eq!(result.projects[0].name, "actual");
}

#[test]
fn scanner_handles_missing_root_and_zero_roots_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    for config in [
        Config::default(),
        Config {
            roots: vec![dir.path().join("missing").to_string_lossy().into_owned()],
            ..Config::default()
        },
    ] {
        let result = scanner::scan_roots(&config).unwrap();
        assert!(result.projects.is_empty());
        assert_eq!(result.projects_found, 0);
    }
}

#[test]
fn project_model_json_round_trip_keeps_public_fields_and_unicode() {
    let (_dir, mut app) = fixture();
    let project = &mut app.projects[0];
    project.note = Some("Nota 🦀 日本".into());
    project.ports = vec![3000, 8080];
    let json = serde_json::to_value(&*project).unwrap();
    assert!(json.get("warnings").is_some());
    assert!(json["health"].get("warnings").is_some());
    let decoded: Project = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.id, project.id);
    assert_eq!(decoded.note, project.note);
    assert_eq!(decoded.path, project.path);
    assert_eq!(decoded.ports, project.ports);
}

#[test]
fn legacy_project_json_without_ports_still_loads() {
    let (_dir, app) = fixture();
    let mut json = serde_json::to_value(&app.projects[0]).unwrap();
    json.as_object_mut().unwrap().remove("ports");
    let project: Project = serde_json::from_value(json).unwrap();
    assert!(project.ports.is_empty());
}

#[test]
fn artifact_detection_reports_existing_node_outputs_and_absent_outputs() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("dist")).unwrap();
    let artifacts = crate::artifacts::detect_artifacts(dir.path(), &["Node".into()]);
    let dist = artifacts
        .iter()
        .find(|a| a.path == dir.path().join("dist"))
        .unwrap();
    assert!(dist.exists);
    assert_eq!(dist.kind, ArtifactKind::Folder);
    assert!(
        !artifacts
            .iter()
            .find(|a| a.path == dir.path().join("build"))
            .unwrap()
            .exists
    );
}

#[test]
fn flutter_artifacts_prefer_release_apk_and_find_windows_executable() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "build/app/outputs/flutter-apk/app-debug.apk",
        "",
    );
    write(
        dir.path(),
        "build/app/outputs/flutter-apk/app-release.apk",
        "",
    );
    write(dir.path(), "build/windows/x64/runner/Release/demo.exe", "");
    let artifacts = crate::artifacts::detect_artifacts(dir.path(), &["Flutter/Dart".into()]);
    assert!(artifacts
        .iter()
        .any(|a| a.kind == ArtifactKind::Executable && a.exists));
    let apks: Vec<_> = artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Apk)
        .collect();
    assert_eq!(apks.len(), 1);
    assert!(apks[0].path.ends_with("app-release.apk"));
}

#[test]
fn unknown_stack_produces_no_guessed_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    assert!(crate::artifacts::detect_artifacts(dir.path(), &[]).is_empty());
}

#[test]
fn navigation_clamps_at_both_ends_and_empty_lists() {
    let (_dir, mut app) = fixture();
    app.move_up();
    assert_eq!(app.selected, 0);
    app.move_end();
    assert_eq!(app.selected, 2);
    app.move_down();
    assert_eq!(app.selected, 2);
    app.move_page_up();
    assert_eq!(app.selected, 0);
    app.move_page_down();
    assert_eq!(app.selected, 2);
    app.move_home();
    assert_eq!(app.selected, 0);
    app.projects.clear();
    app.apply_filter_and_sort();
    app.move_down();
    app.move_end();
    app.move_page_down();
    app.move_up();
    assert_eq!(app.selected, 0);
    assert!(app.selected_project().is_none());
}

#[test]
fn filters_match_status_notes_and_stack_without_losing_selection_identity() {
    let (_dir, mut app) = fixture();
    app.projects[0].status = ProjectStatus::Paused;
    app.projects[1].status = ProjectStatus::Active;
    app.projects[1].note = Some("todo".into());
    app.projects[2].status = ProjectStatus::Archived;
    for (filter, expected) in [
        (FilterField::Paused, vec!["alpha"]),
        (FilterField::Active, vec!["beta"]),
        (FilterField::Archived, vec!["gamma"]),
        (FilterField::WithNotes, vec!["beta"]),
        (FilterField::Node, vec![]),
        (FilterField::Rust, vec!["alpha", "beta", "gamma"]),
    ] {
        app.filter = filter;
        app.apply_filter_and_sort();
        assert_eq!(names(&app), expected);
        assert!(app.selected < app.filtered_count().max(1));
    }
}

#[test]
fn filter_and_sort_cycles_return_to_the_original_value() {
    let (_dir, mut app) = fixture();
    let filter = app.filter;
    let sort = app.sort;
    for _ in FilterField::all() {
        app.next_filter();
    }
    for _ in SortField::all() {
        app.next_sort();
    }
    assert_eq!(app.filter, filter);
    assert_eq!(app.sort, sort);
}

#[test]
fn score_sort_uses_usage_and_keeps_selected_project() {
    let (_dir, mut app) = fixture();
    let selected = app.selected_project().unwrap().id.clone();
    let beta = app
        .projects
        .iter()
        .find(|p| p.name == "beta")
        .unwrap()
        .id
        .clone();
    app.config.scores.insert(
        beta,
        scoring::ScoreEntry {
            visits: 100,
            opens: 100,
            last_used: None,
        },
    );
    app.sort = SortField::Score;
    app.apply_filter_and_sort();
    assert_eq!(names(&app)[0], "beta");
    assert_eq!(app.selected_project().unwrap().id, selected);
}

#[test]
fn search_can_match_notes_and_cancel_restores_list() {
    let (_dir, mut app) = fixture();
    app.projects[1].note = Some("pendiente 日本".into());
    key(&mut app, KeyCode::Char('/'));
    for ch in "日本".chars() {
        key(&mut app, KeyCode::Char(ch));
    }
    assert_eq!(names(&app), vec!["beta"]);
    key(&mut app, KeyCode::Backspace);
    assert_eq!(app.search_query, "日");
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.filtered_count(), 3);
    assert!(app.search_query.is_empty());
}

#[test]
fn cancelled_note_does_not_write_and_successful_note_survives_rescan() {
    let (dir, mut app) = fixture();
    key(&mut app, KeyCode::Char('n'));
    for ch in "discard 🦀".chars() {
        key(&mut app, KeyCode::Char(ch));
    }
    key(&mut app, KeyCode::Esc);
    assert!(!dir.path().join("saved.toml").exists());
    assert!(app.selected_project().unwrap().note.is_none());
    key(&mut app, KeyCode::Char('n'));
    for ch in "persist 日本".chars() {
        key(&mut app, KeyCode::Char(ch));
    }
    key(&mut app, KeyCode::Enter);
    let config: Config =
        toml::from_str(&fs::read_to_string(dir.path().join("saved.toml")).unwrap()).unwrap();
    let result = scanner::scan_roots(&config).unwrap();
    assert_eq!(
        result
            .projects
            .iter()
            .find(|p| p.name == "alpha")
            .unwrap()
            .note
            .as_deref(),
        Some("persist 日本")
    );
}

#[test]
fn status_save_failure_keeps_edit_target_and_original_status() {
    let (dir, mut app) = fixture();
    let original = app.selected_project().unwrap().status.clone();
    key(&mut app, KeyCode::Char('m'));
    let target = app.editing_project_id.clone();
    app.status_selected = 3;
    app.config_save_path = Some(dir.path().to_path_buf());
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.mode, Mode::ChangingStatus);
    assert_eq!(app.editing_project_id, target);
    assert_eq!(app.selected_project().unwrap().status, original);
    assert!(app.config.project_status.is_empty());
    assert!(app.status_message.as_deref().unwrap().contains("save"));
}

#[test]
fn enter_records_one_visit_and_failed_save_records_none() {
    let (dir, mut app) = fixture();
    let id = app.selected_project().unwrap().id.clone();
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.config.scores[&id].visits, 1);
    app.config_save_path = Some(dir.path().to_path_buf());
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.config.scores[&id].visits, 1);
}

#[test]
fn open_menu_captures_selected_project_and_cancel_launches_nothing() {
    let (_dir, mut app) = fixture();
    key(&mut app, KeyCode::Char('o'));
    key(&mut app, KeyCode::Esc);
    assert!(app.pending_action.is_none());
    let selected = app.selected_project().unwrap().path.clone();
    key(&mut app, KeyCode::Char('o'));
    key(&mut app, KeyCode::Char('V'));
    let pending = app.pending_action.as_ref().unwrap();
    assert_eq!(pending.project_path, selected);
    assert_eq!(pending.action.command.as_deref(), Some("code"));
    app.move_down();
    assert_eq!(app.pending_action.as_ref().unwrap().project_path, selected);
}

#[test]
fn empty_list_edit_and_open_actions_never_write_or_panic() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(Config::default());
    app.config_save_path = Some(dir.path().join("must-not-exist.toml"));
    for code in [
        KeyCode::Char('n'),
        KeyCode::Char('m'),
        KeyCode::Enter,
        KeyCode::Char('o'),
        KeyCode::Char('v'),
    ] {
        key(&mut app, code);
    }
    assert!(!dir.path().join("must-not-exist.toml").exists());
    assert!(app.pending_action.is_none());
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn rendering_handles_empty_and_populated_lists_at_small_and_large_sizes() {
    let (_dir, mut app) = fixture();
    for populated in [true, false] {
        if !populated {
            app.projects.clear();
            app.apply_filter_and_sort();
        }
        for mode in [
            Mode::Normal,
            Mode::Search,
            Mode::EditingNote,
            Mode::ChangingStatus,
            Mode::Help,
            Mode::OpenMenu,
            Mode::ConfigMenu,
        ] {
            app.mode = mode;
            app.note_input = "長いメモ 🦀".repeat(80);
            app.search_query = "🦀".into();
            for view in [ViewMode::Compact, ViewMode::Detailed] {
                app.view_mode = view;
                for (width, height) in [(1, 1), (10, 3), (40, 10), (80, 24), (160, 45)] {
                    let _ = render(&app, width, height);
                }
            }
        }
    }
}

#[test]
fn rendering_displays_saved_note_and_selected_project() {
    let (_dir, mut app) = fixture();
    app.projects[0].note = Some("unique-saved-note".into());
    app.apply_filter_and_sort();
    let screen = render(&app, 160, 45);
    assert!(screen.contains("alpha"));
    assert!(screen.contains("unique-saved-note"));
    assert!(screen.contains("quit"));
}

#[test]
fn tokenization_and_matching_cover_unicode_camel_case_and_negative_cases() {
    assert_eq!(
        scoring::tokenize("myHTTPClient-demo_v2"),
        vec!["my", "httpclient", "demo", "v2"]
    );
    for (name, query) in [
        ("my-app", "myapp"),
        ("devScope", "ds"),
        ("Proyecto-Ágil", "ágil"),
        ("日本-project", "日本"),
        ("myHTTPClient", "http"),
    ] {
        assert!(scoring::matches_name(name, query), "{name} / {query}");
    }
    assert!(!scoring::matches_name("alpha", "beta"));
    assert!(!scoring::matches_name("ab", "ba"));
}

#[test]
fn exact_name_ranks_above_prefix_and_subsequence_at_equal_usage() {
    let entry = scoring::ScoreEntry::default();
    let exact = scoring::compute_score("project", "project", &entry);
    let prefix = scoring::compute_score("project-tool", "project", &entry);
    let subsequence = scoring::compute_score("p-r-o-j-e-c-t", "project", &entry);
    assert!(exact > prefix);
    assert!(prefix > subsequence);
}

#[test]
fn cli_parser_accepts_commands_and_rejects_missing_or_unknown_arguments() {
    for args in [
        vec!["ds", "scan"],
        vec!["ds", "list", "--json"],
        vec!["ds", "note", "project", "text"],
        vec!["ds", "status", "project", "paused"],
        vec!["ds", "add-root", "with spaces"],
        vec!["ds", "remove-root", "path"],
        vec!["ds", "roots"],
        vec!["ds", "config", "--edit"],
        vec!["ds", "open", "project"],
        vec!["ds", "discover", "--apply"],
    ] {
        assert!(crate::cli::Cli::try_parse_from(args).is_ok());
    }
    for args in [
        vec!["ds", "note", "project"],
        vec!["ds", "status"],
        vec!["ds", "list", "--unknown"],
        vec!["ds", "add-root"],
        vec!["ds", "unknown-command"],
    ] {
        assert!(crate::cli::Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn cli_project_resolution_handles_exact_partial_missing_and_ambiguous_names() {
    let (_dir, app) = fixture();
    assert!(crate::find_project_path(&app.config, "ALPHA")
        .unwrap()
        .ends_with("alpha"));
    assert!(crate::find_project_path(&app.config, "bet")
        .unwrap()
        .ends_with("beta"));
    assert!(crate::find_project_path(&app.config, "not-found").is_err());
    assert!(crate::find_project_path(&app.config, "a").is_err());
}

#[test]
fn session_root_validation_rejects_missing_paths_and_files() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "file", "");
    assert!(crate::build_temporary_root_config(
        Config::default(),
        dir.path().join("missing").to_str().unwrap()
    )
    .is_err());
    assert!(crate::build_temporary_root_config(
        Config::default(),
        dir.path().join("file").to_str().unwrap()
    )
    .is_err());
}

#[test]
fn relative_times_reject_out_of_range_dates_and_missing_dates() {
    let mut activity = crate::project::ActivityInfo {
        timestamp: None,
        last_modified_ts: None,
        last_git_activity_ts: None,
    };
    assert_eq!(activity.relative_time(), "unknown");
    activity.timestamp = Some(i64::MAX);
    assert_eq!(activity.relative_time(), "unknown");
    activity.last_git_activity_ts = Some(i64::MIN);
    assert_eq!(activity.last_git_activity_display(), Some(String::new()));
}

#[test]
fn git_status_distinguishes_clean_untracked_staged_and_deleted_paths() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    write(dir.path(), "tracked", "first");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("tracked")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("test", "test@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
        .unwrap();
    assert_eq!(
        crate::git::get_git_status(dir.path()).unwrap(),
        (DirtyStatus::Clean, Some(0), Some(0))
    );
    write(dir.path(), "new", "untracked");
    assert_eq!(
        crate::git::get_git_status(dir.path()).unwrap(),
        (DirtyStatus::Dirty, Some(0), Some(1))
    );
    index.add_path(Path::new("new")).unwrap();
    index.write().unwrap();
    fs::remove_file(dir.path().join("tracked")).unwrap();
    assert_eq!(
        crate::git::get_git_status(dir.path()).unwrap(),
        (DirtyStatus::Dirty, Some(2), Some(0))
    );
}

#[test]
fn git_info_supports_unborn_committed_and_detached_head() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let empty = crate::git::get_git_info_fast(dir.path()).unwrap();
    assert_eq!(empty.last_commit_hash, "none");
    assert!(empty.last_commit_timestamp.is_none());
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("test", "test@example.invalid").unwrap();
    let oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "first line\nsecond line",
            &tree,
            &[],
        )
        .unwrap();
    let info = crate::git::get_git_info_fast(dir.path()).unwrap();
    assert_eq!(info.last_commit_hash, &oid.to_string()[..7]);
    assert_eq!(info.last_commit_message, "first line");
    assert!(info.last_commit_timestamp.is_some());
    repo.set_head_detached(oid).unwrap();
    let detached = crate::git::get_git_info_fast(dir.path()).unwrap();
    assert_eq!(detached.last_commit_hash, info.last_commit_hash);
    assert!(detached.upstream.is_none());
}

#[test]
fn cli_mutations_scan_and_list_use_only_the_injected_config() {
    let (dir, app) = fixture();
    let path = dir.path().join("cli/config.toml");
    config::with_test_config_path(path.clone(), || {
        config::save_config(&app.config).unwrap();
        crate::cmd_roots().unwrap();
        crate::cmd_scan().unwrap();
        crate::cmd_list(true).unwrap();
        crate::cmd_list(false).unwrap();
        crate::cmd_config(false).unwrap();
        crate::cmd_config(true).unwrap();
        crate::cmd_note("alpha".into(), "persisted through CLI 🦀".into()).unwrap();
        crate::cmd_status("alpha".into(), "paused".into()).unwrap();
        crate::cmd_open("alpha".into()).unwrap();
        let saved = config::load_config().unwrap();
        let alpha = app.projects.iter().find(|p| p.name == "alpha").unwrap();
        assert_eq!(saved.notes[&alpha.id], "persisted through CLI 🦀");
        assert_eq!(
            config::get_project_status(&saved, &alpha.id),
            Some(ProjectStatus::Paused)
        );
        assert_eq!(saved.scores[&alpha.id].opens, 3);
        assert_eq!(saved.scores[&alpha.id].visits, 0);
        let before = fs::read(&path).unwrap();
        assert!(crate::cmd_status("alpha".into(), "invalid".into()).is_err());
        assert!(crate::cmd_note("missing".into(), "must not save".into()).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    });
}

#[test]
fn cli_add_and_remove_roots_are_idempotent_and_preserve_notes() {
    let (dir, app) = fixture();
    config::with_test_config_path(dir.path().join("config.toml"), || {
        let mut initial = app.config.clone();
        initial.notes.insert("keep".into(), "unchanged".into());
        config::save_config(&initial).unwrap();
        let extra = dir.path().join("extra").to_string_lossy().into_owned();
        crate::cmd_add_root(extra.clone()).unwrap();
        crate::cmd_add_root(extra.clone()).unwrap();
        assert_eq!(config::load_config().unwrap().roots.len(), 2);
        crate::cmd_remove_root(extra.clone()).unwrap();
        crate::cmd_remove_root(extra).unwrap();
        let saved = config::load_config().unwrap();
        assert_eq!(saved.roots, initial.roots);
        assert_eq!(saved.notes, initial.notes);
    });
}

#[test]
fn config_loading_handles_first_run_and_reports_malformed_or_unreadable_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/config.toml");
    config::with_test_config_path(path.clone(), || {
        assert!(config::load_config().unwrap().roots.is_empty());
        assert!(path.is_file());
        assert!(crate::cmd_roots().is_ok());
        fs::write(&path, "roots = [").unwrap();
        assert!(config::load_config()
            .unwrap_err()
            .to_string()
            .contains("parse"));
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(config::load_config()
            .unwrap_err()
            .to_string()
            .contains("read"));
    });
}

#[test]
fn config_test_scopes_restore_on_panic_and_are_isolated_between_threads() {
    let dir = tempfile::tempdir().unwrap();
    let outer = dir.path().join("outer.toml");
    config::with_test_config_path(outer.clone(), || {
        let result = std::panic::catch_unwind(|| {
            config::with_test_config_path(dir.path().join("inner.toml"), || panic!("test unwind"));
        });
        assert!(result.is_err());
        assert_eq!(config::config_path().unwrap(), outer);
        let other = dir.path().join("other.toml");
        std::thread::spawn(move || {
            config::with_test_config_path(other.clone(), || {
                assert_eq!(config::config_path().unwrap(), other)
            });
        })
        .join()
        .unwrap();
        assert_eq!(config::config_path().unwrap(), outer);
    });
}

#[test]
fn render_git_states_and_rich_project_details_without_panics_or_missing_data() {
    let (dir, mut app) = fixture();
    let path = dir.path().join("alpha");
    git2::Repository::init(&path).unwrap();
    let mut info = crate::git::get_git_info_fast(&path).unwrap();
    info.branch = "feature/testing".into();
    info.remote_url = Some("https://github.com/team/project.git".into());
    info.has_remote = true;
    info.upstream = Some("origin/main".into());
    info.ahead = Some(3);
    info.behind = Some(2);
    info.modified_count = Some(5);
    info.untracked_count = Some(1);
    app.projects[0].ports = vec![3000, 8080];
    app.projects[0].note = Some("rich-note".into());
    for status in [
        DirtyStatus::Unknown,
        DirtyStatus::Queued,
        DirtyStatus::Checking,
        DirtyStatus::Clean,
        DirtyStatus::Dirty,
        DirtyStatus::Error,
    ] {
        info.dirty_status = status;
        app.projects[0].git = Some(info.clone());
        scanner::recompute_project_health(&mut app.projects[0]);
        for view in [ViewMode::Compact, ViewMode::Detailed] {
            app.view_mode = view;
            let screen = render(&app, 180, 80);
            assert!(screen.contains("alpha"));
            if view == ViewMode::Detailed {
                assert!(screen.contains("rich-note"));
                assert!(screen.contains("feature/testing"));
                assert!(screen.contains("8080"));
            }
        }
    }
}

#[test]
fn activity_and_status_sorts_have_the_expected_order() {
    let (_dir, mut app) = fixture();
    let alpha = app.projects.iter_mut().find(|p| p.name == "alpha").unwrap();
    alpha.activity.timestamp = Some(100);
    alpha.status = ProjectStatus::Paused;
    let beta = app.projects.iter_mut().find(|p| p.name == "beta").unwrap();
    beta.activity.timestamp = Some(300);
    beta.status = ProjectStatus::Active;
    let gamma = app.projects.iter_mut().find(|p| p.name == "gamma").unwrap();
    gamma.activity.timestamp = Some(200);
    gamma.status = ProjectStatus::Archived;
    app.sort = SortField::Activity;
    app.apply_filter_and_sort();
    assert_eq!(names(&app), vec!["beta", "gamma", "alpha"]);
    app.sort = SortField::Status;
    app.apply_filter_and_sort();
    assert_eq!(names(&app), vec!["beta", "gamma", "alpha"]);
    app.sort = SortField::Path;
    app.apply_filter_and_sort();
    assert_eq!(names(&app), vec!["alpha", "beta", "gamma"]);
}

#[test]
fn status_navigation_clamps_and_escape_cancels_without_saving() {
    let (dir, mut app) = fixture();
    key(&mut app, KeyCode::Char('m'));
    for _ in 0..20 {
        key(&mut app, KeyCode::Down);
    }
    assert_eq!(app.status_selected, app.status_options.len() - 1);
    for _ in 0..20 {
        key(&mut app, KeyCode::Up);
    }
    assert_eq!(app.status_selected, 0);
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.editing_project_id.is_none());
    assert!(!dir.path().join("saved.toml").exists());
}

#[test]
fn unknown_open_action_and_empty_action_list_report_errors_without_launching() {
    let (_dir, mut app) = fixture();
    key(&mut app, KeyCode::Char('o'));
    key(&mut app, KeyCode::Char('!'));
    assert!(app.pending_action.is_none());
    assert!(app
        .status_message
        .as_deref()
        .unwrap()
        .contains("No open action"));
    app.config.open.actions.clear();
    key(&mut app, KeyCode::Char('o'));
    assert_eq!(app.mode, Mode::Normal);
    assert!(app
        .status_message
        .as_deref()
        .unwrap()
        .contains("No open actions"));
}

#[test]
#[cfg(unix)]
fn saved_config_starts_private_and_preserves_existing_unix_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    config::save_config_at(&Config::default(), &path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    config::save_config_at(&Config::default(), &path).unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn scanner_obeys_options_exact_depth_and_nested_workspace_members() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "mono",
        "mono/member",
        "mono/member/deep",
        ".hidden",
        "ignored",
    ] {
        write(
            dir.path(),
            &format!("{name}/Cargo.toml"),
            "[package]\nname='test'",
        );
    }
    write(dir.path(), ".gitignore", "ignored/\n");
    let mut config = Config {
        roots: vec![dir.path().to_string_lossy().into_owned()],
        max_depth: 1,
        ..Config::default()
    };
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 1);
    config.max_depth = 2;
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 2);
    assert_eq!(
        crate::discover::count_projects_under(dir.path(), 2).unwrap(),
        2
    );
    config.scan_hidden = true;
    config.respect_gitignore = false;
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 4);
    config.max_depth = 0;
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 0);
    write(dir.path(), "Cargo.toml", "[workspace]");
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 1);
}

#[test]
fn activity_uses_existing_source_file_edits_not_directory_mtime() {
    use std::time::{Duration, SystemTime};
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]\nname='test'");
    write(dir.path(), "src/nested/existing.rs", "fn main() {}");
    let future = SystemTime::now() + Duration::from_secs(60);
    fs::File::options()
        .write(true)
        .open(dir.path().join("src/nested/existing.rs"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(future))
        .unwrap();
    let result = scanner::scan_roots(&Config {
        roots: vec![dir.path().to_string_lossy().into_owned()],
        ..Config::default()
    })
    .unwrap();
    assert_eq!(
        result.projects[0].activity.last_modified_ts,
        Some(
            future
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        )
    );
}

#[test]
fn cargo_artifacts_use_bin_names_workspace_target_and_library_has_no_guessed_executable() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[workspace]\nmembers=['member']");
    write(
        dir.path(),
        ".cargo/config.toml",
        "[build]\ntarget-dir='output'",
    );
    write(
        dir.path(),
        "member/Cargo.toml",
        "[package]\nname='different-package'\nautobins=false\n[[bin]]\nname='ds'",
    );
    let artifacts =
        crate::artifacts::detect_artifacts(&dir.path().join("member"), &["Rust".into()]);
    let filename = if cfg!(windows) { "ds.exe" } else { "ds" };
    assert!(artifacts
        .iter()
        .any(|a| a.path == dir.path().join("output/release").join(filename)));
    write(dir.path(), "member/Cargo.toml", "[package]\nname='library'");
    let artifacts =
        crate::artifacts::detect_artifacts(&dir.path().join("member"), &["Rust".into()]);
    assert!(artifacts.iter().all(|a| a.kind != ArtifactKind::Executable));
}

#[test]
fn commands_keep_all_stacks_and_node_install_without_scripts() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "package.json", "{}");
    assert_eq!(
        crate::commands::detect_commands(dir.path(), &[])[0].command,
        "npm install"
    );
    write(dir.path(), "Cargo.toml", "[package]\nname='rust'");
    write(dir.path(), "go.mod", "module test");
    write(dir.path(), "docker-compose.yml", "services: {}");
    let commands = crate::commands::detect_commands(dir.path(), &[]);
    assert!(commands.len() > 6);
    assert!(commands.iter().any(|c| c.command == "docker compose down"));
}

#[test]
fn framework_detection_uses_dependencies_not_descriptions_comments_or_script_names() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "package.json",
        r#"{"name":"react","description":"vue next vite","scripts":{"express":"echo tauri"},"dependencies":{"@angular/core":"1"}}"#,
    );
    write(dir.path(), "Cargo.toml", "# tokio ratatui axum\n[package]\nname='bevy'\n[dependencies]\nrenamed={package='serde',version='1'}");
    write(
        dir.path(),
        "requirements.txt",
        "# django flask\nnot-numpy==1\nfastapi>=1",
    );
    let stack = crate::detect::detect_stack(dir.path());
    for expected in ["Angular", "Serde", "FastAPI"] {
        assert!(stack.iter().any(|s| s == expected), "{expected}: {stack:?}");
    }
    for absent in [
        "React", "Vue", "Next.js", "Vite", "Express", "Tauri", "Tokio", "Ratatui", "Axum", "Bevy",
        "Django", "Flask", "NumPy",
    ] {
        assert!(!stack.iter().any(|s| s == absent), "{absent}: {stack:?}");
    }
}

#[test]
fn background_reload_keeps_current_view_and_latest_metadata() {
    let (_dir, mut app) = fixture();
    let id = app.projects[0].id.clone();
    app.start_reload();
    app.config
        .notes
        .insert(id.clone(), "edited during scan".into());
    assert_eq!(app.projects.len(), 3);
    // A second request coalesces; neither request executes IO on this thread.
    app.needs_reload = true;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while app
        .status_message
        .as_deref()
        .is_none_or(|s| !s.starts_with("Scanned"))
    {
        app.poll_reload();
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(
        app.projects
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .note
            .as_deref(),
        Some("edited during scan")
    );
}

#[test]
fn cancelled_scan_exits_without_publishing_partial_results() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]");
    assert!(scanner::scan_roots_cancellable(
        &Config {
            roots: vec![dir.path().to_string_lossy().into_owned()],
            ..Config::default()
        },
        &std::sync::atomic::AtomicBool::new(true)
    )
    .is_err());
}

#[test]
fn health_does_not_penalize_normal_git_work_and_ignored_env_but_flags_tracked_env() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    write(dir.path(), "README.md", "docs");
    write(dir.path(), ".gitignore", ".env*\n");
    write(dir.path(), ".env.local", "never read secrets");
    write(dir.path(), ".env.example", "template");
    let mut git = crate::git::get_git_info_fast(dir.path()).unwrap();
    git.has_remote = true;
    git.branch = "main".into();
    git.upstream = Some("origin/main".into());
    git.dirty_status = DirtyStatus::Clean;
    let timestamp = Some(chrono::Utc::now().timestamp());
    let clean = crate::health::compute_health(dir.path(), &Some(git.clone()), timestamp, true);
    git.branch = "feature/work".into();
    git.dirty_status = DirtyStatus::Dirty;
    git.modified_count = Some(20);
    git.upstream = None;
    git.ahead = Some(3);
    git.behind = Some(0);
    let working = crate::health::compute_health(dir.path(), &Some(git), timestamp, true);
    assert_eq!(clean.score, working.score);
    assert!(working
        .warnings
        .contains(&crate::project::ProjectWarning::DirtyWorkingTree));
    assert!(!working
        .warnings
        .contains(&crate::project::ProjectWarning::EnvFileLocal));
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".env.local")).unwrap();
    index.write().unwrap();
    let tracked = crate::health::compute_health(dir.path(), &None, timestamp, true);
    assert!(tracked
        .warnings
        .contains(&crate::project::ProjectWarning::EnvFileLocal));
}

#[test]
fn config_rejects_ambiguous_action_keys_and_invalid_status_without_overwriting_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    config::save_config_at(&Config::default(), &path).unwrap();
    let before = fs::read(&path).unwrap();
    for key in ["", "xy", " ", "O"] {
        let mut config = Config::default();
        config.open.actions[1].key = key.into();
        assert!(config::save_config_at(&config, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
    }
    assert!(toml::from_str::<Config>("[project_status]\nproject='typo'").is_err());
    let config: Config = toml::from_str("[project_status]\nproject='paused'").unwrap();
    assert_eq!(config.project_status["project"], ProjectStatus::Paused);
    assert!(toml::to_string(&config).unwrap().contains("paused"));
}

#[test]
fn config_lock_serializes_real_concurrent_writers_without_losing_increments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    config::save_config_at(&Config::default(), &path).unwrap();
    let base = config::with_test_config_path(path.clone(), || config::load_config().unwrap());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(12));
    let threads: Vec<_> = (0..12)
        .map(|i| {
            let mut updated = base.clone();
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                updated
                    .notes
                    .insert(format!("project-{i}"), "preserved".into());
                config::record_visit(&mut updated, "shared");
                barrier.wait();
                config::save_config_at(&updated, &path).unwrap();
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let saved = config::with_test_config_path(path, || config::load_config().unwrap());
    assert_eq!(saved.notes.len(), 12);
    assert_eq!(saved.scores["shared"].visits, 12);
}

#[test]
fn migrated_history_is_not_added_twice_by_concurrent_first_saves() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "project/Cargo.toml", "[package]");
    let canonical = config::project_key(&dir.path().join("project"));
    let alias = dir
        .path()
        .join("project/../project")
        .to_string_lossy()
        .into_owned();
    let path = dir.path().join("config.toml");
    let mut initial = Config::default();
    initial.scores.insert(
        alias,
        scoring::ScoreEntry {
            visits: 7,
            opens: 3,
            last_used: Some(1),
        },
    );
    config::save_config_at(&initial, &path).unwrap();
    config::with_test_config_path(path.clone(), || {
        let mut first = config::load_config().unwrap();
        let mut second = config::load_config().unwrap();
        config::record_visit(&mut first, &canonical);
        config::record_visit(&mut second, &canonical);
        config::save_config(&first).unwrap();
        config::save_config(&second).unwrap();
        let saved = config::load_config().unwrap();
        assert_eq!(saved.scores[&canonical].visits, 9);
        assert_eq!(saved.scores[&canonical].opens, 3);
    });
}

#[test]
fn explicit_main_target_has_no_phantom_package_binary_and_marker_directories_are_not_projects() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("package.json")).unwrap();
    assert!(!scanner::is_project(dir.path()));
    write(
        dir.path(),
        "Cargo.toml",
        "[package]\nname='devscope'\n[[bin]]\nname='ds'\npath='src/main.rs'",
    );
    write(dir.path(), "src/main.rs", "fn main() {}");
    let artifacts = crate::artifacts::detect_artifacts(dir.path(), &["Rust".into()]);
    assert_eq!(
        artifacts
            .iter()
            .filter(|a| a.kind == ArtifactKind::Executable)
            .count(),
        2
    );
    assert!(artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Executable)
        .all(|a| a.path.file_stem().unwrap() == "ds"));
}

#[test]
#[cfg(unix)]
fn symlink_scan_policy_deduplicates_targets_and_does_not_recurse_forever() {
    let dir = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    write(external.path(), "Cargo.toml", "[package]");
    std::os::unix::fs::symlink(external.path(), dir.path().join("linked")).unwrap();
    let mut config = Config {
        roots: vec![dir.path().to_string_lossy().into_owned()],
        ..Config::default()
    };
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 0);
    config.follow_symlinks = true;
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 1);
    config
        .roots
        .push(external.path().to_string_lossy().into_owned());
    assert_eq!(scanner::scan_roots(&config).unwrap().projects_found, 1);
    std::os::unix::fs::symlink(external.path(), external.path().join("loop")).unwrap();
    assert!(scanner::scan_roots(&config).is_err());
}

#[test]
fn git_snapshot_counts_remote_and_local_upstreams_from_the_same_head() {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("test", "test@example.invalid").unwrap();
    let base = repo.commit(None, &sig, &sig, "base", &tree, &[]).unwrap();
    let base_commit = repo.find_commit(base).unwrap();
    repo.reference("refs/heads/main", base, true, "fixture")
        .unwrap();
    repo.set_head("refs/heads/main").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "local", &tree, &[&base_commit])
        .unwrap();
    repo.remote("origin", "https://example.invalid/team/project.git")
        .unwrap();
    repo.reference("refs/remotes/origin/main", base, true, "fixture")
        .unwrap();
    let mut branch = repo.find_branch("main", git2::BranchType::Local).unwrap();
    branch.set_upstream(Some("origin/main")).unwrap();
    let info = crate::git::get_git_info_fast(dir.path()).unwrap();
    assert_eq!((info.ahead, info.behind), (Some(1), Some(0)));
    let other = repo
        .commit(None, &sig, &sig, "remote", &tree, &[&base_commit])
        .unwrap();
    repo.reference("refs/remotes/origin/main", other, true, "fixture")
        .unwrap();
    let info = crate::git::get_git_info_fast(dir.path()).unwrap();
    assert_eq!((info.ahead, info.behind), (Some(1), Some(1)));
    repo.branch("shared", &base_commit, false).unwrap();
    branch.set_upstream(Some("shared")).unwrap();
    let info = crate::git::get_git_info_fast(dir.path()).unwrap();
    assert_eq!(info.upstream.as_deref(), Some("shared"));
    assert_eq!((info.ahead, info.behind), (Some(1), Some(0)));
}

#[test]
#[cfg(unix)]
fn tracked_env_warning_survives_a_symlink_to_the_worktree_directory() {
    let repo_dir = tempfile::tempdir().unwrap();
    let alias_dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(repo_dir.path()).unwrap();
    write(
        repo_dir.path(),
        ".env.local",
        "must never read these contents",
    );
    write(repo_dir.path(), ".gitignore", ".env*\n");
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(".env.local")).unwrap();
    index.write().unwrap();
    let alias = alias_dir.path().join("repo-alias");
    std::os::unix::fs::symlink(repo_dir.path(), &alias).unwrap();
    let health = crate::health::compute_health(&alias, &None, None, true);
    assert!(health
        .warnings
        .contains(&crate::project::ProjectWarning::EnvFileLocal));
}
