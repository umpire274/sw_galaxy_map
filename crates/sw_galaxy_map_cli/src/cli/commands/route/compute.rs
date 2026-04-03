use anyhow::{Result, bail};
use rusqlite::Connection;

use super::types::RouteComputeTuiData;
use crate::cli::args::RouteComputeArgs;
use sw_galaxy_map_core::db::queries;
use sw_galaxy_map_core::model::Planet;
use sw_galaxy_map_core::routing::collision::Obstacle;
use sw_galaxy_map_core::routing::geometry::Point;
use sw_galaxy_map_core::routing::route_debug::debug_print_route;
use sw_galaxy_map_core::routing::router::{Route, RouteOptions, compute_route};
use sw_galaxy_map_core::utils::normalize_text;

struct ComputedLeg {
    from_p: Planet,
    to_p: Planet,
    route: Route,
    route_id: i64,
}

fn compute_leg(
    con: &mut Connection,
    args: &RouteComputeArgs,
    from: &str,
    to: &str,
) -> Result<ComputedLeg> {
    // 1) Resolve FROM/TO planets (name or alias)
    let from_norm = normalize_text(from);
    let to_norm = normalize_text(to);

    let from_p = queries::find_planet_for_info(con, &from_norm)?
        .ok_or_else(|| anyhow::anyhow!("Planet not found: {}", from))?;

    let to_p = queries::find_planet_for_info(con, &to_norm)?
        .ok_or_else(|| anyhow::anyhow!("Planet not found: {}", to))?;

    let start = Point::new(from_p.x, from_p.y);
    let end = Point::new(to_p.x, to_p.y);

    if start == end {
        bail!(
            "Start and destination are the same point (fid={})",
            from_p.fid
        );
    }

    // 2) Fetch candidate obstacles in a bbox around the segment (cheap prefilter)
    let min_x = start.x.min(end.x) - args.bbox_margin;
    let max_x = start.x.max(end.x) + args.bbox_margin;
    let min_y = start.y.min(end.y) - args.bbox_margin;
    let max_y = start.y.max(end.y) + args.bbox_margin;

    // Prefer DB-annotated obstacles (waypoint_planets.role), but fall back to the legacy
    // behavior if none are configured yet.
    let mut obstacles: Vec<Obstacle> = Vec::new();

    let raw_db = queries::list_routing_obstacles_in_bbox(
        con,
        min_x,
        max_x,
        min_y,
        max_y,
        args.max_obstacles,
        args.safety,
    )?;

    if !raw_db.is_empty() {
        obstacles.reserve(raw_db.len());
        for ob in raw_db {
            if ob.fid == from_p.fid || ob.fid == to_p.fid {
                continue;
            }
            obstacles.push(Obstacle {
                id: ob.fid,
                name: ob.planet.clone(),
                center: Point::new(ob.x, ob.y),
                radius: ob.radius,
            });
        }
    } else {
        let raw =
            queries::list_planets_in_bbox(con, min_x, max_x, min_y, max_y, args.max_obstacles)?;
        obstacles.reserve(raw.len());
        for (fid, name, x, y) in raw {
            if fid == from_p.fid || fid == to_p.fid {
                continue;
            }
            obstacles.push(Obstacle {
                id: fid,
                name: name.clone(),
                center: Point::new(x, y),
                radius: args.safety,
            });
        }
    }

    // 3) Build routing options
    let opts = RouteOptions {
        clearance: args.clearance,
        max_iters: args.max_iters,
        max_offset_tries: args.max_offset_tries,
        offset_growth: args.offset_growth,
        turn_weight: args.turn_weight,
        back_weight: args.back_weight,
        proximity_weight: args.proximity_weight,
        proximity_margin: args.proximity_margin,
    };

    // 4) Compute route
    let route = compute_route(start, end, &obstacles, opts)?;

    // 5) Persist route
    let route_id = queries::persist_route(con, from_p.fid, to_p.fid, opts, &route)?;

    Ok(ComputedLeg {
        from_p,
        to_p,
        route,
        route_id,
    })
}

pub(crate) fn run_compute(con: &mut Connection, args: &RouteComputeArgs) -> Result<()> {
    let mut total_length = 0.0;
    let mut total_waypoints = 0usize;
    let mut total_detours = 0usize;
    let mut route_ids = Vec::new();

    for (idx, leg) in args.planets.windows(2).enumerate() {
        let from = &leg[0];
        let to = &leg[1];
        let computed = compute_leg(con, args, from, to)?;

        if args.planets.len() > 2 {
            println!(
                "Leg {}/{}: {} → {}",
                idx + 1,
                args.planets.len() - 1,
                computed.from_p.planet,
                computed.to_p.planet
            );
        } else {
            println!(
                "Route: {} → {}",
                computed.from_p.planet, computed.to_p.planet
            );
        }

        println!("Route ID: {}", computed.route_id);
        println!("Waypoints: {}", computed.route.waypoints.len());
        println!("Detours: {}", computed.route.detours.len());
        println!("Length: {:.3} parsec", computed.route.length);
        if args.planets.len() > 2 && idx + 1 < args.planets.len() - 1 {
            println!();
        }

        total_length += computed.route.length;
        total_waypoints += computed.route.waypoints.len();
        total_detours += computed.route.detours.len();
        route_ids.push(computed.route_id);

        // Debug details (only in debug builds)
        debug_print_route(&computed.route);
    }

    if args.planets.len() > 2 {
        let route_ids_txt = route_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "Trip summary: {} legs, {} total waypoints, {} total detours, {:.3} total parsec",
            args.planets.len() - 1,
            total_waypoints,
            total_detours,
            total_length
        );
        println!("Route IDs: {}", route_ids_txt);
    }

    Ok(())
}

pub(crate) fn resolve_compute_for_tui(
    con: &mut Connection,
    args: &RouteComputeArgs,
) -> Result<RouteComputeTuiData> {
    if args.planets.len() != 2 {
        bail!("TUI currently supports only single-leg route compute (exactly 2 planets).");
    }

    let from = &args.planets[0];
    let to = &args.planets[1];
    let computed = compute_leg(con, args, from, to)?;

    Ok(RouteComputeTuiData {
        route_id: computed.route_id,
    })
}
