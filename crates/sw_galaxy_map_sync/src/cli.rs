use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::models::CsvOverlayFormat;

/// ArcGIS-first synchronization and ingestion tool.
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// ArcGIS data ingestion commands.
    #[command(subcommand)]
    Arcgis(ArcgisCommand),

    /// CSV overlay commands.
    #[command(subcommand)]
    Csv(CsvCommand),
}

/// ArcGIS command group.
#[derive(Debug, Subcommand)]
pub enum ArcgisCommand {
    /// Download ArcGIS data and write normalized JSON.
    Fetch {
        /// Output JSON file.
        #[arg(long)]
        out: PathBuf,

        /// ArcGIS page size.
        #[arg(long, default_value_t = 2000)]
        page_size: i64,

        /// Pretty-print JSON.
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Download ArcGIS data and import/upsert it into SQLite.
    Import {
        /// SQLite database path.
        #[arg(long)]
        db: PathBuf,

        /// Target known planets table.
        #[arg(long, default_value = "planets")]
        table: String,

        /// Target unknown planets table.
        #[arg(long, default_value = "planets_unknown")]
        unknown_table: String,

        /// ArcGIS page size.
        #[arg(long, default_value_t = 2000)]
        page_size: i64,

        /// Run without applying database changes.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

/// CSV command group.
#[derive(Debug, Subcommand)]
pub enum CsvCommand {
    /// Apply a curated CSV overlay over the ArcGIS baseline.
    Overlay {
        /// SQLite database path.
        #[arg(long)]
        db: PathBuf,

        /// CSV overlay file.
        #[arg(long)]
        csv: PathBuf,

        /// Target known planets table.
        #[arg(long, default_value = "planets")]
        table: String,

        /// Target unknown planets table.
        #[arg(long, default_value = "planets_unknown")]
        unknown_table: String,

        /// CSV format: auto, official, full.
        #[arg(long, default_value = "auto")]
        format: CsvOverlayFormat,

        /// CSV delimiter. If omitted, it is detected from the header.
        #[arg(long)]
        delimiter: Option<String>,

        /// Mark DB records not present in the CSV as deleted/skipped.
        #[arg(long, default_value_t = false)]
        mark_deleted: bool,

        /// Run without applying database changes.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}
