use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use crate::config::{Config, OpenActionConfig};
use crate::project::{DirtyStatus, Project, ProjectArtifact, ProjectStatus};
use crate::scanner;
use crate::scoring;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    EditingNote,
    ChangingStatus,
    Help,
    OpenMenu,
    ConfigMenu,
}

pub struct PendingOpenAction {
    pub action: OpenActionConfig,
    pub project_path: PathBuf,
    pub project_name: String,
    pub artifacts: Vec<ProjectArtifact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Compact,
    Detailed,
}

impl ViewMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Detailed => "detailed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Activity,
    Name,
    Stack,
    Status,
    DirtyFirst,
    Path,
    Score,
}

impl SortField {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::Name => "name",
            Self::Stack => "stack",
            Self::Status => "status",
            Self::DirtyFirst => "dirty",
            Self::Path => "path",
            Self::Score => "score",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Activity,
            Self::Name,
            Self::Stack,
            Self::Status,
            Self::DirtyFirst,
            Self::Path,
            Self::Score,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterField {
    All,
    Active,
    Dirty,
    Stale,
    Paused,
    Archived,
    Flutter,
    Rust,
    Node,
    Python,
    Go,
    Docker,
    Windows,
    WithNotes,
}

impl FilterField {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Dirty => "dirty",
            Self::Stale => "stale",
            Self::Paused => "paused",
            Self::Archived => "archived",
            Self::Flutter => "flutter",
            Self::Rust => "rust",
            Self::Node => "node",
            Self::Python => "python",
            Self::Go => "go",
            Self::Docker => "docker",
            Self::Windows => "windows",
            Self::WithNotes => "with-notes",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::All,
            Self::Active,
            Self::Dirty,
            Self::Stale,
            Self::Paused,
            Self::Archived,
            Self::Flutter,
            Self::Rust,
            Self::Node,
            Self::Python,
            Self::Go,
            Self::Docker,
            Self::Windows,
            Self::WithNotes,
        ]
    }
}

struct HydrationResult {
    project_index: usize,
    dirty_status: DirtyStatus,
    modified_count: Option<usize>,
    untracked_count: Option<usize>,
    generation: u64,
    health: crate::project::ProjectHealth,
}

pub struct App {
    pub config: Config,
    pub projects: Vec<Project>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub filter: FilterField,
    pub sort: SortField,
    pub scan_duration_ms: u128,
    pub total_projects: usize,
    pub note_input: String,
    pub editing_project_id: Option<String>,
    pub status_options: Vec<ProjectStatus>,
    pub status_selected: usize,
    pub help_scroll: usize,
    pub details_scroll: usize,
    pub details_focus: bool,
    pub menu_selected: usize,
    pub viewport: (u16, u16),
    pub should_quit: bool,
    pub needs_reload: bool,
    pub status_message: Option<String>,
    pub message_level: MessageLevel,
    pub view_mode: ViewMode,
    pub pending_action: Option<PendingOpenAction>,
    pub ports_rx: Option<mpsc::Receiver<HashMap<String, Vec<u16>>>>,
    background_cancel: Arc<AtomicBool>,
    reload_cancel: Arc<AtomicBool>,
    reload_rx: Option<mpsc::Receiver<anyhow::Result<scanner::ScanResult>>>,
    hydration_generation: u64,
    hydration_result_rx: Option<mpsc::Receiver<HydrationResult>>,
    #[cfg(test)]
    pub config_save_path: Option<PathBuf>,
}

impl App {
    pub fn set_message(&mut self, message: String, level: MessageLevel) {
        self.status_message = Some(message);
        self.message_level = level;
    }

