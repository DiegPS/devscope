use std::path::Path;

use anyhow::Result;
use git2::{Repository, StatusOptions};

use crate::project::{DirtyStatus, GitInfo};

/// Extract basic Git info (branch, remote, commits) without scanning the working tree.
pub fn get_git_info_fast(repo_path: &Path) -> Result<GitInfo> {
    let repo = Repository::open(repo_path)?;

    let branch = get_current_branch(&repo);
    let (last_hash, last_message, last_date) = get_last_commit_info(&repo);
    let remote_url = get_remote_url(&repo);

    let upstream = get_upstream_branch(&repo);
    let (ahead, behind) = get_ahead_behind(&repo, &upstream);

    let has_remote = remote_url.is_some();
    let (remote_host, remote_repo) = parse_remote_info(remote_url.as_deref());

    Ok(GitInfo {
        branch,
        last_commit_hash: last_hash,
        last_commit_message: last_message,
        last_commit_date: last_date,
        dirty_status: DirtyStatus::Unknown,
        modified_count: None,
        untracked_count: None,
        remote_url,
        upstream,
        ahead,
        behind,
        has_remote,
        remote_host,
        remote_repo,
    })
}

/// Compute the dirty status (expensive — scans working tree).
pub fn get_git_status(repo_path: &Path) -> Result<(DirtyStatus, Option<usize>, Option<usize>)> {
    let repo = Repository::open(repo_path)?;
    let (modified, untracked) = get_working_tree_status(&repo);

    let status = if modified > 0 || untracked > 0 {
        DirtyStatus::Dirty
    } else {
        DirtyStatus::Clean
    };

    Ok((status, Some(modified), Some(untracked)))
}

fn get_current_branch(repo: &Repository) -> String {
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from))
        .unwrap_or_else(|| "detached".to_string())
}

fn get_upstream_branch(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    let branch = git2::Branch::wrap(head);
    let upstream = branch.upstream().ok()?;
    let name = upstream.name().ok()??; // Result<Option<&str>>
    Some(name.to_string())
}

fn get_ahead_behind(
    repo: &Repository,
    upstream_name: &Option<String>,
) -> (Option<usize>, Option<usize>) {
    let upstream_name = match upstream_name {
        Some(name) => name,
        None => return (None, None),
    };

    let head = match repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
        Some(c) => c,
        None => return (None, None),
    };
    let local_oid = head.id();

    let upstream_ref = match repo.find_reference(&format!("refs/remotes/{}", upstream_name)) {
        Ok(r) => r,
        Err(_) => return (None, None),
    };
    let upstream_oid = match upstream_ref.peel_to_commit().ok() {
        Some(c) => c.id(),
        None => return (None, None),
    };

    match repo.graph_ahead_behind(local_oid, upstream_oid) {
        Ok((a, b)) => (Some(a), Some(b)),
        Err(_) => (None, None),
    }
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

fn get_working_tree_status(repo: &Repository) -> (usize, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.include_ignored(false);
    opts.renames_from_rewrites(false);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => statuses,
        Err(_) => return (0, 0),
    };

    let mut modified = 0;
    let mut untracked = 0;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_wt_modified()
            || status.is_index_modified()
            || status.is_wt_deleted()
            || status.is_index_deleted()
            || status.is_wt_renamed()
            || status.is_index_renamed()
            || status.is_wt_typechange()
            || status.is_index_typechange()
        {
            modified += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    (modified, untracked)
}

fn get_remote_url(repo: &Repository) -> Option<String> {
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url()?;
    Some(sanitize_remote_url(url))
}

/// Remove tokens and credentials from remote URLs.
fn sanitize_remote_url(url: &str) -> String {
    if url.starts_with("git@") {
        return url.to_string();
    }

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

fn parse_remote_info(url: Option<&str>) -> (Option<String>, Option<String>) {
    let url = match url {
        Some(u) => u,
        None => return (None, None),
    };
    let s = url.to_string();

    // git@github.com:user/repo.git
    if let Some(rest) = s.strip_prefix("git@") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            let host = parts[0].to_string();
            let repo = parts[1].trim_end_matches(".git").to_string();
            return (Some(host), Some(repo));
        }
    }

    // https://github.com/user/repo.git
    if let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    {
        if let Some(at) = rest.find('@') {
            let rest = &rest[at + 1..];
            let parts: Vec<&str> = rest.splitn(3, '/').collect();
            if parts.len() >= 3 {
                let host = parts[0].to_string();
                let repo = format!("{}/{}", parts[1], parts[2].trim_end_matches(".git"));
                return (Some(host), Some(repo));
            }
        } else {
            let parts: Vec<&str> = rest.splitn(3, '/').collect();
            if parts.len() >= 3 {
                let host = parts[0].to_string();
                let repo = format!("{}/{}", parts[1], parts[2].trim_end_matches(".git"));
                return (Some(host), Some(repo));
            }
        }
    }

    (None, None)
}

/// Check if a path is a Git repository.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Check if a branch name is a mainline branch.
pub fn is_mainline_branch(branch: &str) -> bool {
    matches!(branch, "main" | "master" | "develop" | "dev")
}
