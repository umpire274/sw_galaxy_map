use crate::cli::DbDriverArg;
use crate::db::config::load_db_config;
use crate::db::postgres::{
    apply_csv_overlay_row_postgres, build_postgres_connect_options, count_postgres_planet_rows,
    mark_deleted_not_in_csv_postgres, upsert_meta_struct,
};
use crate::db::sqlite::{SqlitePlanetRepository, ensure_required_schema, upsert_meta_json};
use crate::models::{CsvOverlayFormat, CsvOverlayOutcome, CsvOverlayStats};
use crate::progress::import_progress_bar;
use crate::sources::csv::load_overlay_csv;
use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde_json::json;
use sqlx::{Connection as SqlxConnection, PgConnection};
use std::path::PathBuf;

/// Options for the official CSV overlay pipeline.
#[derive(Debug, Clone)]
pub struct CsvOverlayOptions {
    pub driver: DbDriverArg,
    pub db: Option<PathBuf>,
    pub db_config: Option<PathBuf>,
    pub csv: PathBuf,
    pub table: String,
    pub unknown_table: String,
    pub format: Option<CsvOverlayFormat>,
    pub delimiter: Option<u8>,
    pub dry_run: bool,
    pub mark_deleted: bool,
}

/// Apply a curated CSV overlay on top of the ArcGIS baseline.
///
/// At the moment this function supports the SQLite backend only.
/// PostgreSQL support will be implemented by a dedicated async pipeline.
pub fn apply_csv_overlay(options: &CsvOverlayOptions) -> Result<CsvOverlayStats> {
    match options.driver {
        DbDriverArg::Sqlite => apply_csv_overlay_sqlite(options),
        DbDriverArg::Postgres => {
            bail!("PostgreSQL CSV overlay is not implemented yet")
        }
        DbDriverArg::Mysql => {
            bail!("MySQL backend is not implemented yet")
        }
    }
}

