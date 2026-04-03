use anyhow::Result;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::commands;
use crate::tui::bridge::route_eta;
use crate::tui::{NavigationPanelKind, TuiCommandOutput, region_name, tui_default_output};
use sw_galaxy_map_core::model::{PlanetSearchRow, RouteLoaded};

const PANEL_LABEL_WIDTH: usize = 9;

/// Formats a left-aligned key/value row for TUI side panels.
fn panel_kv(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label:<PANEL_LABEL_WIDTH$}: {value}")
}

/// Returns `-` if the value is `None` or empty/whitespace.
fn tui_cell(opt: &Option<String>) -> &str {
    match opt.as_deref() {
        Some(s) if !s.trim().is_empty() => s,
        _ => "-",
    }
}

/// Builds a colored panel title for a planet, based on canon/legends flags.
fn build_planet_title(p: &PlanetSearchRow) -> Line<'static> {
    let color = match (p.canon, p.legends) {
        (true, false) => Color::Green,
        (false, true) => Color::Yellow,
        (true, true) => Color::Cyan,
        _ => Color::Gray,
    };

    Line::from(Span::styled(
        format!("{} ({})", p.name, p.fid),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

/// Builds the standard TUI panel for a known planet.
pub(crate) fn build_planet_panel(
    p: &PlanetSearchRow,
    aliases: Option<&[String]>,
) -> (Line<'static>, Vec<String>) {
    let title = build_planet_title(p);

    let mut lines = vec![
        panel_kv("Region", tui_cell(&p.region)),
        panel_kv("Sector", tui_cell(&p.sector)),
        panel_kv("System", tui_cell(&p.system)),
        panel_kv("Grid", tui_cell(&p.grid)),
        panel_kv("X", format!("{:.2}", p.x)),
        panel_kv("Y", format!("{:.2}", p.y)),
        panel_kv("Canon", if p.canon { "Yes" } else { "No" }),
        panel_kv("Legends", if p.legends { "Yes" } else { "No" }),
        panel_kv("Status", tui_cell(&p.status)),
    ];

    if let Some(alias_list) = aliases
        && !alias_list.is_empty()
    {
        lines.push(String::new());
        lines.push("Aliases:".to_string());
        for alias in alias_list {
            lines.push(format!("  - {}", alias));
        }
    }

    (title, lines)
}

/// Builds the secondary TUI panel for a nearby planet.
///
/// At the moment this is equivalent to `build_planet_panel`, but it is kept
/// as a separate function for semantic clarity and future customization.
pub(crate) fn build_near_planet_panel(
    planet: &PlanetSearchRow,
    aliases: Option<&[String]>,
) -> (Line<'static>, Vec<String>) {
    build_planet_panel(planet, aliases)
}

/// Builds the central navigation panel shown between Planet 1 and Planet 2.
pub(crate) fn build_navigation_panel(kind: NavigationPanelKind) -> (Line<'static>, Vec<String>) {
    let title = Line::from(Span::styled(
        "Navigation",
        Style::default()
            .fg(Color::LightYellow)
            .add_modifier(Modifier::BOLD),
    ));

    let lines = match kind {
        NavigationPanelKind::Empty => vec!["No route data".to_string()],

        NavigationPanelKind::Route {
            length_parsec,
            eta_text,
            detours,
            region_text,
        } => {
            let mut lines = vec![
                panel_kv(
                    "Length",
                    length_parsec
                        .map(|v| format!("{:.3} parsec", v))
                        .unwrap_or_else(|| "-".to_string()),
                ),
                panel_kv("ETA", eta_text.unwrap_or_else(|| "-".to_string())),
            ];

            if let Some(detours) = detours {
                lines.push(panel_kv("Detours", detours));
            }

            if let Some(region_text) = region_text {
                lines.push(panel_kv("Region", region_text));
            }

            lines
        }

        NavigationPanelKind::Near {
            distance_parsec,
            reference_name,
        } => {
            let mut lines = vec![panel_kv("Distance", format!("{:.2} pc", distance_parsec))];

            if let Some(reference_name) = reference_name
                && !reference_name.trim().is_empty()
            {
                lines.push(panel_kv("Reference", reference_name));
            }

            lines
        }
    };

    (title, lines)
}

/// Builds the TUI output structure for a persisted route.
pub(crate) fn build_route_show_output(
    con: &rusqlite::Connection,
    loaded: &RouteLoaded,
) -> Result<TuiCommandOutput> {
    let mut out = tui_default_output();

    let (from_planet, from_aliases) =
        commands::info::resolve_by_fid(con, loaded.route.from_planet_fid)?;
    let (to_planet, to_aliases) = commands::info::resolve_by_fid(con, loaded.route.to_planet_fid)?;

    let (p1_title, p1_lines) = build_planet_panel(&from_planet, Some(&from_aliases));
    let (p2_title, p2_lines) = build_planet_panel(&to_planet, Some(&to_aliases));

    out.planet1_title = p1_title;
    out.planet1_lines = p1_lines;
    out.planet2_title = p2_title;
    out.planet2_lines = p2_lines;

    let route = &loaded.route;

    let eta_estimate = route_eta(con, loaded);

    let eta_text = eta_estimate.as_ref().map(|e| e.format_human());

    let region_text = eta_estimate.as_ref().map(|e| {
        format!(
            "{} → {}",
            region_name(e.from_region),
            region_name(e.to_region)
        )
    });

    let (nav_title, nav_lines) = build_navigation_panel(NavigationPanelKind::Route {
        length_parsec: route.length,
        eta_text,
        detours: Some(loaded.detours.len()),
        region_text,
    });
    out.navigation_title = nav_title;
    out.navigation_lines = nav_lines;

    out.log_lines.push(format!("Route #{}", route.id));
    out.log_lines.push(format!(
        "{} → {}",
        route.from_planet_name, route.to_planet_name
    ));

    if route.status != "ok" {
        out.log_lines.push(format!("Status: {}", route.status));
    }

    if let Some(len) = route.length {
        out.log_lines.push(format!("Length: {:.3} parsec", len));
    }

    if let Some(it) = route.iterations {
        out.log_lines.push(format!("Iterations: {}", it));
    }

    if let Some(upd) = route.updated_at.as_deref() {
        out.log_lines.push(format!("Updated: {}", upd));
    } else {
        out.log_lines.push(format!("Created: {}", route.created_at));
    }

    out.log_lines.push(String::new());
    out.log_lines
        .push(format!("Waypoints: {}", loaded.waypoints.len()));
    out.log_lines.push(String::new());

    out.log_lines.push(format!(
        "  {:>3}  {:>10}  {:>10}  {:>10}  {:>10}  {}",
        "Seq", "X", "Y", "Segment", "Cumul.", "Label"
    ));
    out.log_lines.push(format!(
        "  {:->3}  {:->10}  {:->10}  {:->10}  {:->10}  {:->20}",
        "", "", "", "", "", ""
    ));

    let last_seq = loaded.waypoints.len().saturating_sub(1);
    let mut cumulative = 0.0_f64;

    for (i, w) in loaded.waypoints.iter().enumerate() {
        let segment_dist = if i == 0 {
            0.0
        } else {
            let prev = &loaded.waypoints[i - 1];
            use sw_galaxy_map_core::routing::geometry::{Point, dist as geom_dist};
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

        let seg_str = if i == 0 {
            "-".to_string()
        } else {
            format!("{:.3}", segment_dist)
        };

        out.log_lines.push(format!(
            "  {:>3}  {:>10.3}  {:>10.3}  {:>10}  {:>10.3}  {}",
            w.seq, w.x, w.y, seg_str, cumulative, label
        ));
    }

    if let Some(ref eta) = eta_estimate {
        out.log_lines.push(String::new());
        out.log_lines.push("ETA Breakdown:".to_string());
        out.log_lines.push(format!(
            "  Route length     : {:.3} parsec",
            eta.route_length_parsec
        ));
        out.log_lines.push(format!(
            "  Direct distance  : {:.3} parsec",
            eta.direct_length_parsec
        ));

        let overhead_pct = if eta.direct_length_parsec > 0.0 {
            ((eta.route_length_parsec / eta.direct_length_parsec) - 1.0) * 100.0
        } else {
            0.0
        };
        out.log_lines
            .push(format!("  Route overhead   : +{:.1}%", overhead_pct));
        out.log_lines
            .push(format!("  Hyperdrive class : {:.1}", eta.hyperdrive_class));
        out.log_lines.push(String::new());
        out.log_lines.push("  Regions:".to_string());
        out.log_lines.push(format!(
            "    Origin         : {:?} (CF={:.1})",
            eta.from_region,
            eta.from_region.base_compression_factor()
        ));
        out.log_lines.push(format!(
            "    Destination    : {:?} (CF={:.1})",
            eta.to_region,
            eta.to_region.base_compression_factor()
        ));
        out.log_lines.push(format!(
            "    Base CF        : {:.2}",
            eta.base_compression_factor
        ));
        out.log_lines.push(String::new());
        out.log_lines.push("  Detour multipliers:".to_string());
        out.log_lines.push(format!(
            "    Geometric      : {:.4}",
            eta.detour_multiplier_geom
        ));
        out.log_lines.push(format!(
            "    Count          : {:.4} ({} detours)",
            eta.detour_multiplier_count, eta.detour_count
        ));
        out.log_lines.push(format!(
            "    Severity       : {:.4} (sum={:.3})",
            eta.detour_multiplier_severity, eta.severity_sum
        ));
        out.log_lines.push(format!(
            "    Combined       : {:.4}",
            eta.detour_multiplier_total
        ));
        out.log_lines.push(String::new());
        out.log_lines.push(format!(
            "  Effective CF     : {:.2}",
            eta.effective_compression_factor
        ));
        out.log_lines
            .push(format!("  ETA              : {}", eta.format_human()));
    }

    out.log_lines.push(String::new());
    out.log_lines
        .push(format!("Detours: {}", loaded.detours.len()));

    if !loaded.detours.is_empty() {
        let route_len = eta_estimate
            .as_ref()
            .map(|e| e.route_length_parsec)
            .unwrap_or(0.0);
        let direct_len = eta_estimate
            .as_ref()
            .map(|e| e.direct_length_parsec)
            .unwrap_or(0.0);
        let overhead_parsec = route_len - direct_len;
        let overhead_pct = if direct_len > 0.0 {
            (overhead_parsec / direct_len) * 100.0
        } else {
            0.0
        };

        let avg_score: f64 =
            loaded.detours.iter().map(|d| d.score_total).sum::<f64>() / loaded.detours.len() as f64;

        let worst = loaded.detours.iter().max_by(|a, b| {
            a.score_total
                .partial_cmp(&b.score_total)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let exhausted = loaded
            .detours
            .iter()
            .filter(|d| d.tries_exhausted == 1)
            .count();

        out.log_lines.push(String::new());
        out.log_lines.push("Detour Summary:".to_string());
        out.log_lines.push(format!(
            "  Route overhead   : +{:.3} pc (+{:.1}%)",
            overhead_parsec, overhead_pct
        ));
        out.log_lines
            .push(format!("  Avg score        : {:.3}", avg_score));

        if let Some(w) = worst {
            out.log_lines.push(format!(
                "  Worst detour     : det#{} {} score={:.3}",
                w.idx, w.obstacle_name, w.score_total
            ));
        }

        out.log_lines.push(format!(
            "  Exhausted tries  : {}/{}",
            exhausted,
            loaded.detours.len()
        ));

        out.log_lines.push(String::new());
    }

    if !loaded.detours.is_empty() {
        out.log_lines.push(String::new());
    }

    for (i, d) in loaded.detours.iter().enumerate() {
        out.log_lines.push(format!(
            "  {}. {} (ID: {})",
            i + 1,
            d.obstacle_name,
            d.obstacle_id
        ));
        out.log_lines
            .push(format!("     waypoint: ({:.3}, {:.3})", d.wp_x, d.wp_y));
        out.log_lines
            .push(format!("     score: {:.3}", d.score_total));
        out.log_lines.push(String::new());
    }

    Ok(out)
}
