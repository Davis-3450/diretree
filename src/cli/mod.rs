use std::path::PathBuf;

use clap::Parser;

pub mod json;
pub mod report;

#[derive(Parser, Debug)]
#[command(name = "diretree", about = "Disk usage analyzer — TUI + CLI", version)]
pub struct Cli {
    /// Path to scan (defaults to current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output mode
    #[arg(long, value_enum, default_value_t = Mode::Tui)]
    pub mode: Mode,

    /// Show only top-N largest entries (report/json)
    #[arg(long, default_value_t = 20)]
    pub top: usize,

    /// Follow symbolic links (bounded by --max-depth)
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Do not skip well-known dirs (target, node_modules, venv, .git, ...)
    #[arg(long)]
    pub no_ignore: bool,

    /// Directory for the scan-result cache
    #[arg(long, default_value = "./cache/diretree")]
    pub cache_dir: PathBuf,

    /// Disable the on-disk scan cache
    #[arg(long)]
    pub no_cache: bool,

    /// Max recursion depth for symlink-following / scan
    #[arg(long, default_value_t = 32)]
    pub max_depth: u32,

    /// Minimum size in bytes to include in report (0 = all)
    #[arg(long, default_value_t = 0)]
    pub min_size: u64,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq)]
pub enum Mode {
    /// Interactive terminal UI (default)
    #[default]
    Tui,
    /// Plain-text report
    Report,
    /// JSON dump
    Json,
}

pub fn parse() -> Cli {
    Cli::parse()
}
