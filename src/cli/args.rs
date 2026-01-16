use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sw_galaxy_map",
    version,
    about = "CLI to query the Star Wars galaxy map (SQLite)"
)]
pub struct Cli {
    /// Path to the SQLite database
    #[arg(long)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search planets by text (uses FTS if available, otherwise LIKE)
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },

    /// Print all available information about a planet
    Info { planet: String },

    /// Find nearby planets within a radius (parsecs) using Euclidean distance on X/Y
    Near {
        /// Radius (parsecs)
        #[arg(long)]
        r: f64,

        /// Center the search around a planet (by name)
        #[arg(long)]
        planet: Option<String>,

        /// X coordinate (parsecs), if --planet is not used
        #[arg(long)]
        x: Option<f64>,

        /// Y coordinate (parsecs), if --planet is not used
        #[arg(long)]
        y: Option<f64>,

        #[arg(long, default_value_t = 20)]
        limit: i64,
    },

    /// Database provisioning commands (C2: build local DB from remote data source)
    Db {
        #[command(subcommand)]
        cmd: DbCommands,
    },
}

#[derive(Subcommand)]
pub enum DbCommands {
    /// Initialize the local SQLite database by downloading data from the remote service
    Init {
        /// Output path for the generated SQLite database (defaults to OS app data dir)
        #[arg(long)]
        out: Option<String>,

        /// Overwrite existing database if present
        #[arg(long, default_value_t = false)]
        force: bool,
    },

    /// Show local database status (path, meta, counts)
    Status,
    Update {
        /// Permanently remove planets marked as deleted
        #[arg(long)]
        prune: bool,

        /// Perform a dry run without modifying the database
        #[arg(long)]
        dry_run: bool,

        #[arg(long)]
        stats: bool,

        #[arg(long, default_value_t = 10)]
        stats_limit: usize,
    },
}
