use anyhow::Result;
use rusqlite::Connection;

/// Returns aggregated galaxy statistics used by CLI reporting.
pub fn galaxy_stats(con: &Connection, top_n: usize) -> Result<crate::model::GalaxyStats> {
    use crate::model::GalaxyStats;

    let mut s = GalaxyStats {
        total_planets: con.query_row("SELECT COUNT(*) FROM planets", [], |r| r.get(0))?,
        ..GalaxyStats::default()
    };

    // By status
    let mut stmt = con.prepare(
        "SELECT COALESCE(status, ''), COUNT(*) FROM planets GROUP BY COALESCE(status, '') ORDER BY COUNT(*) DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        let (st, cnt) = row?;
        match st.as_str() {
            "active" => s.status_active = cnt,
            "inserted" => s.status_inserted = cnt,
            "modified" => s.status_modified = cnt,
            "skipped" => s.status_skipped = cnt,
            "deleted" => s.status_deleted = cnt,
            "invalid" => s.status_invalid = cnt,
            "" => s.status_null = cnt,
            _ => {}
        }
    }

    // Canon / Legends (on active planets only)
    s.canon_count = con.query_row(
        "SELECT COUNT(*) FROM planets WHERE COALESCE(Canon, 0) = 1 AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid'))",
        [],
        |r| r.get(0),
    )?;

    s.legends_count = con.query_row(
        "SELECT COUNT(*) FROM planets WHERE COALESCE(Legends, 0) = 1 AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid'))",
        [],
        |r| r.get(0),
    )?;

    s.both_count = con.query_row(
        "SELECT COUNT(*) FROM planets WHERE COALESCE(Canon, 0) = 1 AND COALESCE(Legends, 0) = 1 AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid'))",
        [],
        |r| r.get(0),
    )?;

    s.neither_count = con.query_row(
        "SELECT COUNT(*) FROM planets WHERE COALESCE(Canon, 0) = 0 AND COALESCE(Legends, 0) = 0 AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid'))",
        [],
        |r| r.get(0),
    )?;

    // Top regions
    {
        let mut stmt = con.prepare(
            "SELECT Region, COUNT(*) AS cnt FROM planets WHERE Region IS NOT NULL AND TRIM(Region) != '' AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid')) GROUP BY Region ORDER BY cnt DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([top_n as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            s.top_regions.push(row?);
        }
    }

    // Top sectors
    {
        let mut stmt = con.prepare(
            "SELECT Sector, COUNT(*) AS cnt FROM planets WHERE Sector IS NOT NULL AND TRIM(Sector) != '' AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid')) GROUP BY Sector ORDER BY cnt DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([top_n as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            s.top_sectors.push(row?);
        }
    }

    // Grid coverage
    s.distinct_grids = con.query_row(
        "SELECT COUNT(DISTINCT Grid) FROM planets WHERE Grid IS NOT NULL AND TRIM(Grid) != '' AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid'))",
        [],
        |r| r.get(0),
    )?;

    {
        let mut stmt = con.prepare(
            "SELECT Grid, COUNT(*) AS cnt FROM planets WHERE Grid IS NOT NULL AND TRIM(Grid) != '' AND (status IS NULL OR status NOT IN ('deleted', 'skipped', 'invalid')) GROUP BY Grid ORDER BY cnt DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([top_n as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            s.top_grids.push(row?);
        }
    }

    // Routes (if table exists)
    if crate::db::has_table(con, "routes")? {
        s.total_routes = con.query_row("SELECT COUNT(*) FROM routes", [], |r| r.get(0))?;

        s.routes_ok =
            con.query_row("SELECT COUNT(*) FROM routes WHERE status = 'ok'", [], |r| {
                r.get(0)
            })?;

        s.routes_failed = con.query_row(
            "SELECT COUNT(*) FROM routes WHERE status = 'failed'",
            [],
            |r| r.get(0),
        )?;

        s.total_route_length = con
            .query_row(
                "SELECT COALESCE(SUM(length), 0.0) FROM routes WHERE status = 'ok'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);

        if s.routes_ok > 0 && crate::db::has_table(con, "route_detours")? {
            let total_detours: i64 =
                con.query_row("SELECT COUNT(*) FROM route_detours", [], |r| r.get(0))?;
            s.avg_detours_per_route = total_detours as f64 / s.routes_ok as f64;
        }
    }

    Ok(s)
}
