use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "devscope",
    version,
    about = "Fast local project scanner and TUI dashboard"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Scan configured roots and display summary
    Scan,

    /// List projects in text or JSON format
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new root directory to scan
    AddRoot {
        /// Path to add as root
        path: String,
    },

    /// Remove a root directory
    RemoveRoot {
        /// Path to remove
        path: String,
    },

    /// Show configured root directories
    Roots,

    /// Add or update a note for a project
    Note {
        /// Project path or name
        project: String,
        /// Note text
        text: String,
    },

    /// Set project status
    Status {
        /// Project path or name
        project: String,
        /// New status: active, paused, stale, archived
        new_status: String,
    },

    /// Show path to config file
    Config {
        /// Open config in editor
        #[arg(long)]
        edit: bool,
    },

    /// Open project folder (prints path in MVP)
    Open {
        /// Project path or name
        project: String,
    },

    /// Discover possible project roots automatically
    Discover {
        /// Apply discovered roots to config
        #[arg(long)]
        apply: bool,
    },
}
