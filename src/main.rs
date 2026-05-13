mod app;
mod artifacts;
mod cli;
mod commands;
mod config;
mod detect;
mod discover;
mod git;
mod health;
mod input;
mod project;
mod scanner;
mod scoring;
mod tui;
mod ui;

use std::path::Path;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Commands};
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            let mut config = config::load_config()?;
            let had_roots = !config.roots.is_empty();
            if config.roots.is_empty() {
                ensure_roots_or_auto_discover(&mut config)?;
            }
            if config.roots.is_empty() && !had_roots {
                eprintln!(
                    "No project roots found automatically. Use `devscope add-root <path>` or `devscope discover`."
                );
            }
            tui::run_tui(config)?;
            Ok(())
        }
        Some(cmd) => match cmd {
            Commands::Scan => cmd_scan(),
            Commands::List { json } => cmd_list(json),
            Commands::AddRoot { path } => cmd_add_root(path),
            Commands::RemoveRoot { path } => cmd_remove_root(path),
            Commands::Roots => cmd_roots(),
            Commands::Note { project, text } => cmd_note(project, text),
            Commands::Status {
                project,
                new_status,
            } => cmd_status(project, new_status),
            Commands::Config { edit } => cmd_config(edit),
            Commands::Open { project } => cmd_open(project),
            Commands::Discover { apply } => cmd_discover(apply),
        },
    }
}

fn cmd_scan() -> Result<()> {
    let mut config = config::load_config()?;
    ensure_roots_or_auto_discover(&mut config)?;
    let result = scanner::scan_roots(&config)?;

    println!(
        "Scanned {} projects in {}ms\n",
        result.projects_found, result.duration_ms
    );

    for project in &result.projects {
        let stack = project.stack.join(", ");
        let status = project.status.as_str();
        let activity = &project.activity.relative_time;
        let git_info = match &project.git {
            Some(g) => {
                if g.is_dirty {
                    format!("{} (dirty)", g.branch)
                } else {
                    g.branch.clone()
                }
            }
            None => "no git".to_string(),
        };

        println!(
            "  {:<30} {:<30} {:<10} {:<10} {}",
            project.name, stack, activity, status, git_info
        );
    }

    Ok(())
}

fn cmd_list(json_output: bool) -> Result<()> {
    let mut config = config::load_config()?;
    ensure_roots_or_auto_discover(&mut config)?;
    let result = scanner::scan_roots(&config)?;

    if json_output {
        let json = serde_json::to_string_pretty(&result.projects)?;
        println!("{}", json);
    } else {
        for project in &result.projects {
            let stack = project.stack.join(", ");
            let path = project.path.display();
            let status = project.status.as_str();
            let activity = &project.activity.relative_time;

            println!(
                "{} [{}] ({}) {} - {}",
                project.name, stack, status, activity, path
            );
        }
    }

    Ok(())
}

fn cmd_add_root(path: String) -> Result<()> {
    let mut config = config::load_config()?;
    let expanded = config::expand_tilde(&path);
    let normalized = config::normalize_path(&expanded);
    let path_str = normalized.to_string_lossy().to_string();

    if config.roots.contains(&path_str) {
        println!("Root already exists: {}", path_str);
        return Ok(());
    }

    if !normalized.exists() {
        println!("Warning: path does not exist: {}", path_str);
    }

    config.roots.push(path_str.clone());
    config::save_config(&config)?;
    println!("Added root: {}", path_str);

    Ok(())
}

fn cmd_remove_root(path: String) -> Result<()> {
    let mut config = config::load_config()?;
    let expanded = config::expand_tilde(&path);
    let normalized = config::normalize_path(&expanded);
    let path_str = normalized.to_string_lossy().to_string();

    let initial_len = config.roots.len();
    config.roots.retain(|r| r != &path_str);

    if config.roots.len() == initial_len {
        println!("Root not found: {}", path_str);
    } else {
        config::save_config(&config)?;
        println!("Removed root: {}", path_str);
    }

    Ok(())
}

fn cmd_roots() -> Result<()> {
    let config = config::load_config()?;

    if config.roots.is_empty() {
        println!("No roots configured. Use 'devscope add-root <path>' to add one.");
        return Ok(());
    }

    println!("Configured roots:");
    for root in &config.roots {
        let expanded = config::expand_tilde(root);
        let exists = expanded.exists();
        let marker = if exists { "✓" } else { "✗" };
        println!("  {} {}", marker, root);
    }

    Ok(())
}

fn cmd_note(project: String, text: String) -> Result<()> {
    let mut config = config::load_config()?;
    ensure_roots_or_auto_discover(&mut config)?;
    let resolved = find_project_path(&config, &project)?;
    config::set_note(&mut config, &resolved, text.clone());
    config::record_open(&mut config, &resolved);
    config::save_config(&config)?;
    println!("Note set for {}: {}", resolved, text);
    Ok(())
}

