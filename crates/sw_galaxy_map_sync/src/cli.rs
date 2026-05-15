use crate::models::CsvOverlayFormat;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DbDriverArg {
    Sqlite,
    Postgres,
    Mysql,
}

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

    /// Database Operations commands.
    #[command(subcommand)]
    Db(DbCommands),
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
        /// Database backend driver.
        #[arg(long, value_enum, default_value_t = DbDriverArg::Sqlite)]
        driver: DbDriverArg,

        /// SQLite database path.
        #[arg(long)]
        db: Option<PathBuf>,

        /// JSON database configuration for remote backends.
        #[arg(long)]
        db_config: Option<PathBuf>,

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
        /// Database backend driver.
        #[arg(long, value_enum, default_value_t = DbDriverArg::Sqlite)]
        driver: DbDriverArg,

        /// SQLite database path.
        #[arg(long)]
        db: Option<PathBuf>,

        /// JSON database configuration for remote backends.
        #[arg(long)]
        db_config: Option<PathBuf>,

        /// CSV input path.
        #[arg(long)]
        csv: PathBuf,

        /// CSV format.
        #[arg(long, value_enum)]
        format: Option<CsvOverlayFormat>,

        /// CSV delimiter.
        #[arg(long)]
        delimiter: Option<char>,

        /// Target known planets table.
        #[arg(long, default_value = "planets")]
        table: String,

        /// Target unknown planets table.
        #[arg(long, default_value = "planets_unknown")]
        unknown_table: String,

        /// Mark DB rows not represented by the CSV overlay as deleted.
        #[arg(long, default_value_t = false)]
        mark_deleted: bool,

        /// Run without applying database changes.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

/// Enum representing commands for interacting with the database.
///
/// This enum is used to define the different commands that can be executed on the database.
/// It contains a single variant called `Test`, which takes a database configuration path as an argument.
/// The database configuration path is used to specify the location and settings of the database to be tested.
/// The purpose of this command is to test the database connection and ensure that it is working correctly.
/// It is important to note that this command should only be used for testing purposes and should not be used in a production environment.
#[derive(Subcommand, Debug)]
pub enum DbCommands {
    Test {
        #[arg(long)]
        db_config: PathBuf,
    },

    Bootstrap {
        #[arg(long, value_enum, default_value_t = DbDriverArg::Sqlite)]
        driver: DbDriverArg,

        #[arg(long)]
        db: Option<PathBuf>,

        #[arg(long)]
        db_config: Option<PathBuf>,
    },
}
