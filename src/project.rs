use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::commands::ProjectCommand;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactKind {
    Executable,
    Folder,
    Apk,
    Web,
    Bundle,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectArtifact {
    pub label: String,
    pub path: PathBuf,
    pub kind: ArtifactKind,
    pub exists: bool,
}

impl ProjectArtifact {
    pub fn new(label: &str, path: PathBuf, kind: ArtifactKind) -> Self {
        let exists = path.exists();
        Self {
            label: label.to_string(),
            path,
            kind,
            exists,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectStatus {
    Active,
    Paused,
    Stale,
    Archived,
    Unknown,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Stale => "stale",
            Self::Archived => "archived",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "stale" => Self::Stale,
            "archived" => Self::Archived,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DirtyStatus {
    Unknown,
    Queued,
    Checking,
    Clean,
    Dirty,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub last_commit_hash: String,
    pub last_commit_message: String,
    pub last_commit_date: String,
    pub last_commit_timestamp: Option<i64>,
    pub dirty_status: DirtyStatus,
    pub modified_count: Option<usize>,
    pub untracked_count: Option<usize>,
    pub remote_url: Option<String>,
    pub upstream: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub has_remote: bool,
    pub remote_host: Option<String>,
    pub remote_repo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInfo {
    pub last_modified_ts: Option<i64>,
    pub last_git_activity_ts: Option<i64>,
    pub timestamp: Option<i64>,
}

impl ActivityInfo {
    pub fn relative_time(&self) -> String {
        self.timestamp
            .map(format_relative_time)
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn last_git_activity_display(&self) -> Option<String> {
        self.last_git_activity_ts.map(format_timestamp)
    }
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn format_relative_time(ts: i64) -> String {
    let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) else {
        return "unknown".to_string();
    };

    let duration = Utc::now().signed_duration_since(dt);

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectWarning {
    EnvFilePresent,
    EnvFileLocal,
    EnvFileProduction,
    EnvFileCustom(String),
    LargeFileDetected,
    NoReadme,
    NoGitignore,
    OutdatedDependencies,
    NoRemote,
    NoUpstream,
    DirtyWorkingTree,
    ManyUncommittedChanges(usize),
    BranchAhead,
    BranchBehind,
    BranchDiverged,
    NonMainlineBranch(String),
    StaleBranch,
    MixedLockfiles(String),
    NoGit,
    LowActivity,
    NoCommands,
}

impl ProjectWarning {
    pub fn as_str(&self) -> String {
        match self {
            Self::EnvFilePresent => ".env present".to_string(),
            Self::EnvFileLocal => ".env.local present".to_string(),
            Self::EnvFileProduction => ".env.production present".to_string(),
            Self::EnvFileCustom(name) => format!(".env.{} present", name),
            Self::LargeFileDetected => "large file detected".to_string(),
            Self::NoReadme => "no README".to_string(),
            Self::NoGitignore => "no .gitignore".to_string(),
            Self::OutdatedDependencies => "outdated deps".to_string(),
            Self::NoRemote => "no git remote".to_string(),
            Self::NoUpstream => "no upstream branch".to_string(),
            Self::DirtyWorkingTree => "working tree has uncommitted changes".to_string(),
            Self::ManyUncommittedChanges(total) => {
                format!("many uncommitted files: {}", total)
            }
            Self::BranchAhead => "branch ahead of upstream".to_string(),
            Self::BranchBehind => "branch behind upstream".to_string(),
            Self::BranchDiverged => "branch diverged from upstream".to_string(),
            Self::NonMainlineBranch(b) => format!("branch '{}' not main/master/develop", b),
            Self::StaleBranch => "stale branch, inactive 90d+".to_string(),
            Self::MixedLockfiles(mgr) => format!("mixed package managers: {}", mgr),
            Self::NoGit => "no git repository".to_string(),
            Self::LowActivity => "project activity is low".to_string(),
            Self::NoCommands => "no commands detected".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthLevel {
    Unknown,
    Bad,
    Warn,
    Good,
}

impl HealthLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Warn => "warn",
            Self::Bad => "bad",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealth {
    pub score: u8,
    pub level: HealthLevel,
    pub positives: Vec<String>,
    pub warnings: Vec<ProjectWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub stack: Vec<String>,
    pub manager: Option<String>,
    pub scripts: Vec<String>,
    pub git: Option<GitInfo>,
    pub activity: ActivityInfo,
    pub status: ProjectStatus,
    pub note: Option<String>,
    pub warnings: Vec<ProjectWarning>,
    pub commands: Vec<ProjectCommand>,
    pub health: ProjectHealth,
    pub artifacts: Vec<ProjectArtifact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<u16>,
}
