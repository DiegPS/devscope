use std::path::Path;

use crate::git;
use crate::project::{DirtyStatus, GitInfo, HealthLevel, ProjectHealth, ProjectWarning};

/// Calculate a health score (0-100) and collect warnings/positives.
pub fn compute_health(
    path: &Path,
    git: &Option<GitInfo>,
    activity_timestamp: Option<i64>,
    has_commands: bool,
) -> ProjectHealth {
    let mut score: i32 = 100;
    let mut positives = Vec::new();

    // ── README ──────────────────────────────────────────────────────
    let has_readme = path.join("README.md").exists()
        || path.join("readme.md").exists()
        || path.join("README").exists();
    if has_readme {
        positives.push("README".to_string());
    }

    // ── .gitignore ──────────────────────────────────────────────────
    let has_gitignore = path.join(".gitignore").exists();
    if has_gitignore {
        positives.push(".gitignore".to_string());
    }

    // ── Env files (detect existence, never read) ────────────────────
    let env_files = detect_env_files(path);

    // ── Commands ────────────────────────────────────────────────────
    if has_commands {
        positives.push("commands".to_string());
    }

    // ── Git info ────────────────────────────────────────────────────
    if let Some(git_info) = git {
        if git_info.has_remote {
            positives.push("remote".to_string());
        }
        if git_info.dirty_status == DirtyStatus::Clean {
            positives.push("clean".to_string());
        }
        if git::is_mainline_branch(&git_info.branch) {
            positives.push("mainline".to_string());
        }
    }

    // ── Deduct points ───────────────────────────────────────────────
    let mut warnings = Vec::new();

    if !has_readme {
        score -= 10;
        warnings.push(ProjectWarning::NoReadme);
    }

    if !has_gitignore {
        score -= 8;
        warnings.push(ProjectWarning::NoGitignore);
    }

    // Env warnings
    for env_warn in &env_files {
        score -= match env_warn {
            ProjectWarning::EnvFileProduction => 10,
            ProjectWarning::EnvFileLocal => 8,
            ProjectWarning::EnvFilePresent => 8,
            _ => 5,
        };
        warnings.push(env_warn.clone());
    }

    if !has_commands {
        score -= 5;
        warnings.push(ProjectWarning::NoCommands);
    }

    // Git-specific deductions
    if let Some(git_info) = git {
        if !git_info.has_remote {
            score -= 12;
            warnings.push(ProjectWarning::NoRemote);
        }

        if git_info.dirty_status == DirtyStatus::Dirty {
            score -= 12;
            warnings.push(ProjectWarning::DirtyWorkingTree);
            let total =
                git_info.modified_count.unwrap_or(0) + git_info.untracked_count.unwrap_or(0);
            if total > 10 {
                score -= 8;
                warnings.push(ProjectWarning::ManyUncommittedChanges(total));
            }
        }

        if git_info.upstream.is_none() && git_info.has_remote {
            score -= 5;
            warnings.push(ProjectWarning::NoUpstream);
        }

        if !git::is_mainline_branch(&git_info.branch) {
            score -= 7;
            warnings.push(ProjectWarning::NonMainlineBranch(git_info.branch.clone()));
        }

        // Ahead/behind
        match (git_info.ahead, git_info.behind) {
            (Some(a), Some(b)) if a > 0 && b > 0 => {
                score -= 5;
                warnings.push(ProjectWarning::BranchDiverged);
            }
            (Some(a), _) if a > 0 => {
                score -= 3;
                warnings.push(ProjectWarning::BranchAhead);
            }
            (_, Some(b)) if b > 0 => {
                score -= 3;
                warnings.push(ProjectWarning::BranchBehind);
            }
            _ => {}
        }

        // Stale branch detection via commit date
        if let Some(ts) = activity_timestamp {
            let now = chrono::Utc::now().timestamp();
            let days = (now - ts) / 86400;
            if days > 90 {
                score -= 10;
                warnings.push(ProjectWarning::StaleBranch);
            }
        }
    } else {
        score -= 5;
        warnings.push(ProjectWarning::NoGit);
    }

    // Activity staleness
    if let Some(ts) = activity_timestamp {
        let now = chrono::Utc::now().timestamp();
        let days = (now - ts) / 86400;
        if days > 90 {
            score -= 10;
            if !warnings.contains(&ProjectWarning::StaleBranch) {
                warnings.push(ProjectWarning::LowActivity);
            }
        }
    }

    // Mixed lockfiles
    if let Some(mgr) = detect_mixed_lockfiles(path) {
        score -= 15;
        warnings.push(ProjectWarning::MixedLockfiles(mgr));
    }

    // Clamp score
    let score = score.clamp(0, 100) as u8;

    let level = match score {
        0..=49 => HealthLevel::Bad,
        50..=79 => HealthLevel::Warn,
        _ => HealthLevel::Good,
    };

    ProjectHealth {
        score,
        level,
        positives,
        warnings,
    }
}

/// Detect .env files by checking existence only (never read contents).
fn detect_env_files(path: &Path) -> Vec<ProjectWarning> {
    let mut found = Vec::new();

    if path.join(".env").exists() {
        found.push(ProjectWarning::EnvFilePresent);
    }
    if path.join(".env.local").exists() {
        found.push(ProjectWarning::EnvFileLocal);
    }
    if path.join(".env.production").exists() {
        found.push(ProjectWarning::EnvFileProduction);
    }

    // Check for other .env.* files
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(".env.")
                && name_str != ".env.local"
                && name_str != ".env.production"
            {
                let suffix = name_str.strip_prefix(".env.").unwrap_or(&name_str);
                if !["local", "production"].contains(&suffix) {
                    found.push(ProjectWarning::EnvFileCustom(suffix.to_string()));
                }
            }
        }
    }

    found
}

