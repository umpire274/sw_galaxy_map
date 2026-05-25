//! Local pull staging and backup helpers.

use rusqlite::{Connection, params};

/// Summary of local pull preparation.
#[derive(Debug, Clone, Default)]
pub struct LocalPullPreparationReport {
    pub backup_tables_created: usize,
    pub staging_tables_created: usize,
    pub local_planets_backed_up: usize,
    pub local_unknown_backed_up: usize,
    pub dry_run: bool,
}

/// Creates local backup and staging tables required by local pull workflows.
pub fn create_local_pull_staging_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS local_pull_planets_backup (
            backup_id TEXT NOT NULL,
            FID INTEGER NOT NULL,
            Planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            Region TEXT,
            Sector TEXT,
            System TEXT,
            Grid TEXT,
            X REAL,
            Y REAL,
            arcgis_hash TEXT,
            deleted INTEGER,
            Canon INTEGER,
            Legends INTEGER,
            status TEXT,
            grid_unit TEXT,
            backup_timestamp TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS local_pull_planets_unknown_backup (
            backup_id TEXT NOT NULL,
            id INTEGER,
            fid INTEGER,
            planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            region TEXT,
            sector TEXT,
            system TEXT,
            grid TEXT,
            x REAL,
            y REAL,
            arcgis_hash TEXT,
            deleted INTEGER,
            canon INTEGER,
            legends INTEGER,
            status TEXT,
            grid_unit TEXT,
            backup_timestamp TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS remote_planets_staging (
            FID INTEGER PRIMARY KEY,
            Planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            Region TEXT,
            Sector TEXT,
            System TEXT,
            Grid TEXT,
            X REAL,
            Y REAL,
            arcgis_hash TEXT,
            deleted INTEGER,
            Canon INTEGER,
            Legends INTEGER,
            status TEXT,
            grid_unit TEXT
        );

        CREATE TABLE IF NOT EXISTS remote_planets_unknown_staging (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fid INTEGER,
            planet TEXT NOT NULL,
            planet_norm TEXT NOT NULL,
            region TEXT,
            sector TEXT,
            system TEXT,
            grid TEXT,
            x REAL,
            y REAL,
            arcgis_hash TEXT,
            deleted INTEGER,
            canon INTEGER,
            legends INTEGER,
            status TEXT,
            grid_unit TEXT
        );
        "#,
    )?;

    Ok(())
}

/// Backs up current local planet tables before a pull workflow.
pub fn backup_local_planet_tables(
    conn: &Connection,
    backup_id: &str,
) -> anyhow::Result<(usize, usize)> {
    let backup_timestamp = chrono::Utc::now().to_rfc3339();

    let planets = conn.execute(
        r#"
        INSERT INTO local_pull_planets_backup (
            backup_id,
            FID,
            Planet,
            planet_norm,
            Region,
            Sector,
            System,
            Grid,
            X,
            Y,
            arcgis_hash,
            deleted,
            Canon,
            Legends,
            status,
            grid_unit,
            backup_timestamp
        )
        SELECT
            ?1,
            FID,
            Planet,
            planet_norm,
            Region,
            Sector,
            System,
            Grid,
            X,
            Y,
            arcgis_hash,
            deleted,
            Canon,
            Legends,
            status,
            grid_unit,
            ?2
        FROM planets
        "#,
        params![backup_id, backup_timestamp],
    )?;

    let unknown = conn.execute(
        r#"
        INSERT INTO local_pull_planets_unknown_backup (
            backup_id,
            id,
            fid,
            planet,
            planet_norm,
            region,
            sector,
            system,
            grid,
            x,
            y,
            arcgis_hash,
            deleted,
            canon,
            legends,
            status,
            grid_unit,
            backup_timestamp
        )
        SELECT
            ?1,
            id,
            fid,
            planet,
            planet_norm,
            region,
            sector,
            system,
            grid,
            x,
            y,
            arcgis_hash,
            deleted,
            canon,
            legends,
            status,
            grid_unit,
            ?2
        FROM planets_unknown
        "#,
        params![backup_id, backup_timestamp],
    )?;

    Ok((planets, unknown))
}

/// Prepares the local SQLite database for a future pull workflow.
///
/// This creates the local backup/staging tables and snapshots the current
/// local planet tables using the supplied backup identifier.
pub fn prepare_local_pull_staging(
    conn: &Connection,
    backup_id: &str,
    dry_run: bool,
) -> anyhow::Result<LocalPullPreparationReport> {
    if dry_run {
        return Ok(LocalPullPreparationReport {
            dry_run: true,
            ..LocalPullPreparationReport::default()
        });
    }

    create_local_pull_staging_tables(conn)?;

    let (local_planets_backed_up, local_unknown_backed_up) =
        backup_local_planet_tables(conn, backup_id)?;

    Ok(LocalPullPreparationReport {
        backup_tables_created: 2,
        staging_tables_created: 2,
        local_planets_backed_up,
        local_unknown_backed_up,
        dry_run: false,
    })
}
