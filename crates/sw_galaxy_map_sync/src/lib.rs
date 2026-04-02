pub mod db;
pub mod models;
pub mod report;
pub mod sync;
pub mod utils;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

use crate::models::{InsertDefaults, ReportKind, SyncReport, SyncStats};
use crate::report::push_report_row;
use crate::sync::{load_csv, make_report_row, mark_deleted_not_in_csv, sync_row};
use crate::utils::{is_invalid_csv_row, make_progress_bar};

/// Options for `run_sync()`.
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Path to the official CSV file.
    pub csv: std::path::PathBuf,
    /// Target table name (default: "planets").
    pub table: String,
    /// CSV delimiter byte (default: b',').
    pub delimiter: u8,
    /// Perform a dry run without changing the database.
    pub dry_run: bool,
    /// Mark records not present in CSV as deleted.
    pub mark_deleted: bool,
    /// Path for the XLSX report (None = skip report).
    pub report_path: Option<String>,
}

/// Outcome counters returned by `run_sync()`.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub stats: SyncStats,
}

/// Run the synchronization process on an already-open connection.
///
/// This is the library entry point used by `db sync` in the main CLI.
/// The caller is responsible for opening the DB and (optionally)
/// rebuilding search indexes afterward.
pub fn run_sync(conn: &mut Connection, opts: &SyncOptions) -> Result<SyncResult> {
    let csv_path = std::path::PathBuf::from(&opts.csv);
    let rows = load_csv(&csv_path, opts.delimiter)
        .with_context(|| format!("Unable to load CSV: {}", csv_path.display()))?;

    let tx = conn.transaction()?;
    let mut stats = SyncStats::default();
    let defaults = InsertDefaults::default();
    let mut report_data = SyncReport::default();
    let sync_pb = make_progress_bar(rows.len() as u64, "Sync process")?;

    for row in &rows {
        if is_invalid_csv_row(row) {
            stats.invalid_csv_rows += 1;

            push_report_row(
                &mut report_data,
                make_report_row(None, row),
                ReportKind::Invalid,
            );

            if !row.system.is_empty()
                && !opts.dry_run
                && let Some(existing) = db::find_exact_match(&tx, &opts.table, &row.system)?
            {
                db::set_status(&tx, &opts.table, existing.fid, "invalid")?;
                stats.invalid_marked += 1;
            }

            sync_pb.inc(1);
            continue;
        }

        sync_row(
            &tx,
            &opts.table,
            row,
            &defaults,
            opts.dry_run,
            &mut stats,
            &mut report_data,
        )?;

        sync_pb.inc(1);
    }

    sync_pb.finish_with_message("Sync process completed");

    if opts.mark_deleted {
        mark_deleted_not_in_csv(
            &tx,
            &opts.table,
            &rows,
            opts.dry_run,
            &mut stats,
            &mut report_data,
        )?;
    }

    if opts.dry_run {
        tx.rollback()?;
    } else {
        tx.commit()?;
    }

    if let Some(ref path) = opts.report_path {
        report::write_report(path, &report_data)?;
    }

    Ok(SyncResult { stats })
}

/// Convenience: resolve a CSV path, checking it exists.
pub fn resolve_csv_path(path: &str) -> Result<std::path::PathBuf> {
    let p = Path::new(path);
    if !p.exists() {
        anyhow::bail!("CSV file not found: {}", p.display());
    }
    Ok(p.to_path_buf())
}
