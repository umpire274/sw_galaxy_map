mod cli;
mod db;
mod models;
mod report;
mod sync;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::Connection;

use crate::cli::Cli;
use crate::models::{InsertDefaults, ReportKind, SyncReport, SyncStats};
use crate::report::push_report_row;
use crate::sync::{load_csv, make_report_row, mark_deleted_not_in_csv, sync_row};
use crate::utils::{is_invalid_csv_row, make_progress_bar};

/// Run the synchronization process from CSV to SQLite.
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

    let rows = load_csv(&cli.csv, delimiter)
        .with_context(|| format!("Unable to load CSV: {}", cli.csv.display()))?;

    let tx = conn.transaction()?;
    let mut stats = SyncStats::default();
    let defaults = InsertDefaults::default();
    let mut report = SyncReport::default();
    let sync_pb = make_progress_bar(rows.len() as u64, "Sync process")?;

    println!();

    for row in &rows {
        if is_invalid_csv_row(row) {
            stats.invalid_csv_rows += 1;

            push_report_row(&mut report, make_report_row(None, row), ReportKind::Invalid);

            if !row.system.is_empty()
                && !cli.dry_run
                && let Some(existing) = db::find_exact_match(&tx, &cli.table, &row.system)?
            {
                db::set_status(&tx, &cli.table, existing.fid, "invalid")?;
                stats.invalid_marked += 1;
            }

            sync_pb.inc(1);
            continue;
        }

        sync_row(
            &tx,
            &cli.table,
            row,
            &defaults,
            cli.dry_run,
            &mut stats,
            &mut report,
        )?;

        sync_pb.inc(1);
    }

    sync_pb.finish_with_message("Sync process completed");

    if cli.mark_deleted {
        mark_deleted_not_in_csv(&tx, &cli.table, &rows, cli.dry_run, &mut stats, &mut report)?;
    }

    if cli.dry_run {
        tx.rollback()?;
    } else {
        tx.commit()?;
    }

    report::write_report("sync_report.xlsx", &report)?;

    println!();
    println!("Done.");
    println!("Inserted         : {}", stats.inserted);
    println!("Updated exact    : {}", stats.updated_exact);
    println!("Updated suffix   : {}", stats.updated_suffix);
    println!("Invalid CSV rows : {}", stats.invalid_csv_rows);
    println!("Marked invalid   : {}", stats.invalid_marked);
    println!("Skipped DB       : {}", stats.skipped_db);
    println!("Logically deleted: {}", stats.deleted_logically);

    Ok(())
}
