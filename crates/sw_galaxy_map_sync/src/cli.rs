use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for the synchronization tool.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// Path to the SQLite database.
    #[arg(long)]
    pub db: PathBuf,

    /// Path to the official CSV file.
    #[arg(long)]
    pub csv: PathBuf,

    /// Target table name.
    #[arg(long, default_value = "planets")]
    pub table: String,

    /// CSV delimiter. Use ';' for semicolon-separated files.
    #[arg(long, default_value = ",")]
    pub delimiter: char,

    /// Perform a dry run without changing the database.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Mark records not present in CSV as deleted.
    #[arg(long, default_value_t = false)]
    pub mark_deleted: bool,
}
