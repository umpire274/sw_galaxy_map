use super::near_planets;
use super::row_mappers::unknown_planet_from_row;
use crate::model::{NearHit, UnknownNearHit, UnknownPlanet};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

const UNKNOWN_PLANET_SELECT: &str = r#"
  u.id          AS id,
  u.fid         AS fid,
  u.planet      AS planet,
  u.planet_norm AS planet_norm,
  u.region      AS region,
  u.sector      AS sector,
  u.system      AS system,
  u.grid        AS grid,
  u.x           AS x,
  u.y           AS y,
  u.arcgis_hash AS arcgis_hash,
  u.deleted     AS deleted,
  u.canon       AS canon,
  u.legends     AS legends,
  u.zm          AS zm,
  u.name0       AS name0,
  u.name1       AS name1,
  u.name2       AS name2,
  u.lat         AS lat,
  u.long        AS long,
  u.ref         AS reference,
  u.status      AS status,
  u.cregion     AS c_region,
  u.cregion_li  AS c_region_li,
  u.reason      AS reason,
  u.reviewed    AS reviewed,
  u.promoted    AS promoted,
  u.notes       AS notes
"#;

/// Partial update payload for a row in `planets_unknown`.
#[derive(Debug, Clone, Default)]
pub struct UnknownPlanetUpdate {
    pub planet: Option<String>,
    pub region: Option<Option<String>>,
    pub sector: Option<Option<String>>,
    pub system: Option<Option<String>>,
    pub grid: Option<Option<String>>,
    pub canon: Option<Option<i64>>,
    pub legends: Option<Option<i64>>,
    pub c_region: Option<Option<String>>,
    pub c_region_li: Option<Option<String>>,
    pub reviewed: Option<i64>,
    pub notes: Option<Option<String>>,
}

/// Returns unknown planets near the given coordinates using SQL squared distance.
pub fn near_unknown_planets(
    con: &Connection,
    x: f64,
    y: f64,
    range: f64,
    limit: i64,
) -> rusqlite::Result<Vec<UnknownNearHit>> {
    let sql = r#"
        SELECT
            u.id,
            u.fid,
            u.planet,
            u.x,
            u.y,
            u.reason,
            u.reviewed,
            u.promoted,
            (((u.x - ?1) * (u.x - ?1)) +
             ((u.y - ?2) * (u.y - ?2))) AS distance_sq
        FROM planets_unknown u
        WHERE u.x IS NOT NULL
          AND u.y IS NOT NULL
          AND (((u.x - ?1) * (u.x - ?1)) +
               ((u.y - ?2) * (u.y - ?2))) <= (?3 * ?3)
        ORDER BY distance_sq ASC, u.planet ASC
        LIMIT ?4
    "#;

    let mut stmt = con.prepare(sql)?;
    let rows = stmt.query_map((x, y, range, limit), |r| {
        let distance_sq: f64 = r.get(8)?;

        Ok(UnknownNearHit {
            id: r.get(0)?,
            fid: r.get(1)?,
            planet: r.get(2)?,
            x: r.get(3)?,
            y: r.get(4)?,
            reason: r.get(5)?,
            reviewed: r.get(6)?,
            promoted: r.get(7)?,
            distance: distance_sq.sqrt(),
        })
    })?;

    rows.collect()
}

/// Returns a single unknown planet by internal id.
pub fn get_unknown_planet_by_id(con: &Connection, id: i64) -> Result<Option<UnknownPlanet>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM planets_unknown u
        WHERE u.id = ?1
        LIMIT 1
        "#,
        select = UNKNOWN_PLANET_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let planet = stmt
        .query_row([id], unknown_planet_from_row)
        .optional()
        .context("Failed to query planets_unknown by id")?;
    Ok(planet)
}

/// Returns the first unknown planet matching the given FID.
pub fn get_unknown_planet_by_fid(con: &Connection, fid: i64) -> Result<Option<UnknownPlanet>> {
    let sql = format!(
        r#"
        SELECT
          {select}
        FROM planets_unknown u
        WHERE u.fid = ?1
        ORDER BY u.id
        LIMIT 1
        "#,
        select = UNKNOWN_PLANET_SELECT
    );

    let mut stmt = con.prepare(&sql)?;
    let planet = stmt
        .query_row([fid], unknown_planet_from_row)
        .optional()
        .context("Failed to query planets_unknown by fid")?;
    Ok(planet)
}

