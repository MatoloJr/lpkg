use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "lpkg",
    about = "A winget-like package orchestrator for Ubuntu/Debian Linux",
    version = env!("CARGO_PKG_VERSION"),
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for packages across all enabled backends
    Search {
        /// One or more search terms (searches all backends for each)
        #[arg(required = true, num_args = 1..)]
        queries: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        source: Option<Vec<String>>,
    },
    /// Install package(s) via the best matching backend
    Install {
        /// Package id(s) to install
        #[arg(num_args = 0..)]
        ids: Vec<String>,
        /// Install every package from a JSON manifest (default: packages.json)
        #[arg(long, num_args = 0..=1, default_missing_value = "packages.json")]
        all: Option<PathBuf>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        version: Option<String>,
        /// Remove any existing install of the same package before installing
        #[arg(short = 'a', long)]
        auto: bool,
    },
    /// List installed packages
    List {
        #[arg(long)]
        outdated: bool,
        #[arg(long)]
        duplicates: bool,
        #[arg(long)]
        all: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
        #[arg(long, value_delimiter = ',')]
        source: Option<Vec<String>>,
    },
    /// Upgrade packages
    Upgrade {
        /// Package id to upgrade (omit to upgrade all outdated)
        id: Option<String>,
        /// Run each backend's full upgrade_all
        #[arg(long)]
        all: bool,
        /// Upgrade only outdated packages / skip if already current
        #[arg(short = 'a', long)]
        auto: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        source: Option<String>,
    },
    /// Uninstall a package
    Uninstall {
        id: String,
        #[arg(long)]
        purge: bool,
        #[arg(long)]
        source: Option<String>,
    },
    /// Clean up old versions and unused packages
    Cleanup {
        #[arg(long, value_delimiter = ',')]
        source: Option<Vec<String>>,
    },
    /// Show installed copies and search candidates for a package
    Info {
        id: String,
    },
    /// Show which backend(s) provide an installed package
    Which {
        id: String,
    },
    /// Refresh inventory cache
    Scan,
    /// Health checks
    Doctor,
    /// Pin management
    Pin {
        #[command(subcommand)]
        action: PinCommands,
    },
    /// Export installed packages to JSON manifest
    Export {
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Import packages from JSON manifest
    Import {
        file: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum PinCommands {
    /// Pin a package to prevent upgrades
    Add {
        id: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Remove a pin
    Remove {
        id: String,
        #[arg(long)]
        source: Option<String>,
    },
    /// List pinned packages
    List,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}
