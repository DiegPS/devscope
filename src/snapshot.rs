use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub(crate) struct DirEntrySnapshot {
    pub name: String,
    pub is_file: bool,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub(crate) struct DirSnapshot {
    root: PathBuf,
    names: HashSet<String>,
    entries: Vec<DirEntrySnapshot>,
    contents: Arc<Mutex<HashMap<String, Option<String>>>>,
}

impl DirSnapshot {
    pub fn read(path: &Path) -> Self {
        let mut names = HashSet::new();
        let mut entries = Vec::new();

        if let Ok(dir_entries) = std::fs::read_dir(path) {
            for entry in dir_entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let metadata = entry.metadata().ok();
                let is_file = metadata.as_ref().is_some_and(|m| m.is_file());
                let modified = metadata.and_then(|m| m.modified().ok());

                names.insert(name.clone());
                entries.push(DirEntrySnapshot {
                    name,
                    is_file,
                    modified,
                });
            }
        }

        Self {
            root: path.to_path_buf(),
            names,
            entries,
            contents: Arc::default(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn has(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn has_any(&self, names: &[&str]) -> bool {
        names.iter().any(|name| self.has(name))
    }

    pub fn read_to_string(&self, name: &str) -> Option<String> {
        if !self.has(name) {
            return None;
        }

        self.contents
            .lock()
            .expect("snapshot contents lock")
            .entry(name.to_owned())
            .or_insert_with(|| std::fs::read_to_string(self.root.join(name)).ok())
            .clone()
    }

    pub fn entries(&self) -> &[DirEntrySnapshot] {
        &self.entries
    }

    pub fn modified(&self, name: &str) -> Option<SystemTime> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .and_then(|entry| entry.modified)
    }
}
