use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::project::ProjectStatus;
use crate::scoring::ScoreMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub roots: Vec<String>,

    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    #[serde(default = "default_true")]
    pub respect_gitignore: bool,

    #[serde(default)]
    pub scan_hidden: bool,

    #[serde(default)]
    pub follow_symlinks: bool,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub project_status: std::collections::HashMap<String, String>,

    #[serde(default)]
    pub notes: std::collections::HashMap<String, String>,

    #[serde(default)]
    pub scores: ScoreMap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_true")]
    pub show_icons: bool,

    #[serde(default = "default_true")]
    pub right_panel: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            show_icons: true,
            right_panel: true,
        }
    }
}

fn default_max_depth() -> usize {
    4
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            max_depth: default_max_depth(),
            respect_gitignore: true,
            scan_hidden: false,
            follow_symlinks: false,
            ui: UiConfig::default(),
            project_status: std::collections::HashMap::new(),
            notes: std::collections::HashMap::new(),
            scores: ScoreMap::new(),
        }
    }
}

pub fn config_dir() -> Result<PathBuf> {
    let dirs =
        ProjectDirs::from("", "", "devscope").context("Could not determine config directory")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/") || path.starts_with("~\\") {
        if let Some(home) = dirs::home() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

mod dirs {
    use std::path::PathBuf;

    pub fn home() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME").ok().map(PathBuf::from)
        }
    }
}

pub fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        let config = Config::default();
        save_config(&config)?;
        return Ok(config);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config at {}", path.display()))?;

    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir at {}", parent.display()))?;
    }

    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;

    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write config at {}", path.display()))?;

    Ok(())
}

pub fn get_project_status(config: &Config, path: &str) -> Option<ProjectStatus> {
    config
        .project_status
        .get(path)
        .map(|s| ProjectStatus::from_str(s))
}

pub fn set_project_status(config: &mut Config, path: &str, status: ProjectStatus) {
    config
        .project_status
        .insert(path.to_string(), status.as_str().to_string());
}

pub fn get_note(config: &Config, path: &str) -> Option<String> {
    config.notes.get(path).cloned()
}

pub fn set_note(config: &mut Config, path: &str, note: String) {
    config.notes.insert(path.to_string(), note);
}

pub fn record_visit(config: &mut Config, path: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let entry = config.scores.entry(path.to_string()).or_default();
    entry.visits = entry.visits.saturating_add(1);
    entry.last_used = Some(now);
}

pub fn record_open(config: &mut Config, path: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let entry = config.scores.entry(path.to_string()).or_default();
    entry.opens = entry.opens.saturating_add(1);
    entry.last_used = Some(now);
}
