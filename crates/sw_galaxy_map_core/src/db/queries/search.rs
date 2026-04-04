use crate::db::has_table;
use crate::model::{PlanetSearchRow, SearchFilter};
use crate::utils::fuzzy::fuzzy_search;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

/// Searches planets by normalized free-text query.
///
/// Uses FTS when available, otherwise falls back to a LIKE-based query
/// that also matches aliases.
pub fn search_planets(
    con: &Connection,
    query_norm: &str,
    limit: i64,
) -> Result<Vec<PlanetSearchRow>> {
    if limit <= 0 {
        return Ok(Vec::new());
    }

    let query_norm = query_norm.trim();
    if query_norm.is_empty() {
        return Ok(Vec::new());
    }

    if has_table(con, "planets_fts")? {
        return search_planets_fts(con, query_norm, limit);
    }

    search_planets_like(con, query_norm, limit)
}

fn search_planets_like(
    con: &Connection,
    query_norm: &str,
    limit: i64,
) -> Result<Vec<PlanetSearchRow>> {
    let like = format!("%{}%", query_norm);

    let mut stmt = con
        .prepare(
            r#"
            SELECT DISTINCT
                p.FID,
                p.Planet,
                p.Region,
                p.Sector,
                p.System,
                p.Grid,
                p.X,
                p.Y,
                COALESCE(p.Canon, 0),
                COALESCE(p.Legends, 0),
                p.status
            FROM planets p
            LEFT JOIN planet_aliases pa
                ON pa.planet_fid = p.FID
            WHERE
                p.status NOT IN ('deleted', 'skipped', 'invalid')
                AND (
                    p.planet_norm LIKE ?1
                    OR pa.alias_norm LIKE ?1
                )
            ORDER BY p.planet_norm ASC
            LIMIT ?2
            "#,
        )
        .context("Failed to prepare LIKE search query")?;

    let rows = stmt
        .query_map((like, limit), |r| {
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
        .context("Failed to execute LIKE search query")?;

    let items = rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
    Ok(items)
}

fn search_planets_fts(
    con: &Connection,
    query_norm: &str,
    limit: i64,
) -> Result<Vec<PlanetSearchRow>> {
    let mut stmt = con
        .prepare(
            r#"
            SELECT p.FID, p.Planet, p.Region, p.Sector, p.System, p.Grid,
                   p.X, p.Y, p.Canon, p.Legends, p.status
            FROM planets_fts f
            JOIN planets p ON p.FID = f.planet_fid
            WHERE p.status NOT IN ('deleted', 'skipped', 'invalid') AND planets_fts MATCH ?1
            ORDER BY bm25(planets_fts)
            LIMIT ?2
            "#,
        )
        .context("Failed to prepare FTS search query")?;

    let rows = stmt
        .query_map((query_norm, limit), |r| {
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
        .context("Failed to execute FTS search query")?;

    let items = rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
    Ok(items)
}

/// Combined search with optional text query and multiple filters.
///
/// All active filters are combined with AND.
/// If no text query is provided, only filter-based results are returned.
/// The caller must ensure at least one criterion is present.
pub fn search_planets_filtered(
    con: &Connection,
    filter: &SearchFilter,
) -> Result<Vec<PlanetSearchRow>> {
    use crate::utils::normalize::normalize_text;
    use rusqlite::params_from_iter;
    use rusqlite::types::Value;

    if filter.limit <= 0 {
        return Ok(Vec::new());
    }

    let query_norm = filter
        .query
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(normalize_text);

    let has_text_query = query_norm.is_some();

    let mut sql = String::with_capacity(512);

    sql.push_str(
        r#"SELECT DISTINCT
            p.FID,
            p.Planet,
            p.Region,
            p.Sector,
            p.System,
            p.Grid,
            p.X,
            p.Y,
            COALESCE(p.Canon, 0),
            COALESCE(p.Legends, 0),
            p.status
        FROM planets p
        "#,
    );

    if has_text_query {
        sql.push_str("LEFT JOIN planet_aliases pa ON pa.planet_fid = p.FID\n");
    }

    sql.push_str("WHERE 1=1\n");

    let mut params: Vec<Value> = Vec::new();

    if let Some(st) = filter
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sql.push_str(" AND p.status = ? COLLATE NOCASE\n");
        params.push(Value::from(st.to_string()));
    } else {
        sql.push_str(
            " AND (p.status IS NULL OR p.status NOT IN ('deleted', 'skipped', 'invalid'))\n",
        );
    }

    if let Some(ref qn) = query_norm {
        let like = format!("%{}%", qn);
        sql.push_str(" AND (p.planet_norm LIKE ? OR pa.alias_norm LIKE ?)\n");
        params.push(Value::from(like.clone()));
        params.push(Value::from(like));
    }

    if let Some(r) = filter
        .region
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let like = format!("%{}%", r);
        sql.push_str(" AND p.Region LIKE ? COLLATE NOCASE\n");
        params.push(Value::from(like));
    }

    if let Some(s) = filter
        .sector
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let like = format!("%{}%", s);
        sql.push_str(" AND p.Sector LIKE ? COLLATE NOCASE\n");
        params.push(Value::from(like));
    }

    if let Some(g) = filter
        .grid
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sql.push_str(" AND p.Grid = ? COLLATE NOCASE\n");
        params.push(Value::from(g.to_string()));
    }

    if filter.canon == Some(true) {
        sql.push_str(" AND COALESCE(p.Canon, 0) = 1\n");
    }

    if filter.legends == Some(true) {
        sql.push_str(" AND COALESCE(p.Legends, 0) = 1\n");
    }

    sql.push_str(" ORDER BY p.planet_norm ASC\n");
    sql.push_str(" LIMIT ?\n");
    params.push(Value::from(filter.limit));

    let mut stmt = con
        .prepare(&sql)
        .context("Failed to prepare filtered search query")?;

    let rows = stmt
        .query_map(params_from_iter(params), |r| {
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
        .context("Failed to execute filtered search query")?;

    let items = rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
    Ok(items)
}

/// Executes fuzzy search and then applies structured filters without
/// truncating the fuzzy candidate pool too early.
///
/// Pipeline:
/// 1. collect fuzzy candidates
/// 2. hydrate them as `PlanetSearchRow`
/// 3. apply structured filters
/// 4. restore original fuzzy order
/// 5. truncate to `filter.limit`
///
/// This function uses adaptive over-fetching to avoid losing valid matches
/// when structured filters (region/sector/grid/status/canon/legends) are
/// restrictive compared to the fuzzy candidate pool.
pub fn fuzzy_search_filtered(
    con: &Connection,
    query_norm: &str,
    max_distance: usize,
    filter: &SearchFilter,
) -> Result<Vec<PlanetSearchRow>> {
    use std::collections::{HashMap, HashSet};

    if query_norm.trim().is_empty() || filter.limit <= 0 {
        return Ok(Vec::new());
    }

    let target_limit = filter.limit as usize;

    let status_filter = filter.status.as_deref().map(|s| s.to_ascii_lowercase());
    let region_filter = filter.region.as_deref().map(|s| s.to_ascii_lowercase());
    let sector_filter = filter.sector.as_deref().map(|s| s.to_ascii_lowercase());
    let grid_filter = filter.grid.as_deref().map(|s| s.to_ascii_lowercase());

    // Start with a reasonably wide batch and grow until we either:
    // - have enough filtered results
    // - exhaust fuzzy candidates
    let mut fetch_limit = target_limit.saturating_mul(10).max(50);

    loop {
        let candidates = fuzzy_search(
            con,
            query_norm,
            max_distance,
            fetch_limit,
            filter.status.as_deref(),
        )
        .context("Failed to execute fuzzy search")?;

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Preserve fuzzy order so we can restore it after SQL hydration/filtering.
        let mut order_by_fid: HashMap<i64, usize> = HashMap::with_capacity(candidates.len());
        for (idx, hit) in candidates.iter().enumerate() {
            order_by_fid.entry(hit.fid).or_insert(idx);
        }

        let fids: Vec<i64> = candidates.iter().map(|h| h.fid).collect();
        let fid_set: HashSet<i64> = fids.iter().copied().collect();

        // Hydrate candidates as PlanetSearchRow.
        let mut stmt = con
            .prepare(
                r#"
                SELECT
                    FID,
                    Planet,
                    Region,
                    Sector,
                    System,
                    Grid,
                    X,
                    Y,
                    Canon,
                    Legends,
                    status
                FROM planets
                WHERE FID = ?1
                "#,
            )
            .context("Failed to prepare fuzzy candidate hydration query")?;

        let mut hydrated: Vec<PlanetSearchRow> = Vec::with_capacity(fids.len());

        for fid in &fids {
            let row = stmt
                .query_row(params![fid], |r| {
                    Ok(PlanetSearchRow {
                        fid: r.get(0)?,
                        name: r.get(1)?,
                        region: r.get(2)?,
                        sector: r.get(3)?,
                        system: r.get(4)?,
                        grid: r.get(5)?,
                        x: r.get(6)?,
                        y: r.get(7)?,
                        canon: r.get::<_, Option<i64>>(8)?.unwrap_or(0) == 1,
                        legends: r.get::<_, Option<i64>>(9)?.unwrap_or(0) == 1,
                        status: r.get(10)?,
                    })
                })
                .optional()
                .with_context(|| format!("Failed to hydrate fuzzy candidate FID={fid}"))?;

            if let Some(row) = row {
                hydrated.push(row);
            }
        }

        // Defensive filter: only keep rows that originated from fuzzy hits.
        let mut filtered: Vec<PlanetSearchRow> = hydrated
            .into_iter()
            .filter(|row| fid_set.contains(&row.fid))
            .filter(|row| {
                if let Some(ref wanted) = status_filter {
                    let actual = row.status.as_deref().unwrap_or("").to_ascii_lowercase();
                    if actual != *wanted {
                        return false;
                    }
                }

                if let Some(ref wanted) = region_filter {
                    let actual = row.region.as_deref().unwrap_or("").to_ascii_lowercase();
                    if !actual.contains(wanted) {
                        return false;
                    }
                }

                if let Some(ref wanted) = sector_filter {
                    let actual = row.sector.as_deref().unwrap_or("").to_ascii_lowercase();
                    if !actual.contains(wanted) {
                        return false;
                    }
                }

                if let Some(ref wanted) = grid_filter {
                    let actual = row.grid.as_deref().unwrap_or("").to_ascii_lowercase();
                    if actual != *wanted {
                        return false;
                    }
                }

                if filter.canon == Some(true) && !row.canon {
                    return false;
                }

                if filter.legends == Some(true) && !row.legends {
                    return false;
                }

                true
            })
            .collect();

        // Restore original fuzzy order.
        filtered.sort_by_key(|row| order_by_fid.get(&row.fid).copied().unwrap_or(usize::MAX));

        // Stop if:
        // 1) we have enough results
        // 2) fuzzy_search returned fewer items than requested => no more candidates available
        if filtered.len() >= target_limit || candidates.len() < fetch_limit {
            filtered.truncate(target_limit);
            return Ok(filtered);
        }

        // Otherwise, widen the fuzzy window and try again.
        fetch_limit = fetch_limit.saturating_mul(2);
    }
}
