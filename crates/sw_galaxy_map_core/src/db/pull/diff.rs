//! Local/remote database diff workflows.

use crate::db::pull::config::RemoteDbConfig;
use crate::db::pull::postgres::build_postgres_connect_options;
use rusqlite::Connection;
use sqlx::{Connection as SqlxConnection, PgConnection};
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

impl PullDiffReport {
    pub fn expected_synthetic_fid_mismatches(&self) -> usize {
        self.fid_mismatches
            .iter()
            .filter(|m| m.severity == FidMismatchSeverity::ExpectedSyntheticRemap)
            .count()
    }

    pub fn suspicious_positive_fid_mismatches(&self) -> usize {
        self.fid_mismatches
            .iter()
            .filter(|m| m.severity == FidMismatchSeverity::SuspiciousPositiveMismatch)
            .count()
    }

    pub fn other_fid_mismatches(&self) -> usize {
        self.fid_mismatches
            .iter()
            .filter(|m| m.severity == FidMismatchSeverity::Other)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidMismatchSeverity {
    ExpectedSyntheticRemap,
    SuspiciousPositiveMismatch,
    Other,
}

/// Represents a FID mismatch between local and remote datasets.
#[derive(Debug, Clone)]
pub struct FidMismatch {
    pub planet: String,
    pub local_fid: i64,
    pub remote_fid: i64,
    pub severity: FidMismatchSeverity,
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
    grid_unit: String,
    identity_key: String,
}

fn diff_cmp_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['’', '`'], "'")
        .replace("  ", " ")
}

/// Loads the minimal local SQLite planet dataset used by pull diff workflows.
fn load_local_planets(conn: &Connection) -> anyhow::Result<Vec<DiffPlanetRow>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT
            FID,
            Planet,
            COALESCE(grid_unit, '') AS grid_unit
        FROM planets
        WHERE deleted = 0
        ORDER BY planet_norm
        "#,
    )?;

    let rows = stmt
        .query_map([], |row| {
            let np: String = row.get(1)?;
            Ok(DiffPlanetRow {
                fid: row.get(0)?,
                identity_key: diff_cmp_key(np.as_str()),
                planet: np,
                grid_unit: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Loads the minimal remote PostgreSQL planet dataset used by pull diff workflows.
async fn load_remote_planets(conn: &mut PgConnection) -> anyhow::Result<Vec<DiffPlanetRow>> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        r#"
        SELECT
            FID,
            Planet,
            COALESCE(grid_unit, '') AS grid_unit
        FROM planets
        WHERE deleted = 0
        ORDER BY Planet
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(fid, planet, grid_unit)| {
            let identity_key = diff_cmp_key(&planet);

            DiffPlanetRow {
                fid,
                planet,
                grid_unit,
                identity_key,
            }
        })
        .collect())
}

fn classify_fid_mismatch(local_fid: i64, remote_fid: i64) -> FidMismatchSeverity {
    match (local_fid, remote_fid) {
        (local, remote) if local > 0 && remote < 0 => FidMismatchSeverity::ExpectedSyntheticRemap,
        (local, remote) if local > 0 && remote > 0 && local != remote => {
            FidMismatchSeverity::SuspiciousPositiveMismatch
        }
        _ => FidMismatchSeverity::Other,
    }
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

    for remote in &remote_planets {
        match find_matching_planet(remote, &local_planets) {
            Some(local) => {
                if local.fid != remote.fid {
                    report.fid_mismatches.push(FidMismatch {
                        planet: remote.planet.clone(),
                        local_fid: local.fid,
                        remote_fid: remote.fid,
                        severity: classify_fid_mismatch(local.fid, remote.fid),
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
        if find_matching_planet(local, &remote_planets).is_none() {
            report.stale_local_planets.push(local.planet.clone());
        }
    }

    Ok(report)
}

fn strip_roman_suffix(value: &str) -> String {
    const ROMAN_SUFFIXES: &[&str] = &[
        " i", " ii", " iii", " iv", " v", " vi", " vii", " viii", " ix", " x",
    ];

    let value = value.trim().to_lowercase();

    for suffix in ROMAN_SUFFIXES {
        if value.ends_with(suffix) {
            return value[..value.len() - suffix.len()].trim().to_string();
        }
    }

    value
}

fn same_planet_identity(left: &DiffPlanetRow, right: &DiffPlanetRow) -> bool {
    if left.identity_key == right.identity_key {
        return true;
    }

    let left_base = strip_roman_suffix(&left.identity_key);
    let right_base = strip_roman_suffix(&right.identity_key);

    left.identity_key == right_base || left_base == right.identity_key || left_base == right_base
}
fn find_matching_planet<'a>(
    target: &DiffPlanetRow,
    candidates: &'a [DiffPlanetRow],
) -> Option<&'a DiffPlanetRow> {
    candidates
        .iter()
        .find(|candidate| candidate.identity_key == target.identity_key)
        .or_else(|| {
            candidates
                .iter()
                .find(|candidate| same_planet_identity(target, candidate))
        })
}
