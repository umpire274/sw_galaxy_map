use super::row_mappers::{route_detour_from_row, route_from_row, route_waypoint_from_row};
use super::{link_waypoint_to_planet, upsert_computed_waypoint};
use crate::model::{RouteDetourRow, RouteListRow, RouteLoaded, RouteRow, RouteWaypointRow};
use crate::routing::router::{DetourDecision, Route as ComputedRoute, RouteOptions};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

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

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

fn detour_fingerprint(from_fid: i64, to_fid: i64, d: &DetourDecision) -> String {
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
          score_base, score_turn, score_back, score_proximity, score_total,
          tries_used, tries_exhausted
        ) VALUES (
          ?1, ?2,
          ?3, ?4,
          ?5, ?6, ?7, ?8,
          ?9, ?10, ?11, ?12,
          ?13,
          ?14, ?15, ?16,
          ?17, ?18, ?19, ?20, ?21,
          ?22, ?23
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
            total,
            d.tries_used as i64,
            d.tries_exhausted as i64,
        ],
    )?;

    Ok(())
}

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

    delete_route_children(&tx, route_id)?;

    use std::collections::HashMap;
    let mut detour_wp_ids: HashMap<String, i64> = HashMap::new();

    for (idx, d) in route.detours.iter().enumerate() {
        let fp = detour_fingerprint(from_planet_fid, to_planet_fid, d);

        let wp_name = format!("Detour {}", fp.get(0..8).unwrap_or("detour"));
        let wp_norm = crate::utils::normalize::normalize_text(&wp_name);

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

        let dist_to_ob = crate::routing::geometry::dist(d.waypoint, d.obstacle_center);
        let _ = link_waypoint_to_planet(&tx, wp_id, d.obstacle_id, "avoid", Some(dist_to_ob));

        insert_route_detour(&tx, route_id, idx, d, Some(wp_id))?;

        let key = format!("{:.4},{:.4}", round4(d.waypoint.x), round4(d.waypoint.y));
        detour_wp_ids.insert(key, wp_id);
    }

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
          d.score_total      AS score_total,
          d.tries_used       AS tries_used,
          d.tries_exhausted  AS tries_exhausted
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

