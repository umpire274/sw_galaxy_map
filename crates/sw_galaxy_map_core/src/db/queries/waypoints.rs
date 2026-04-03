use super::row_mappers::{link_from_row, waypoint_from_row};
use crate::model::{
    Waypoint, WaypointLinkRow, WaypointListRow, WaypointPlanetLink, WaypointRouteRow,
};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

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

/// Inserts a new waypoint and returns its row id.
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

/// Returns a waypoint by normalized name.
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

/// Returns a paginated waypoint list together with the total count.
pub fn list_waypoints(
    con: &Connection,
    limit: usize,
    offset: usize,
) -> Result<(Vec<WaypointListRow>, usize)> {
    let total: i64 = con.query_row(r#"SELECT COUNT(*) FROM waypoints"#, [], |row| row.get(0))?;

    let sql = format!(
        r#"
        WITH
          lp AS (
            SELECT waypoint_id, COUNT(*) AS cnt
            FROM waypoint_planets
            GROUP BY waypoint_id
          ),
          rw AS (
            SELECT waypoint_id, COUNT(*) AS cnt
            FROM route_waypoints
            WHERE waypoint_id IS NOT NULL
            GROUP BY waypoint_id
          )
        SELECT
          {select},
          COALESCE(lp.cnt, 0) AS links_count,
          COALESCE(rw.cnt, 0) AS routes_count
        FROM waypoints w
        LEFT JOIN lp ON lp.waypoint_id = w.id
        LEFT JOIN rw ON rw.waypoint_id = w.id
        ORDER BY w.name COLLATE NOCASE
        LIMIT ?1 OFFSET ?2
        "#,
        select = WAYPOINT_SELECT
    );

    let mut stmt = con.prepare(&sql)?;

    let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
        let wp = waypoint_from_row(row)?;
        let links_count: i64 = row.get("links_count")?;
        let routes_count: i64 = row.get("routes_count")?;

        Ok(WaypointListRow {
            waypoint: wp,
            links_count,
            routes_count,
        })
    })?;

    let mut out: Vec<WaypointListRow> = Vec::new();
    for r in rows {
        out.push(r?);
    }

    Ok((out, total.max(0) as usize))
}

/// Deletes a waypoint by id.
pub fn delete_waypoint(con: &Connection, id: i64) -> Result<usize> {
    let n = con.execute("DELETE FROM waypoints WHERE id = ?1", [id])?;
    Ok(n)
}

/// Returns a waypoint by id.
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

/// Creates or updates a waypoint <-> planet link.
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

/// Deletes a single waypoint <-> planet link.
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

/// Deletes all links for a waypoint.
pub fn delete_waypoint_links(con: &Connection, waypoint_id: i64) -> Result<usize> {
    let n = con.execute(
        "DELETE FROM waypoint_planets WHERE waypoint_id = ?1",
        [waypoint_id],
    )?;
    Ok(n)
}

/// Returns all planet links for a waypoint with planet names.
pub fn list_waypoint_links(con: &Connection, waypoint_id: i64) -> Result<Vec<WaypointLinkRow>> {
    let mut stmt = con.prepare(
        r#"
        SELECT
          wp.planet_fid AS planet_fid,
          p.Planet      AS planet_name,
          COALESCE(wp.role, '') AS role,
          wp.distance   AS distance
        FROM waypoint_planets wp
        JOIN planets p ON p.FID = wp.planet_fid
        WHERE wp.waypoint_id = ?1
        ORDER BY p.Planet COLLATE NOCASE
        "#,
    )?;

    let rows = stmt.query_map([waypoint_id], |row| {
        Ok(WaypointLinkRow {
            planet_fid: row.get("planet_fid")?,
            planet_name: row.get("planet_name")?,
            role: row.get("role")?,
            distance: row.get("distance")?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Returns all routes that reference the given waypoint.
pub fn list_routes_for_waypoint(
    con: &Connection,
    waypoint_id: i64,
) -> Result<Vec<WaypointRouteRow>> {
    let mut stmt = con.prepare(
        r#"
        SELECT
          r.id AS id,

          r.from_planet_fid AS from_planet_fid,
          pf.Planet AS from_planet_name,

          r.to_planet_fid AS to_planet_fid,
          pt.Planet AS to_planet_name,

          r.status AS status,
          r.length AS length,

          COALESCE(r.updated_at, r.created_at) AS updated_at,

          COUNT(*) AS occurrences
        FROM route_waypoints rw
        JOIN routes r ON r.id = rw.route_id
        JOIN planets pf ON pf.FID = r.from_planet_fid
        JOIN planets pt ON pt.FID = r.to_planet_fid
        WHERE rw.waypoint_id = ?1
        GROUP BY
          r.id, r.from_planet_fid, pf.Planet, r.to_planet_fid, pt.Planet,
          r.status, r.length, COALESCE(r.updated_at, r.created_at)
        ORDER BY COALESCE(r.updated_at, r.created_at) DESC, r.id DESC
        "#,
    )?;

    let rows = stmt.query_map([waypoint_id], |row| {
        Ok(WaypointRouteRow {
            id: row.get("id")?,
            from_planet_fid: row.get("from_planet_fid")?,
            from_planet_name: row.get("from_planet_name")?,
            to_planet_fid: row.get("to_planet_fid")?,
            to_planet_name: row.get("to_planet_name")?,
            status: row.get("status")?,
            length: row.get("length")?,
            updated_at: row.get("updated_at")?,
            occurrences: row.get("occurrences")?,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

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

/// Returns waypoints linked to a specific planet, optionally filtered by role.
pub fn list_waypoints_for_planet(
    con: &Connection,
    planet_fid: i64,
    role: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<Waypoint>> {
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

/// Inserts a computed waypoint if not already present by fingerprint.
/// Returns `(waypoint_id, created_new)`.
#[allow(clippy::too_many_arguments)]
pub fn upsert_computed_waypoint(
    con: &Connection,
    name: &str,
    name_norm: &str,
    x: f64,
    y: f64,
    kind: &str,
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