fn cmd_status(project: String, new_status: String) -> Result<()> {
    let mut config = config::load_config()?;
    ensure_roots_or_auto_discover(&mut config)?;
    let resolved = find_project_path(&config, &project)?;
    let status = project::ProjectStatus::from_str(&new_status);
    config::set_project_status(&mut config, &resolved, status.clone());
    config::record_open(&mut config, &resolved);
    config::save_config(&config)?;
    println!("Status set for {}: {}", resolved, status.as_str());
    Ok(())
}

fn cmd_config(edit: bool) -> Result<()> {
    let path = config::config_path()?;
    if edit {
        // MVP: just print the path. Don't use shell commands.
        println!("Config path: {}", path.display());
        println!("Open this file in your editor to modify settings.");
    } else {
        println!("{}", path.display());
    }
    Ok(())
}

fn cmd_open(project: String) -> Result<()> {
    let mut config = config::load_config()?;
    ensure_roots_or_auto_discover(&mut config)?;
    let resolved = find_project_path(&config, &project)?;

    config::record_open(&mut config, &resolved);
    let _ = config::save_config(&config);

    // MVP: just print the path
    println!("{}", resolved);

    // Future: use platform-specific safe opener

    Ok(())
}

fn cmd_discover(apply: bool) -> Result<()> {
    let config = config::load_config()?;
    let discovered = discover::discover_roots(&config)?;

    if discovered.is_empty() {
        println!("No project roots found in common locations.");
        println!("Try:");
        println!("  devscope add-root \"C:\\path\\to\\your\\projects\"");
        return Ok(());
    }

    println!("Discovered possible project roots:\n");
    for root in &discovered {
        println!(
            "  {:<8} {:>3} projects   {}",
            root.confidence.as_str(),
            root.project_count,
            root.path.display()
        );
    }

    if !apply {
        println!("\nRun `devscope discover --apply` to add these roots.");
        return Ok(());
    }

    let mut config = config::load_config()?;
    let mut added: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for root in &discovered {
        let path_str = root.path.to_string_lossy().to_string();
        let normalized = discover::normalize_for_compare(&root.path);

        let already_exists = config
            .roots
            .iter()
            .any(|r| discover::normalize_for_compare(Path::new(r)) == normalized);

        if already_exists {
            skipped.push(path_str);
        } else {
            config.roots.push(path_str.clone());
            added.push(path_str);
        }
    }

    if !added.is_empty() {
        config::save_config(&config)?;
        println!("\nAdded {} roots:", added.len());
        for path in &added {
            println!("  {}", path);
        }
    }

    if !skipped.is_empty() {
        println!("\nSkipped {} existing roots:", skipped.len());
        for path in &skipped {
            println!("  {}", path);
        }
    }

    Ok(())
}

fn ensure_roots_or_auto_discover(config: &mut Config) -> Result<()> {
    if !config.roots.is_empty() {
        return Ok(());
    }

    let discovered = discover::discover_roots(config)?;

    if discovered.is_empty() {
        return Ok(());
    }

    for root in &discovered {
        let path_str = root.path.to_string_lossy().to_string();
        let normalized = discover::normalize_for_compare(&root.path);

        let already_exists = config
            .roots
            .iter()
            .any(|r| discover::normalize_for_compare(Path::new(r)) == normalized);

        if !already_exists {
            config.roots.push(path_str);
        }
    }

    config::save_config(config)?;
    eprintln!(
        "No roots configured. Auto-discovered {} roots.",
        discovered.len()
    );

    Ok(())
}

/// Resolve a project identifier (name or path) to its full path.
fn find_project_path(config: &config::Config, identifier: &str) -> Result<String> {
    // If it looks like a path, use it directly
    if identifier.contains('/') || identifier.contains('\\') {
        let expanded = config::expand_tilde(identifier);
        let normalized = config::normalize_path(&expanded);
        return Ok(normalized.to_string_lossy().to_string());
    }

    // Otherwise, scan and find by name
    let result = scanner::scan_roots(config)?;
    let identifier_lower = identifier.to_lowercase();

    let matches: Vec<_> = result
        .projects
        .iter()
        .filter(|p| p.name.to_lowercase() == identifier_lower)
        .collect();

    match matches.len() {
        0 => {
            // Try partial match
            let partial: Vec<_> = result
                .projects
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&identifier_lower))
                .collect();

            if partial.is_empty() {
                anyhow::bail!("No project found matching '{}'", identifier);
            }
            if partial.len() > 1 {
                println!("Multiple matches found:");
                for p in partial {
                    println!("  {} ({})", p.name, p.path.display());
                }
                anyhow::bail!("Please be more specific");
            }
            Ok(partial[0].path.to_string_lossy().to_string())
        }
        1 => Ok(matches[0].path.to_string_lossy().to_string()),
        _ => {
            // Multiple exact matches (unlikely but possible)
            println!("Multiple exact matches:");
            for p in matches {
                println!("  {} ({})", p.name, p.path.display());
            }
            anyhow::bail!("Please use the full path instead");
        }
    }
}