pub fn list_routes(
    con: &Connection,
    limit: usize,
    status: Option<&str>,
    from: Option<i64>,
    to: Option<i64>,
    wp: Option<usize>,
    sort: crate::domain::RouteListSort,
) -> Result<(Vec<RouteListRow>, usize)> {
    use rusqlite::types::Value;

    let mut where_parts: Vec<&'static str> = Vec::new();
    let mut params: Vec<Value> = Vec::new();

    if let Some(s) = status {
        where_parts.push("r.status = ? COLLATE NOCASE");
        params.push(Value::Text(s.to_string()));
    }
    if let Some(fid) = from {
        where_parts.push("r.from_planet_fid = ?");
        params.push(Value::Integer(fid));
    }
    if let Some(fid) = to {
        where_parts.push("r.to_planet_fid = ?");
        params.push(Value::Integer(fid));
    }

    let order_sql = match sort {
        crate::domain::RouteListSort::Updated => {
            "ORDER BY COALESCE(r.updated_at, r.created_at) DESC, r.id DESC"
        }
        crate::domain::RouteListSort::Id => "ORDER BY r.id DESC",
        crate::domain::RouteListSort::Length => {
            "ORDER BY (r.length IS NULL) ASC, r.length ASC, r.id DESC"
        }
    };

    let mut where_sql = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    let use_wp = wp.is_some();
    if let Some(n) = wp {
        if where_sql.is_empty() {
            where_sql = "WHERE COALESCE(wp.cnt, 0) = ?".to_string();
        } else {
            where_sql.push_str(" AND COALESCE(wp.cnt, 0) = ?");
        }
        params.push(Value::Integer(n as i64));
    }

    let total: usize = if use_wp {
        let sql_count = format!(
            r#"
            WITH
              wp AS (
                SELECT route_id, COUNT(*) AS cnt
                FROM route_waypoints
                GROUP BY route_id
              )
            SELECT COUNT(*)
            FROM routes r
            JOIN planets fp ON fp.FID = r.from_planet_fid
            JOIN planets tp ON tp.FID = r.to_planet_fid
            LEFT JOIN wp ON wp.route_id = r.id
            {where_sql}
            "#,
            where_sql = where_sql
        );

        let n: i64 = con.query_row(
            &sql_count,
            rusqlite::params_from_iter(params.iter()),
            |row| row.get(0),
        )?;
        n.max(0) as usize
    } else {
        let sql_count = format!(
            r#"
            SELECT COUNT(*)
            FROM routes r
            JOIN planets fp ON fp.FID = r.from_planet_fid
            JOIN planets tp ON tp.FID = r.to_planet_fid
            {where_sql}
            "#,
            where_sql = where_sql
        );

        let n: i64 = con.query_row(
            &sql_count,
            rusqlite::params_from_iter(params.iter()),
            |row| row.get(0),
        )?;
        n.max(0) as usize
    };

    let mut list_params = params.clone();
    list_params.push(Value::Integer(limit as i64));

    let sql_list = if use_wp {
        format!(
            r#"
            WITH
              wp AS (
                SELECT route_id, COUNT(*) AS cnt
                FROM route_waypoints
                GROUP BY route_id
              ),
              dt AS (
                SELECT route_id, COUNT(*) AS cnt
                FROM route_detours
                GROUP BY route_id
              )
            SELECT
              r.id AS id,
              r.from_planet_fid AS from_planet_fid,
              fp.Planet AS from_planet_name,
              r.to_planet_fid AS to_planet_fid,
              tp.Planet AS to_planet_name,
              r.status AS status,
              r.length AS length,
              r.iterations AS iterations,
              r.created_at AS created_at,
              r.updated_at AS updated_at,
              COALESCE(wp.cnt, 0) AS waypoints_count,
              COALESCE(dt.cnt, 0) AS detours_count
            FROM routes r
            JOIN planets fp ON fp.FID = r.from_planet_fid
            JOIN planets tp ON tp.FID = r.to_planet_fid
            LEFT JOIN wp ON wp.route_id = r.id
            LEFT JOIN dt ON dt.route_id = r.id
            {where_sql}
            {order_sql}
            LIMIT ?
            "#,
            where_sql = where_sql,
            order_sql = order_sql
        )
    } else {
        format!(
            r#"
            SELECT
              r.id AS id,
              r.from_planet_fid AS from_planet_fid,
              fp.Planet AS from_planet_name,
              r.to_planet_fid AS to_planet_fid,
              tp.Planet AS to_planet_name,
              r.status AS status,
              r.length AS length,
              r.iterations AS iterations,
              r.created_at AS created_at,
              r.updated_at AS updated_at,
              (SELECT COUNT(*) FROM route_waypoints w WHERE w.route_id = r.id) AS waypoints_count,
              (SELECT COUNT(*) FROM route_detours d WHERE d.route_id = r.id) AS detours_count
            FROM routes r
            JOIN planets fp ON fp.FID = r.from_planet_fid
            JOIN planets tp ON tp.FID = r.to_planet_fid
            {where_sql}
            {order_sql}
            LIMIT ?
            "#,
            where_sql = where_sql,
            order_sql = order_sql
        )
    };

    let mut stmt = con.prepare(&sql_list)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(list_params.iter()), |row| {
            Ok(RouteListRow {
                id: row.get("id")?,
                from_planet_fid: row.get("from_planet_fid")?,
                from_planet_name: row.get("from_planet_name")?,
                to_planet_fid: row.get("to_planet_fid")?,
                to_planet_name: row.get("to_planet_name")?,
                status: row.get("status")?,
                length: row.get("length")?,
                iterations: row.get("iterations")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
                waypoints_count: row.get("waypoints_count")?,
                detours_count: row.get("detours_count")?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok((rows, total))
}
