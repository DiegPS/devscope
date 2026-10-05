use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::project::ProjectStatus;
use crate::scoring::ScoreMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub(crate) persisted: Option<toml::Value>,
    #[serde(default)]
    pub roots: Vec<String>,

    #[serde(skip)]
    pub session_roots: Option<Vec<String>>,

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

    #[serde(default, with = "status_map")]
    pub project_status: std::collections::HashMap<String, ProjectStatus>,

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
            persisted: None,
            roots: Vec::new(),
            session_roots: None,
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

impl Config {
    pub fn active_roots(&self) -> &[String] {
        self.session_roots.as_deref().unwrap_or(&self.roots)
    }
}

pub fn config_dir() -> Result<PathBuf> {
    // Keep the storage identifier for compatibility with existing user config;
    // the installed command and user-facing application name are `ds`.
    let dirs =
        ProjectDirs::from("", "", "devscope").context("Could not determine config directory")?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn config_path() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(path) = TEST_CONFIG_PATH.with(|path| path.borrow().clone()) {
        return Ok(path);
    }
    Ok(config_dir()?.join("config.toml"))
}

#[cfg(test)]
thread_local! {
    static TEST_CONFIG_PATH: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Test-only, thread-local injection. It cannot redirect production binaries,
/// does not mutate process environment, and restores its scope during unwind.
#[cfg(test)]
pub(crate) fn with_test_config_path<T>(path: PathBuf, test: impl FnOnce() -> T) -> T {
    struct Restore(Option<PathBuf>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_CONFIG_PATH.with(|path| *path.borrow_mut() = self.0.take());
        }
    }
    let previous = TEST_CONFIG_PATH.with(|current| current.replace(Some(path)));
    let _restore = Restore(previous);
    test()
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
            std::path::Component::ParentDir => match components.last().copied() {
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
                Some(std::path::Component::RootDir) | Some(std::path::Component::Prefix(_)) => {}
            },
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

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        let mut config = Config::default();
        save_config(&config)?;
        config.persisted = Some(toml::Value::try_from(&config)?);
        return Ok(config);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;

    let mut config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config at {}", path.display()))?;
    config.validate()?;
    migrate_project_keys(&mut config)?;
    config.persisted = Some(toml::Value::try_from(&config)?);
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    save_config_at(config, &path)
}

pub(crate) fn save_config_at(config: &Config, path: &Path) -> Result<()> {
    commit_config_at(config, path).map(|_| ())
}

pub(crate) fn commit_config_at(config: &Config, path: &Path) -> Result<Config> {
    let resolved = if path.is_symlink() {
        Some(std::fs::canonicalize(path)?)
    } else {
        None
    };
    let path = resolved.as_deref().unwrap_or(path);
    let mut merged = toml::Value::try_from(config)?;
    // Only read-modify-write operations need conflict protection. Explicit
    // construction/replacement remains supported for initial configuration.
    let _lock = if config.persisted.is_some() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock_path = path.with_extension("toml.lock");
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(lock_path)?;
        lock.lock()?;
        let mut current_config: Config = toml::from_str(
            &std::fs::read_to_string(path).context("Config disappeared during editing")?,
        )?;
        current_config.validate()?;
        migrate_project_keys(&mut current_config)?;
        let current = toml::Value::try_from(&current_config)?;
        merged = merge_value(config.persisted.as_ref(), Some(&merged), Some(&current), "")?
            .context("Config cannot be deleted")?;
        Some(lock)
    } else {
        None
    };
    let mut updated: Config = merged.clone().try_into()?;
    updated.validate()?;
    updated.session_roots = config.session_roots.clone();
    write_config_at(&updated, path)?;
    updated.persisted = Some(merged);
    Ok(updated)
}

