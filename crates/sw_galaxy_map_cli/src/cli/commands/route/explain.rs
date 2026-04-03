use anyhow::Result;
use crossterm::style::Stylize;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::cli::args::RouteExplainArgs;
use crate::cli::color::Colors;
use crate::cli::export::{
    ExplainClosest, ExplainDetour, ExplainDominantPenalty, ExplainEndpoint, ExplainExport,
    ExplainNote, ExplainObstacle, ExplainRouteMeta, ExplainScore, ExplainWaypoint,
};
use crate::ui::Style;

use sw_galaxy_map_core::db::queries;
use sw_galaxy_map_core::model::{RouteLoaded, RouteOptionsJson};
use sw_galaxy_map_core::routing::geometry::Point;
use sw_galaxy_map_core::routing::geometry::{dist as geom_dist, polyline_length_waypoints_parsec};
use sw_galaxy_map_core::routing::hyperspace::{
    DetourPenaltyParams, GalacticRegion, detour_penalty_multiplier, estimate_travel_time_hours,
    extract_galactic_region,
};
use sw_galaxy_map_core::routing::sublight::estimate_sublight_time_hours;

const DEFAULT_DETOUR_COUNT_BASE: f64 = 0.97;
const DEFAULT_SEVERITY_K: f64 = 0.35;

#[derive(Debug, Clone, Copy)]
pub(crate) enum RegionBlend {
    Avg,
    Conservative,
    Weighted(f64),
}

pub(crate) fn parse_region_blend(s: &str) -> RegionBlend {
    match s {
        "avg" => RegionBlend::Avg,
        "conservative" => RegionBlend::Conservative,
        _ => {
            if let Ok(w) = s.parse::<f64>() {
                RegionBlend::Weighted(w.clamp(0.0, 1.0))
            } else {
                RegionBlend::Avg
            }
        }
    }
}

pub(crate) fn format_duration_compact(hours: f64) -> String {
    if !hours.is_finite() || hours < 0.0 {
        return "-".to_string();
    }
    if hours < 24.0 {
        return format!("{:.1} h", hours);
    }
    let days = hours / 24.0;
    if days < 365.0 {
        return format!("{:.1} d", days);
    }
    let years = days / 365.25;
    if years < 1000.0 {
        return format!("{:.1} y", years);
    }
    let kyr = years / 1000.0;
    if kyr < 1000.0 {
        return format!("{:.1} kyr", kyr);
    }
    let myr = kyr / 1000.0;

    format!("{:.1} Myr", myr)
}