/// Detect mixed package manager lockfiles in Node projects.
fn detect_mixed_lockfiles(path: &Path) -> Option<String> {
    let package_json = path.join("package.json");
    if !package_json.exists() {
        return None;
    }

    let lockfiles = [
        ("npm", "package-lock.json"),
        ("pnpm", "pnpm-lock.yaml"),
        ("yarn", "yarn.lock"),
        ("bun1", "bun.lock"),
        ("bun2", "bun.lockb"),
    ];

    let mut found: Vec<&str> = Vec::new();
    for (name, file) in &lockfiles {
        if path.join(file).exists() {
            let normalized = if name.starts_with("bun") {
                "bun"
            } else {
                *name
            };
            if !found.contains(&normalized) {
                found.push(normalized);
            }
        }
    }

    if found.len() > 1 {
        Some(found.join(" + "))
    } else {
        None
    }
}

/// Build a compact git label for the table column.
pub fn format_git_label(git: &GitInfo) -> String {
    let mut label = git.branch.clone();

    if git.dirty_status == DirtyStatus::Dirty {
        label.push('*');
    }

    match (git.ahead, git.behind) {
        (Some(a), Some(b)) if a > 0 && b > 0 => {
            label.push_str(&format!(" ↑{}↓{}", a, b));
        }
        (Some(a), _) if a > 0 => {
            label.push_str(&format!(" ↑{}", a));
        }
        (_, Some(b)) if b > 0 => {
            label.push_str(&format!(" ↓{}", b));
        }
        _ => {}
    }

    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn good_project_high_score() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        fs::write(dir.path().join(".gitignore"), "target").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let ts = Some(chrono::Utc::now().timestamp());
        let health = compute_health(dir.path(), &None, ts, true);
        assert!(health.score >= 80);
        assert_eq!(health.level, HealthLevel::Good);
    }

    #[test]
    fn no_readme_lower_score() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "target").unwrap();
        let health = compute_health(dir.path(), &None, None, false);
        assert!(health.score < 100);
        assert!(health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::NoReadme)));
    }

    #[test]
    fn env_local_warning_no_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".env.local"), "").unwrap();
        let health = compute_health(dir.path(), &None, None, false);
        assert!(health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::EnvFileLocal)));
    }

    #[test]
    fn mixed_lockfiles_warning() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("package-lock.json"), "").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        let health = compute_health(dir.path(), &None, None, false);
        assert!(health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::MixedLockfiles(_))));
    }

    #[test]
    fn bun_lockfiles_not_mixed() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("bun.lock"), "").unwrap();
        fs::write(dir.path().join("bun.lockb"), "").unwrap();
        let health = compute_health(dir.path(), &None, None, false);
        assert!(!health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::MixedLockfiles(_))));
    }

    #[test]
    fn score_never_below_zero_or_above_100() {
        let dir = tempfile::tempdir().unwrap();
        let health = compute_health(dir.path(), &None, None, false);
        // score is u8, can't go below 0
        assert!(health.score <= 100);
    }

    #[test]
    fn health_level_mapping() {
        assert_eq!(compute_score_level(90), HealthLevel::Good);
        assert_eq!(compute_score_level(70), HealthLevel::Warn);
        assert_eq!(compute_score_level(40), HealthLevel::Bad);
    }

    fn compute_score_level(s: u8) -> HealthLevel {
        match s {
            0..=49 => HealthLevel::Bad,
            50..=79 => HealthLevel::Warn,
            _ => HealthLevel::Good,
        }
    }

    #[test]
    fn format_git_label_dirty_ahead_behind() {
        let git = GitInfo {
            branch: "main".into(),
            last_commit_hash: "abc".into(),
            last_commit_message: "msg".into(),
            last_commit_date: "2024".into(),
            last_commit_timestamp: Some(1_700_000_000),
            dirty_status: DirtyStatus::Dirty,
            modified_count: Some(1),
            untracked_count: Some(0),
            remote_url: None,
            upstream: None,
            ahead: Some(2),
            behind: Some(1),
            has_remote: true,
            remote_host: None,
            remote_repo: None,
        };
        assert_eq!(format_git_label(&git), "main* ↑2↓1");
    }

    #[test]
    fn dirty_repo_penalties_are_visible_in_warnings() {
        let git = GitInfo {
            branch: "main".into(),
            last_commit_hash: "abc".into(),
            last_commit_message: "msg".into(),
            last_commit_date: "2024".into(),
            last_commit_timestamp: Some(1_700_000_000),
            dirty_status: DirtyStatus::Dirty,
            modified_count: Some(6),
            untracked_count: Some(5),
            remote_url: None,
            upstream: None,
            ahead: None,
            behind: None,
            has_remote: false,
            remote_host: None,
            remote_repo: None,
        };

        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        fs::write(dir.path().join(".gitignore"), "target").unwrap();

        let health = compute_health(
            dir.path(),
            &Some(git),
            Some(chrono::Utc::now().timestamp()),
            true,
        );

        assert!(health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::DirtyWorkingTree)));
        assert!(health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::ManyUncommittedChanges(11))));
        assert!(health
            .warnings
            .iter()
            .any(|w| matches!(w, ProjectWarning::NoRemote)));
        assert_eq!(health.score, 68);
    }

    #[test]
    fn is_mainline_branch_true() {
        assert!(git::is_mainline_branch("main"));
        assert!(git::is_mainline_branch("master"));
        assert!(git::is_mainline_branch("develop"));
        assert!(git::is_mainline_branch("dev"));
    }

    #[test]
    fn is_mainline_branch_false() {
        assert!(!git::is_mainline_branch("feature/x"));
        assert!(!git::is_mainline_branch("fix/bug"));
    }
}
