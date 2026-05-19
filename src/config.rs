use std::path::{Path, PathBuf};

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
    pub open: OpenConfig,

    #[serde(default)]
    pub project_status: std::collections::HashMap<String, String>,

    #[serde(default)]
    pub notes: std::collections::HashMap<String, String>,

    #[serde(default)]
    pub scores: ScoreMap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenConfig {
    #[serde(default)]
    pub default: Option<String>,

    #[serde(default = "default_open_actions")]
    pub actions: Vec<OpenActionConfig>,
}

impl Default for OpenConfig {
    fn default() -> Self {
        Self {
            default: None,
            actions: default_open_actions(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenActionConfig {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub current_dir: bool,
    #[serde(default)]
    pub terminal_mode: bool,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub kind: Option<OpenActionKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenActionKind {
    Command,
    FileManager,
    BuildOutput,
    Executable,
}

impl OpenActionConfig {
    pub fn key_char(&self) -> char {
        self.key.chars().next().unwrap_or(' ')
    }

    pub fn resolve_args(&self, path: &Path, name: &str) -> Vec<String> {
        self.args
            .iter()
            .map(|a| {
                a.replace("{path}", &path.to_string_lossy())
                    .replace("{name}", name)
            })
            .collect()
    }
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
            open: OpenConfig::default(),
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
                match components.last().copied() {
                    Some(std::path::Component::Normal(_)) => {
                        components.pop();
                    }
                    Some(std::path::Component::CurDir) => {
                        components.pop();
                        components.push(component);
                    }
                    Some(std::path::Component::ParentDir) | None => {
                        components.push(component);
                    }
                    Some(std::path::Component::RootDir)
                    | Some(std::path::Component::Prefix(_)) => {}
                }
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }

    let normalized: PathBuf = components.iter().collect();
    if normalized.as_os_str().is_empty() && !path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_path;
    use std::path::Path;

    #[test]
    fn normalize_path_keeps_current_directory_when_components_collapse() {
        let dot = normalize_path(Path::new("."));
        assert_eq!(dot.display().to_string(), ".");
        assert!(!dot.as_os_str().is_empty());

        let collapsed = normalize_path(Path::new("a/.."));
        assert_eq!(collapsed.display().to_string(), ".");
        assert!(!collapsed.as_os_str().is_empty());
    }

    #[test]
    fn normalize_path_preserves_leading_parent_components() {
        assert_eq!(normalize_path(Path::new("..")), Path::new(".."));
        assert_eq!(normalize_path(Path::new("../projects")), Path::new("../projects"));
        assert_eq!(
            normalize_path(Path::new("..\\projects")),
            Path::new("..\\projects")
        );
    }

    #[test]
    fn normalize_path_collapses_children_before_parent_components() {
        assert_eq!(normalize_path(Path::new("a/b/../c")), Path::new("a/c"));
        assert_eq!(normalize_path(Path::new("a/../../b")), Path::new("../b"));
    }
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
    config.project_status.get(path).and_then(|s| s.parse().ok())
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

fn default_open_actions() -> Vec<OpenActionConfig> {
    let mut actions = vec![
        OpenActionConfig {
            key: "o".to_string(),
            name: "opencode".to_string(),
            command: Some("opencode".to_string()),
            args: vec![".".to_string()],
            current_dir: true,
            terminal_mode: true,
            env: {
                let mut m = std::collections::HashMap::new();
                m.insert("OPENCODE_DISABLE_MOUSE".to_string(), "true".to_string());
                m
            },
            kind: None,
        },
        OpenActionConfig {
            key: "p".to_string(),
            name: "pi".to_string(),
            command: Some("pi".to_string()),
            args: vec![".".to_string()],
            current_dir: true,
            terminal_mode: true,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "c".to_string(),
            name: "cursor".to_string(),
            command: Some("cursor".to_string()),
            args: vec![".".to_string()],
            current_dir: false,
            terminal_mode: false,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "v".to_string(),
            name: "vscode".to_string(),
            command: Some("code".to_string()),
            args: vec![".".to_string()],
            current_dir: false,
            terminal_mode: false,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "n".to_string(),
            name: "nvim".to_string(),
            command: Some("nvim".to_string()),
            args: vec![".".to_string()],
            current_dir: true,
            terminal_mode: true,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "h".to_string(),
            name: "helix".to_string(),
            command: Some("hx".to_string()),
            args: vec![".".to_string()],
            current_dir: true,
            terminal_mode: true,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "g".to_string(),
            name: "lazygit".to_string(),
            command: Some("lazygit".to_string()),
            args: Vec::new(),
            current_dir: true,
            terminal_mode: true,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "y".to_string(),
            name: "yazi".to_string(),
            command: Some("yazi".to_string()),
            args: Vec::new(),
            current_dir: true,
            terminal_mode: true,
            env: std::collections::HashMap::new(),
            kind: None,
        },
    ];

    #[cfg(target_os = "windows")]
    actions.push(OpenActionConfig {
        key: "t".to_string(),
        name: "terminal".to_string(),
        command: Some("wt".to_string()),
        args: vec!["-d".to_string(), "{path}".to_string()],
        current_dir: false,
        terminal_mode: false,
        env: std::collections::HashMap::new(),
        kind: None,
    });

    #[cfg(not(target_os = "windows"))]
    actions.push(OpenActionConfig {
        key: "t".to_string(),
        name: "terminal".to_string(),
        command: Some("open".to_string()),
        args: vec![
            "-a".to_string(),
            "Terminal".to_string(),
            "{path}".to_string(),
        ],
        current_dir: false,
        terminal_mode: false,
        env: std::collections::HashMap::new(),
        kind: None,
    });

    actions.push(OpenActionConfig {
        key: "f".to_string(),
        name: "folder".to_string(),
        command: None,
        args: Vec::new(),
        current_dir: false,
        terminal_mode: false,
        env: std::collections::HashMap::new(),
        kind: Some(OpenActionKind::FileManager),
    });

    actions.push(OpenActionConfig {
        key: "b".to_string(),
        name: "build output".to_string(),
        command: None,
        args: Vec::new(),
        current_dir: false,
        terminal_mode: false,
        env: std::collections::HashMap::new(),
        kind: Some(OpenActionKind::BuildOutput),
    });

    actions.push(OpenActionConfig {
        key: "x".to_string(),
        name: "executable".to_string(),
        command: None,
        args: Vec::new(),
        current_dir: false,
        terminal_mode: false,
        env: std::collections::HashMap::new(),
        kind: Some(OpenActionKind::Executable),
    });

    actions
}
