mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::Connection;

use cli::Cli;
use sw_galaxy_map_sync::{SyncOptions, run_sync};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let delimiter = cli
        .delimiter
        .to_string()
        .as_bytes()
        .first()
        .copied()
        .context("Invalid delimiter")?;

    let mut conn = Connection::open(&cli.db)
        .with_context(|| format!("Unable to open DB: {}", cli.db.display()))?;

    let opts = SyncOptions {
        csv: cli.csv,
        table: cli.table,
        delimiter,
        dry_run: cli.dry_run,
        mark_deleted: cli.mark_deleted,
        report_path: Some("sync_report.xlsx".to_string()),
    };

    let result = run_sync(&mut conn, &opts)?;

    println!();
    println!("Done.");
    println!("Inserted         : {}", result.stats.inserted);
    println!("Updated exact    : {}", result.stats.updated_exact);
    println!("Updated suffix   : {}", result.stats.updated_suffix);
    println!("Invalid CSV rows : {}", result.stats.invalid_csv_rows);
    println!("Marked invalid   : {}", result.stats.invalid_marked);
    println!("Skipped DB       : {}", result.stats.skipped_db);
    println!("Logically deleted: {}", result.stats.deleted_logically);

    Ok(())
}
