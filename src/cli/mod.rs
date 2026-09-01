use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "lpkg", about = "A winget-like package orchestrator for Ubuntu/Debian Linux")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search for packages across all backends
    Search {
        query: String,
        #[arg(long, value_delimiter = ',')]
        source: Option<Vec<String>>,
    },
    /// Install a package
    Install {
        id: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        version: Option<String>,
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
        id: Option<String>,
        #[arg(long)]
        all: bool,
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
