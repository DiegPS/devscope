use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::config::{expand_tilde, normalize_path, Config};
use crate::detect;
use crate::git;
use crate::project::{ActivityInfo, DirtyStatus, Project, ProjectStatus};

/// Directories to skip during scanning.
pub(crate) const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".dart_tool",
    ".flutter-plugins",
    ".flutter-plugins-dependencies",
    ".idea",
    ".vscode",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    "env",
    ".gradle",
    ".mvn",
    "coverage",
    ".turbo",
    ".cache",
    ".pytest_cache",
    ".pub-cache",
    "Pods",
    ".fvm",
    ".tox",
    "egg-info",
    ".eggs",
    "bower_components",
    "bin",
    "obj",
    "DerivedData",
];

/// Files that indicate a project root.
pub(crate) const PROJECT_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "package-lock.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "requirements.txt",
    "Pipfile",
    "poetry.lock",
    "pubspec.yaml",
    "composer.json",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "CMakeLists.txt",
    "Makefile",
    "Gemfile",
    "Package.swift",
    "deno.json",
    "deno.jsonc",
    "setup.py",
    "setup.cfg",
];

/// Result of a scan operation.
pub struct ScanResult {
    pub projects: Vec<Project>,
    pub duration_ms: u128,
    pub projects_found: usize,
}

/// Scan all configured roots and return detected projects.
pub fn scan_roots(config: &Config) -> Result<ScanResult> {
    let start = Instant::now();

    let mut all_projects = Vec::new();
    let mut visited = HashSet::new();

    for root_str in &config.roots {
        let root = normalize_path(&expand_tilde(root_str));
        if !root.exists() {
            eprintln!("Warning: root path does not exist: {}", root.display());
            continue;
        }

        let projects = scan_single_root(&root, config, &mut visited)?;
        all_projects.extend(projects);
    }

    // Apply user-configured status and notes
    for project in &mut all_projects {
        let path_str = project.path.to_string_lossy().to_string();

        if let Some(status) = crate::config::get_project_status(config, &path_str) {
            project.status = status;
        }

        if let Some(note) = crate::config::get_note(config, &path_str) {
            project.note = Some(note);
        }
    }

    let duration_ms = start.elapsed().as_millis();
    let count = all_projects.len();

    Ok(ScanResult {
        projects: all_projects,
        duration_ms,
        projects_found: count,
    })
}

/// Rehydrate expensive git working-tree status for already scanned projects.
/// Intended for CLI flows where correctness matters more than first-paint
/// latency.
pub fn hydrate_git_statuses(projects: &mut [Project]) {
    projects.par_iter_mut().for_each(hydrate_project_git_status);
}

/// Recompute derived health fields after git state changes.
pub fn recompute_project_health(project: &mut Project) {
    let health = crate::health::compute_health(
        &project.path,
        &project.git,
        project.activity.timestamp,
        !project.commands.is_empty(),
    );
    project.warnings = health.warnings.clone();
    project.health = health;
}

fn scan_single_root(
    root: &Path,
    config: &Config,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<Project>> {
    // Check if the root itself is a project
    let mut projects = Vec::new();

    if is_project(root) {
        if let Some(project) = analyze_project(root, config) {
            visited.insert(root.to_path_buf());
            projects.push(project);
            // If root is a project, still scan subdirectories for monorepo-like structures
        }
    }

    // Walk subdirectories
    let subdirs = collect_subdirs(root, config.max_depth, visited);
    let new_projects: Vec<Project> = subdirs
        .par_iter()
        .filter_map(|path| analyze_project(path, config))
        .collect();

    projects.extend(new_projects);
    Ok(projects)
}

fn collect_subdirs(root: &Path, max_depth: usize, visited: &mut HashSet<PathBuf>) -> Vec<PathBuf> {
    let root = root.to_path_buf();
    let visited_snapshot = visited.clone();
    let discovered = Arc::new(Mutex::new(HashSet::new()));
    let output = Arc::new(Mutex::new(Vec::new()));

    let mut builder = WalkBuilder::new(&root);
    builder
        .max_depth(Some(max_depth.saturating_add(1)))
        .hidden(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);

    builder.filter_entry({
        let root = root.clone();
        let discovered = Arc::clone(&discovered);
        let output = Arc::clone(&output);
        move |entry| {
        if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
            return true;
        }

        let path = entry.path();
        if path == root {
            return true;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();

        if name.starts_with('.') && name != ".git" {
            return false;
        }

        if SKIP_DIRS.contains(&name.as_ref()) {
            return false;
        }

        if visited_snapshot.contains(path)
            || discovered
                .lock()
                .expect("project-discovery mutex poisoned")
                .contains(path)
        {
            return false;
        }

        if is_project(path) {
            let path_buf = path.to_path_buf();
            discovered
                .lock()
                .expect("project-discovery mutex poisoned")
                .insert(path_buf.clone());
            output
                .lock()
                .expect("project-output mutex poisoned")
                .push(path_buf);
            return false;
        }

        true
    }});

    for entry in builder.build() {
        if entry.is_err() {
            continue;
        }
    }

    let dirs = output
        .lock()
        .expect("project-output mutex poisoned")
        .clone();
    visited.extend(dirs.iter().cloned());
    dirs
}

fn project_markers_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| PROJECT_MARKERS.iter().copied().collect())
}

