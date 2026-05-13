use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;

use crate::config::Config;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiscoveryConfidence {
    Low,
    Medium,
    High,
}

impl DiscoveryConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredRoot {
    pub path: PathBuf,
    pub project_count: usize,
    pub confidence: DiscoveryConfidence,
}

/// Normalize a path for comparison/deduplication.
/// On Windows: lowercase after canonicalization.
/// On Unix: just canonicalize.
pub fn normalize_for_compare(path: &Path) -> String {
    let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = canonical.to_string_lossy().to_string();
    if cfg!(target_os = "windows") {
        s.to_lowercase().replace('/', "\\")
    } else {
        s
    }
}

/// Build the list of common candidate root paths, deduplicated and filtered.
pub fn default_candidate_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let home = dirs_home();

    if let Some(ref home) = home {
        let names = &[
            "source",
            "Source",
            "sources",
            "Sources",
            "projects",
            "Projects",
            "project",
            "dev",
            "Dev",
            "developer",
            "workspace",
            "Workspace",
            "workspaces",
            "code",
            "Code",
            "src",
            "repos",
            "Repos",
        ];
        for name in names {
            candidates.push(home.join(name));
        }

        if cfg!(target_os = "windows") {
            candidates.push(home.join("Documents"));
            candidates.push(home.join("Documentos"));
            candidates.push(home.join("Desktop"));
            candidates.push(home.join("Escritorio"));
            candidates.push(home.join("OneDrive").join("Documents"));
            candidates.push(home.join("OneDrive").join("Documentos"));
            candidates.push(home.join("OneDrive").join("Desktop"));
            candidates.push(home.join("OneDrive").join("Escritorio"));
            candidates.push(home.join("OneDrive - Personal").join("Documents"));
            candidates.push(home.join("OneDrive - Personal").join("Documentos"));
        }
    }

    if let Some(user_dirs) = directories::UserDirs::new() {
        if let Some(doc) = user_dirs.document_dir() {
            candidates.push(doc.to_path_buf());
        }
        if let Some(desktop) = user_dirs.desktop_dir() {
            candidates.push(desktop.to_path_buf());
        }
    }

    dedupe_existing_dirs(candidates)
}

fn dirs_home() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}

/// Remove duplicates (case-insensitive on Windows) and non-existent paths.
fn dedupe_existing_dirs(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for path in paths {
        if !path.exists() || !path.is_dir() {
            continue;
        }
        if is_system_protected(&path) {
            continue;
        }
        let key = normalize_for_compare(&path);
        if seen.insert(key) {
            result.push(path);
        }
    }

    result
}

/// Check if a path is a system-protected location that should never be scanned.
fn is_system_protected(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    if cfg!(target_os = "windows") {
        let lower = path_str.to_lowercase();
        let system_drive = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".to_string())
            .to_lowercase();

        // Drive root
        let drive_root = format!("{}\\", system_drive);
        let drive_root_alt = system_drive.clone();
        if lower == drive_root || lower == drive_root_alt {
            return true;
        }

        let protected = [
            format!("{}\\windows", system_drive),
            format!("{}\\windows\\", system_drive),
            format!("{}\\program files", system_drive),
            format!("{}\\program files\\", system_drive),
            format!("{}\\program files (x86)", system_drive),
            format!("{}\\program files (x86)\\", system_drive),
            format!("{}\\programdata", system_drive),
            format!("{}\\programdata\\", system_drive),
        ];

        for p in &protected {
            if lower.starts_with(p) {
                return true;
            }
        }

        // AppData
        if lower.contains("\\appdata\\") || lower.contains("/appdata/") {
            return true;
        }
    }

    // Unix: root filesystem
    if path_str == "/" {
        return true;
    }

    false
}

/// Check if a directory should be skipped during discovery.
/// Reuses the scanner's skip list for consistency.
pub fn should_skip_dir(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        return crate::scanner::SKIP_DIRS.contains(&name);
    }
    false
}

/// Check if a directory looks like a project (has known marker files).
/// Reuses the scanner's project detection for consistency.
pub fn looks_like_project_dir(path: &Path) -> bool {
    crate::scanner::is_project(path)
}

/// Count how many projects exist under a root directory, up to max_depth.
/// Uses the `ignore` crate for efficient, .gitignore-aware walking.
pub fn count_projects_under(root: &Path, max_depth: usize) -> Result<usize> {
    let count = std::sync::Arc::new(AtomicUsize::new(0));
    let count_clone = count.clone();

    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .max_depth(Some(max_depth))
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .follow_links(false);

    builder.filter_entry(move |entry| {
        if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
            return true;
        }

        let path = entry.path();

        if looks_like_project_dir(path) {
            count_clone.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        if should_skip_dir(path) {
            return false;
        }

        true
    });

    for result in builder.build() {
        if result.is_err() {
            continue;
        }
    }

    Ok(count.load(Ordering::Relaxed))
}

