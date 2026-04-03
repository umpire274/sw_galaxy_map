//! Fuzzy text matching for planet search.
//!
//! Provides Levenshtein distance calculation and a fuzzy search function
//! that loads all planet names + aliases from the DB and ranks them by
//! edit distance against the query.

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::model::PlanetSearchRow;

/// Compute the Levenshtein edit distance between two strings.
///
/// This is the classic dynamic-programming O(n*m) implementation.
/// For our use case (~7000 planet names, short strings) this is fast enough.
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.chars().count();
    let b_len = b.chars().count();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Use a single-row buffer to save memory.
    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;

        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost)
                .min(prev[j + 1] + 1) // deletion
                .min(curr[j] + 1); // insertion
        }

        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// A fuzzy match candidate with its edit distance.
#[derive(Debug, Clone)]
pub struct FuzzyHit {
    pub fid: i64,
    pub name: String,
    pub matched_on: String, // the actual string that matched (planet_norm or alias_norm)
    pub distance: usize,
}

/// Load all active planet names and aliases, compute Levenshtein distance
/// against `query_norm`, and return the best matches within `max_distance`.
///
/// Results are sorted by distance (ascending), then name (ascending).
pub fn fuzzy_search(
    con: &Connection,
    query_norm: &str,
    max_distance: usize,
    limit: usize,
) -> Result<Vec<FuzzyHit>> {
    if query_norm.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let mut hits: Vec<FuzzyHit> = Vec::new();

    // --- Match against planet_norm ---
    {
        let mut stmt = con
            .prepare(
                r#"
                SELECT FID, Planet, planet_norm
                FROM planets
                WHERE (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid'))
                "#,
            )
            .context("Failed to prepare fuzzy planet_norm query")?;

        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (fid, name, pnorm) = row?;
            let dist = levenshtein(query_norm, &pnorm);
            if dist <= max_distance {
                hits.push(FuzzyHit {
                    fid,
                    name,
                    matched_on: pnorm,
                    distance: dist,
                });
            }
        }
    }

    // --- Match against alias_norm ---
    {
        let mut stmt = con
            .prepare(
                r#"
                SELECT pa.planet_fid, p.Planet, pa.alias_norm
                FROM planet_aliases pa
                JOIN planets p ON p.FID = pa.planet_fid
                WHERE (p.status IS NULL OR p.status NOT IN ('deleted', 'skipped', 'invalid'))
                "#,
            )
            .context("Failed to prepare fuzzy alias_norm query")?;

        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (fid, name, anorm) = row?;
            let dist = levenshtein(query_norm, &anorm);
            if dist <= max_distance {
                // Avoid duplicates: only add if this FID isn't already in hits
                // with a better or equal distance.
                let dominated = hits.iter().any(|h| h.fid == fid && h.distance <= dist);

                if !dominated {
                    // Remove worse hit for same FID if present
                    hits.retain(|h| h.fid != fid || h.distance <= dist);

                    hits.push(FuzzyHit {
                        fid,
                        name,
                        matched_on: anorm,
                        distance: dist,
                    });
                }
            }
        }
    }

    // Sort by distance, then by name
    hits.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.name.cmp(&b.name)));
    hits.truncate(limit);

    Ok(hits)
}

/// Given fuzzy hits, load the full `PlanetSearchRow` data for each.
pub fn resolve_fuzzy_hits(
    con: &Connection,
    hits: &[FuzzyHit],
) -> Result<Vec<(PlanetSearchRow, usize)>> {
    let mut results = Vec::with_capacity(hits.len());

    let mut stmt = con
        .prepare(
            r#"
            SELECT FID, Planet, Region, Sector, System, Grid,
                   X, Y, COALESCE(Canon, 0), COALESCE(Legends, 0), status
            FROM planets
            WHERE FID = ?1
            "#,
        )
        .context("Failed to prepare fuzzy resolve query")?;

    for hit in hits {
        let row = stmt
            .query_row([hit.fid], |r| {
                Ok(PlanetSearchRow {
                    fid: r.get::<_, i64>(0)?,
                    name: r.get::<_, String>(1)?,
                    region: r.get::<_, Option<String>>(2)?,
                    sector: r.get::<_, Option<String>>(3)?,
                    system: r.get::<_, Option<String>>(4)?,
                    grid: r.get::<_, Option<String>>(5)?,
                    x: r.get(6)?,
                    y: r.get(7)?,
                    canon: r.get(8)?,
                    legends: r.get(9)?,
                    status: r.get::<_, Option<String>>(10)?,
                })
            })
            .with_context(|| format!("Failed to resolve fuzzy hit FID {}", hit.fid))?;

        results.push((row, hit.distance));
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein("coruscant", "coruscant"), 0);
    }

    #[test]
    fn levenshtein_one_char_missing() {
        assert_eq!(levenshtein("corusant", "coruscant"), 1);
    }

    #[test]
    fn levenshtein_one_char_replaced() {
        assert_eq!(levenshtein("corustant", "coruscant"), 1);
    }

    #[test]
    fn levenshtein_two_edits() {
        assert_eq!(levenshtein("corusnt", "coruscant"), 2);
    }

    #[test]
    fn levenshtein_completely_different() {
        assert!(levenshtein("xyz", "coruscant") > 5);
    }

    #[test]
    fn levenshtein_empty_strings() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", "abc"), 3);
    }

    #[test]
    fn levenshtein_case_sensitive() {
        // Our fuzzy search normalizes before calling, but the raw function is case-sensitive
        assert_eq!(levenshtein("coruscant", "Coruscant"), 1);
    }
}