fn apply_csv_overlay_sqlite(options: &CsvOverlayOptions) -> Result<CsvOverlayStats> {
    let db_path = options
        .db
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--db is required when --driver sqlite"))?;

    let rows = load_overlay_csv(&options.csv, options.format, options.delimiter)?;

    let mut stats = CsvOverlayStats {
        rows_read: rows.len(),
        dry_run: options.dry_run,
        ..CsvOverlayStats::default()
    };

    if options.dry_run {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Unable to open DB: {}", db_path.display()))?;

        ensure_required_schema(
            &conn,
            &options.table,
            &options.unknown_table,
            !options.dry_run,
        )?;

        let repo = SqlitePlanetRepository::new(&conn, &options.table);
        repo.ensure_table_exists()?;

        let pb = import_progress_bar(rows.len(), "Dry-running CSV overlay on SQLite...")?;

        for row in &rows {
            match repo.apply_csv_overlay_row(row, true)? {
                CsvOverlayOutcome::Inserted => stats.inserted += 1,
                CsvOverlayOutcome::Active => stats.active += 1,
                CsvOverlayOutcome::ModifiedExact => stats.modified_exact += 1,
                CsvOverlayOutcome::ModifiedSuffix => stats.modified_suffix += 1,
            }

            pb.inc(1);
        }

        pb.finish_with_message("SQLite CSV overlay dry-run completed.");

        if options.mark_deleted {
            let db_rows_count = repo.count_planet_rows()?;

            let delete_pb = import_progress_bar(
                db_rows_count,
                "Dry-running SQLite CSV delete reconciliation...",
            )?;

            let (deleted, skipped) = repo.mark_deleted_not_in_csv(&rows, true, Some(&delete_pb))?;

            delete_pb.finish_with_message("SQLite CSV delete reconciliation dry-run completed.");

            stats.deleted = deleted;
            stats.skipped = skipped;
        }

        return Ok(stats);
    }

    let mut conn = Connection::open(db_path)
        .with_context(|| format!("Unable to open DB: {}", db_path.display()))?;

    ensure_required_schema(
        &conn,
        &options.table,
        &options.unknown_table,
        !options.dry_run,
    )?;

    println!();
    let pb = import_progress_bar(rows.len(), "Applying CSV overlay to SQLite...")?;

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
            pb.inc(1);
        }

        if options.mark_deleted {
            let db_rows_count = repo.count_planet_rows()?;
            let delete_pb = Some(import_progress_bar(
                db_rows_count,
                "Reconciling deleted CSV overlay rows in SQLite...",
            )?);
            let (deleted, skipped) =
                repo.mark_deleted_not_in_csv(&rows, false, delete_pb.as_ref())?;
            if let Some(pb) = delete_pb {
                pb.finish_with_message("SQLite CSV delete reconciliation completed.");
            }
            stats.deleted = deleted;
            stats.skipped = skipped;
        }
    }
    tx.commit()?;

    let format = options
        .format
        .map(|value| value.to_string())
        .unwrap_or_else(|| "auto".to_string());

    let delimiter = options.delimiter.map(char::from).map(|c| c.to_string());

    let meta = json!({
        "crate_version": env!("CARGO_PKG_VERSION"),
        "operation": "csv_overlay",
        "backend": "sqlite",
        "completed_at_utc": chrono::Utc::now().to_rfc3339(),
        "csv_path": options.csv.display().to_string(),
        "format": format,
        "delimiter": delimiter,
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

    upsert_meta_json(&conn, "sync.csv_overlay.sqlite", &meta)?;

    pb.finish_with_message("SQLite ArcGIS import completed.");
    println!();

    Ok(stats)
}

pub async fn apply_csv_overlay_postgres(options: &CsvOverlayOptions) -> Result<CsvOverlayStats> {
    let db_config = options
        .db_config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--db-config is required when --driver postgres"))?;

    let cfg = load_db_config(db_config)?;
    let rows = load_overlay_csv(&options.csv, options.format, options.delimiter)?;

    let mut stats = CsvOverlayStats {
        rows_read: rows.len(),
        dry_run: options.dry_run,
        ..CsvOverlayStats::default()
    };

    let db_options = build_postgres_connect_options(&cfg)?;
    let mut conn = PgConnection::connect_with(&db_options).await?;

    crate::db::schema::postgres::create_postgres_schema(&mut conn).await?;

    let mut tx = conn.begin().await?;

    println!();
    let pb = import_progress_bar(
        rows.len(),
        if options.dry_run {
            "Dry-running CSV overlay on PostgreSQL..."
        } else {
            "Applying CSV overlay to PostgreSQL..."
        },
    )?;

    for row in &rows {
        match apply_csv_overlay_row_postgres(&mut tx, row, options.dry_run, Some(&pb)).await? {
            CsvOverlayOutcome::Inserted => stats.inserted += 1,
            CsvOverlayOutcome::Active => stats.active += 1,
            CsvOverlayOutcome::ModifiedExact => stats.modified_exact += 1,
            CsvOverlayOutcome::ModifiedSuffix => stats.modified_suffix += 1,
        }
    }

    pb.finish_with_message(if options.dry_run {
        "PostgreSQL CSV overlay dry-run completed."
    } else {
        "PostgreSQL CSV overlay completed."
    });
    println!();

    if options.mark_deleted {
        let db_rows_count = count_postgres_planet_rows(&mut tx).await?;

        let delete_pb = import_progress_bar(
            db_rows_count,
            if options.dry_run {
                "Dry-running PostgreSQL CSV delete reconciliation..."
            } else {
                "Reconciling deleted CSV overlay rows in PostgreSQL..."
            },
        )?;

        let (deleted, skipped) =
            mark_deleted_not_in_csv_postgres(&mut tx, &rows, options.dry_run, Some(&delete_pb))
                .await?;

        delete_pb.finish_with_message(if options.dry_run {
            "PostgreSQL CSV delete reconciliation dry-run completed."
        } else {
            "PostgreSQL CSV delete reconciliation completed."
        });

        stats.deleted = deleted;
        stats.skipped = skipped;
    }

    if !options.dry_run {
        let format = options
            .format
            .map(|value| value.to_string())
            .unwrap_or_else(|| "auto".to_string());

        let delimiter = options.delimiter.map(char::from).map(|c| c.to_string());

        let meta = serde_json::json!({
            "crate_version": env!("CARGO_PKG_VERSION"),
            "operation": "csv_overlay",
            "backend": "postgresql",
            "completed_at_utc": chrono::Utc::now().to_rfc3339(),
            "csv_path": options.csv.display().to_string(),
            "format": format,
            "delimiter": delimiter,
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

        upsert_meta_struct(&mut tx, "sync.csv_overlay.postgresql", &meta).await?;
    }

    tx.commit().await?;
    Ok(stats)
}
