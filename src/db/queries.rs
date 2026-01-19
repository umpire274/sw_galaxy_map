use crate::db::has_table;
use crate::model::{AliasRow, NearHit, Planet, Waypoint, WaypointPlanetLink};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};
// adatta: crate::models::Planet ecc.

const PLANET_SELECT_CANON: &str = r#"
  p.FID         AS fid,
  p.Planet      AS planet,
  p.planet_norm AS planet_norm,
  p.Region      AS region,
  p.Sector      AS sector,
  p.System      AS system,
  p.Grid        AS grid,
  p.X           AS x,
  p.Y           AS y,
  p.Canon       AS canon,
  p.Legends     AS legends,
  p.zm          AS zm,
  p.name0       AS name0,
  p.name1       AS name1,
  p.name2       AS name2,
  p.lat         AS lat,
  p.long        AS long,
  p.ref         AS reference,
  p.status      AS status,
  p.CRegion     AS c_region,
  p.CRegion_li  AS c_region_li
"#;

pub fn find_planet_by_norm(con: &Connection, planet_norm: &str) -> Result<Option<Planet>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM planets p
        WHERE p.planet_norm = ?1
        LIMIT 1
        "#,
        select = PLANET_SELECT_CANON
    );

    let mut stmt = con.prepare(&sql)?;
    let planet = stmt.query_row([planet_norm], Planet::from_row).optional()?;

    Ok(planet)
}

pub fn find_planet_by_alias_norm(con: &Connection, alias_norm: &str) -> Result<Option<Planet>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM planet_aliases a
        JOIN planets p ON p.FID = a.planet_fid
        WHERE a.alias_norm = ?1
        ORDER BY p.Planet COLLATE NOCASE
        LIMIT 1
        "#,
        select = PLANET_SELECT_CANON
    );

    let mut stmt = con.prepare(&sql)?;
    let planet = stmt.query_row([alias_norm], Planet::from_row).optional()?;

    Ok(planet)
}

