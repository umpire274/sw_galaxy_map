//! Planet lookup helpers for sw_galaxy_map_edit.

use anyhow::{Result, bail};
use rusqlite::Connection;
use sw_galaxy_map_core::db::queries::{
    find_planet_by_alias_norm, find_planet_by_norm, get_planet_by_fid, search_planets,
};
use sw_galaxy_map_core::model::{Planet, PlanetSearchRow};
use sw_galaxy_map_core::utils::normalize_text;

/// Resolves a planet by exact FID.
pub fn resolve_by_fid(con: &Connection, fid: i64) -> Result<Option<Planet>> {
    get_planet_by_fid(con, fid)
}

/// Resolves a planet by exact normalized planet name, then exact alias.
pub fn resolve_by_name_or_alias(con: &Connection, raw: &str) -> Result<Option<Planet>> {
    let normalized = normalize_text(raw);

    if normalized.trim().is_empty() {
        return Ok(None);
    }

    if let Some(planet) = find_planet_by_norm(con, &normalized)? {
        return Ok(Some(planet));
    }

    if let Some(planet) = find_planet_by_alias_norm(con, &normalized)? {
        return Ok(Some(planet));
    }

    Ok(None)
}

/// Searches planets by free-text query.
pub fn search(con: &Connection, query: &str, limit: i64) -> Result<Vec<PlanetSearchRow>> {
    let normalized = normalize_text(query);
    search_planets(con, &normalized, limit)
}

/// Resolves a single planet from a free-text query.
///
/// Strategy:
/// - if numeric, try FID
/// - exact planet name
/// - exact alias
/// - fallback search:
///   - 0 hits => None
///   - 1 hit  => exact selected planet
///   - >1 hits => error asking the caller to refine the query
pub fn resolve_single(con: &Connection, query: &str) -> Result<Option<Planet>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Ok(fid) = trimmed.parse::<i64>() {
        return resolve_by_fid(con, fid);
    }

    if let Some(planet) = resolve_by_name_or_alias(con, trimmed)? {
        return Ok(Some(planet));
    }

    let hits = search(con, trimmed, 10)?;
    match hits.len() {
        0 => Ok(None),
        1 => get_planet_by_fid(con, hits[0].fid),
        _ => {
            bail!(
                "Multiple planets matched '{}'. Use a numeric FID or a more specific name.",
                trimmed
            );
        }
    }
}