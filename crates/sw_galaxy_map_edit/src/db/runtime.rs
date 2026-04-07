//! Database runtime helpers for sw_galaxy_map_edit.

use anyhow::Result;
use std::path::Path;
use sw_galaxy_map_core::db::db_status::resolve_db_path;

/// Opens the local database, initializing it if needed.
pub fn open_db() -> Result<rusqlite::Connection> {
    let db_path = resolve_db_path(None)?;
    ensure_db_ready(&db_path)?;
    let mut con = sw_galaxy_map_core::db::open_db(&db_path.to_string_lossy())?;
    let _ = sw_galaxy_map_core::db::migrate::run(&mut con, false, false)?;
    crate::audit::log::ensure_audit_table(&con)?;
    Ok(con)
}

fn ensure_db_ready(db_path: &Path) -> Result<()> {
    if db_path.exists() {
        return Ok(());
    }

    println!("Local database not found at: {}", db_path.display());
    println!("Initializing it now...");

    let report =
        sw_galaxy_map_core::db::db_init::run(Some(db_path.to_string_lossy().to_string()), false)?;

    println!();
    println!("Database initialized.");
    println!();
    println!("Path: {}", report.out_path.display());
    println!("Overwritten existing: {}", report.overwritten_existing);
    println!("Downloaded features: {}", report.downloaded_features);
    println!("FTS enabled: {}", report.fts_enabled);

    Ok(())
}