    pub fn new(config: Config) -> Self {
        let view_mode = if config.ui.right_panel {
            ViewMode::Detailed
        } else {
            ViewMode::Compact
        };
        let mut app = Self {
            config,
            projects: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            filter: FilterField::All,
            sort: SortField::Activity,
            scan_duration_ms: 0,
            total_projects: 0,
            note_input: String::new(),
            editing_project_id: None,
            status_options: vec![
                ProjectStatus::Active,
                ProjectStatus::Paused,
                ProjectStatus::Stale,
                ProjectStatus::Archived,
            ],
            status_selected: 0,
            help_scroll: 0,
            details_scroll: 0,
            details_focus: false,
            menu_selected: 0,
            viewport: (80, 24),
            should_quit: false,
            needs_reload: true,
            status_message: None,
            message_level: MessageLevel::Info,
            view_mode,
            pending_action: None,
            ports_rx: None,
            background_cancel: Arc::new(AtomicBool::new(false)),
            reload_cancel: Arc::new(AtomicBool::new(false)),
            reload_rx: None,
            hydration_generation: 0,
            hydration_result_rx: None,
            #[cfg(test)]
            config_save_path: None,
        };
        app.reload();

        let tui_apps = ["opencode", "pi", "nvim", "vim", "hx", "lazygit", "yazi"];
        for action in &app.config.open.actions {
            let is_tui = tui_apps.contains(&action.name.as_str())
                || action
                    .command
                    .as_deref()
                    .is_some_and(|c| tui_apps.contains(&c));

            if is_tui && !action.terminal_mode {
                app.set_message(format!(
                    "Warning: action '{}' is configured with terminal_mode=false. This may cause visual glitches. Set terminal_mode=true.",
                    action.name
                ), crate::app::MessageLevel::Warning);
                break;
            }
        }

        app
    }

    pub fn reload(&mut self) {
        let start = Instant::now();

        match scanner::scan_roots(&self.config) {
            Ok(result) => {
                self.projects = result.projects;
                self.scan_duration_ms = result.duration_ms;
                self.total_projects = result.projects_found;
            }
            Err(e) => {
                eprintln!("Scan error: {}", e);
                self.projects.clear();
                self.scan_duration_ms = start.elapsed().as_millis();
                self.total_projects = 0;
            }
        }

        self.apply_filter_and_sort();
        self.selected = 0;
        self.needs_reload = false;
    }

    pub(crate) fn start_background_jobs(&mut self) {
        self.start_background_hydration();
        self.spawn_port_detection();
    }

