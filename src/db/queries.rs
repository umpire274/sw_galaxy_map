use crate::db::has_table;
use crate::model::{AliasRow, NearHit, Planet, Waypoint, WaypointPlanetLink};
use crate::model::{RouteDetourRow, RouteLoaded, RouteRow, RouteWaypointRow};
use crate::routing::router::{DetourDecision, Route as ComputedRoute, RouteOptions};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};

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

const ROUTE_SELECT: &str = r#"
  r.id             AS id,
  r.from_planet_fid AS from_planet_fid,
  r.to_planet_fid   AS to_planet_fid,
  pf.Planet         AS from_planet_name,
  pt.Planet         AS to_planet_name,
  r.algo_version    AS algo_version,
  r.options_json    AS options_json,
  r.length          AS length,
  r.iterations      AS iterations,
  r.status          AS status,
  r.error           AS error,
  r.created_at      AS created_at,
  r.updated_at      AS updated_at
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

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

fn detour_fingerprint(from_fid: i64, to_fid: i64, d: &DetourDecision) -> String {
    // fingerprint deterministico (con rounding), sufficiente per deduplica dei computed-waypoints
    let s = format!(
        "detour|from={}|to={}|ob={}|it={}|seg={}|x={:.4}|y={:.4}",
        from_fid,
        to_fid,
        d.obstacle_id,
        d.iteration,
        d.segment_index,
        round4(d.waypoint.x),
        round4(d.waypoint.y)
    );

    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

pub fn insert_route_waypoint(
    con: &Connection,
    route_id: i64,
    seq: usize,
    x: f64,
    y: f64,
    waypoint_id: Option<i64>,
) -> Result<()> {
    con.execute(
        r#"
        INSERT INTO route_waypoints(route_id, seq, x, y, waypoint_id)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![route_id, seq as i64, x, y, waypoint_id],
    )?;
    Ok(())
}

pub fn insert_route_detour(
    con: &Connection,
    route_id: i64,
    idx: usize,
    d: &DetourDecision,
    waypoint_id: Option<i64>,
) -> Result<()> {
    let total = d.score.total();

    con.execute(
        r#"
        INSERT INTO route_detours(
          route_id, idx,
          iteration, segment_index,
          obstacle_id, obstacle_x, obstacle_y, obstacle_radius,
          closest_t, closest_qx, closest_qy, closest_dist,
          offset_used,
          wp_x, wp_y, waypoint_id,
          score_base, score_turn, score_back, score_proximity, score_total
        ) VALUES (
          ?1, ?2,
          ?3, ?4,
          ?5, ?6, ?7, ?8,
          ?9, ?10, ?11, ?12,
          ?13,
          ?14, ?15, ?16,
          ?17, ?18, ?19, ?20, ?21
        )
        "#,
        params![
            route_id,
            idx as i64,
            d.iteration as i64,
            d.segment_index as i64,
            d.obstacle_id,
            d.obstacle_center.x,
            d.obstacle_center.y,
            d.obstacle_radius,
            d.closest_t,
            d.closest_q.x,
            d.closest_q.y,
            d.closest_dist,
            d.offset_used,
            d.waypoint.x,
            d.waypoint.y,
            waypoint_id,
            d.score.base,
            d.score.turn,
            d.score.back,
            d.score.proximity,
            total
        ],
    )?;

    Ok(())
}

/// Persist a computed route and its detours.
/// - Creates a routes row
/// - Upserts detour waypoints into catalog (waypoints) using fingerprint
/// - Stores polyline (route_waypoints) and detour details (route_detours)
pub fn persist_route(
    con: &mut Connection,
    from_planet_fid: i64,
    to_planet_fid: i64,
    opts: RouteOptions,
    route: &ComputedRoute,
) -> Result<i64> {
    let tx = con
        .transaction()
        .context("Failed to start route persistence transaction")?;

    let options_json = serde_json::to_string(&serde_json::json!({
        "clearance": opts.clearance,
        "max_iters": opts.max_iters,
        "max_offset_tries": opts.max_offset_tries,
        "offset_growth": opts.offset_growth,
        "turn_weight": opts.turn_weight,
        "back_weight": opts.back_weight,
        "proximity_weight": opts.proximity_weight,
        "proximity_margin": opts.proximity_margin,
    }))?;

    let route_id = upsert_route_id(
        &tx,
        from_planet_fid,
        to_planet_fid,
        "router_v1",
        &options_json,
        route.length,
        route.iterations,
    )?;

    // IMPORTANT: replace existing polyline/detours for this from->to
    delete_route_children(&tx, route_id)?;

    // 1) Detours: upsert computed waypoint + store detour row.
    // Build a small lookup (rounded) to attach waypoint_id to the polyline too.
    use std::collections::HashMap;
    let mut detour_wp_ids: HashMap<String, i64> = HashMap::new();

    for (idx, d) in route.detours.iter().enumerate() {
        let fp = detour_fingerprint(from_planet_fid, to_planet_fid, d);

        let wp_name = format!("Detour {}", fp.get(0..8).unwrap_or("detour"));
        let wp_norm = crate::normalize::normalize_text(&wp_name);

        let (wp_id, _created) = upsert_computed_waypoint(
            &tx,
            &wp_name,
            &wp_norm,
            d.waypoint.x,
            d.waypoint.y,
            "computed",
            Some("Computed detour waypoint"),
            &fp,
        )?;

        // Optional but useful: link computed waypoint to the obstacle planet as "avoid"
        // (obstacle_id is a planet fid in your current model)
        // distance = dist between waypoint and obstacle center
        let dist_to_ob = crate::routing::geometry::dist(d.waypoint, d.obstacle_center);
        // Ignore errors if already linked (depending on your UNIQUE/PK constraints you may want INSERT OR REPLACE)
        let _ = link_waypoint_to_planet(&tx, wp_id, d.obstacle_id, "avoid", Some(dist_to_ob));

        insert_route_detour(&tx, route_id, idx, d, Some(wp_id))?;

        let key = format!("{:.4},{:.4}", round4(d.waypoint.x), round4(d.waypoint.y));
        detour_wp_ids.insert(key, wp_id);
    }

    // 2) Route polyline
    for (seq, p) in route.waypoints.iter().enumerate() {
        let key = format!("{:.4},{:.4}", round4(p.x), round4(p.y));
        let waypoint_id = detour_wp_ids.get(&key).copied();
        insert_route_waypoint(&tx, route_id, seq, p.x, p.y, waypoint_id)?;
    }

    tx.commit()
        .context("Failed to commit route persistence transaction")?;
    Ok(route_id)
}

pub fn upsert_route_id(
    con: &Connection,
    from_planet_fid: i64,
    to_planet_fid: i64,
    algo_version: &str,
    options_json: &str,
    length: f64,
    iterations: usize,
) -> Result<i64> {
    con.execute(
        r#"
        INSERT INTO routes(
          from_planet_fid, to_planet_fid, algo_version, options_json,
          length, iterations, status, error, created_at, updated_at
        )
        VALUES (
          ?1, ?2, ?3, ?4,
          ?5, ?6, 'ok', NULL,
          strftime('%Y-%m-%dT%H:%M:%fZ','now'),
          strftime('%Y-%m-%dT%H:%M:%fZ','now')
        )
        ON CONFLICT(from_planet_fid, to_planet_fid) DO UPDATE SET
          algo_version = excluded.algo_version,
          options_json = excluded.options_json,
          length       = excluded.length,
          iterations   = excluded.iterations,
          status       = 'ok',
          error        = NULL,
          updated_at   = excluded.updated_at
        "#,
        params![
            from_planet_fid,
            to_planet_fid,
            algo_version,
            options_json,
            length,
            iterations as i64
        ],
    )?;

    let id: i64 = con.query_row(
        r#"
        SELECT id
        FROM routes
        WHERE from_planet_fid = ?1 AND to_planet_fid = ?2
        "#,
        params![from_planet_fid, to_planet_fid],
        |r| r.get(0),
    )?;

    Ok(id)
}

fn delete_route_children(con: &Connection, route_id: i64) -> Result<()> {
    con.execute(
        "DELETE FROM route_waypoints WHERE route_id = ?1",
        [route_id],
    )?;
    con.execute("DELETE FROM route_detours WHERE route_id = ?1", [route_id])?;
    Ok(())
}

fn route_from_row(r: &Row<'_>) -> rusqlite::Result<RouteRow> {
    Ok(RouteRow {
        id: r.get("id")?,
        from_planet_fid: r.get("from_planet_fid")?,
        to_planet_fid: r.get("to_planet_fid")?,
        from_planet_name: r.get("from_planet_name")?,
        to_planet_name: r.get("to_planet_name")?,
        algo_version: r.get("algo_version")?,
        options_json: r.get("options_json")?,
        length: r.get("length")?,
        iterations: r.get("iterations")?,
        status: r.get("status")?,
        error: r.get("error")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

fn route_waypoint_from_row(r: &Row<'_>) -> rusqlite::Result<RouteWaypointRow> {
    Ok(RouteWaypointRow {
        seq: r.get("seq")?,
        x: r.get("x")?,
        y: r.get("y")?,
        waypoint_id: r.get("waypoint_id")?,
        waypoint_name: r.get("waypoint_name")?,
        waypoint_kind: r.get("waypoint_kind")?,
    })
}

fn route_detour_from_row(r: &Row<'_>) -> rusqlite::Result<RouteDetourRow> {
    Ok(RouteDetourRow {
        idx: r.get("idx")?,
        iteration: r.get("iteration")?,
        segment_index: r.get("segment_index")?,

        obstacle_id: r.get("obstacle_id")?,
        obstacle_name: r.get("obstacle_name")?,
        obstacle_x: r.get("obstacle_x")?,
        obstacle_y: r.get("obstacle_y")?,
        obstacle_radius: r.get("obstacle_radius")?,

        closest_t: r.get("closest_t")?,
        closest_qx: r.get("closest_qx")?,
        closest_qy: r.get("closest_qy")?,
        closest_dist: r.get("closest_dist")?,

        offset_used: r.get("offset_used")?,

        wp_x: r.get("wp_x")?,
        wp_y: r.get("wp_y")?,
        waypoint_id: r.get("waypoint_id")?,

        score_base: r.get("score_base")?,
        score_turn: r.get("score_turn")?,
        score_back: r.get("score_back")?,
        score_proximity: r.get("score_proximity")?,
        score_total: r.get("score_total")?,
    })
}

pub fn get_route_by_from_to(
    con: &Connection,
    from_planet_fid: i64,
    to_planet_fid: i64,
) -> Result<Option<RouteRow>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM routes r
        JOIN planets pf ON pf.FID = r.from_planet_fid
        JOIN planets pt ON pt.FID = r.to_planet_fid
        WHERE r.from_planet_fid = ?1 AND r.to_planet_fid = ?2
        LIMIT 1
        "#,
        select = ROUTE_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let row = stmt
        .query_row(params![from_planet_fid, to_planet_fid], route_from_row)
        .optional()?;

    Ok(row)
}

pub fn load_route(con: &Connection, route_id: i64) -> Result<Option<RouteLoaded>> {
    // 1) Route header
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM routes r
        JOIN planets pf ON pf.FID = r.from_planet_fid
        JOIN planets pt ON pt.FID = r.to_planet_fid
        WHERE r.id = ?1
        LIMIT 1
        "#,
        select = ROUTE_SELECT
    );

    let mut stmt_route = con.prepare(&sql)?;
    let route = stmt_route
        .query_row([route_id], route_from_row)
        .optional()?;

    let Some(route) = route else {
        return Ok(None);
    };

    // 2) Polyline
    let mut stmt_wp = con.prepare(
        r#"
        SELECT
          rw.seq         AS seq,
          rw.x           AS x,
          rw.y           AS y,
          rw.waypoint_id AS waypoint_id,

          w.name         AS waypoint_name,
          w.kind         AS waypoint_kind
        FROM route_waypoints rw
        LEFT JOIN waypoints w ON w.id = rw.waypoint_id
        WHERE rw.route_id = ?1
        ORDER BY rw.seq ASC
        "#,
    )?;

    let mut waypoints: Vec<RouteWaypointRow> = Vec::new();
    let rows = stmt_wp.query_map([route_id], route_waypoint_from_row)?;
    for r in rows {
        waypoints.push(r?);
    }

    // 3) Detours
    let mut stmt_det = con.prepare(
        r#"
        SELECT
          d.idx              AS idx,
          d.iteration        AS iteration,
          d.segment_index    AS segment_index,
    
          d.obstacle_id      AS obstacle_id,
          COALESCE(p.Planet, '') AS obstacle_name,
    
          d.obstacle_x       AS obstacle_x,
          d.obstacle_y       AS obstacle_y,
          d.obstacle_radius  AS obstacle_radius,
    
          d.closest_t        AS closest_t,
          d.closest_qx       AS closest_qx,
          d.closest_qy       AS closest_qy,
          d.closest_dist     AS closest_dist,
    
          d.offset_used      AS offset_used,
    
          d.wp_x             AS wp_x,
          d.wp_y             AS wp_y,
          d.waypoint_id      AS waypoint_id,
    
          d.score_base       AS score_base,
          d.score_turn       AS score_turn,
          d.score_back       AS score_back,
          d.score_proximity  AS score_proximity,
          d.score_total      AS score_total
        FROM route_detours d
        LEFT JOIN planets p ON p.FID = d.obstacle_id
        WHERE d.route_id = ?1
        ORDER BY d.idx ASC
        "#,
    )?;

    let mut detours: Vec<RouteDetourRow> = Vec::new();
    let rows = stmt_det.query_map([route_id], route_detour_from_row)?;
    for r in rows {
        detours.push(r?);
    }

    Ok(Some(RouteLoaded {
        route,
        waypoints,
        detours,
    }))
}
