use crate::model::{
    RouteDetourRow, RouteRow, RouteWaypointRow, UnknownPlanet, Waypoint, WaypointPlanetLink,
};
use rusqlite::Row;

/// Maps a `planets_unknown` row into an [`UnknownPlanet`].
pub(super) fn unknown_planet_from_row(r: &Row<'_>) -> rusqlite::Result<UnknownPlanet> {
    Ok(UnknownPlanet {
        id: r.get("id")?,
        fid: r.get::<_, Option<i64>>("fid")?,
        planet: r.get("planet")?,
        planet_norm: r.get("planet_norm")?,
        region: r.get("region")?,
        sector: r.get("sector")?,
        system: r.get("system")?,
        grid: r.get("grid")?,
        x: r.get("x")?,
        y: r.get("y")?,
        arcgis_hash: r.get("arcgis_hash")?,
        deleted: r.get("deleted")?,
        canon: r.get("canon")?,
        legends: r.get("legends")?,
        zm: r.get("zm")?,
        name0: r.get("name0")?,
        name1: r.get("name1")?,
        name2: r.get("name2")?,
        lat: r.get("lat")?,
        long: r.get("long")?,
        reference: r.get("reference")?,
        status: r.get("status")?,
        c_region: r.get("c_region")?,
        c_region_li: r.get("c_region_li")?,
        reason: r.get("reason")?,
        reviewed: r.get("reviewed")?,
        promoted: r.get("promoted")?,
        notes: r.get("notes")?,
    })
}

/// Maps a `waypoints` row into a [`Waypoint`].
pub(super) fn waypoint_from_row(r: &Row<'_>) -> rusqlite::Result<Waypoint> {
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

/// Maps a `waypoint_planets` row into a [`WaypointPlanetLink`].
pub(super) fn link_from_row(r: &Row<'_>) -> rusqlite::Result<WaypointPlanetLink> {
    Ok(WaypointPlanetLink {
        waypoint_id: r.get("waypoint_id")?,
        planet_fid: r.get("planet_fid")?,
        role: r.get("role")?,
        distance: r.get("distance")?,
    })
}

/// Maps a `routes` row into a [`RouteRow`].
pub(super) fn route_from_row(r: &Row<'_>) -> rusqlite::Result<RouteRow> {
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

/// Maps a `route_waypoints` row into a [`RouteWaypointRow`].
pub(super) fn route_waypoint_from_row(r: &Row<'_>) -> rusqlite::Result<RouteWaypointRow> {
    Ok(RouteWaypointRow {
        seq: r.get("seq")?,
        x: r.get("x")?,
        y: r.get("y")?,
        waypoint_id: r.get("waypoint_id")?,
        waypoint_name: r.get("waypoint_name")?,
        waypoint_kind: r.get("waypoint_kind")?,
    })
}

/// Maps a `route_detours` row into a [`RouteDetourRow`].
pub(super) fn route_detour_from_row(r: &Row<'_>) -> rusqlite::Result<RouteDetourRow> {
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

        tries_used: r.get("tries_used")?,
        tries_exhausted: r.get("tries_exhausted")?,
    })
}