fn merge_value(
    base: Option<&toml::Value>,
    ours: Option<&toml::Value>,
    theirs: Option<&toml::Value>,
    key: &str,
) -> Result<Option<toml::Value>> {
    if ours == base {
        return Ok(theirs.cloned());
    }
    if theirs == base {
        return Ok(ours.cloned());
    }
    if let (Some(toml::Value::Table(ours)), Some(toml::Value::Table(theirs))) = (ours, theirs) {
        let base = base.and_then(toml::Value::as_table);
        let keys: std::collections::BTreeSet<_> = ours
            .keys()
            .chain(theirs.keys())
            .chain(base.into_iter().flat_map(|b| b.keys()))
            .collect();
        let mut merged = toml::map::Map::new();
        for field in keys {
            if let Some(value) = merge_value(
                base.and_then(|b| b.get(field)),
                ours.get(field),
                theirs.get(field),
                &format!("{key}/{field}"),
            )? {
                merged.insert(field.clone(), value);
            }
        }
        return Ok(Some(toml::Value::Table(merged)));
    }
    if key.starts_with("/scores/") {
        if let (Some(b), Some(o), Some(t)) = (
            Some(base.and_then(toml::Value::as_integer).unwrap_or(0)),
            ours.and_then(toml::Value::as_integer),
            theirs.and_then(toml::Value::as_integer),
        ) {
            if key.ends_with("/visits") || key.ends_with("/opens") {
                if o >= b && t >= b {
                    return Ok(Some(toml::Value::Integer(
                        t.saturating_add(o - b).min(u32::MAX as i64),
                    )));
                }
            } else if key.ends_with("/last_used") {
                return Ok(Some(toml::Value::Integer(o.max(t))));
            }
        }
    }
    if ours == theirs {
        return Ok(ours.cloned());
    }
    anyhow::bail!("Concurrent config edit at {key}; reload before retrying (no data overwritten)")
}

