use anyhow::{Result, bail};
use rusqlite::Connection;

use crate::cli::args::{RouteCmd, RouteComputeArgs};
use crate::db::queries;
use crate::normalize::normalize_text;
use crate::routing::collision::Obstacle;
use crate::routing::geometry::Point;
use crate::routing::route_debug::debug_print_route;
use crate::routing::router::{RouteOptions, compute_route};

pub fn run(con: &mut Connection, cmd: &RouteCmd) -> Result<()> {
    match cmd {
        RouteCmd::Compute(args) => run_compute(con, args),
        RouteCmd::Show { route_id } => run_show(con, *route_id),
        RouteCmd::Last { from, to } => run_last(con, from, to),
    }
}

fn run_compute(con: &mut Connection, args: &RouteComputeArgs) -> Result<()> {
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

    // 6) Persist route (v7)
    let route_id = queries::persist_route(con, from_p.fid, to_p.fid, opts, &route)?;

    println!("Route: {} → {}", from_p.planet, to_p.planet);
    println!("Route ID: {}", route_id);
    println!("Waypoints: {}", route.waypoints.len());
    println!("Detours: {}", route.detours.len());
    println!("Length: {:.3} parsec", route.length);

    // Debug details (only in debug builds)
    debug_print_route(&route);

    Ok(())
}

fn run_show(con: &Connection, route_id: i64) -> Result<()> {
    let loaded = queries::load_route(con, route_id)?
        .ok_or_else(|| anyhow::anyhow!("Route not found: id={}", route_id))?;

    println!(
        "Route #{} (FROM {} [{}] TO {} [{}]) status={}",
        loaded.route.id,
        loaded.route.from_planet_name,
        loaded.route.from_planet_fid,
        loaded.route.to_planet_name,
        loaded.route.to_planet_fid,
        loaded.route.status
    );
    if let Some(len) = loaded.route.length {
        println!("Length: {:.3} parsec", len);
    }
    if let Some(it) = loaded.route.iterations {
        println!("Iterations: {}", it);
    }
    if let Some(upd) = loaded.route.updated_at.as_deref() {
        println!("Updated: {}", upd);
    } else {
        println!("Created: {}", loaded.route.created_at);
    }

    let last_seq = loaded.waypoints.len().saturating_sub(1);
    println!("Waypoints: {}", loaded.waypoints.len());
    for w in &loaded.waypoints {
        let label = if w.seq as usize == 0 {
            "Start".to_string()
        } else if w.seq as usize == last_seq {
            "End".to_string()
        } else {
            match (
                w.waypoint_id,
                w.waypoint_name.as_deref(),
                w.waypoint_kind.as_deref(),
            ) {
                (Some(_id), Some(name), Some(kind)) => {
                    format!("{} (kind={})", name, kind)
                }
                (Some(id), Some(name), None) => {
                    format!("{} (id={})", name, id)
                }
                (Some(id), None, _) => {
                    format!("wp_id={}", id)
                }
                (None, _, _) => "intermediate".to_string(),
            }
        };

        println!("  {:>3}: ({:>10.3}, {:>10.3}) {}", w.seq, w.x, w.y, label);
    }

    println!("Detours: {}", loaded.detours.len());
    for d in &loaded.detours {
        println!(
            "  det#{:<3} it={} seg={} obstacle={} [{}] wp=({:.3},{:.3}) score={:.3}",
            d.idx,
            d.iteration,
            d.segment_index,
            d.obstacle_name,
            d.obstacle_id,
            d.wp_x,
            d.wp_y,
            d.score_total
        );
    }

    Ok(())
}

fn run_last(con: &Connection, from: &str, to: &str) -> Result<()> {
    use crate::normalize::normalize_text;

    let from_norm = normalize_text(from);
    let to_norm = normalize_text(to);

    let from_p = queries::find_planet_for_info(con, &from_norm)?
        .ok_or_else(|| anyhow::anyhow!("Planet not found: {}", from))?;
    let to_p = queries::find_planet_for_info(con, &to_norm)?
        .ok_or_else(|| anyhow::anyhow!("Planet not found: {}", to))?;

    let r = queries::get_route_by_from_to(con, from_p.fid, to_p.fid)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No persisted route found for {} → {}",
            from_p.planet,
            to_p.planet
        )
    })?;

    run_show(con, r.id)
}
