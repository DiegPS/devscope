use std::path::PathBuf;
use std::time::Instant;

use crate::config::{Config, OpenActionConfig};
use crate::project::{Project, ProjectArtifact, ProjectStatus};
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
pub enum ViewMode {
    Compact,
    Detailed,
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
    pub fn all() -> Vec<Self> {
        vec![
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

    pub fn all() -> Vec<Self> {
        vec![
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
    pub status_options: Vec<ProjectStatus>,
    pub status_selected: usize,
    pub help_scroll: usize,
    pub should_quit: bool,
    pub needs_reload: bool,
    pub status_message: Option<String>,
    pub view_mode: ViewMode,
    pub pending_action: Option<PendingOpenAction>,
}

impl App {
    pub fn new(config: Config) -> Self {
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
            status_options: vec![
                ProjectStatus::Active,
                ProjectStatus::Paused,
                ProjectStatus::Stale,
                ProjectStatus::Archived,
            ],
            status_selected: 0,
            help_scroll: 0,
            should_quit: false,
            needs_reload: true,
            status_message: None,
            view_mode: ViewMode::Detailed,
            pending_action: None,
        };
        app.reload();
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

    pub fn apply_filter_and_sort(&mut self) {
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

        if self.selected >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    fn matches_filter(&self, project: &Project) -> bool {
        match self.filter {
            FilterField::All => true,
            FilterField::Active => project.status == ProjectStatus::Active,
            FilterField::Dirty => project.git.as_ref().is_some_and(|g| g.is_dirty),
            FilterField::Stale => project.status == ProjectStatus::Stale,
            FilterField::Paused => project.status == ProjectStatus::Paused,
            FilterField::Archived => project.status == ProjectStatus::Archived,
            FilterField::Flutter => project.stack.iter().any(|s| s.contains("Flutter")),
            FilterField::Rust => project.stack.contains(&"Rust".to_string()),
            FilterField::Node => project.stack.contains(&"Node".to_string()),
            FilterField::Python => project.stack.contains(&"Python".to_string()),
            FilterField::Go => project.stack.contains(&"Go".to_string()),
            FilterField::Docker => project.stack.contains(&"Docker".to_string()),
            FilterField::Windows => project.stack.contains(&"Windows".to_string()),
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
                let sa = a.stack.first().cloned().unwrap_or_default();
                let sb = b.stack.first().cloned().unwrap_or_default();
                sa.cmp(&sb)
            }
            SortField::Status => a.status.as_str().cmp(b.status.as_str()),
            SortField::DirtyFirst => {
                let da = a.git.as_ref().is_some_and(|g| g.is_dirty);
                let db = b.git.as_ref().is_some_and(|g| g.is_dirty);
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
            .map(|&i| &self.projects[i])
    }

    pub fn selected_project_mut(&mut self) -> Option<&mut Project> {
        let idx = self.filtered_indices.get(self.selected).copied()?;
        Some(&mut self.projects[idx])
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
        self.view_mode = match self.view_mode {
            ViewMode::Compact => ViewMode::Detailed,
            ViewMode::Detailed => ViewMode::Compact,
        };
    }

    pub fn selected_path_str(&self) -> Option<String> {
        self.selected_project()
            .map(|p| p.path.to_string_lossy().to_string())
    }
}
