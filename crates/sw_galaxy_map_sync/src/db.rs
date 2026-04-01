use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::{DbPlanetRow, InsertDefaults, SyncRow};
use crate::utils::{build_planet_norm, names_match_by_roman_suffix};

/// Find an exact match by planet name, case-insensitive.
pub fn find_exact_match(
    conn: &Connection,
    table: &str,
    system: &str,
) -> Result<Option<DbPlanetRow>> {
    let sql = format!(
        "SELECT FID, Planet, COALESCE(Sector, ''), COALESCE(Region, ''), COALESCE(Grid, '')
         FROM {table}
         WHERE lower(trim(Planet)) = lower(trim(?1))
         LIMIT 1"
    );

    conn.query_row(&sql, params![system], |row| {
        Ok(DbPlanetRow {
            fid: row.get(0)?,
            planet: row.get(1)?,
            sector: row.get(2)?,
            region: row.get(3)?,
            grid: row.get(4)?,
        })
    })
    .optional()
    .map_err(Into::into)
}

/// Find a match where only a Roman numeral suffix differs and location is identical.
pub fn find_suffix_match(
    conn: &Connection,
    table: &str,
    row: &SyncRow,
) -> Result<Option<DbPlanetRow>> {
    let sql = format!(
        "SELECT FID, Planet, COALESCE(Sector, ''), COALESCE(Region, ''), COALESCE(Grid, '')
         FROM {table}
         WHERE lower(trim(COALESCE(Sector, ''))) = lower(trim(?1))
           AND lower(trim(COALESCE(Region, ''))) = lower(trim(?2))
           AND lower(trim(COALESCE(Grid, '')))   = lower(trim(?3))"
    );

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![row.sector, row.region, row.grid])?;

    while let Some(db_row) = rows.next()? {
        let candidate = DbPlanetRow {
            fid: db_row.get(0)?,
            planet: db_row.get(1)?,
            sector: db_row.get(2)?,
            region: db_row.get(3)?,
            grid: db_row.get(4)?,
        };

        if names_match_by_roman_suffix(&row.system, &candidate.planet) {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

/// Set only the status field of an existing DB row.
pub fn set_status(conn: &Connection, table: &str, fid: i64, status: &str) -> Result<()> {
    let sql = format!(
        "UPDATE {table}
         SET status = ?1
         WHERE FID = ?2"
    );

    conn.execute(&sql, params![status, fid])?;
    Ok(())
}

/// Update an existing planets row using the official CSV as source of truth.
pub fn update_planet_row(
    conn: &Connection,
    table: &str,
    fid: i64,
    row: &SyncRow,
    status: &str,
) -> Result<()> {
    let planet_norm = build_planet_norm(&row.system);

    let sql = format!(
        "UPDATE {table}
         SET Planet      = ?1,
             planet_norm = ?2,
             System      = ?3,
             Sector      = ?4,
             Region      = ?5,
             Grid        = ?6,
             status      = ?7
         WHERE FID = ?8"
    );

    conn.execute(
        &sql,
        params![
            row.system,
            planet_norm,
            row.system,
            row.sector,
            row.region,
            row.grid,
            status,
            fid
        ],
    )?;

    Ok(())
}

/// Insert a new row into the planets table using placeholder defaults.
pub fn insert_planet_row(
    conn: &Connection,
    table: &str,
    row: &SyncRow,
    defaults: &InsertDefaults,
) -> Result<()> {
    let planet_norm = build_planet_norm(&row.system);

    let sql = format!(
        "INSERT INTO {table} (
             Planet,
             planet_norm,
             Region,
             Sector,
             System,
             Grid,
             X,
             Y,
             arcgis_hash,
             Canon,
             Legends,
             status
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
    );

    conn.execute(
        &sql,
        params![
            row.system,
            planet_norm,
            row.region,
            row.sector,
            row.system,
            row.grid,
            defaults.x,
            defaults.y,
            defaults.arcgis_hash,
            defaults.canon,
            defaults.legends,
            "inserted"
        ],
    )?;

    Ok(())
}
