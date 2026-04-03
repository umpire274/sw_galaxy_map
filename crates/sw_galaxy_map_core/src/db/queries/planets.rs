use crate::model::{Planet, RoutingObstacleRow};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

fn planet_select() -> &'static str {
    r#"
    FID,
    Planet,
    planet_norm,
    Region,
    Sector,
    System,
    Grid,
    X,
    Y,
    Canon,
    Legends,
    zm,
    name0,
    name1,
    name2,
    lat,
    long,
    ref,
    status,
    cregion,
    cregion_li
    "#
}

fn planet_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Planet> {
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
}

/// Returns a planet by exact normalized name.
pub fn find_planet_by_norm(con: &Connection, planet_norm: &str) -> Result<Option<Planet>> {
    let sql = format!(
        r#"
        SELECT
            {}
        FROM planets
        WHERE planet_norm = ?1
        LIMIT 1
        "#,
        planet_select()
    );

    let mut stmt = con
        .prepare(&sql)
        .context("Failed to prepare find_planet_by_norm query")?;

    let row = stmt
        .query_row(params![planet_norm], planet_from_row)
        .optional()
        .context("Failed to execute find_planet_by_norm query")?;

    Ok(row)
}

/// Returns a planet by FID.
pub fn get_planet_by_fid(con: &Connection, fid: i64) -> Result<Option<Planet>> {
    let sql = format!(
        r#"
        SELECT
            {}
        FROM planets
        WHERE FID = ?1
        LIMIT 1
        "#,
        planet_select()
    );

    let mut stmt = con
        .prepare(&sql)
        .context("Failed to prepare get_planet_by_fid query")?;

    let row = stmt
        .query_row(params![fid], planet_from_row)
        .optional()
        .context("Failed to execute get_planet_by_fid query")?;

    Ok(row)
}

/// Returns a planet matched through an exact normalized alias.
pub fn find_planet_by_alias_norm(con: &Connection, alias_norm: &str) -> Result<Option<Planet>> {
    let sql = r#"
        SELECT
            p.FID,
            p.Planet,
            p.planet_norm,
            p.Region,
            p.Sector,
            p.System,
            p.Grid,
            p.X,
            p.Y,
            p.Canon,
            p.Legends,
            p.zm,
            p.name0,
            p.name1,
            p.name2,
            p.lat,
            p.long,
            p.ref,
            p.status,
            p.cregion,
            p.cregion_li
        FROM planet_aliases pa
        JOIN planets p
          ON p.FID = pa.planet_fid
        WHERE pa.alias_norm = ?1
        ORDER BY p.Planet ASC
        LIMIT 1
        "#
    .to_string();

    let mut stmt = con
        .prepare(&sql)
        .context("Failed to prepare find_planet_by_alias_norm query")?;

    let row = stmt
        .query_row(params![alias_norm], planet_from_row)
        .optional()
        .context("Failed to execute find_planet_by_alias_norm query")?;

    Ok(row)
}

/// Returns a planet for an info lookup, first by normalized planet name,
/// then by normalized alias.
pub fn find_planet_for_info(con: &Connection, query_norm: &str) -> Result<Option<Planet>> {
    if let Some(planet) = find_planet_by_norm(con, query_norm)? {
        return Ok(Some(planet));
    }

    find_planet_by_alias_norm(con, query_norm)
}

/// Returns planets inside the given bounding box, limited to the fields
/// needed by the route command fallback path.
pub fn list_planets_in_bbox(
    con: &Connection,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    limit: usize,
) -> Result<Vec<(i64, String, f64, f64)>> {
    let mut stmt = con
        .prepare(
            r#"
            SELECT
                FID,
                Planet,
                X,
                Y
            FROM planets
            WHERE X BETWEEN ?1 AND ?2
              AND Y BETWEEN ?3 AND ?4
            ORDER BY Planet ASC
            LIMIT ?5
            "#,
        )
        .context("Failed to prepare list_planets_in_bbox query")?;

    let rows = stmt
        .query_map(params![min_x, max_x, min_y, max_y, limit as i64], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, f64>(2)?,
                r.get::<_, f64>(3)?,
            ))
        })
        .context("Failed to execute list_planets_in_bbox query")?;

    let items = rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
    Ok(items)
}

/// Returns routing obstacles inside the given bounding box.
///
/// The `safety` value is assigned as obstacle radius, and `limit` bounds
/// the number of returned rows.
pub fn list_routing_obstacles_in_bbox(
    con: &Connection,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    limit: usize,
    safety: f64,
) -> Result<Vec<RoutingObstacleRow>> {
    let mut stmt = con
        .prepare(
            r#"
            SELECT
                FID,
                Planet,
                X,
                Y
            FROM planets
            WHERE X BETWEEN ?1 AND ?2
              AND Y BETWEEN ?3 AND ?4
            ORDER BY Planet ASC
            LIMIT ?5
            "#,
        )
        .context("Failed to prepare list_routing_obstacles_in_bbox query")?;

    let rows = stmt
        .query_map(params![min_x, max_x, min_y, max_y, limit as i64], |r| {
            Ok(RoutingObstacleRow {
                fid: r.get(0)?,
                planet: r.get(1)?,
                x: r.get(2)?,
                y: r.get(3)?,
                radius: safety,
            })
        })
        .context("Failed to execute list_routing_obstacles_in_bbox query")?;

    let items = rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()?;
    Ok(items)
}
