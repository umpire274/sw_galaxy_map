use anyhow::{Result, bail};
use owo_colors::OwoColorize;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::args::{RouteCmd, RouteComputeArgs};
use crate::cli::color::Colors;
use crate::cli::export::{
    ExplainClosest, ExplainDetour, ExplainDominantPenalty, ExplainEndpoint, ExplainExport,
    ExplainNote, ExplainObstacle, ExplainRouteMeta, ExplainScore, ExplainWaypoint,
};
use crate::db::queries;
use crate::model::RouteOptionsJson;
use crate::normalize::normalize_text;
use crate::routing::collision::Obstacle;
use crate::routing::geometry::Point;
use crate::routing::route_debug::debug_print_route;
use crate::routing::router::{RouteOptions, compute_route};

use crate::ui::Style;

pub fn run(con: &mut Connection, cmd: &RouteCmd) -> Result<()> {
    match cmd {
        RouteCmd::Compute(args) => run_compute(con, args),
        RouteCmd::Show { route_id } => run_show(con, *route_id),
        RouteCmd::Explain {
            route_id,
            json,
            file,
        } => run_explain(con, *route_id, *json, file.as_deref()),
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
                radius: args.safety, // safety circle model
            });
        }
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

    let style = Style::default();
    let c = Colors::new(&style);

    // ---- Header -------------------------------------------------------------
    let status_txt = loaded.route.status.as_str();
    let status = if status_txt == "ok" {
        c.ok(status_txt)
    } else {
        c.err(status_txt)
    };

    let from_name = c.from_name(&loaded.route.from_planet_name);
    let to_name = c.to_name(&loaded.route.to_planet_name);

    println!(
        "Route #{} (FROM {} [{}] TO {} [{}]) status={}",
        loaded.route.id,
        from_name,
        loaded.route.from_planet_fid,
        to_name,
        loaded.route.to_planet_fid,
        status
    );

    let route_len = loaded.route.length;
    if let Some(len) = route_len {
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

    // ---- Waypoints ----------------------------------------------------------
    let last_seq = loaded.waypoints.len().saturating_sub(1);

    println!("Waypoints: {}", loaded.waypoints.len());
    for w in &loaded.waypoints {
        let is_start = w.seq as usize == 0;
        let is_end = w.seq as usize == last_seq;

        let (label, is_detour) = if is_start {
            ("Start".to_string(), false)
        } else if is_end {
            ("End".to_string(), false)
        } else {
            let det = w.waypoint_kind.as_deref() == Some("computed")
                || w.waypoint_name
                    .as_deref()
                    .is_some_and(|n| n.starts_with("Detour"));

            let lbl = match (
                w.waypoint_id,
                w.waypoint_name.as_deref(),
                w.waypoint_kind.as_deref(),
            ) {
                (Some(_id), Some(name), Some(kind)) => format!("{} (kind={})", name, kind),
                (Some(id), Some(name), None) => format!("{} (id={})", name, id),
                (Some(id), None, _) => format!("wp_id={}", id),
                (None, _, _) => "intermediate".to_string(),
            };

            (lbl, det)
        };

        let colored_label = if is_start {
            c.label_start(label)
        } else if is_end {
            c.label_end(label)
        } else if is_detour {
            c.label_detour(label)
        } else {
            label
        };

        println!(
            "  {:>3}: ({:>10.3}, {:>10.3}) {}",
            w.seq, w.x, w.y, colored_label
        );
    }

    // ---- Detours ------------------------------------------------------------
    // Score coloring thresholds (relative to route length, if available)
    let (good, bad) = match route_len {
        Some(len) => (Some(len * 1.05), Some(len * 1.15)),
        None => (None, None),
    };

    println!("Detours: {}", loaded.detours.len());
    for (i, d) in loaded.detours.iter().enumerate() {
        if style.color {
            let obstacle_plain = format!("{} [{}]", d.obstacle_name, d.obstacle_id);
            let obstacle = c.obstacle(obstacle_plain);

            let wp_plain = format!("({:.3},{:.3})", d.wp_x, d.wp_y);
            let wp = c.waypoint(wp_plain);

            let score_raw = d.score_total;
            let score_txt = format!("{:.3}", score_raw);

            let score_out = c.by_thresholds(score_raw, good, bad, &score_txt);

            println!(
                "  det#{:<2} it={} seg={} obstacle={} wp={} score={}",
                i, d.iteration, d.segment_index, obstacle, wp, score_out
            );
        } else {
            println!(
                "  det#{:<2} it={} seg={} obstacle={} [{}] wp=({:.3},{:.3}) score={:.3}",
                i,
                d.iteration,
                d.segment_index,
                d.obstacle_name,
                d.obstacle_id,
                d.wp_x,
                d.wp_y,
                d.score_total,
            );
        }
    }

    Ok(())
}

