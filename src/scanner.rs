use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use rayon::prelude::*;

use crate::config::{expand_tilde, normalize_path, Config};
use crate::detect;
use crate::git;
use crate::project::{ActivityInfo, Project, ProjectStatus};

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
    let subdirs = collect_subdirs(root, config.max_depth, visited)?;
    let new_projects: Vec<Project> = subdirs
        .par_iter()
        .filter_map(|path| analyze_project(path, config))
        .collect();

    projects.extend(new_projects);
    Ok(projects)
}

fn collect_subdirs(
    root: &Path,
    max_depth: usize,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    collect_subdirs_recursive(root, 0, max_depth, visited, &mut dirs);
    Ok(dirs)
}

fn collect_subdirs_recursive(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    visited: &mut HashSet<PathBuf>,
    output: &mut Vec<PathBuf>,
) {
    if current_depth > max_depth {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip hidden directories unless configured
        if name.starts_with('.') && name != ".git" {
            continue;
        }

        // Skip known heavy directories
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }

        if visited.contains(&path) {
            continue;
        }

        // Check if this is a project
        if is_project(&path) {
            visited.insert(path.clone());
            output.push(path.clone());
            // Don't recurse deeper into detected projects (except for monorepo support TODO)
            continue;
        }

        // Recurse into non-project directories
        collect_subdirs_recursive(&path, current_depth + 1, max_depth, visited, output);
    }
}

/// Check if a directory is a project by looking for marker files.
pub(crate) fn is_project(dir: &Path) -> bool {
    // Exact-filename markers
    if PROJECT_MARKERS
        .iter()
        .any(|marker| dir.join(marker).exists())
    {
        return true;
    }

    // Extension-based markers (*.sln, *.csproj)
    const EXT_MARKERS: &[&str] = &[".sln", ".csproj"];

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if EXT_MARKERS.iter().any(|ext| name_str.ends_with(ext))
                && entry.file_type().is_ok_and(|ft| ft.is_file())
            {
                return true;
            }
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

    // Git info
    let git_info = if git::is_git_repo(path) {
        git::get_git_info(path).ok()
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
    })
}

/// Calculate activity information for a project.
fn calculate_activity(path: &Path, git_info: &Option<crate::project::GitInfo>) -> ActivityInfo {
    // Find the most recent modification time of relevant files
    let last_modified = find_last_modified(path);

    // Use git activity if available
    let last_git_activity = git_info
        .as_ref()
        .map(|g| g.last_commit_date.clone())
        .filter(|d| !d.is_empty());

    // Determine the most recent activity
    let most_recent = match (&last_modified, &last_git_activity) {
        (Some(local), Some(git)) => {
            if local >= git {
                local.clone()
            } else {
                git.clone()
            }
        }
        (Some(local), None) => local.clone(),
        (None, Some(git)) => git.clone(),
        (None, None) => String::new(),
    };

    let relative_time = if most_recent.is_empty() {
        "unknown".to_string()
    } else {
        calculate_relative_time(&most_recent)
    };

    let timestamp = parse_date_to_timestamp(&most_recent);

    ActivityInfo {
        last_modified: last_modified.filter(|_| false), // Don't expose raw in MVP
        last_git_activity,
        relative_time,
        timestamp,
    }
}

/// Find the most recent modification time of relevant files in a project.
fn find_last_modified(path: &Path) -> Option<String> {
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

    latest.map(|t| {
        chrono::DateTime::<chrono::Utc>::from(t)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    })
}

/// Calculate a human-readable relative time string.
fn calculate_relative_time(date_str: &str) -> String {
    let now = chrono::Utc::now();

    // Try parsing common date formats
    let parsed = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M")
        .ok()
        .map(|naive| {
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc)
        });

    let Some(dt) = parsed else {
        return "unknown".to_string();
    };

    let duration = now.signed_duration_since(dt);

    if duration.num_minutes() < 1 {
        return "now".to_string();
    }
    if duration.num_minutes() < 60 {
        return format!("{}m", duration.num_minutes());
    }
    if duration.num_hours() < 24 {
        return format!("{}h", duration.num_hours());
    }
    if duration.num_days() < 30 {
        return format!("{}d", duration.num_days());
    }
    if duration.num_days() < 365 {
        return format!("{}mo", duration.num_days() / 30);
    }
    format!("{}y", duration.num_days() / 365)
}

fn parse_date_to_timestamp(date_str: &str) -> Option<i64> {
    chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M")
        .ok()
        .map(|naive| naive.and_utc().timestamp())
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
            return ProjectStatus::Active;
        }
        if days_since < 90 {
            return ProjectStatus::Stale;
        }
        return ProjectStatus::Stale;
    }

    // If git has recent activity
    if let Some(git) = git_info {
        if !git.last_commit_date.is_empty() {
            return ProjectStatus::Active;
        }
    }

    ProjectStatus::Unknown
}
