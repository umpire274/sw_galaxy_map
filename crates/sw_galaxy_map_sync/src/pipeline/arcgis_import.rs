use crate::db::sqlite::{SqlitePlanetRepository, ensure_required_schema, upsert_meta_json};
use crate::models::{ArcgisImportResult, UpsertOutcome};
use crate::sources::arcgis::fetch_arcgis_dataset;
use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
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

    ensure_required_schema(&conn, &options.table, &options.unknown_table)?;

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
        }

        for planet in &dataset.unknown {
            match unknown_repo.upsert_arcgis_planet(planet)? {
                UpsertOutcome::Inserted => result.unknown_inserted += 1,
                UpsertOutcome::Updated => result.unknown_updated += 1,
                UpsertOutcome::Skipped => result.unknown_skipped += 1,
            }
        }
    }
    tx.commit()?;

    if !options.dry_run {
        let meta = json!({
            "crate_version": env!("CARGO_PKG_VERSION"),
            "operation": "arcgis_import",
            "completed_at_utc": chrono::Utc::now().to_rfc3339(),
            "fetched": result.fetched,
            "known": result.known,
            "unknown": result.unknown,
            "known_inserted": result.inserted,
            "known_updated": result.updated,
            "known_skipped": result.skipped,
            "unknown_inserted": result.unknown_inserted,
            "unknown_updated": result.unknown_updated,
            "unknown_skipped": result.unknown_skipped,
            "dry_run": result.dry_run,
        });

        upsert_meta_json(&conn, "sync.arcgis.last_run", &meta)?;
    }

    Ok(result)
}
