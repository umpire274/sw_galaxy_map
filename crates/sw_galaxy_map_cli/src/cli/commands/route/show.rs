use anyhow::Result;
use rusqlite::Connection;

use crate::cli::color::Colors;
use crate::ui::Style;

use super::types::RouteShowTuiData;
use super::{RegionBlend, compute_eta_summary};
use sw_galaxy_map_core::db::queries;
use sw_galaxy_map_core::utils::normalize_text;

pub(crate) fn run_show(con: &Connection, route_id: i64) -> Result<()> {
    // ETA model defaults for `route show`
    const SHOW_DEFAULT_HYPERDRIVE_CLASS: f64 = 1.0;
    const SHOW_DEFAULT_DETOUR_COUNT_BASE: f64 = 0.97;
    const SHOW_DEFAULT_SEVERITY_K: f64 = 0.35;
    const SHOW_DEFAULT_REGION_BLEND: RegionBlend = RegionBlend::Avg;

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

        if let Some(eta) = compute_eta_summary(
            con,
            &loaded,
            SHOW_DEFAULT_HYPERDRIVE_CLASS,
            SHOW_DEFAULT_REGION_BLEND,
            SHOW_DEFAULT_DETOUR_COUNT_BASE,
            SHOW_DEFAULT_SEVERITY_K,
        ) {
            println!("{}", eta);
        }
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

pub(crate) fn run_last(con: &Connection, from: &str, to: &str) -> Result<()> {
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

pub(crate) fn resolve_show_for_tui(con: &Connection, route_id: i64) -> Result<RouteShowTuiData> {
    let loaded = queries::load_route(con, route_id)?
        .ok_or_else(|| anyhow::anyhow!("Route not found: id={}", route_id))?;

    Ok(RouteShowTuiData { loaded })
}
