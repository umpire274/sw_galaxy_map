use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};

use crate::model::{AliasRow, NearHit, Planet};

pub fn open_db(path: &str) -> Result<Connection> {
    let con = Connection::open(path).with_context(|| format!("Unable to open database: {path}"))?;
    Ok(con)
}

pub fn has_table(con: &Connection, table: &str) -> Result<bool> {
    let n: i64 = con.query_row(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table' AND name = ?1
        "#,
        [table],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn get_planet_by_norm(con: &Connection, planet_norm: &str) -> Result<Planet> {
    // 1) Match diretto sul nome normalizzato
    if let Some(p) = get_planet_by_norm_direct(con, planet_norm)? {
        return Ok(p);
    }

    // 2) Fallback: match su alias_norm
    if let Some(p) = get_planet_by_alias_norm(con, planet_norm)? {
        return Ok(p);
    }

    bail!("Planet not found: {}", planet_norm)
}

fn get_planet_by_norm_direct(con: &Connection, planet_norm: &str) -> Result<Option<Planet>> {
    let mut stmt = con
        .prepare(
            r#"
            SELECT
                FID, Planet, planet_norm, Region, Sector, System, Grid,
                X, Y, Canon, Legends, zm, name0, name1, name2, lat, long,
                ref, status, CRegion, CRegion_li
            FROM planets
            WHERE planet_norm = ?1
            LIMIT 1
            "#,
        )
        .context("Failed to prepare planet lookup query")?;

    let mut rows = stmt.query([planet_norm])?;
    if let Some(r) = rows.next()? {
        Ok(Some(Planet::from_row(r)?))
    } else {
        Ok(None)
    }
}

fn get_planet_by_alias_norm(con: &Connection, alias_norm: &str) -> Result<Option<Planet>> {
    let mut stmt = con
        .prepare(
            r#"
            SELECT
                p.FID, p.Planet, p.planet_norm, p.Region, p.Sector, p.System, p.Grid,
                p.X, p.Y, p.Canon, p.Legends, p.zm, p.name0, p.name1, p.name2, p.lat, p.long,
                p.ref, p.status, p.CRegion, p.CRegion_li
            FROM planet_aliases a
            JOIN planets p ON p.FID = a.planet_fid
            WHERE a.alias_norm = ?1
            ORDER BY p.Planet COLLATE NOCASE
            LIMIT 1
            "#,
        )
        .context("Failed to prepare alias lookup query")?;

    let mut rows = stmt.query([alias_norm])?;
    if let Some(r) = rows.next()? {
        Ok(Some(Planet::from_row(r)?))
    } else {
        Ok(None)
    }
}

pub fn get_aliases(con: &Connection, fid: i64) -> Result<Vec<AliasRow>> {
    let mut stmt = con.prepare(
        r#"
        SELECT alias, source
        FROM planet_aliases
        WHERE planet_fid = ?1
        ORDER BY source, alias
        "#,
    )?;

    let rows = stmt
        .query_map(params![fid], |r| {
            Ok(AliasRow {
                alias: r.get(0)?,
                source: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

pub fn search_planets(
    con: &Connection,
    query_norm: &str,
    limit: i64,
) -> Result<Vec<(i64, String)>> {
    if has_table(con, "planets_fts")? {
        return search_planets_fts(con, query_norm, limit);
    }

    search_planets_like(con, query_norm, limit)
}

fn search_planets_like(
    con: &Connection,
    query_norm: &str,
    limit: i64,
) -> Result<Vec<(i64, String)>> {
    let like = format!("%{}%", query_norm);

    let mut stmt = con
        .prepare(
            r#"
            SELECT p.FID, p.Planet
            FROM planet_search s
            JOIN planets p ON p.FID = s.planet_fid
            WHERE p.deleted = 0 AND s.search_norm LIKE ?1
            ORDER BY p.Planet COLLATE NOCASE
            LIMIT ?2
            "#,
        )
        .context("Failed to prepare LIKE search query")?;

    let rows = stmt
        .query_map((like, limit), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .context("Failed to execute LIKE search query")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn search_planets_fts(
    con: &Connection,
    query_norm: &str,
    limit: i64,
) -> Result<Vec<(i64, String)>> {
    // For FTS5, search terms are tokenized; normalized text works well.
    // bm25() provides a reasonable relevance score (lower is better).
    let mut stmt = con
        .prepare(
            r#"
            SELECT p.FID, p.Planet
            FROM planets_fts f
            JOIN planets p ON p.FID = f.planet_fid
            WHERE p.deleted = 0 AND planets_fts MATCH ?1
            ORDER BY bm25(planets_fts)
            LIMIT ?2
            "#,
        )
        .context("Failed to prepare FTS search query")?;

    let rows = stmt
        .query_map((query_norm, limit), |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .context("Failed to execute FTS search query")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn near_planets(con: &Connection, x: f64, y: f64, r: f64, limit: i64) -> Result<Vec<NearHit>> {
    let r2 = r * r;

    let mut stmt = con.prepare(
        r#"
        SELECT FID, Planet, X, Y,
               ((X - ?1)*(X - ?1) + (Y - ?2)*(Y - ?2)) AS d2
        FROM planets
        WHERE ((X - ?1)*(X - ?1) + (Y - ?2)*(Y - ?2)) <= ?3
        ORDER BY d2 ASC
        LIMIT ?4
        "#,
    )?;

    let rows = stmt
        .query_map(params![x, y, r2, limit], |r| {
            let fid: i64 = r.get(0)?;
            let planet: String = r.get(1)?;
            let px: f64 = r.get(2)?;
            let py: f64 = r.get(3)?;
            let d2: f64 = r.get(4)?;
            Ok(NearHit {
                fid,
                planet,
                x: px,
                y: py,
                distance: d2.sqrt(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

pub fn near_planets_excluding_fid(
    con: &Connection,
    center_fid: i64,
    x: f64,
    y: f64,
    r: f64,
    limit: i64,
) -> Result<Vec<NearHit>> {
    let r2 = r * r;

    let mut stmt = con.prepare(
        r#"
        SELECT FID, Planet, X, Y,
               ((X - ?2)*(X - ?2) + (Y - ?3)*(Y - ?3)) AS d2
        FROM planets
        WHERE FID != ?1
          AND ((X - ?2)*(X - ?2) + (Y - ?3)*(Y - ?3)) <= ?4
        ORDER BY d2 ASC
        LIMIT ?5
        "#,
    )?;

    let rows = stmt
        .query_map(params![center_fid, x, y, r2, limit], |r| {
            let fid: i64 = r.get(0)?;
            let planet: String = r.get(1)?;
            let px: f64 = r.get(2)?;
            let py: f64 = r.get(3)?;
            let d2: f64 = r.get(4)?;
            Ok(NearHit {
                fid,
                planet,
                x: px,
                y: py,
                distance: d2.sqrt(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}