/// Discover potential project roots from common candidate locations.
/// Returns ranked results: by project count (desc), then confidence, then path.
pub fn discover_roots(config: &Config) -> Result<Vec<DiscoveredRoot>> {
    let candidates = default_candidate_roots();
    let mut results = Vec::new();

    for candidate in &candidates {
        let depth = if is_high_confidence_path(candidate) {
            config.max_depth.max(4)
        } else {
            2
        };

        let count = count_projects_under(candidate, depth).unwrap_or(0);
        if count > 0 {
            results.push(DiscoveredRoot {
                path: candidate.clone(),
                project_count: count,
                confidence: determine_confidence(candidate),
            });
        }
    }

    results.sort_by(|a, b| {
        b.project_count
            .cmp(&a.project_count)
            .then_with(|| a.confidence.cmp(&b.confidence))
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(results)
}

fn is_high_confidence_path(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = name.to_lowercase();
        let dev_names = [
            "source",
            "sources",
            "projects",
            "project",
            "dev",
            "developer",
            "workspace",
            "workspaces",
            "code",
            "src",
            "repos",
        ];
        return dev_names.contains(&lower.as_str());
    }
    false
}

fn determine_confidence(path: &Path) -> DiscoveryConfidence {
    if is_high_confidence_path(path) {
        return DiscoveryConfidence::High;
    }

    if let Some(user_dirs) = directories::UserDirs::new() {
        let norm = normalize_for_compare(path);
        if let Some(doc) = user_dirs.document_dir() {
            if norm == normalize_for_compare(doc) {
                return DiscoveryConfidence::Medium;
            }
        }
        if let Some(desktop) = user_dirs.desktop_dir() {
            if norm == normalize_for_compare(desktop) {
                return DiscoveryConfidence::Medium;
            }
        }
    }

    DiscoveryConfidence::Low
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn looks_like_project_detects_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert!(looks_like_project_dir(dir.path()));
    }

    #[test]
    fn looks_like_project_detects_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        assert!(looks_like_project_dir(dir.path()));
    }

    #[test]
    fn looks_like_project_rejects_only_readme() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        assert!(!looks_like_project_dir(dir.path()));
    }

    #[test]
    fn looks_like_project_rejects_only_env() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".env"), "KEY=val").unwrap();
        assert!(!looks_like_project_dir(dir.path()));
    }

    #[test]
    fn looks_like_project_detects_dotnet_sln() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("MyApp.sln"), "").unwrap();
        assert!(looks_like_project_dir(dir.path()));
    }

    #[test]
    fn should_skip_dir_ignores_node_modules() {
        assert!(should_skip_dir(Path::new("/some/path/node_modules")));
    }

    #[test]
    fn should_skip_dir_ignores_target() {
        assert!(should_skip_dir(Path::new("/some/path/target")));
    }

    #[test]
    fn should_skip_dir_ignores_dot_git() {
        assert!(should_skip_dir(Path::new("/some/path/.git")));
    }

    #[test]
    fn should_skip_dir_allows_normal_dir() {
        assert!(!should_skip_dir(Path::new("/some/path/my_project")));
    }

    #[test]
    fn normalize_for_compare_handles_duplicates_windows_style() {
        let a = Path::new("C:\\Users\\Test\\Source");
        let b = Path::new("c:\\users\\test\\source");
        // On Windows they'd be equal after canonicalize, but canonicalize
        // fails on non-existent paths. With mock paths we test the fallback.
        let na = normalize_for_compare(a);
        let nb = normalize_for_compare(b);
        if cfg!(target_os = "windows") {
            assert_eq!(na, nb);
        }
    }

    #[test]
    fn discover_does_not_panic_on_empty_config() {
        let config = Config::default();
        let result = discover_roots(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn default_candidate_roots_includes_home_dev() {
        let roots = default_candidate_roots();
        // HOME variable is always set in tests
        if let Some(home) = dirs_home() {
            let dev_path = home.join("dev");
            if dev_path.exists() && dev_path.is_dir() {
                assert!(roots
                    .iter()
                    .any(|r| { normalize_for_compare(r) == normalize_for_compare(&dev_path) }));
            }
        }
    }

    #[test]
    fn count_projects_under_finds_project() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("my-app");
        fs::create_dir(&proj).unwrap();
        fs::write(proj.join("package.json"), "{}").unwrap();
        let count = count_projects_under(dir.path(), 4).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn count_projects_under_skips_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("my-app");
        fs::create_dir(&proj).unwrap();
        fs::write(proj.join("package.json"), "{}").unwrap();
        let nm = proj.join("node_modules");
        fs::create_dir(&nm).unwrap();
        fs::write(nm.join("package.json"), "{}").unwrap();
        // node_modules is skipped, should only count my-app
        let count = count_projects_under(dir.path(), 4).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn is_system_protected_rejects_windows_path() {
        if cfg!(target_os = "windows") {
            assert!(is_system_protected(Path::new("C:\\Windows")));
            assert!(is_system_protected(Path::new("C:\\Program Files")));
            assert!(is_system_protected(Path::new("C:\\Program Files (x86)")));
            assert!(is_system_protected(Path::new("C:\\ProgramData")));
            // AppData check
            assert!(is_system_protected(Path::new(
                "C:\\Users\\test\\AppData\\Local"
            )));
        }
    }

    #[test]
    fn is_system_protected_rejects_root() {
        if cfg!(target_os = "windows") {
            let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
            assert!(is_system_protected(Path::new(&format!("{}\\", drive))));
        }
    }

    #[test]
    fn confidence_high_for_dev_paths() {
        let p = Path::new("/home/user/dev");
        assert_eq!(determine_confidence(p), DiscoveryConfidence::High);
        let p = Path::new("/home/user/source");
        assert_eq!(determine_confidence(p), DiscoveryConfidence::High);
        let p = Path::new("/home/user/projects");
        assert_eq!(determine_confidence(p), DiscoveryConfidence::High);
    }

    #[test]
    fn confidence_low_for_unknown() {
        let p = Path::new("/home/user/music");
        assert_eq!(determine_confidence(p), DiscoveryConfidence::Low);
    }
}