fn analyze_detour_drivers(
    d: &crate::model::RouteDetourRow,
    opts: Option<&RouteOptionsJson>,
) -> Vec<String> {
    let mut out = Vec::new();

    // 1) Constraint driver: breach amount (always meaningful)
    let clearance = opts.map(|o| o.clearance).unwrap_or(0.0);
    let required = d.obstacle_radius + clearance;
    let breach = required - d.closest_dist;
    if breach > 0.0 {
        out.push(format!(
            "constraint: safety breach {:.3} (closest {:.3} < required {:.3})",
            breach, d.closest_dist, required
        ));
    } else {
        out.push(format!(
            "constraint: no breach at logging time (margin {:.3})",
            -breach
        ));
    }

    // 2) Limit driver: offset tries saturation (heuristic)
    // We can’t see the number of tries directly, but we *can* infer “offset got large”
    // relative to required clearance and the configured growth/tries.
    if let Some(o) = opts {
        // theoretical max offset scale: required * growth^(tries-1)
        // base offset in router is usually (radius + clearance), which matches `required`.
        let tries = o.max_offset_tries.max(1) as i32;
        let theo_max = required * o.offset_growth.powi((tries - 1).max(0));

        // If we are very close to the theoretical max, we flag “limited by tries/growth”.
        // Threshold 0.90 is conservative; adjust as you like.
        if theo_max.is_finite() && theo_max > 0.0 && d.offset_used >= theo_max * 0.90 {
            out.push(format!(
                "limit: offset near theoretical max (offset_used {:.3} ≈ max {:.3}); likely limited by max_offset_tries={} and offset_growth={:.3}",
                d.offset_used, theo_max, o.max_offset_tries, o.offset_growth
            ));
        }
    }

    // 3) Cost driver: dominant penalty component (real signal, you persist these)
    let mut comps = [
        ("turn_weight", d.score_turn),
        ("back_weight", d.score_back),
        ("proximity_weight", d.score_proximity),
    ];
    comps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (name, val) = comps[0];

    // Only claim it if it is actually non-trivial.
    if val.abs() > 1e-9 {
        out.push(format!(
            "cost: dominant penalty component is {} ({:.3})",
            name, val
        ));
    }

    // 4) Another useful signal: “detour is mostly base length” vs “penalty heavy”
    let penalties = d.score_turn + d.score_back + d.score_proximity;
    if penalties > d.score_base * 0.25 {
        out.push(format!(
            "cost: penalties are significant ({:.3}) vs base ({:.3})",
            penalties, d.score_base
        ));
    } else {
        out.push(format!(
            "cost: route length dominates (base {:.3}, penalties {:.3})",
            d.score_base, penalties
        ));
    }

    out
}

