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