pub fn find_planet_for_info(con: &Connection, key_norm: &str) -> Result<Option<Planet>> {
    if let Some(p) = find_planet_by_norm(con, key_norm)? {
        return Ok(Some(p));
    }
    find_planet_by_alias_norm(con, key_norm)
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

const WAYPOINT_SELECT: &str = r#"
  w.id          AS id,
  w.name        AS name,
  w.name_norm   AS name_norm,
  w.x           AS x,
  w.y           AS y,
  w.kind        AS kind,
  w.fingerprint AS fingerprint,
  w.note        AS note,
  w.created_at  AS created_at,
  w.updated_at  AS updated_at
"#;

fn waypoint_from_row(r: &Row<'_>) -> rusqlite::Result<Waypoint> {
    Ok(Waypoint {
        id: r.get("id")?,
        name: r.get("name")?,
        name_norm: r.get("name_norm")?,
        x: r.get("x")?,
        y: r.get("y")?,
        kind: r.get("kind")?,
        fingerprint: r.get("fingerprint")?,
        note: r.get("note")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

pub fn insert_waypoint(
    con: &Connection,
    name: &str,
    name_norm: &str,
    x: f64,
    y: f64,
    kind: &str,
    note: Option<&str>,
) -> Result<i64> {
    con.execute(
        r#"
        INSERT INTO waypoints (name, name_norm, x, y, kind, note)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![name, name_norm, x, y, kind, note],
    )?;

    Ok(con.last_insert_rowid())
}

pub fn find_waypoint_by_norm(con: &Connection, name_norm: &str) -> Result<Option<Waypoint>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM waypoints w
        WHERE w.name_norm = ?1
        LIMIT 1
        "#,
        select = WAYPOINT_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let wp = stmt.query_row([name_norm], waypoint_from_row).optional()?;
    Ok(wp)
}

pub fn list_waypoints(con: &Connection, limit: usize, offset: usize) -> Result<Vec<Waypoint>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM waypoints w
        ORDER BY w.name COLLATE NOCASE
        LIMIT ?1 OFFSET ?2
        "#,
        select = WAYPOINT_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let rows = stmt.query_map(params![limit as i64, offset as i64], waypoint_from_row)?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn delete_waypoint(con: &Connection, id: i64) -> Result<usize> {
    let n = con.execute("DELETE FROM waypoints WHERE id = ?1", [id])?;
    Ok(n)
}

pub fn find_waypoint_by_id(con: &Connection, id: i64) -> Result<Option<Waypoint>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM waypoints w
        WHERE w.id = ?1
        LIMIT 1
        "#,
        select = WAYPOINT_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let wp = stmt.query_row([id], waypoint_from_row).optional()?;
    Ok(wp)
}

// ---------- Waypoint <-> Planet links ----------

pub fn link_waypoint_to_planet(
    con: &Connection,
    waypoint_id: i64,
    planet_fid: i64,
    role: &str,
    distance: Option<f64>,
) -> Result<()> {
    con.execute(
        r#"
        INSERT INTO waypoint_planets(waypoint_id, planet_fid, role, distance)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(waypoint_id, planet_fid) DO UPDATE SET
          role=excluded.role,
          distance=excluded.distance
        "#,
        params![waypoint_id, planet_fid, role, distance],
    )?;
    Ok(())
}

pub fn unlink_waypoint_from_planet(
    con: &Connection,
    waypoint_id: i64,
    planet_fid: i64,
) -> Result<usize> {
    let n = con.execute(
        "DELETE FROM waypoint_planets WHERE waypoint_id = ?1 AND planet_fid = ?2",
        params![waypoint_id, planet_fid],
    )?;
    Ok(n)
}

pub fn delete_waypoint_links(con: &Connection, waypoint_id: i64) -> Result<usize> {
    let n = con.execute(
        "DELETE FROM waypoint_planets WHERE waypoint_id = ?1",
        [waypoint_id],
    )?;
    Ok(n)
}

fn link_from_row(r: &Row<'_>) -> rusqlite::Result<WaypointPlanetLink> {
    Ok(WaypointPlanetLink {
        waypoint_id: r.get("waypoint_id")?,
        planet_fid: r.get("planet_fid")?,
        role: r.get("role")?,
        distance: r.get("distance")?,
    })
}

pub fn list_links_for_waypoint(
    con: &Connection,
    waypoint_id: i64,
) -> Result<Vec<WaypointPlanetLink>> {
    let mut stmt = con.prepare(
        r#"
        SELECT
          waypoint_id AS waypoint_id,
          planet_fid  AS planet_fid,
          role        AS role,
          distance    AS distance
        FROM waypoint_planets
        WHERE waypoint_id = ?1
        ORDER BY role, planet_fid
        "#,
    )?;

    let rows = stmt.query_map([waypoint_id], link_from_row)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

#[allow(dead_code)]
pub fn list_links_for_planet(con: &Connection, planet_fid: i64) -> Result<Vec<WaypointPlanetLink>> {
    let mut stmt = con.prepare(
        r#"
        SELECT
          waypoint_id AS waypoint_id,
          planet_fid  AS planet_fid,
          role        AS role,
          distance    AS distance
        FROM waypoint_planets
        WHERE planet_fid = ?1
        ORDER BY role, waypoint_id
        "#,
    )?;

    let rows = stmt.query_map([planet_fid], link_from_row)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

// ---------- List waypoints for a given planet ----------

pub fn list_waypoints_for_planet(
    con: &Connection,
    planet_fid: i64,
    role: Option<&str>, // filter optional
    limit: usize,
    offset: usize,
) -> Result<Vec<Waypoint>> {
    // If role is provided, filter by role; else list all.
    let sql = if role.is_some() {
        format!(
            r#"
            SELECT {select}
            FROM waypoint_planets wp
            JOIN waypoints w ON w.id = wp.waypoint_id
            WHERE wp.planet_fid = ?1 AND wp.role = ?2
            ORDER BY w.name COLLATE NOCASE
            LIMIT ?3 OFFSET ?4
            "#,
            select = WAYPOINT_SELECT
        )
    } else {
        format!(
            r#"
            SELECT {select}
            FROM waypoint_planets wp
            JOIN waypoints w ON w.id = wp.waypoint_id
            WHERE wp.planet_fid = ?1
            ORDER BY w.name COLLATE NOCASE
            LIMIT ?2 OFFSET ?3
            "#,
            select = WAYPOINT_SELECT
        )
    };

    let mut stmt = con.prepare(&sql)?;

    let rows = if let Some(role) = role {
        stmt.query_map(
            params![planet_fid, role, limit as i64, offset as i64],
            waypoint_from_row,
        )?
    } else {
        stmt.query_map(
            params![planet_fid, limit as i64, offset as i64],
            waypoint_from_row,
        )?
    };

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

#[allow(dead_code)]
pub fn find_waypoint_by_fingerprint(con: &Connection, fp: &str) -> Result<Option<Waypoint>> {
    let sql = format!(
        r#"
        SELECT {select}
        FROM waypoints w
        WHERE w.fingerprint = ?1
        LIMIT 1
        "#,
        select = WAYPOINT_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let wp = stmt.query_row([fp], waypoint_from_row).optional()?;
    Ok(wp)
}

/// Insert a computed waypoint if not already present (by fingerprint).
/// Returns (waypoint_id, created_new).
#[allow(dead_code, clippy::too_many_arguments)]
pub fn upsert_computed_waypoint(
    con: &Connection,
    name: &str,
    name_norm: &str,
    x: f64,
    y: f64,
    kind: &str, // e.g. "computed"
    note: Option<&str>,
    fingerprint: &str,
) -> Result<(i64, bool)> {
    if let Some(existing) = find_waypoint_by_fingerprint(con, fingerprint)? {
        return Ok((existing.id, false));
    }

    con.execute(
        r#"
        INSERT INTO waypoints (name, name_norm, x, y, kind, note, fingerprint)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![name, name_norm, x, y, kind, note, fingerprint],
    )?;

    Ok((con.last_insert_rowid(), true))
}

pub fn list_planets_in_bbox(
    con: &Connection,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    limit: usize,
) -> Result<Vec<(i64, String, f64, f64)>> {
    let mut stmt = con.prepare(
        r#"
        SELECT FID, Planet, X, Y
        FROM planets
        WHERE deleted = 0
          AND X BETWEEN ?1 AND ?2
          AND Y BETWEEN ?3 AND ?4
        ORDER BY Planet COLLATE NOCASE
        LIMIT ?5
        "#,
    )?;

    let rows = stmt.query_map(
        rusqlite::params![min_x, max_x, min_y, max_y, limit as i64],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}
