use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{AliasRow, NearHit, Planet};

pub fn open_db(path: &str) -> Result<Connection> {
    let con = Connection::open(path).with_context(|| format!("Unable to open database: {path}"))?;
    Ok(con)
}

pub fn has_table(con: &Connection, table: &str) -> Result<bool> {
    let exists: Option<i64> = con
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
            params![table],
            |row| row.get(0),
        )
        .optional()?;
    Ok(exists.is_some())
}

pub fn get_planet_by_norm(con: &Connection, planet_norm: &str) -> Result<Planet> {
    let mut stmt = con.prepare(
        r#"
        SELECT
            FID, Planet, planet_norm, Region, Sector, System, Grid, X, Y, Canon, Legends, zm,
            name0, name1, name2, lat, long, ref, status, CRegion, CRegion_li
        FROM planets
        WHERE planet_norm = ?1
        LIMIT 1
        "#,
    )?;

    let p = stmt
        .query_row(params![planet_norm], |r| {
            Ok(Planet {
                fid: r.get(0)?,
                planet: r.get(1)?,
                planet_norm: r.get(2)?,
                region: r.get(3)?,
                sector: r.get(4)?,
                system: r.get(5)?,
                grid: r.get(6)?,
                x: r.get(7)?,
                y: r.get(8)?,
                canon: r.get(9)?,
                legends: r.get(10)?,
                zm: r.get(11)?,
                name0: r.get(12)?,
                name1: r.get(13)?,
                name2: r.get(14)?,
                lat: r.get(15)?,
                long: r.get(16)?,
                reference: r.get(17)?,
                status: r.get(18)?,
                c_region: r.get(19)?,
                c_region_li: r.get(20)?,
            })
        })
        .with_context(|| format!("Planet not found (planet_norm={planet_norm})"))?;

    Ok(p)
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
    let use_fts = has_table(con, "planets_fts")?;

    if use_fts {
        // MATCH su search_norm. Supporta anche prefissi con '*', se l’utente li inserisce.
        let mut stmt = con.prepare(
            r#"
            SELECT p.FID, p.Planet
            FROM planets_fts f
            JOIN planets p ON p.FID = f.planet_fid
            WHERE planets_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )?;

        let rows = stmt
            .query_map(params![query_norm, limit], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        return Ok(rows);
    }

    // Fallback: LIKE su planet_search.search_norm
    let like = format!("%{}%", query_norm);
    let mut stmt = con.prepare(
        r#"
        SELECT p.FID, p.Planet
        FROM planet_search s
        JOIN planets p ON p.FID = s.planet_fid
        WHERE s.search_norm LIKE ?1
        ORDER BY p.Planet COLLATE NOCASE
        LIMIT ?2
        "#,
    )?;

    let rows = stmt
        .query_map(params![like, limit], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
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
