use anyhow::{Result, bail};
use rusqlite::Connection;

use crate::cli::args::RouteArgs;
use crate::db::queries;
use crate::normalize::normalize_text;
use crate::routing::collision::Obstacle;
use crate::routing::geometry::Point;
use crate::routing::route_debug::debug_print_route;
use crate::routing::router::{RouteOptions, compute_route};

pub fn run(con: &Connection, args: &RouteArgs) -> Result<()> {
    // 1) Resolve FROM/TO planets (name or alias)
    let from_norm = normalize_text(&args.from);
    let to_norm = normalize_text(&args.to);

    let from_p = queries::find_planet_for_info(con, &from_norm)?
        .ok_or_else(|| anyhow::anyhow!("Planet not found: {}", args.from))?;

    let to_p = queries::find_planet_for_info(con, &to_norm)?
        .ok_or_else(|| anyhow::anyhow!("Planet not found: {}", args.to))?;

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

    let raw = queries::list_planets_in_bbox(con, min_x, max_x, min_y, max_y, args.max_obstacles)?;

    // 3) Convert to obstacles, excluding endpoints (optional but recommended)
    let mut obstacles: Vec<Obstacle> = Vec::with_capacity(raw.len());

    for (fid, _name, x, y) in raw {
        if fid == from_p.fid || fid == to_p.fid {
            continue;
        }
        obstacles.push(Obstacle {
            id: fid,
            center: Point::new(x, y),
            radius: args.safety, // safety circle model
        });
    }

    // 4) Build routing options
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

    // 5) Compute route
    let route = compute_route(start, end, &obstacles, opts)?;

    println!("Route: {} → {}", from_p.planet, to_p.planet);
    println!("Waypoints: {}", route.waypoints.len());
    println!("Detours: {}", route.detours.len());
    println!("Length: {:.3} parsec", route.length);

    // Debug details (only in debug builds)
    debug_print_route(&route);

    Ok(())
}
