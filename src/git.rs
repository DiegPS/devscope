use std::path::Path;

use anyhow::Result;
use git2::{Repository, StatusOptions};

use crate::project::GitInfo;

/// Extract Git information from a repository path using libgit2.
pub fn get_git_info(repo_path: &Path) -> Result<GitInfo> {
    let repo = Repository::open(repo_path)?;

    // Current branch
    let branch = get_current_branch(&repo);

    // Last commit info
    let (last_hash, last_message, last_date) = get_last_commit_info(&repo);

    // Working tree status
    let (is_dirty, modified, untracked) = get_working_tree_status(&repo);

    // Remote origin
    let remote_url = get_remote_url(&repo);

    Ok(GitInfo {
        branch,
        last_commit_hash: last_hash,
        last_commit_message: last_message,
        last_commit_date: last_date,
        is_dirty,
        modified_count: modified,
        untracked_count: untracked,
        remote_url,
    })
}

fn get_current_branch(repo: &Repository) -> String {
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from))
        .unwrap_or_else(|| "detached".to_string())
}

fn get_last_commit_info(repo: &Repository) -> (String, String, String) {
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => return ("none".to_string(), "no commits".to_string(), String::new()),
    };

    let commit = match head.peel_to_commit() {
        Ok(commit) => commit,
        Err(_) => return ("none".to_string(), "no commits".to_string(), String::new()),
    };

    let hash = commit.id().to_string().chars().take(7).collect::<String>();

    let message = commit
        .message()
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

    let timestamp = commit.time().seconds();
    let date = chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();

    (hash, message, date)
}

fn get_working_tree_status(repo: &Repository) -> (bool, usize, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => statuses,
        Err(_) => return (false, 0, 0),
    };

    let mut modified = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.contains(git2::Status::WT_MODIFIED)
            || status.contains(git2::Status::INDEX_MODIFIED)
            || status.contains(git2::Status::WT_DELETED)
            || status.contains(git2::Status::INDEX_DELETED)
            || status.contains(git2::Status::WT_RENAMED)
            || status.contains(git2::Status::INDEX_RENAMED)
            || status.contains(git2::Status::WT_TYPECHANGE)
            || status.contains(git2::Status::INDEX_TYPECHANGE)
        {
            modified += 1;
        }
        if status.contains(git2::Status::WT_NEW) {
            untracked += 1;
        }
    }

    let is_dirty = modified > 0 || untracked > 0;

    (is_dirty, modified, untracked)
}

fn get_remote_url(repo: &Repository) -> Option<String> {
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url()?;
    Some(sanitize_remote_url(url))
}

/// Remove tokens and credentials from remote URLs.
fn sanitize_remote_url(url: &str) -> String {
    // Handle SSH URLs: git@github.com:user/repo.git
    if url.starts_with("git@") {
        return url.to_string();
    }

    // Handle HTTPS URLs: https://token@github.com/...
    if let Some(at_pos) = url.find('@') {
        let scheme_end = url.find("://").unwrap_or(0) + 3;
        if at_pos > scheme_end {
            let prefix = &url[..scheme_end];
            let rest = &url[at_pos..];
            return format!("{}***@{}", prefix, rest.trim_start_matches('@'));
        }
    }

    url.to_string()
}

/// Check if a path is a Git repository.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}