pub(crate) fn compute_eta_summary(
    con: &Connection,
    loaded: &RouteLoaded,
    hyperdrive_class: f64,
    blend: RegionBlend,
    detour_count_base: f64,
    severity_k: f64,
) -> Option<String> {
    if loaded.waypoints.len() < 2 {
        return None;
    }
    if hyperdrive_class <= 0.0 || detour_count_base <= 0.0 || severity_k < 0.0 {
        return None;
    }

    let route_len: f64 = polyline_length_waypoints_parsec(&loaded.waypoints, |w| (w.x, w.y));

    let a = loaded.waypoints.first().unwrap();
    let b = loaded.waypoints.last().unwrap();
    let direct = geom_dist(Point::new(a.x, a.y), Point::new(b.x, b.y));

    if direct <= 0.0 || route_len <= 0.0 {
        return None;
    }

    let detour_params = DetourPenaltyParams::default();
    let detour_mult_geom = detour_penalty_multiplier(direct, route_len, detour_params);

    let detour_count = loaded.detours.len() as i32;
    let detour_mult_count = detour_count_base.powi(detour_count);

    let mut severity_sum: f64 = loaded
        .detours
        .iter()
        .map(|d| {
            let req = d.offset_used.max(1e-9);
            ((req - d.closest_dist) / req).clamp(0.0, 1.0)
        })
        .sum();

    if severity_sum.abs() < 1e-12 {
        severity_sum = 0.0;
    }

    let detour_mult_severity = 1.0 / (1.0 + severity_k * severity_sum);

    let detour_mult = (detour_mult_geom * detour_mult_count * detour_mult_severity)
        .clamp(detour_params.floor, 1.0);

    let from_p = queries::get_planet_by_fid(con, loaded.route.from_planet_fid).ok()??;
    let to_p = queries::get_planet_by_fid(con, loaded.route.to_planet_fid).ok()??;

    let from_region = extract_galactic_region(&from_p);
    let to_region = extract_galactic_region(&to_p);

    let rf = from_region.unwrap_or(GalacticRegion::OuterRim);
    let rt = to_region.unwrap_or(GalacticRegion::OuterRim);

    let cf_from = rf.base_compression_factor();
    let cf_to = rt.base_compression_factor();

    let cf_base_eff = match blend {
        RegionBlend::Avg => (cf_from + cf_to) / 2.0,
        RegionBlend::Conservative => cf_from * 0.4 + cf_to * 0.6,
        RegionBlend::Weighted(w) => {
            let w = w.clamp(0.0, 1.0);
            cf_from * w + cf_to * (1.0 - w)
        }
    };

    let compression = (cf_base_eff * detour_mult).max(5.0);
    let eta_hours = estimate_travel_time_hours(route_len, compression, hyperdrive_class);

    Some(format!(
        "ETA: {:.1} h (~{:.1} d) [class={:.1}, from={:?}, to={:?}, blend={:?}, cf={:.2}, detours={}, mult={:.3}]",
        eta_hours,
        eta_hours / 24.0,
        hyperdrive_class,
        rf,
        rt,
        blend,
        compression,
        loaded.detours.len(),
        detour_mult,
    ))
}