    /// At most one scan runs; repeated requests coalesce into one next scan.
    pub fn start_reload(&mut self) {
        if self.reload_rx.is_some() {
            return;
        }
        self.needs_reload = false;
        self.background_cancel.store(true, Ordering::Relaxed);
        self.hydration_result_rx = None;
        self.ports_rx = None;
        let config = self.config.clone();
        let cancelled = Arc::clone(&self.reload_cancel);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(scanner::scan_roots_cancellable(&config, &cancelled));
        });
        self.reload_rx = Some(rx);
        self.set_message("Scanning…".into(), MessageLevel::Info);
    }

    pub fn poll_reload(&mut self) -> bool {
        let Some(rx) = &self.reload_rx else {
            return false;
        };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err(anyhow::anyhow!("Scan worker disconnected")),
        };
        self.reload_rx = None;
        if self.needs_reload {
            self.start_reload();
            return true;
        }
        match result {
            Ok(mut result) => {
                // Notes and statuses may have been edited while scanning.
                for project in &mut result.projects {
                    project.note = self.config.notes.get(&project.id).cloned();
                    if let Some(status) =
                        crate::config::get_project_status(&self.config, &project.id)
                    {
                        project.status = status;
                    }
                }
                let selected_id = self.selected_project().map(|p| p.id.clone());
                self.projects = result.projects;
                self.total_projects = result.projects_found;
                self.scan_duration_ms = result.duration_ms;
                self.apply_filter_and_sort();
                if let Some(id) = selected_id {
                    if let Some(index) = self
                        .filtered_indices
                        .iter()
                        .position(|&i| self.projects[i].id == id)
                    {
                        self.selected = index;
                    }
                }
                self.start_background_hydration();
                self.spawn_port_detection();
                self.set_message(
                    format!(
                        "Scanned {} projects in {}ms",
                        self.total_projects, self.scan_duration_ms
                    ),
                    MessageLevel::Info,
                );
            }
            Err(error) => self.set_message(format!("Scan failed: {error:#}"), MessageLevel::Error),
        }
        true
    }

    fn spawn_port_detection(&mut self) {
        self.ports_rx = None;
        let paths: Vec<String> = self
            .projects
            .iter()
            .map(|p| p.path.to_string_lossy().to_string())
            .collect();
        if paths.is_empty() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let cancelled = Arc::clone(&self.background_cancel);
        std::thread::spawn(move || {
            if cancelled.load(Ordering::Relaxed) {
                return;
            }
            let map = crate::ports::detect_project_ports(&paths);
            let _ = tx.send(map);
        });
        self.ports_rx = Some(rx);
    }

    pub fn apply_filter_and_sort(&mut self) {
        let selected_id = self.selected_project().map(|project| project.id.clone());
        let mut indices: Vec<usize> = self
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| self.matches_filter(p))
            .filter(|(_, p)| self.matches_search(p))
            .map(|(i, _)| i)
            .collect();

        indices.sort_by(|&a, &b| {
            let pa = &self.projects[a];
            let pb = &self.projects[b];
            self.compare_projects(pa, pb)
        });

        self.filtered_indices = indices;

        self.selected = selected_id
            .and_then(|id| {
                self.filtered_indices
                    .iter()
                    .position(|&i| self.projects[i].id == id)
            })
            .unwrap_or_else(|| {
                self.selected
                    .min(self.filtered_indices.len().saturating_sub(1))
            });
    }

    fn matches_filter(&self, project: &Project) -> bool {
        match self.filter {
            FilterField::All => true,
            FilterField::Active => project.status == ProjectStatus::Active,
            FilterField::Dirty => project
                .git
                .as_ref()
                .is_some_and(|g| g.dirty_status == DirtyStatus::Dirty),
            FilterField::Stale => project.status == ProjectStatus::Stale,
            FilterField::Paused => project.status == ProjectStatus::Paused,
            FilterField::Archived => project.status == ProjectStatus::Archived,
            FilterField::Flutter => project.stack.iter().any(|s| s.contains("Flutter")),
            FilterField::Rust => stack_contains(&project.stack, "Rust"),
            FilterField::Node => stack_contains(&project.stack, "Node"),
            FilterField::Python => stack_contains(&project.stack, "Python"),
            FilterField::Go => stack_contains(&project.stack, "Go"),
            FilterField::Docker => stack_contains(&project.stack, "Docker"),
            FilterField::Windows => stack_contains(&project.stack, "Windows"),
            FilterField::WithNotes => project.note.is_some(),
        }
    }

    fn matches_search(&self, project: &Project) -> bool {
        if self.search_query.is_empty() {
            return true;
        }

        let q = self.search_query.to_lowercase();

        scoring::matches_name(&project.name, &q)
            || project.path.to_string_lossy().to_lowercase().contains(&q)
            || project.stack.iter().any(|s| s.to_lowercase().contains(&q))
            || project
                .note
                .as_ref()
                .is_some_and(|n| n.to_lowercase().contains(&q))
            || project.status.as_str().contains(&q)
            || project
                .git
                .as_ref()
                .is_some_and(|g| g.branch.to_lowercase().contains(&q))
    }

    fn compare_projects(&self, a: &Project, b: &Project) -> std::cmp::Ordering {
        match self.sort {
            SortField::Activity => {
                let ta = a.activity.timestamp.unwrap_or(0);
                let tb = b.activity.timestamp.unwrap_or(0);
                tb.cmp(&ta)
            }
            SortField::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortField::Stack => {
                let sa = a.stack.first().map(String::as_str).unwrap_or_default();
                let sb = b.stack.first().map(String::as_str).unwrap_or_default();
                sa.cmp(sb)
            }
            SortField::Status => a.status.as_str().cmp(b.status.as_str()),
            SortField::DirtyFirst => {
                let da = a
                    .git
                    .as_ref()
                    .is_some_and(|g| g.dirty_status == DirtyStatus::Dirty);
                let db = b
                    .git
                    .as_ref()
                    .is_some_and(|g| g.dirty_status == DirtyStatus::Dirty);
                db.cmp(&da).then_with(|| a.name.cmp(&b.name))
            }
            SortField::Path => a.path.cmp(&b.path),
            SortField::Score => {
                let sa = self.score_project(a);
                let sb = self.score_project(b);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    }

    fn score_project(&self, project: &Project) -> f64 {
        let entry = self
            .config
            .scores
            .get(&project.id)
            .cloned()
            .unwrap_or_default();
        scoring::compute_score(&project.name, &self.search_query, &entry)
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.projects.get(i))
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered_indices.len() {
            self.selected += 1;
        }
    }

    pub fn move_page_up(&mut self) {
        self.selected = self.selected.saturating_sub(10);
    }

    pub fn move_page_down(&mut self) {
        self.selected = (self.selected + 10).min(self.filtered_indices.len().saturating_sub(1));
    }

    pub fn move_home(&mut self) {
        self.selected = 0;
    }

    pub fn move_end(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    pub fn next_filter(&mut self) {
        let all = FilterField::all();
        let current_idx = all.iter().position(|f| *f == self.filter).unwrap_or(0);
        self.filter = all[(current_idx + 1) % all.len()];
        self.apply_filter_and_sort();
    }

    pub fn next_sort(&mut self) {
        let all = SortField::all();
        let current_idx = all.iter().position(|s| *s == self.sort).unwrap_or(0);
        self.sort = all[(current_idx + 1) % all.len()];
        self.apply_filter_and_sort();
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn toggle_view(&mut self) {
        self.details_focus = false;
        self.details_scroll = 0;
        self.view_mode = match self.view_mode {
            ViewMode::Compact => ViewMode::Detailed,
            ViewMode::Detailed => ViewMode::Compact,
        };
    }

    fn start_background_hydration(&mut self) {
        self.background_cancel.store(true, Ordering::Relaxed);
        self.background_cancel = Arc::new(AtomicBool::new(false));
        self.hydration_generation = self.hydration_generation.wrapping_add(1);
        self.hydration_result_rx = None;
        let gen = self.hydration_generation;
        let jobs: Vec<_> = self
            .projects
            .iter_mut()
            .enumerate()
            .filter(|(_, p)| p.git.is_some())
            .map(|(index, project)| {
                if let Some(git) = &mut project.git {
                    git.dirty_status = DirtyStatus::Checking;
                }
                (index, project.clone())
            })
            .collect();
        if jobs.is_empty() {
            return;
        }
        let cancelled = Arc::clone(&self.background_cancel);
        let (result_tx, result_rx) = mpsc::sync_channel::<HydrationResult>(32);
        std::thread::spawn(move || {
            use rayon::prelude::*;
            jobs.into_par_iter()
                .for_each(|(project_index, mut project)| {
                    if cancelled.load(Ordering::Relaxed) {
                        return;
                    }
                    scanner::hydrate_project_git_status(&mut project);
                    if cancelled.load(Ordering::Relaxed) {
                        return;
                    }
                    if let Some(git) = project.git {
                        let _ = result_tx.send(HydrationResult {
                            project_index,
                            dirty_status: git.dirty_status,
                            modified_count: git.modified_count,
                            untracked_count: git.untracked_count,
                            health: project.health,
                            generation: gen,
                        });
                    }
                });
        });
        self.hydration_result_rx = Some(result_rx);
    }

    pub fn poll_hydration_results(&mut self) -> bool {
        let mut changed = false;

        while self.hydration_result_rx.is_some() {
            let recv_result = {
                let rx = self.hydration_result_rx.as_ref().expect("checked is_some");
                rx.try_recv()
            };

            match recv_result {
                Ok(result) => {
                    if result.generation != self.hydration_generation {
                        continue;
                    }

                    if let Some(project) = self.projects.get_mut(result.project_index) {
                        if let Some(ref mut git) = project.git {
                            git.dirty_status = result.dirty_status;
                            git.modified_count = result.modified_count;
                            git.untracked_count = result.untracked_count;
                        }
                        project.warnings = result.health.warnings.clone();
                        project.health = result.health;
                    }

                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.hydration_result_rx = None;
                    break;
                }
            }
        }

        if changed {
            self.apply_filter_and_sort();
        }

        changed
    }

    pub fn persist_config(&mut self, updated: Config) -> bool {
        #[cfg(test)]
        let result = match self.config_save_path.as_ref() {
            Some(path) => crate::config::commit_config_at(&updated, path),
            None => crate::config::config_path()
                .and_then(|path| crate::config::commit_config_at(&updated, &path)),
        };
        #[cfg(not(test))]
        let result = crate::config::config_path()
            .and_then(|path| crate::config::commit_config_at(&updated, &path));
        match result {
            Ok(saved) => {
                self.config = saved;
                true
            }
            Err(error) => {
                self.set_message(
                    format!("Could not save config: {error:#}"),
                    crate::app::MessageLevel::Error,
                );
                false
            }
        }
    }
}

fn stack_contains(stack: &[String], needle: &str) -> bool {
    stack.iter().any(|entry| entry == needle)
}

impl Drop for App {
    fn drop(&mut self) {
        self.background_cancel.store(true, Ordering::Relaxed);
        self.reload_cancel.store(true, Ordering::Relaxed);
    }
}
