//! Local/remote database diff workflows.

use crate::db::pull::config::RemoteDbConfig;
use crate::db::pull::postgres::build_postgres_connect_options;
use rusqlite::Connection;
use sqlx::{Connection as SqlxConnection, PgConnection};
use std::collections::HashMap;
use std::path::Path;

/// Summary of a local/remote pull diff operation.
#[derive(Debug, Clone, Default)]
pub struct PullDiffReport {
    /// Number of local planets.
    pub local_planets: i64,

    /// Number of remote planets.
    pub remote_planets: i64,

    /// Planets present remotely but missing locally.
    pub missing_local_planets: Vec<String>,

    /// Planets present locally but missing remotely.
    pub stale_local_planets: Vec<String>,

    /// Planets with mismatched FID values.
    pub fid_mismatches: Vec<FidMismatch>,

    /// Coordinate-unit mismatches.
    pub grid_unit_mismatches: Vec<GridUnitMismatch>,
}

/// Represents a FID mismatch between local and remote datasets.
#[derive(Debug, Clone)]
pub struct FidMismatch {
    pub planet: String,
    pub local_fid: i64,
    pub remote_fid: i64,
}

/// Represents a coordinate unit mismatch between local and remote datasets.
#[derive(Debug, Clone)]
pub struct GridUnitMismatch {
    pub planet: String,
    pub local_grid_unit: String,
    pub remote_grid_unit: String,
}

/// Minimal planet row used by local/remote diff workflows.
#[derive(Debug, Clone)]
struct DiffPlanetRow {
    fid: i64,
    planet: String,
    planet_norm: String,
    grid_unit: String,
}

/// Loads the minimal local SQLite planet dataset used by pull diff workflows.
fn load_local_planets(conn: &Connection) -> anyhow::Result<Vec<DiffPlanetRow>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            FID,
            Planet,
            planet_norm,
            COALESCE(grid_unit, '') AS grid_unit
        FROM planets
        WHERE deleted = 0
        ORDER BY planet_norm
        "#,
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(DiffPlanetRow {
                fid: row.get(0)?,
                planet: row.get(1)?,
                planet_norm: row.get(2)?,
                grid_unit: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Loads the minimal remote PostgreSQL planet dataset used by pull diff workflows.
async fn load_remote_planets(conn: &mut PgConnection) -> anyhow::Result<Vec<DiffPlanetRow>> {
    let rows = sqlx::query_as::<_, (i64, String, String, String)>(
        r#"
        SELECT
            FID,
            Planet,
            planet_norm,
            COALESCE(grid_unit, '') AS grid_unit
        FROM planets
        WHERE deleted = 0
        ORDER BY planet_norm
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(fid, planet, planet_norm, grid_unit)| DiffPlanetRow {
            fid,
            planet,
            planet_norm,
            grid_unit,
        })
        .collect())
}

/// Computes a read-only diff between the local SQLite database and the remote
/// canonical PostgreSQL database.
pub async fn diff_local_with_remote(
    local_db_path: &Path,
    remote_config: &RemoteDbConfig,
) -> anyhow::Result<PullDiffReport> {
    let local_conn = Connection::open(local_db_path)?;

    let options = build_postgres_connect_options(remote_config)?;
    let mut remote_conn = PgConnection::connect_with(&options).await?;

    let local_planets = load_local_planets(&local_conn)?;
    let remote_planets = load_remote_planets(&mut remote_conn).await?;

    let mut report = PullDiffReport {
        local_planets: local_planets.len() as i64,
        remote_planets: remote_planets.len() as i64,
        ..PullDiffReport::default()
    };

    let local_by_norm: HashMap<&str, &DiffPlanetRow> = local_planets
        .iter()
        .map(|row| (row.planet_norm.as_str(), row))
        .collect();

    let remote_by_norm: HashMap<&str, &DiffPlanetRow> = remote_planets
        .iter()
        .map(|row| (row.planet_norm.as_str(), row))
        .collect();

    for remote in &remote_planets {
        match local_by_norm.get(remote.planet_norm.as_str()) {
            Some(local) => {
                if local.fid != remote.fid {
                    report.fid_mismatches.push(FidMismatch {
                        planet: remote.planet.clone(),
                        local_fid: local.fid,
                        remote_fid: remote.fid,
                    });
                }

                if !local.grid_unit.eq_ignore_ascii_case(&remote.grid_unit) {
                    report.grid_unit_mismatches.push(GridUnitMismatch {
                        planet: remote.planet.clone(),
                        local_grid_unit: local.grid_unit.clone(),
                        remote_grid_unit: remote.grid_unit.clone(),
                    });
                }
            }
            None => {
                report.missing_local_planets.push(remote.planet.clone());
            }
        }
    }

    for local in &local_planets {
        if !remote_by_norm.contains_key(local.planet_norm.as_str()) {
            report.stale_local_planets.push(local.planet.clone());
        }
    }

    Ok(report)
}
