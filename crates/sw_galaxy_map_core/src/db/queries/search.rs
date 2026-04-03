use crate::db::has_table;
use crate::model::{PlanetSearchRow, SearchFilter};
use crate::utils::fuzzy::fuzzy_search;
use anyhow::{Context, Result};
use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use std::collections::HashMap;

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

/// Performs fuzzy search while preserving all active structured filters.
///
/// Strategy:
/// 1. get fuzzy candidate FIDs in ranked order
/// 2. hydrate them as `PlanetSearchRow`
/// 3. apply structured filters in SQL
/// 4. restore original fuzzy order
/// 5. truncate to `filter.limit`
pub fn fuzzy_search_filtered(
    con: &Connection,
    query_norm: &str,
    max_distance: usize,
    filter: &SearchFilter,
) -> Result<Vec<PlanetSearchRow>> {
    if query_norm.trim().is_empty() || filter.limit <= 0 {
        return Ok(Vec::new());
    }

    // Pull more than the final limit so post-filtering still has enough candidates.
    let candidate_limit = (filter.limit.max(1) as usize).saturating_mul(10);

    let fuzzy_hits = fuzzy_search(
        con,
        query_norm,
        max_distance,
        candidate_limit,
        filter.status.as_deref(),
    )?;

    if fuzzy_hits.is_empty() {
        return Ok(Vec::new());
    }

    let ordered_fids: Vec<i64> = fuzzy_hits.iter().map(|h| h.fid).collect();

    let mut sql = String::from(
        r#"
        SELECT
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
        WHERE p.FID IN (
        "#,
    );

    let mut params: Vec<Value> = Vec::new();

    for (idx, fid) in ordered_fids.iter().enumerate() {
        if idx > 0 {
            sql.push_str(", ");
        }
        sql.push('?');
        params.push(Value::Integer(*fid));
    }

    sql.push_str(")\n");

    // Status filter
    //
    // Important:
    // - if user explicitly requested a status, honor that exact status
    // - otherwise preserve the historical "active only" behavior
    if let Some(st) = filter
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sql.push_str(" AND p.status = ? COLLATE NOCASE\n");
        params.push(Value::Text(st.to_string()));
    } else {
        sql.push_str(
            " AND (p.status IS NULL OR p.status NOT IN ('deleted', 'skipped', 'invalid'))\n",
        );
    }

    if let Some(region) = filter
        .region
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sql.push_str(" AND p.Region LIKE ? COLLATE NOCASE\n");
        params.push(Value::Text(format!("%{}%", region)));
    }

    if let Some(sector) = filter
        .sector
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sql.push_str(" AND p.Sector LIKE ? COLLATE NOCASE\n");
        params.push(Value::Text(format!("%{}%", sector)));
    }

    if let Some(grid) = filter
        .grid
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        sql.push_str(" AND p.Grid = ? COLLATE NOCASE\n");
        params.push(Value::Text(grid.to_string()));
    }

    if filter.canon == Some(true) {
        sql.push_str(" AND COALESCE(p.Canon, 0) = 1\n");
    }

    if filter.legends == Some(true) {
        sql.push_str(" AND COALESCE(p.Legends, 0) = 1\n");
    }

    let mut stmt = con
        .prepare(&sql)
        .context("Failed to prepare fuzzy filtered hydration query")?;

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
        .context("Failed to execute fuzzy filtered hydration query")?;

    let hydrated = rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;

    // Restore original fuzzy ranking after SQL hydration/filtering.
    let rank: HashMap<i64, usize> = ordered_fids
        .iter()
        .enumerate()
        .map(|(idx, fid)| (*fid, idx))
        .collect();

    let mut out = hydrated;
    out.sort_by_key(|row| rank.get(&row.fid).copied().unwrap_or(usize::MAX));
    out.truncate(filter.limit as usize);

    Ok(out)
}
