use crate::db::sqlite::{SqlitePlanetRepository, ensure_required_schema, upsert_meta_json};
use crate::models::{CsvOverlayFormat, CsvOverlayOutcome, CsvOverlayStats};
use crate::sources::csv::load_overlay_csv;
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
use std::path::PathBuf;

/// Options for the official CSV overlay pipeline.
#[derive(Debug, Clone)]
pub struct CsvOverlayOptions {
    pub db: PathBuf,
    pub csv: PathBuf,
    pub table: String,
    pub unknown_table: String,
    pub format: CsvOverlayFormat,
    pub delimiter: Option<u8>,
    pub dry_run: bool,
    pub mark_deleted: bool,
}

/// Apply a curated CSV overlay on top of the ArcGIS baseline.
pub fn apply_csv_overlay(options: &CsvOverlayOptions) -> Result<CsvOverlayStats> {
    let rows = load_overlay_csv(&options.csv, options.format, options.delimiter)?;

    let mut stats = CsvOverlayStats {
        rows_read: rows.len(),
        dry_run: options.dry_run,
        ..CsvOverlayStats::default()
    };

    if options.dry_run {
        let conn = Connection::open(&options.db)
            .with_context(|| format!("Unable to open DB: {}", options.db.display()))?;
        ensure_required_schema(&conn, &options.table, &options.unknown_table)?;
        let repo = SqlitePlanetRepository::new(&conn, &options.table);
        repo.ensure_table_exists()?;

        for row in &rows {
            match repo.apply_csv_overlay_row(row, true)? {
                CsvOverlayOutcome::Inserted => stats.inserted += 1,
                CsvOverlayOutcome::Active => stats.active += 1,
                CsvOverlayOutcome::ModifiedExact => stats.modified_exact += 1,
                CsvOverlayOutcome::ModifiedSuffix => stats.modified_suffix += 1,
            }
        }

        if options.mark_deleted {
            let (deleted, skipped) = repo.mark_deleted_not_in_csv(&rows, true)?;
            stats.deleted = deleted;
            stats.skipped = skipped;
        }

        return Ok(stats);
    }

    let mut conn = Connection::open(&options.db)
        .with_context(|| format!("Unable to open DB: {}", options.db.display()))?;

    ensure_required_schema(&conn, &options.table, &options.unknown_table)?;

    let tx = conn.transaction()?;
    {
        let repo = SqlitePlanetRepository::new(&tx, &options.table);
        repo.ensure_table_exists()?;

        for row in &rows {
            match repo.apply_csv_overlay_row(row, false)? {
                CsvOverlayOutcome::Inserted => stats.inserted += 1,
                CsvOverlayOutcome::Active => stats.active += 1,
                CsvOverlayOutcome::ModifiedExact => stats.modified_exact += 1,
                CsvOverlayOutcome::ModifiedSuffix => stats.modified_suffix += 1,
            }
        }

        if options.mark_deleted {
            let (deleted, skipped) = repo.mark_deleted_not_in_csv(&rows, false)?;
            stats.deleted = deleted;
            stats.skipped = skipped;
        }
    }
    tx.commit()?;

    if !options.dry_run {
        let meta = json!({
            "crate_version": env!("CARGO_PKG_VERSION"),
            "operation": "csv_overlay",
            "completed_at_utc": chrono::Utc::now().to_rfc3339(),
            "csv_path": options.csv.display().to_string(),
            "format": format!("{:?}", options.format),
            "delimiter": options
                .delimiter
                .map(char::from)
                .map(|c| c.to_string()),
            "rows_read": stats.rows_read,
            "inserted": stats.inserted,
            "active": stats.active,
            "modified_exact": stats.modified_exact,
            "modified_suffix": stats.modified_suffix,
            "deleted": stats.deleted,
            "skipped": stats.skipped,
            "mark_deleted": options.mark_deleted,
            "dry_run": stats.dry_run,
        });

        upsert_meta_json(&conn, "sync.csv_overlay.last_run", &meta)?;
    }

    Ok(stats)
}
