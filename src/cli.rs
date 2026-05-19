use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "devscope",
    version,
    about = "Fast local project scanner and TUI dashboard",
    long_about = "Fast local project scanner and TUI dashboard.\n\nRun without a subcommand to launch the interactive TUI, or pass a path like `devscope .` to scan only that directory tree for the current session.",
    after_help = "Examples:\n  devscope\n  devscope .\n  devscope scan\n  devscope list --json\n  devscope add-root C:\\Users\\me\\projects\n  devscope note devscope \"Needs perf review\"\n  devscope status devscope active\n  devscope discover --apply"
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
        /// Output projects as JSON
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
        /// Project identifier: exact name, partial name, or path
        project: String,
        /// Note text
        text: String,
    },

    /// Set project status
    Status {
        /// Project identifier: exact name, partial name, or path
        project: String,
        /// New status: active, paused, stale, or archived
        new_status: String,
    },

    /// Show the path to config.toml
    Config {
        /// Print the config path with a note to edit it manually
        #[arg(long)]
        edit: bool,
    },

    /// Resolve a project and print its path
    Open {
        /// Project identifier: exact name, partial name, or path
        project: String,
    },

    /// Discover possible project roots automatically
    Discover {
        /// Add discovered roots to config.toml
        #[arg(long)]
        apply: bool,
    },
}
