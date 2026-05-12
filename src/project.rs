use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub last_commit_hash: String,
    pub last_commit_message: String,
    pub last_commit_date: String,
    pub is_dirty: bool,
    pub modified_count: usize,
    pub untracked_count: usize,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInfo {
    pub last_modified: Option<String>,
    pub last_git_activity: Option<String>,
    pub relative_time: String,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectWarning {
    EnvFilePresent,
    LargeFileDetected,
    NoReadme,
    OutdatedDependencies,
}

impl ProjectWarning {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EnvFilePresent => ".env present",
            Self::LargeFileDetected => "large file detected",
            Self::NoReadme => "no README",
            Self::OutdatedDependencies => "outdated deps",
        }
    }
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
}