/// Returns all unknown planets ordered by internal id.
pub fn list_unknown_planets(conn: &Connection) -> rusqlite::Result<Vec<UnknownPlanet>> {
    let sql = format!(
        r#"
        SELECT
          {}
        FROM planets_unknown u
        ORDER BY u.id
        "#,
        UNKNOWN_PLANET_SELECT
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], unknown_planet_from_row)?;

    rows.collect()
}

/// Returns the total number of unknown planets.
pub fn count_unknown_planets(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM planets_unknown u", [], |row| {
        row.get::<_, i64>(0)
    })
}

/// Returns a paginated list of unknown planets.
pub fn list_unknown_planets_paginated(
    conn: &Connection,
    page: usize,
    page_size: usize,
) -> rusqlite::Result<Vec<UnknownPlanet>> {
    let safe_page = page.max(1);
    let safe_page_size = page_size.max(1);

    let offset = ((safe_page - 1) * safe_page_size) as i64;
    let limit = safe_page_size as i64;

    let sql = format!(
        r#"
        SELECT
          {}
        FROM planets_unknown u
        ORDER BY u.id
        LIMIT ?1 OFFSET ?2
        "#,
        UNKNOWN_PLANET_SELECT
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([limit, offset], unknown_planet_from_row)?;

    rows.collect()
}

/// Returns known planets near the given unknown planet id.
pub fn near_planets_for_unknown_id(
    con: &Connection,
    unknown_id: i64,
    radius: f64,
    limit: i64,
) -> Result<(UnknownPlanet, Vec<NearHit>)> {
    let unknown = get_unknown_planet_by_id(con, unknown_id)?
        .ok_or_else(|| anyhow::anyhow!("No unknown planet found for id {}", unknown_id))?;

    let origin_x = unknown.x.ok_or(rusqlite::Error::InvalidQuery)?;
    let origin_y = unknown.y.ok_or(rusqlite::Error::InvalidQuery)?;

    let rows = near_planets(con, origin_x, origin_y, radius, limit)?;

    Ok((unknown, rows))
}

/// Updates a row in `planets_unknown` and returns the refreshed record.
pub fn update_unknown_planet(
    con: &Connection,
    id: i64,
    update: &UnknownPlanetUpdate,
) -> Result<UnknownPlanet> {
    let Some(current) = get_unknown_planet_by_id(con, id)? else {
        anyhow::bail!("No unknown planet found for id {}", id);
    };

    let planet = update
        .planet
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| current.planet.clone());
    let planet_norm = planet.to_lowercase();

    let region = update.region.clone().unwrap_or(current.region.clone());
    let sector = update.sector.clone().unwrap_or(current.sector.clone());
    let system = update.system.clone().unwrap_or(current.system.clone());
    let grid = update.grid.clone().unwrap_or(current.grid.clone());
    let canon = update.canon.unwrap_or(current.canon);
    let legends = update.legends.unwrap_or(current.legends);
    let c_region = update.c_region.clone().unwrap_or(current.c_region.clone());
    let c_region_li = update
        .c_region_li
        .clone()
        .unwrap_or(current.c_region_li.clone());
    let reviewed = update.reviewed.unwrap_or(current.reviewed);
    let notes = update.notes.clone().unwrap_or(current.notes.clone());

    con.execute(
        r#"
        UPDATE planets_unknown
        SET planet = ?2,
            planet_norm = ?3,
            region = ?4,
            sector = ?5,
            system = ?6,
            grid = ?7,
            canon = ?8,
            legends = ?9,
            cregion = ?10,
            cregion_li = ?11,
            reviewed = ?12,
            notes = ?13
        WHERE id = ?1
        "#,
        params![
            id,
            planet,
            planet_norm,
            region,
            sector,
            system,
            grid,
            canon,
            legends,
            c_region,
            c_region_li,
            reviewed,
            notes,
        ],
    )
    .with_context(|| format!("Failed to update planets_unknown record id={id}"))?;

    get_unknown_planet_by_id(con, id)?
        .ok_or_else(|| anyhow::anyhow!("Unknown planet disappeared after update: id={id}"))
}