/// Check if a directory is a project by looking for marker files.
/// Uses a single `read_dir()` syscall instead of one per marker (28 → 1).
pub(crate) fn is_project(dir: &Path) -> bool {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    let markers = project_markers_set();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if markers.contains(name_str.as_ref()) {
            return true;
        }

        if (name_str.ends_with(".sln") || name_str.ends_with(".csproj"))
            && entry.file_type().is_ok_and(|ft| ft.is_file())
        {
            return true;
        }
    }

    false
}

/// Analyze a project directory and create a Project struct.
fn analyze_project(path: &Path, config: &Config) -> Option<Project> {
    if !path.exists() || !path.is_dir() {
        return None;
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let path_str = path.to_string_lossy().to_string();

    // Detect stack
    let stack = detect::detect_stack(path);

    // Detect package manager
    let manager = detect::detect_manager(path);

    // Detect scripts
    let scripts = detect::detect_scripts(path);

    // Git info (fast — no working tree scan)
    let git_info = if git::is_git_repo(path) {
        git::get_git_info_fast(path).ok()
    } else {
        None
    };

    // Activity info
    let activity = calculate_activity(path, &git_info);

    // Detect commands
    let commands = crate::commands::detect_commands(path, &stack);

    // Detect build artifacts
    let artifacts = crate::artifacts::detect_artifacts(path, &stack);

    // Compute health (includes warnings)
    let health =
        crate::health::compute_health(path, &git_info, activity.timestamp, !commands.is_empty());
    let warnings = health.warnings.clone();

    // Status (will be overridden by config if set)
    let status = determine_status(&activity, &git_info);

    // Note from config
    let note = config.notes.get(&path_str).cloned();

    Some(Project {
        id: path_str.clone(),
        name,
        path: path.to_path_buf(),
        stack,
        manager,
        scripts,
        git: git_info,
        activity,
        status,
        note,
        warnings,
        commands,
        health,
        artifacts,
        ports: Vec::new(),
    })
}

fn hydrate_project_git_status(project: &mut Project) {
    if project.git.is_none() {
        return;
    }

    let (dirty_status, modified_count, untracked_count) = crate::git::get_git_status(&project.path)
        .unwrap_or((DirtyStatus::Error, None, None));

    if let Some(git) = project.git.as_mut() {
        git.dirty_status = dirty_status;
        git.modified_count = modified_count;
        git.untracked_count = untracked_count;
    }

    recompute_project_health(project);
}

/// Calculate activity information for a project.
fn calculate_activity(path: &Path, git_info: &Option<crate::project::GitInfo>) -> ActivityInfo {
    // Find the most recent modification time of relevant files
    let last_modified_ts = find_last_modified(path);

    // Use git activity if available
    let last_git_activity_ts = git_info.as_ref().and_then(|g| g.last_commit_timestamp);
    let timestamp = match (last_modified_ts, last_git_activity_ts) {
        (Some(local), Some(git)) => Some(local.max(git)),
        (Some(local), None) => Some(local),
        (None, Some(git)) => Some(git),
        (None, None) => None,
    };

    ActivityInfo {
        last_modified_ts,
        last_git_activity_ts,
        timestamp,
    }
}

/// Find the most recent modification time of relevant files in a project.
fn find_last_modified(path: &Path) -> Option<i64> {
    let mut latest: Option<std::time::SystemTime> = None;

    let relevant_files = [
        "package.json",
        "Cargo.toml",
        "pubspec.yaml",
        "go.mod",
        "pyproject.toml",
        "requirements.txt",
        "Dockerfile",
        "docker-compose.yml",
        "README.md",
        "src",
        "lib",
        "app",
    ];

    for file in &relevant_files {
        let file_path = path.join(file);
        if let Ok(metadata) = std::fs::metadata(&file_path) {
            if let Ok(modified) = metadata.modified() {
                if latest.is_none() || modified > latest.unwrap() {
                    latest = Some(modified);
                }
            }
        }
    }

    // Also check top-level source files
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.ends_with(".rs")
                || file_name.ends_with(".js")
                || file_name.ends_with(".ts")
                || file_name.ends_with(".py")
                || file_name.ends_with(".go")
                || file_name.ends_with(".dart")
            {
                if let Ok(metadata) = std::fs::metadata(entry.path()) {
                    if let Ok(modified) = metadata.modified() {
                        if latest.is_none() || modified > latest.unwrap() {
                            latest = Some(modified);
                        }
                    }
                }
            }
        }
    }

    latest.and_then(system_time_to_timestamp)
}