pub(crate) fn project_key(path: &Path) -> String {
    dunce::canonicalize(path)
        .unwrap_or_else(|_| normalize_path(path))
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn migrate_project_keys(config: &mut Config) -> Result<()> {
    migrate_aliases(&mut config.notes)?;
    migrate_aliases(&mut config.project_status)?;
    let originals = config.scores.clone();
    for (old, value) in &originals {
        if Path::new(&old).exists() {
            let key = project_key(Path::new(old));
            if originals.contains_key(&key) {
                continue;
            }
            let entry = config.scores.entry(key).or_default();
            // Aliases can be copies of the same history: adding them would
            // inflate ranking on every load. Keep original entries and use
            // deterministic maxima for the canonical identity.
            entry.visits = entry.visits.max(value.visits);
            entry.opens = entry.opens.max(value.opens);
            entry.last_used = entry.last_used.max(value.last_used);
        }
    }
    Ok(())
}

fn migrate_aliases<T: Clone + PartialEq>(
    map: &mut std::collections::HashMap<String, T>,
) -> Result<()> {
    let originals = map.clone();
    let mut aliases: Vec<_> = originals.iter().collect();
    aliases.sort_by_key(|(key, _)| *key);
    for (old, value) in aliases {
        if !Path::new(&old).exists() {
            continue;
        }
        let key = project_key(Path::new(old));
        if originals.contains_key(&key) {
            continue;
        }
        if map.get(&key).is_some_and(|existing| existing != value) {
            anyhow::bail!("Conflicting metadata aliases for {key}; original config preserved");
        }
        map.entry(key).or_insert(value.clone());
    }
    Ok(())
}

mod status_map {
    use super::ProjectStatus;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::collections::HashMap;
    pub fn serialize<S: Serializer>(
        map: &HashMap<String, ProjectStatus>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_map(map.iter().map(|(path, status)| (path, status.as_str())))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<String, ProjectStatus>, D::Error> {
        HashMap::<String, String>::deserialize(deserializer)?
            .into_iter()
            .map(|(path, value)| {
                value
                    .parse()
                    .map(|status| (path, status))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

impl Config {
    pub(crate) fn validate(&self) -> Result<()> {
        let mut keys = std::collections::HashSet::new();
        for action in &self.open.actions {
            let mut chars = action.key.chars();
            let key = chars
                .next()
                .filter(|key| !key.is_whitespace() && !key.is_control())
                .context("Open action key must be one printable character")?;
            anyhow::ensure!(
                chars.next().is_none(),
                "Open action key must be one character: {}",
                action.key
            );
            anyhow::ensure!(
                keys.insert(key.to_ascii_lowercase()),
                "Duplicate open action key: {}",
                action.key
            );
            anyhow::ensure!(
                !action.name.trim().is_empty(),
                "Open action name must not be empty"
            );
            if matches!(action.kind, None | Some(OpenActionKind::Command)) {
                anyhow::ensure!(
                    action
                        .command
                        .as_deref()
                        .is_some_and(|command| !command.trim().is_empty()),
                    "Missing command for open action {}",
                    action.name
                );
            }
        }
        if let Some(default) = &self.open.default {
            anyhow::ensure!(
                self.open
                    .actions
                    .iter()
                    .any(|action| &action.name == default),
                "Unknown default open action: {default}"
            );
        }
        Ok(())
    }
}

fn write_config_at(config: &Config, path: &Path) -> Result<()> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    // Preserve an existing config symlink rather than replacing the link itself.
    let resolved = if path.is_symlink() {
        Some(std::fs::canonicalize(path).context("Failed to resolve config symlink")?)
    } else {
        None
    };
    let path = resolved.as_deref().unwrap_or(path);
    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;
    if std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().readonly()) {
        anyhow::bail!("Config is read-only: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir at {}", parent.display()))?;
    }

    let parent = path.parent().unwrap_or(Path::new("."));
    let file_name = path.file_name().context("Config path has no file name")?;
    let (temporary, mut file) = loop {
        let mut name = file_name.to_os_string();
        name.push(format!(
            ".{}.{}.tmp",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let temporary = parent.join(name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&temporary) {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to prepare config at {}", path.display()))
            }
        }
    };
    let cleanup = TemporaryConfig(temporary);
    if let Ok(metadata) = std::fs::metadata(path) {
        file.set_permissions(metadata.permissions())?;
    }
    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write config at {}", path.display()))?;
    file.sync_all().context("Failed to flush config")?;
    drop(file);
    replace_config(&cleanup.0, path)
        .with_context(|| format!("Failed to replace config at {}", path.display()))?;
    Ok(())
}

struct TemporaryConfig(PathBuf);

impl Drop for TemporaryConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(not(windows))]
fn replace_config(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_config(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: both buffers are owned, NUL-terminated UTF-16 paths and remain
    // alive throughout the call. The temporary is on the same filesystem.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x1 | 0x8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn get_project_status(config: &Config, path: &str) -> Option<ProjectStatus> {
    config.project_status.get(path).cloned()
}

pub fn set_project_status(config: &mut Config, path: &str, status: ProjectStatus) {
    config.project_status.insert(path.to_string(), status);
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
            args: vec!["{path}".to_string()],
            current_dir: false,
            terminal_mode: false,
            env: std::collections::HashMap::new(),
            kind: None,
        },
        OpenActionConfig {
            key: "v".to_string(),
            name: "vscode".to_string(),
            command: Some("code".to_string()),
            args: vec!["{path}".to_string()],
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

    #[cfg(target_os = "macos")]
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

    #[cfg(all(unix, not(target_os = "macos")))]
    actions.push(OpenActionConfig {
        key: "t".into(),
        name: "terminal".into(),
        command: Some("x-terminal-emulator".into()),
        args: Vec::new(),
        current_dir: true,
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

#[cfg(test)]
mod tests {
    use super::{normalize_path, Config};
    use std::path::Path;

    #[test]
    fn config_replacement_round_trips_and_leaves_no_temporary_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = Config::default();
        super::save_config_at(&config, &path).unwrap();
        config.notes.insert("project".into(), "saved note".into());
        super::save_config_at(&config, &path).unwrap();
        let saved: Config = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.notes, config.notes);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_replacement_preserves_existing_target_and_cleans_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("keep"), "original").unwrap();
        assert!(super::save_config_at(&Config::default(), &path).is_err());
        assert_eq!(
            std::fs::read_to_string(path.join("keep")).unwrap(),
            "original"
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    #[cfg(unix)]
    fn config_replacement_preserves_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("actual.toml");
        let link = dir.path().join("config.toml");
        std::fs::write(&target, "").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        super::save_config_at(&Config::default(), &link).unwrap();
        assert!(link.is_symlink());
        assert!(toml::from_str::<Config>(&std::fs::read_to_string(target).unwrap()).is_ok());
    }

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
        assert_eq!(
            normalize_path(Path::new("../projects")),
            Path::new("../projects")
        );
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

    #[test]
    fn active_roots_uses_session_override_without_mutating_roots() {
        let config = Config {
            roots: vec!["C:\\projects".to_string(), "D:\\work".to_string()],
            session_roots: Some(vec!["C:\\temp-only".to_string()]),
            ..Config::default()
        };

        assert_eq!(config.active_roots(), &["C:\\temp-only".to_string()]);
        assert_eq!(
            config.roots,
            vec!["C:\\projects".to_string(), "D:\\work".to_string()]
        );
    }

    #[test]
    fn session_roots_are_not_serialized() {
        let config = Config {
            roots: vec!["C:\\projects".to_string()],
            session_roots: Some(vec!["C:\\temp-only".to_string()]),
            ..Config::default()
        };

        let toml = toml::to_string(&config).unwrap();
        let parsed: toml::Value = toml::from_str(&toml).unwrap();

        assert_eq!(
            parsed
                .get("roots")
                .and_then(|roots| roots.as_array())
                .and_then(|roots| roots.first())
                .and_then(|root| root.as_str()),
            Some("C:\\projects")
        );
        assert!(parsed.get("session_roots").is_none());
        assert!(!toml.contains("temp-only"));
    }
}