fn run_explain(con: &Connection, route_id: i64, json: bool, file: Option<&Path>) -> Result<()> {
    let loaded = queries::load_route(con, route_id)?
        .ok_or_else(|| anyhow::anyhow!("Route not found: id={}", route_id))?;

    // Parse options_json (best-effort: explain still works even if parsing fails)
    let opts: Option<RouteOptionsJson> = serde_json::from_str(&loaded.route.options_json).ok();
    let clearance = opts.as_ref().map(|o| o.clearance).unwrap_or(0.0);

    if json {
        // Build export payload
        let mut detours_out = Vec::with_capacity(loaded.detours.len());

        for d in &loaded.detours {
            let required = d.obstacle_radius + clearance;
            let violated_by = required - d.closest_dist;

            // dominant penalty
            let mut comps = [
                ("turn".to_string(), d.score_turn),
                ("back".to_string(), d.score_back),
                ("proximity".to_string(), d.score_proximity),
            ];
            comps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let dom = ExplainDominantPenalty {
                component: comps[0].0.clone(),
                value: comps[0].1,
            };

            // drivers (usa la tua funzione già esistente)
            let drivers = analyze_detour_drivers(d, opts.as_ref());

            // tries_* (adatta ai tuoi tipi: spesso `tries_exhausted` è i64 0/1)
            let tries_exhausted = d.tries_exhausted == 1;

            detours_out.push(ExplainDetour {
                idx: d.idx,
                iteration: d.iteration as usize,
                segment_index: d.segment_index as usize,

                obstacle: ExplainObstacle {
                    id: d.obstacle_id,
                    name: d.obstacle_name.clone(),
                    x: d.obstacle_x,
                    y: d.obstacle_y,
                    radius: d.obstacle_radius,
                },
                closest: ExplainClosest {
                    t: d.closest_t,
                    qx: d.closest_qx,
                    qy: d.closest_qy,
                    dist: d.closest_dist,
                    required,
                    violated_by,
                },

                offset_used: d.offset_used,
                waypoint: ExplainWaypoint {
                    x: d.wp_x,
                    y: d.wp_y,
                    computed_waypoint_id: d.waypoint_id,
                },

                score: ExplainScore {
                    base: d.score_base,
                    turn: d.score_turn,
                    back: d.score_back,
                    proximity: d.score_proximity,
                    total: d.score_total,
                },

                tries_used: d.tries_used, // Option<i64>
                tries_exhausted,          // bool

                dominant_penalty: dom,
                decision_drivers: drivers,
            });
        }

        let export = ExplainExport {
                route: ExplainRouteMeta {
                    id: loaded.route.id,
                    from: ExplainEndpoint {
                        fid: loaded.route.from_planet_fid,
                        name: loaded.route.from_planet_name.clone(),
                    },
                    to: ExplainEndpoint {
                        fid: loaded.route.to_planet_fid,
                        name: loaded.route.to_planet_name.clone(),
                    },
                    status: loaded.route.status.clone(),
                    length_parsec: loaded.route.length,
                    iterations: loaded.route.iterations,
                    created_at: loaded.route.created_at.clone(),
                    updated_at: loaded.route.updated_at.clone(),
                },
                options: opts.clone(),
                detours: detours_out,
                note: ExplainNote {
                    text: "The above detour explanation reflects the state at the time of route computation. Subsequent changes to route parameters or obstacle data will not be reflected here.".to_string(),
                    units: "parsec".to_string(),
                },
            };

        // JSON only, no colors, stdout
        let s = serde_json::to_string_pretty(&export)?;

        if let Some(path) = file {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }

            let mut f = fs::File::create(path)?;
            f.write_all(s.as_bytes())?;
            f.write_all(b"\n")?;

            // stdout remains clean for scripting; optional confirmation on stderr
            eprintln!("JSON written to {}", path.display());
        } else {
            println!("{}", s);
        }

        return Ok(());
    }

    let style = Style::default();
    let c = Colors::new(&style);

    let status_txt = loaded.route.status.as_str();
    let status = if status_txt == "ok" {
        c.ok(status_txt)
    } else {
        c.err(status_txt)
    };

    let from_name = c.from_name(&loaded.route.from_planet_name);
    let to_name = c.to_name(&loaded.route.to_planet_name);

    println!(
        "Route #{} — {} [{}] → {} [{}] — status={}",
        loaded.route.id,
        from_name,
        loaded.route.from_planet_fid,
        to_name,
        loaded.route.to_planet_fid,
        status
    );

    if let Some(len) = loaded.route.length {
        println!("Length: {:.3} parsec", len);
    }
    if let Some(it) = loaded.route.iterations {
        println!("Iterations: {}", it);
    }

    if let Some(ref o) = opts {
        println!("Router params:");
        println!("  clearance={:.3}", o.clearance);
        println!(
            "  limits: max_iters={}  max_offset_tries={}  offset_growth={:.3}",
            o.max_iters, o.max_offset_tries, o.offset_growth
        );
        println!(
            "  weights: turn={:.3}  back={:.3}  proximity={:.3}  proximity_margin={:.3}",
            o.turn_weight, o.back_weight, o.proximity_weight, o.proximity_margin
        );
    }

    println!();
    println!("Detours: {}", loaded.detours.len());
    if loaded.detours.is_empty() {
        println!("(no detours)");
        return Ok(());
    }

    for (i, d) in loaded.detours.iter().enumerate() {
        println!("  det#{}:", i);

        // context (neutro)
        println!("    context: it={} seg={}", d.iteration, d.segment_index);

        let exhausted = d.tries_exhausted == 1;
        // tries line: green if not exhausted, red if exhausted
        let tries_line = match d.tries_used {
            Some(tu) => {
                if exhausted {
                    format!("tries_used={} (EXHAUSTED max_offset_tries)", tu)
                } else {
                    format!("tries_used={} (found before exhaustion)", tu)
                }
            }
            None => "tries_used=n/a (telemetry not available for this route)".to_string(),
        };
        println!("    offset search: {}", c.tries(exhausted, tries_line));

        // obstacle: always red in color mode
        let obstacle_plain = format!(
            "{} [{}] center=({:.3},{:.3}) radius={:.3}",
            d.obstacle_name, d.obstacle_id, d.obstacle_x, d.obstacle_y, d.obstacle_radius
        );
        println!("    obstacle: {}", c.obstacle(obstacle_plain));

        // why: highlight "violated by" in red when positive
        let clearance = opts.as_ref().map(|o| o.clearance).unwrap_or(0.0);
        let required = d.obstacle_radius + clearance;
        let violated_by = required - d.closest_dist;

        let why_plain = if violated_by > 0.0 {
            format!(
                "closest_dist={:.3} < required={:.3} (violated by {:.3})  (Q=({:.3},{:.3}), t={:.3})",
                d.closest_dist, required, violated_by, d.closest_qx, d.closest_qy, d.closest_t
            )
        } else {
            format!(
                "closest_dist={:.3} >= required={:.3} (margin {:.3})  (Q=({:.3},{:.3}), t={:.3})",
                d.closest_dist, required, -violated_by, d.closest_qx, d.closest_qy, d.closest_t
            )
        };

        let why_out = if style.color && violated_by > 0.0 {
            // Color only the "violated by X" part red by rebuilding two segments
            // (keeps it simple and avoids substring indexing headaches)
            let head = format!(
                "closest_dist={:.3} < required={:.3} (",
                d.closest_dist, required
            );
            let viol = format!("violated by {:.3}", violated_by);
            let tail = format!(
                ")  (Q=({:.3},{:.3}), t={:.3})",
                d.closest_qx, d.closest_qy, d.closest_t
            );

            format!("{}{}{}", head, c.violated(viol), tail)
        } else if style.color && violated_by <= 0.0 {
            // margin case: green because it's safe (rare for detours, but possible)
            why_plain.green().to_string()
        } else {
            why_plain
        };
        println!("    why: {}", why_out);

        // action: waypoint in yellow, offset in default (or yellow)
        let wp_plain = format!("({:.3},{:.3})", d.wp_x, d.wp_y);
        let wp_out = c.waypoint(wp_plain);

        let action_plain = match d.waypoint_id {
            Some(id) => format!(
                "waypoint={} offset_used={:.3} (computed_waypoint_id={})",
                wp_out, d.offset_used, id
            ),
            None => format!("waypoint={} offset_used={:.3}", wp_out, d.offset_used),
        };
        println!("    action: {}", action_plain);

        // score: color total based on penalty magnitude vs base
        let penalties = d.score_turn + d.score_back + d.score_proximity;
        let ratio = if d.score_base.abs() > 1e-12 {
            penalties / d.score_base
        } else {
            0.0
        };

        let total_out = c.score_total_by_ratio(ratio, format!("{:.3}", d.score_total));

        println!(
            "    score: base={:.3}  turn={:.3}  back={:.3}  proximity={:.3}  total={}",
            d.score_base, d.score_turn, d.score_back, d.score_proximity, total_out
        );

        // dominant penalty: name yellow, value colored by magnitude
        let mut comps = [
            ("turn", d.score_turn),
            ("back", d.score_back),
            ("proximity", d.score_proximity),
        ];
        comps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (dom_name, dom_val) = comps[0];

        println!("    dominant_penalty: {}", c.dom_penalty(dom_val, dom_name));

        // decision drivers: color prefixes
        println!("    decision_drivers:");
        for line in analyze_detour_drivers(d, opts.as_ref()) {
            println!("      - {}", c.driver_line(line));
        }
    }

    if !loaded.detours.is_empty() {
        let sep = "------------------------------------------------------------------------------------------";
        println!("{}", c.dim(sep));

        let note = c.warn("NOTE:");
        println!(
            "{} The above detour explanation reflects the state at the time of route computation.",
            note
        );
        println!(
            "      Subsequent changes to route parameters or obstacle data will not be reflected here."
        );
        println!("      All the distances are explained in parsec units.");
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