pub(crate) fn analyze_detour_drivers(
    d: &sw_galaxy_map_core::model::RouteDetourRow,
    opts: Option<&RouteOptionsJson>,
) -> Vec<String> {
    let mut out = Vec::new();

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

    if let Some(o) = opts {
        let tries = o.max_offset_tries.max(1) as i32;
        let theo_max = required * o.offset_growth.powi((tries - 1).max(0));

        if theo_max.is_finite() && theo_max > 0.0 && d.offset_used >= theo_max * 0.90 {
            out.push(format!(
                "limit: offset near theoretical max (offset_used {:.3} ≈ max {:.3}); likely limited by max_offset_tries={} and offset_growth={:.3}",
                d.offset_used, theo_max, o.max_offset_tries, o.offset_growth
            ));
        }
    }

    let mut comps = [
        ("turn_weight", d.score_turn),
        ("back_weight", d.score_back),
        ("proximity_weight", d.score_proximity),
    ];
    comps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let (name, val) = comps[0];

    if val.abs() > 1e-9 {
        out.push(format!(
            "cost: dominant penalty component is {} ({:.3})",
            name, val
        ));
    }

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

pub(crate) fn run_explain(con: &Connection, args: &RouteExplainArgs) -> Result<()> {
    let loaded = queries::load_route(con, args.route_id)?
        .ok_or_else(|| anyhow::anyhow!("Route not found: id={}", args.route_id))?;

    let opts: Option<RouteOptionsJson> = serde_json::from_str(&loaded.route.options_json).ok();
    let clearance = opts.as_ref().map(|o| o.clearance).unwrap_or(0.0);

    if args.json {
        let mut detours_out = Vec::with_capacity(loaded.detours.len());

        for d in &loaded.detours {
            let required = d.obstacle_radius + clearance;
            let violated_by = required - d.closest_dist;

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

            let drivers = analyze_detour_drivers(d, opts.as_ref());
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

                tries_used: d.tries_used,
                tries_exhausted,

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

        let s = serde_json::to_string_pretty(&export)?;

        if let Some(path) = &args.file {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(parent)?;
            }

            let mut f = fs::File::create(path)?;
            f.write_all(s.as_bytes())?;
            f.write_all(b"\n")?;
            eprintln!("JSON written to {}", path.display());
        } else {
            println!("{}", s);
        }

        return Ok(());
    }

    if let Some(csv_path) = &args.csv {
        export_polyline_csv(&loaded, csv_path)?;
        eprintln!("CSV polyline written to {}", csv_path.display());
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

        print_eta_breakdown(
            con,
            &loaded,
            args.hyperdrive_class,
            parse_region_blend(&args.region_blend),
            DEFAULT_DETOUR_COUNT_BASE,
            DEFAULT_SEVERITY_K,
            &c,
        );

        if let Some(kmps) = args.sublight_kmps
            && kmps > 0.0
        {
            let route_len_geom: f64 =
                polyline_length_waypoints_parsec(&loaded.waypoints, |w| (w.x, w.y));

            if route_len_geom > 0.0 {
                let h = estimate_sublight_time_hours(route_len_geom, kmps);
                println!(
                    "ETA (sublight, {:.0} km/s): {}",
                    kmps,
                    format_duration_compact(h)
                );
            }
        }
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
    println!("Waypoints: {} (segment distances)", loaded.waypoints.len());
    print_waypoint_segments(&loaded.waypoints, &c);

    if !loaded.detours.is_empty() {
        println!();
        print_detour_summary(&loaded, &c);
    }

    println!();
    println!("Detours: {}", loaded.detours.len());
    if loaded.detours.is_empty() {
        println!("(no detours)");
        return Ok(());
    }

    for (i, d) in loaded.detours.iter().enumerate() {
        println!("  det#{}:", i);

        println!("    context: it={} seg={}", d.iteration, d.segment_index);

        let exhausted = d.tries_exhausted == 1;
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

        let obstacle_plain = format!(
            "{} [{}] center=({:.3},{:.3}) radius={:.3}",
            d.obstacle_name, d.obstacle_id, d.obstacle_x, d.obstacle_y, d.obstacle_radius
        );
        println!("    obstacle: {}", c.obstacle(obstacle_plain));

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
            why_plain.green().to_string()
        } else {
            why_plain
        };
        println!("    why: {}", why_out);

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

        let mut comps = [
            ("turn", d.score_turn),
            ("back", d.score_back),
            ("proximity", d.score_proximity),
        ];
        comps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let (dom_name, dom_val) = comps[0];

        println!("    dominant_penalty: {}", c.dom_penalty(dom_val, dom_name));

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

pub(crate) fn print_eta_breakdown(
    con: &Connection,
    loaded: &RouteLoaded,
    hyperdrive_class: f64,
    blend: RegionBlend,
    detour_count_base: f64,
    severity_k: f64,
    c: &Colors,
) {
    if loaded.waypoints.len() < 2 || hyperdrive_class <= 0.0 {
        return;
    }

    let route_len: f64 = polyline_length_waypoints_parsec(&loaded.waypoints, |w| (w.x, w.y));
    let a = loaded.waypoints.first().unwrap();
    let b = loaded.waypoints.last().unwrap();
    let direct = geom_dist(Point::new(a.x, a.y), Point::new(b.x, b.y));

    if direct <= 0.0 || route_len <= 0.0 {
        return;
    }

    let from_p = match queries::get_planet_by_fid(con, loaded.route.from_planet_fid)
        .ok()
        .flatten()
    {
        Some(p) => p,
        None => return,
    };
    let to_p = match queries::get_planet_by_fid(con, loaded.route.to_planet_fid)
        .ok()
        .flatten()
    {
        Some(p) => p,
        None => return,
    };

    let from_region = extract_galactic_region(&from_p).unwrap_or(GalacticRegion::OuterRim);
    let to_region = extract_galactic_region(&to_p).unwrap_or(GalacticRegion::OuterRim);

    let cf_from = from_region.base_compression_factor();
    let cf_to = to_region.base_compression_factor();

    let cf_base = match blend {
        RegionBlend::Avg => (cf_from + cf_to) / 2.0,
        RegionBlend::Conservative => cf_from * 0.4 + cf_to * 0.6,
        RegionBlend::Weighted(w) => {
            let w = w.clamp(0.0, 1.0);
            cf_from * w + cf_to * (1.0 - w)
        }
    };

    let detour_params = DetourPenaltyParams::default();
    let mult_geom = detour_penalty_multiplier(direct, route_len, detour_params);

    let detour_count = loaded.detours.len() as i32;
    let mult_count = detour_count_base.powi(detour_count);

    let severity_sum: f64 = loaded
        .detours
        .iter()
        .map(|d| {
            let req = d.offset_used.max(1e-9);
            ((req - d.closest_dist) / req).clamp(0.0, 1.0)
        })
        .sum();

    let mult_severity = 1.0 / (1.0 + severity_k * severity_sum);
    let mult_total = (mult_geom * mult_count * mult_severity).clamp(detour_params.floor, 1.0);

    let compression = (cf_base * mult_total).max(5.0);
    let eta_hours = estimate_travel_time_hours(route_len, compression, hyperdrive_class);

    let overhead_pct = if direct > 0.0 {
        ((route_len / direct) - 1.0) * 100.0
    } else {
        0.0
    };

    println!();
    println!("ETA Breakdown:");
    println!("  Route length     : {:.3} parsec", route_len);
    println!("  Direct distance  : {:.3} parsec", direct);
    println!("  Route overhead   : +{:.1}%", overhead_pct);
    println!("  Hyperdrive class : {:.1}", hyperdrive_class);
    println!();
    println!("  Regions:");
    println!("    Origin         : {:?} (CF={:.1})", from_region, cf_from);
    println!("    Destination    : {:?} (CF={:.1})", to_region, cf_to);
    println!("    Blend policy   : {:?}", blend);
    println!("    Base CF        : {:.2}", cf_base);
    println!();
    println!("  Detour multipliers:");
    println!(
        "    Geometric      : {:.4} (route/direct ratio penalty)",
        mult_geom
    );
    println!(
        "    Count ({} det)  : {:.4} (base={:.2}^{})",
        loaded.detours.len(),
        mult_count,
        detour_count_base,
        detour_count
    );
    println!(
        "    Severity       : {:.4} (sum={:.3}, k={:.2})",
        mult_severity, severity_sum, severity_k
    );
    println!("    Combined       : {:.4}", mult_total);
    println!();
    println!("  Effective CF     : {:.2}", compression);
    println!(
        "  {} ETA: {} ({:.1} h, ~{:.1} d)",
        c.ok("→"),
        format_duration_compact(eta_hours),
        eta_hours,
        eta_hours / 24.0
    );
}

pub(crate) fn print_waypoint_segments(
    waypoints: &[sw_galaxy_map_core::model::RouteWaypointRow],
    c: &Colors,
) {
    if waypoints.is_empty() {
        return;
    }

    let last_seq = waypoints.len().saturating_sub(1);
    let mut cumulative = 0.0_f64;

    println!(
        "  {:>3}  {:>10}  {:>10}  {:>10}  {:>10}  Label",
        "Seq", "X", "Y", "Segment", "Cumulative"
    );
    println!(
        "  {:->3}  {:->10}  {:->10}  {:->10}  {:->10}  {:->20}",
        "", "", "", "", "", ""
    );

    for (i, w) in waypoints.iter().enumerate() {
        let segment_dist = if i == 0 {
            0.0
        } else {
            let prev = &waypoints[i - 1];
            geom_dist(Point::new(prev.x, prev.y), Point::new(w.x, w.y))
        };

        cumulative += segment_dist;

        let is_start = i == 0;
        let is_end = i == last_seq;

        let label = if is_start {
            "Start".to_string()
        } else if is_end {
            "End".to_string()
        } else {
            match (w.waypoint_name.as_deref(), w.waypoint_kind.as_deref()) {
                (Some(name), Some(kind)) => format!("{} ({})", name, kind),
                (Some(name), None) => name.to_string(),
                _ => "waypoint".to_string(),
            }
        };

        let colored_label = if is_start {
            c.label_start(label)
        } else if is_end {
            c.label_end(label)
        } else {
            c.label_detour(label)
        };

        let seg_str = if i == 0 {
            "-".to_string()
        } else {
            format!("{:.3}", segment_dist)
        };

        println!(
            "  {:>3}  {:>10.3}  {:>10.3}  {:>10}  {:>10.3}  {}",
            w.seq, w.x, w.y, seg_str, cumulative, colored_label
        );
    }
}

pub(crate) fn print_detour_summary(loaded: &RouteLoaded, c: &Colors) {
    let detours = &loaded.detours;
    if detours.is_empty() {
        return;
    }

    let route_len: f64 = polyline_length_waypoints_parsec(&loaded.waypoints, |w| (w.x, w.y));
    let a = loaded.waypoints.first().unwrap();
    let b = loaded.waypoints.last().unwrap();
    let direct = geom_dist(Point::new(a.x, a.y), Point::new(b.x, b.y));

    let overhead_parsec = route_len - direct;
    let overhead_pct = if direct > 0.0 {
        (overhead_parsec / direct) * 100.0
    } else {
        0.0
    };

    let avg_score: f64 = detours.iter().map(|d| d.score_total).sum::<f64>() / detours.len() as f64;

    let worst = detours.iter().max_by(|a, b| {
        a.score_total
            .partial_cmp(&b.score_total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let avg_severity: f64 = detours
        .iter()
        .map(|d| {
            let req = d.offset_used.max(1e-9);
            ((req - d.closest_dist) / req).clamp(0.0, 1.0)
        })
        .sum::<f64>()
        / detours.len() as f64;

    let exhausted_count = detours.iter().filter(|d| d.tries_exhausted == 1).count();

    println!("Detour Summary:");
    println!("  Total detours    : {}", detours.len());
    println!(
        "  Route overhead   : +{:.3} parsec (+{:.1}% vs direct)",
        overhead_parsec, overhead_pct
    );
    println!("  Avg score        : {:.3}", avg_score);
    println!("  Avg severity     : {:.3}", avg_severity);

    if let Some(w) = worst {
        println!(
            "  Worst detour     : det#{} obstacle={} score={:.3}",
            w.idx,
            c.obstacle(w.obstacle_name.clone()),
            w.score_total
        );
    }

    if exhausted_count > 0 {
        println!(
            "  Exhausted tries  : {}/{} ({})",
            c.err(exhausted_count.to_string()),
            detours.len(),
            c.warn("may indicate suboptimal detours")
        );
    } else {
        println!(
            "  Exhausted tries  : 0/{} ({})",
            detours.len(),
            c.ok("all resolved cleanly")
        );
    }
}

pub(crate) fn export_polyline_csv(loaded: &RouteLoaded, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut f = fs::File::create(path)?;
    writeln!(f, "seq,x,y,segment_parsec,cumulative_parsec,label")?;

    let last_seq = loaded.waypoints.len().saturating_sub(1);
    let mut cumulative = 0.0_f64;

    for (i, w) in loaded.waypoints.iter().enumerate() {
        let segment_dist = if i == 0 {
            0.0
        } else {
            let prev = &loaded.waypoints[i - 1];
            geom_dist(Point::new(prev.x, prev.y), Point::new(w.x, w.y))
        };

        cumulative += segment_dist;

        let is_start = i == 0;
        let is_end = i == last_seq;

        let label = if is_start {
            "Start".to_string()
        } else if is_end {
            "End".to_string()
        } else {
            match (w.waypoint_name.as_deref(), w.waypoint_kind.as_deref()) {
                (Some(name), Some(kind)) => format!("{} ({})", name, kind),
                (Some(name), None) => name.to_string(),
                _ => "waypoint".to_string(),
            }
        };

        let label_csv = if label.contains(',') || label.contains('"') {
            format!("\"{}\"", label.replace('"', "\"\""))
        } else {
            label
        };

        writeln!(
            f,
            "{},{:.6},{:.6},{:.6},{:.6},{}",
            w.seq, w.x, w.y, segment_dist, cumulative, label_csv
        )?;
    }

    Ok(())
}
