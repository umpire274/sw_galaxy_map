use crate::db::config::DbConfig;
use crate::db::postgres::{
    bootstrap_schema, build_postgres_url, upsert_known_arcgis_planet, upsert_unknown_arcgis_planet,
};
use crate::db::sqlite::{SqlitePlanetRepository, ensure_required_schema};
use crate::models::ArcgisImportMeta;
use crate::models::{ArcgisImportResult, UpsertOutcome};
use crate::progress::import_progress_bar;
use crate::sources::arcgis::fetch_arcgis_dataset;
use anyhow::{Context, Result};
use rusqlite::Connection;
use sqlx::{Connection as SqlxConnection, PgConnection};
use std::fs;
use std::path::PathBuf;

/// Options for ArcGIS JSON export.
#[derive(Debug, Clone)]
pub struct ArcgisFetchOptions {
    pub out: PathBuf,
    pub page_size: i64,
    pub pretty: bool,
}

/// Options for ArcGIS SQLite import.
#[derive(Debug, Clone)]
pub struct ArcgisImportOptions {
    pub db: PathBuf,
    pub table: String,
    pub unknown_table: String,
    pub page_size: i64,
    pub dry_run: bool,
}

/// Fetch ArcGIS planets and write normalized JSON to disk.
pub fn fetch_arcgis_to_file(options: &ArcgisFetchOptions) -> Result<()> {
    let dataset = fetch_arcgis_dataset(options.page_size)?;

    if let Some(parent) = options.out.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Unable to create {}", parent.display()))?;
    }

    let json = if options.pretty {
        serde_json::to_string_pretty(&dataset)?
    } else {
        serde_json::to_string(&dataset)?
    };

    fs::write(&options.out, json)
        .with_context(|| format!("Unable to write {}", options.out.display()))?;

    println!(
        "Wrote {} ArcGIS records to {} (known: {}, unknown: {})",
        dataset.len(),
        options.out.display(),
        dataset.known.len(),
        dataset.unknown.len(),
    );

    Ok(())
}

fn build_arcgis_import_meta<'a>(
    backend: &'a str,
    result: &ArcgisImportResult,
) -> ArcgisImportMeta<'a> {
    ArcgisImportMeta {
        crate_version: env!("CARGO_PKG_VERSION"),
        operation: "arcgis_import",
        backend,
        completed_at_utc: chrono::Utc::now().to_rfc3339(),
        fetched: result.fetched,
        known: result.known,
        unknown: result.unknown,
        known_inserted: result.inserted,
        known_updated: result.updated,
        known_skipped: result.skipped,
        unknown_inserted: result.unknown_inserted,
        unknown_updated: result.unknown_updated,
        unknown_skipped: result.unknown_skipped,
        dry_run: result.dry_run,
    }
}

/// Fetch ArcGIS planets and import/upsert them into SQLite.
pub fn import_arcgis_to_sqlite(options: &ArcgisImportOptions) -> Result<ArcgisImportResult> {
    let dataset = fetch_arcgis_dataset(options.page_size)?;

    let mut result = ArcgisImportResult {
        fetched: dataset.len(),
        known: dataset.known.len(),
        unknown: dataset.unknown.len(),
        dry_run: options.dry_run,
        ..ArcgisImportResult::default()
    };

    if options.dry_run {
        result.skipped = dataset.known.len();
        result.unknown_skipped = dataset.unknown.len();
        return Ok(result);
    }

    let mut conn = Connection::open(&options.db)
        .with_context(|| format!("Unable to open DB: {}", options.db.display()))?;

    ensure_required_schema(
        &conn,
        &options.table,
        &options.unknown_table,
        !options.dry_run,
    )?;

    println!();
    let pb = import_progress_bar(dataset.len(), "Importing ArcGIS records into SQLite...")?;

    let tx = conn.transaction()?;
    {
        let known_repo = SqlitePlanetRepository::new(&tx, &options.table);
        known_repo.ensure_table_exists()?;

        let unknown_repo = SqlitePlanetRepository::new(&tx, &options.unknown_table);
        unknown_repo.ensure_table_exists()?;

        for planet in &dataset.known {
            match known_repo.upsert_arcgis_planet(planet)? {
                UpsertOutcome::Inserted => result.inserted += 1,
                UpsertOutcome::Updated => result.updated += 1,
                UpsertOutcome::Skipped => result.skipped += 1,
            }
            pb.inc(1);
        }

        for planet in &dataset.unknown {
            match unknown_repo.upsert_arcgis_planet(planet)? {
                UpsertOutcome::Inserted => result.unknown_inserted += 1,
                UpsertOutcome::Updated => result.unknown_updated += 1,
                UpsertOutcome::Skipped => result.unknown_skipped += 1,
            }
            pb.inc(1);
        }
    }
    tx.commit()?;

    pb.finish_with_message("SQLite ArcGIS import completed.");
    println!();

    if !options.dry_run {
        let meta = build_arcgis_import_meta("sqlite", &result);
        crate::db::sqlite::upsert_meta_struct(&conn, "sync.import.arcgis.sqlite", &meta)?;
    }

    Ok(result)
}

/// Fetch ArcGIS planets and import/upsert them into PostgreSQL.
pub async fn import_arcgis_to_postgres(
    cfg: &DbConfig,
    _table: String,
    _unknown_table: String,
    page_size: i64,
    dry_run: bool,
) -> Result<ArcgisImportResult> {
    let dataset = tokio::task::spawn_blocking(move || fetch_arcgis_dataset(page_size)).await??;

    let mut result = ArcgisImportResult {
        fetched: dataset.len(),
        known: dataset.known.len(),
        unknown: dataset.unknown.len(),
        dry_run,
        ..ArcgisImportResult::default()
    };

    if dry_run {
        result.skipped = dataset.known.len();
        result.unknown_skipped = dataset.unknown.len();
        return Ok(result);
    }

    bootstrap_schema(cfg).await?;

    let url = build_postgres_url(cfg)?;
    let mut conn = PgConnection::connect(&url).await?;

    println!();
    let pb = import_progress_bar(dataset.len(), "Importing ArcGIS records into PostgreSQL...")?;

    for planet in &dataset.known {
        match upsert_known_arcgis_planet(&mut conn, planet).await? {
            UpsertOutcome::Inserted => result.inserted += 1,
            UpsertOutcome::Updated => result.updated += 1,
            UpsertOutcome::Skipped => result.skipped += 1,
        }
        pb.inc(1);
    }

    for planet in &dataset.unknown {
        match upsert_unknown_arcgis_planet(&mut conn, planet).await? {
            UpsertOutcome::Inserted => result.unknown_inserted += 1,
            UpsertOutcome::Updated => result.unknown_updated += 1,
            UpsertOutcome::Skipped => result.unknown_skipped += 1,
        }
        pb.inc(1);
    }

    pb.finish_with_message("PostgreSQL ArcGIS import completed.");
    println!();

    let meta = build_arcgis_import_meta("postgresql", &result);
    crate::db::postgres::upsert_meta_struct(&mut conn, "sync.import.arcgis.postgresql", &meta)
        .await?;

    Ok(result)
}