/// Determine project status based on activity.
fn determine_status(
    activity: &ActivityInfo,
    git_info: &Option<crate::project::GitInfo>,
) -> ProjectStatus {
    // If we have a timestamp, check how recent
    if let Some(ts) = activity.timestamp {
        let now = chrono::Utc::now().timestamp();
        let days_since = (now - ts) / 86400;

        if days_since < 7 {
            return ProjectStatus::Active;
        }
        if days_since < 30 {
            return ProjectStatus::Paused;
        }
        if days_since < 90 {
            return ProjectStatus::Stale;
        }
        return ProjectStatus::Archived;
    }

    // If git has recent activity
    if let Some(git) = git_info {
        if !git.last_commit_date.is_empty() {
            return ProjectStatus::Active;
        }
    }

    ProjectStatus::Unknown
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::{Repository, Signature};

    use super::*;
    use crate::project::{ActivityInfo, DirtyStatus, HealthLevel, ProjectHealth};

    #[test]
    fn hydrate_git_statuses_updates_dirty_state_and_health() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        fs::write(dir.path().join("README.md"), "# test").unwrap();
        fs::write(dir.path().join(".gitignore"), "target").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(dir.path().join("tracked.txt"), "v1").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("tracked.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::now("devscope", "devscope@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        fs::write(dir.path().join("tracked.txt"), "v2").unwrap();

        let git = crate::git::get_git_info_fast(dir.path()).unwrap();
        let initial_health = crate::health::compute_health(
            dir.path(),
            &Some(git.clone()),
            Some(chrono::Utc::now().timestamp()),
            true,
        );

        let mut project = Project {
            id: dir.path().to_string_lossy().to_string(),
            name: "test".to_string(),
            path: dir.path().to_path_buf(),
            stack: vec!["Rust".to_string()],
            manager: Some("cargo".to_string()),
            scripts: Vec::new(),
            git: Some(git),
            activity: ActivityInfo {
                last_modified_ts: None,
                last_git_activity_ts: None,
                timestamp: Some(chrono::Utc::now().timestamp()),
            },
            status: ProjectStatus::Active,
            note: None,
            warnings: initial_health.warnings.clone(),
            commands: vec![crate::commands::ProjectCommand {
                label: "run".to_string(),
                command: "cargo run".to_string(),
                kind: crate::commands::ProjectCommandKind::Start,
            }],
            health: ProjectHealth {
                score: initial_health.score,
                level: initial_health.level.clone(),
                positives: initial_health.positives.clone(),
                warnings: initial_health.warnings.clone(),
            },
            artifacts: Vec::new(),
            ports: Vec::new(),
        };

        let before_score = project.health.score;

        hydrate_git_statuses(std::slice::from_mut(&mut project));

        let git = project.git.as_ref().unwrap();
        assert_eq!(git.dirty_status, DirtyStatus::Dirty);
        assert_eq!(git.modified_count, Some(1));
        assert!(project.health.score < before_score);
        assert!(matches!(
            project.health.level,
            HealthLevel::Good | HealthLevel::Warn | HealthLevel::Bad
        ));
    }

    #[test]
    fn determine_status_uses_all_status_buckets() {
        let now = chrono::Utc::now().timestamp();

        let active = ActivityInfo {
            last_modified_ts: None,
            last_git_activity_ts: None,
            timestamp: Some(now - (3 * 86_400)),
        };
        let paused = ActivityInfo {
            last_modified_ts: None,
            last_git_activity_ts: None,
            timestamp: Some(now - (20 * 86_400)),
        };
        let stale = ActivityInfo {
            last_modified_ts: None,
            last_git_activity_ts: None,
            timestamp: Some(now - (60 * 86_400)),
        };
        let archived = ActivityInfo {
            last_modified_ts: None,
            last_git_activity_ts: None,
            timestamp: Some(now - (180 * 86_400)),
        };

        assert_eq!(determine_status(&active, &None), ProjectStatus::Active);
        assert_eq!(determine_status(&paused, &None), ProjectStatus::Paused);
        assert_eq!(determine_status(&stale, &None), ProjectStatus::Stale);
        assert_eq!(determine_status(&archived, &None), ProjectStatus::Archived);
    }
}

fn system_time_to_timestamp(time: std::time::SystemTime) -> Option<i64> {
    time.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64)
}
