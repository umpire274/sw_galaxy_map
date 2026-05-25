//! Planet FID remapping helpers.

use crate::db::pull::diff::{FidMismatchSeverity, PullDiffReport};

/// Strategy used to associate a local planet FID with a remote planet FID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidRemapStrategy {
    /// Exact normalized planet identity match.
    PlanetIdentity,

    /// Manual review or manual confirmation required.
    ManualRequired,

    /// Remap is currently unresolved.
    Unresolved,
}

/// A candidate FID remap between local SQLite and remote canonical datasets.
#[derive(Debug, Clone)]
pub struct FidRemapCandidate {
    pub planet: String,
    pub local_fid: i64,
    pub remote_fid: i64,
    pub strategy: FidRemapStrategy,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct StoredFidRemapCandidate {
    pub id: i64,
    pub planet: String,
    pub local_fid: i64,
    pub remote_fid: i64,
    pub strategy: String,
    pub confidence: f64,
    pub approved: bool,
    pub applied: bool,
    pub created_at: String,
}

/// Builds FID remap candidates from a pull diff report.
pub fn build_fid_remap_candidates(report: &PullDiffReport) -> Vec<FidRemapCandidate> {
    report
        .fid_mismatches
        .iter()
        .filter(|mismatch| mismatch.severity == FidMismatchSeverity::SuspiciousPositiveMismatch)
        .map(|mismatch| FidRemapCandidate {
            planet: mismatch.planet.clone(),
            local_fid: mismatch.local_fid,
            remote_fid: mismatch.remote_fid,
            strategy: FidRemapStrategy::ManualRequired,
            confidence: 0.5,
        })
        .collect()
}

use rusqlite::{Connection, params};

/// Persists FID remap candidates into the local SQLite database.
///
/// Existing candidates with the same `(local_fid, remote_fid)` pair are ignored.
pub fn persist_fid_remap_candidates(
    conn: &Connection,
    candidates: &[FidRemapCandidate],
) -> anyhow::Result<usize> {
    let created_at = chrono::Utc::now().to_rfc3339();
    let mut inserted = 0usize;

    for candidate in candidates {
        let affected = conn.execute(
            r#"
            INSERT OR IGNORE INTO planet_fid_remap (
                planet,
                local_fid,
                remote_fid,
                strategy,
                confidence,
                approved,
                applied,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6)
            "#,
            params![
                candidate.planet,
                candidate.local_fid,
                candidate.remote_fid,
                format!("{:?}", candidate.strategy),
                candidate.confidence,
                created_at,
            ],
        )?;

        inserted += affected;
    }

    Ok(inserted)
}

pub fn list_fid_remap_candidates(
    conn: &Connection,
    show_approved: bool,
    show_applied: bool,
) -> anyhow::Result<Vec<StoredFidRemapCandidate>> {
    let mut query = String::from(
        r#"
        SELECT
            id,
            planet,
            local_fid,
            remote_fid,
            strategy,
            confidence,
            approved,
            applied,
            created_at
        FROM planet_fid_remap
        WHERE 1=1
        "#,
    );

    if !show_approved {
        query.push_str(" AND approved = 0");
    }

    if !show_applied {
        query.push_str(" AND applied = 0");
    }

    query.push_str(" ORDER BY confidence DESC, planet ASC");

    let mut stmt = conn.prepare(&query)?;

    let rows = stmt.query_map([], |row| {
        Ok(StoredFidRemapCandidate {
            id: row.get(0)?,
            planet: row.get(1)?,
            local_fid: row.get(2)?,
            remote_fid: row.get(3)?,
            strategy: row.get(4)?,
            confidence: row.get(5)?,
            approved: row.get::<_, i64>(6)? != 0,
            applied: row.get::<_, i64>(7)? != 0,
            created_at: row.get(8)?,
        })
    })?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
