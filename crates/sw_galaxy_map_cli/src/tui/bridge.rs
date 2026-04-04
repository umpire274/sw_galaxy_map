use crate::cli::commands::route::list::resolve_list_for_tui;
use crate::cli::commands::route::resolve_show_for_tui;
use crate::cli::{args, commands};
use crate::tui::{
    NavigationPanelKind, TuiCommandOutput, build_navigation_panel, build_near_planet_panel,
    build_planet_panel, build_route_show_output, tui_default_output,
};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use sw_galaxy_map_core::model::RouteLoaded;
use sw_galaxy_map_core::routing::eta::{RouteEtaEstimate, estimate_route_eta};
use sw_galaxy_map_core::validate;

pub(crate) fn run_one_shot_for_tui(
    cli: &args::Cli,
    cmd: &args::Commands,
) -> anyhow::Result<TuiCommandOutput> {
    match cmd {
        args::Commands::Search {
            query,
            region,
            sector,
            grid,
            status,
            canon,
            legends,
            fuzzy,
            limit,
        } => {
            let filter = sw_galaxy_map_core::model::SearchFilter {
                query: query.clone(),
                region: region.clone(),
                sector: sector.clone(),
                grid: grid.clone(),
                status: status.clone(),
                canon: if *canon { Some(true) } else { None },
                legends: if *legends { Some(true) } else { None },
                fuzzy: *fuzzy,
                limit: *limit,
            };
            validate::validate_search(&filter)?;
            let con = crate::cli::open_db_migrating(cli.db.clone())?;

            let mut out = tui_default_output();
            let query_label = query.as_deref().unwrap_or("(filter)");

            // --- Explicit fuzzy mode: resolve and show as selectable results ---
            if filter.fuzzy {
                return if let Some(qn) = query
                    .as_deref()
                    .map(sw_galaxy_map_core::utils::normalize_text)
                    .filter(|s| !s.is_empty())
                {
                    let hits = sw_galaxy_map_core::utils::fuzzy::fuzzy_search(
                        &con,
                        &qn,
                        3,
                        filter.limit as usize,
                        filter.status.as_deref(),
                    )?;

                    if hits.is_empty() {
                        out.log_lines.push(format!(
                            "Fuzzy search for \"{}\": no matches found (max distance: 3)",
                            query_label
                        ));
                        return Ok(out);
                    }

                    let resolved =
                        sw_galaxy_map_core::utils::fuzzy::resolve_fuzzy_hits(&con, &hits)?;

                    if resolved.len() == 1 {
                        let (planet, dist) = &resolved[0];
                        let (title, lines) = build_planet_panel(planet, None);

                        out.log_lines.push(format!(
                            "Fuzzy search for \"{}\": 1 match (distance: {})",
                            query_label, dist
                        ));
                        out.log_lines
                            .push(format!("Displaying result: {}", planet.name));

                        out.planet1_title = title;
                        out.planet1_lines = lines;

                        return Ok(out);
                    }

                    out.log_lines.push(format!(
                        "Fuzzy search for \"{}\": {} matches found",
                        query_label,
                        resolved.len()
                    ));
                    out.log_lines.push(String::new());

                    let mut search_rows = Vec::new();
                    for (idx, (planet, dist)) in resolved.iter().enumerate() {
                        out.log_lines.push(format!(
                            "  {}. {} (distance: {})",
                            idx + 1,
                            planet.name,
                            dist
                        ));
                        search_rows.push(planet.clone());
                    }

                    out.log_lines.push(String::new());
                    out.log_lines
                        .push("Type a number or `option N` to inspect a result.".to_string());

                    out.search_results = search_rows;

                    Ok(out)
                } else {
                    out.log_lines
                        .push("--fuzzy requires a text query".to_string());
                    Ok(out)
                };
            }

            let rows = sw_galaxy_map_core::db::queries::search_planets_filtered(&con, &filter)?;

            if rows.is_empty() {
                // --- Fuzzy fallback: suggest alternatives when exact search finds nothing ---
                if let Some(qn) = query
                    .as_deref()
                    .map(sw_galaxy_map_core::utils::normalize_text)
                    .filter(|s| !s.is_empty())
                {
                    let hits =
                        sw_galaxy_map_core::utils::fuzzy::fuzzy_search(&con, &qn, 3, 5, None)?;
                    if !hits.is_empty() {
                        out.log_lines.push(format!(
                            "Search result for \"{}\": no planets found",
                            query_label
                        ));
                        out.log_lines.push(String::new());
                        out.log_lines.push("Did you mean?".to_string());
                        for hit in &hits {
                            out.log_lines
                                .push(format!("  - {} (distance: {})", hit.name, hit.distance));
                        }
                        return Ok(out);
                    }
                }

                out.log_lines.push(format!(
                    "Search result for \"{}\": no planets found",
                    query_label
                ));
                return Ok(out);
            }

            if rows.len() == 1 {
                let planet = &rows[0];
                let (title, lines) = build_planet_panel(planet, None);

                out.log_lines.push(format!(
                    "Search result for \"{}\": 1 planet found",
                    query_label
                ));
                out.log_lines
                    .push(format!("Displaying result: {}", planet.name));

                out.planet1_title = title;
                out.planet1_lines = lines;

                return Ok(out);
            }

            out.log_lines.push(format!(
                "Search result for \"{}\": {} planets found",
                query_label,
                rows.len()
            ));
            out.log_lines.push(String::new());

            for (idx, p) in rows.iter().enumerate() {
                out.log_lines.push(format!("  {}. {}", idx + 1, p.name));
            }

            out.log_lines.push(String::new());
            out.log_lines
                .push("Type a number or `option N` to inspect a result.".to_string());

            out.search_results = rows;

            Ok(out)
        }

        args::Commands::Info { planet } => {
            let con = crate::cli::open_db_migrating(cli.db.clone())?;
            let (row, aliases) = commands::info::resolve(&con, planet)?;

            let mut out = tui_default_output();
            let (title, lines) = build_planet_panel(&row, Some(&aliases));

            out.log_lines
                .push(format!("Info result for \"{}\": planet found", planet));
            out.planet1_title = title;
            out.planet1_lines = lines;

            Ok(out)
        }

        args::Commands::Near {
            range,
            planet,
            unknown,
            fid,
            x,
            y,
            limit,
            ..
        } => {
            validate::validate_near(*unknown, fid, planet, x, y)?;
            let con = crate::cli::open_db_migrating(cli.db.clone())?;

            let (reference, hits) = commands::near::resolve(
                &con,
                *range,
                *unknown,
                *fid,
                planet.clone(),
                *x,
                *y,
                *limit,
            )?;

            let mut out = tui_default_output();

            match &reference {
                commands::near::NearReference::Planet(reference_planet) => {
                    let (title, lines) = build_planet_panel(reference_planet, None);
                    out.planet1_title = title;
                    out.planet1_lines = lines;
                    out.log_lines
                        .push(format!("Reference planet: {}", reference_planet.name));
                }
                commands::near::NearReference::Coordinates { x, y } => {
                    out.planet1_title = Line::from(Span::styled(
                        format!("Coordinates ({:.2}, {:.2})", x, y),
                        Style::default()
                            .fg(Color::LightYellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                    out.planet1_lines = vec![
                        format!("X: {:.2}", x),
                        format!("Y: {:.2}", y),
                        format!("Radius: {:.2} pc", range),
                    ];
                    out.log_lines
                        .push(format!("Reference coordinates: X={:.2}, Y={:.2}", x, y));
                }
            }

            if hits.is_empty() {
                out.log_lines.push(format!(
                    "Near result within {:.2} parsecs: no planets found",
                    range
                ));
                return Ok(out);
            }

            out.log_lines.push(format!(
                "Near result within {:.2} parsecs: {} planet{} found",
                range,
                hits.len(),
                if hits.len() == 1 { "" } else { "s" }
            ));
            out.log_lines.push(String::new());

            for (idx, hit) in hits.iter().enumerate() {
                out.log_lines.push(format!(
                    "  {}. {} ({:.2} pc)",
                    idx + 1,
                    hit.planet,
                    hit.distance
                ));
            }

            out.log_lines.push(String::new());
            out.log_lines
                .push("Type a number or `option N` to inspect a nearby planet.".to_string());

            if hits.len() == 1 {
                let hit = &hits[0];
                let (planet, aliases) = commands::info::resolve_by_fid(&con, hit.fid)?;
                let (title2, lines2) = build_near_planet_panel(&planet, Some(&aliases));

                out.planet2_title = title2;
                out.planet2_lines = lines2;

                let reference_name = match &reference {
                    commands::near::NearReference::Planet(p) => Some(p.name.clone()),
                    commands::near::NearReference::Coordinates { x, y } => {
                        Some(format!("({:.2}, {:.2})", x, y))
                    }
                };
                let (nav_title, nav_lines) = build_navigation_panel(NavigationPanelKind::Near {
                    distance_parsec: hit.distance,
                    reference_name: reference_name.clone(),
                });
                out.navigation_title = nav_title;
                out.navigation_lines = nav_lines;
            } else {
                out.near_results = hits;
            }

            Ok(out)
        }

        args::Commands::Route { cmd } => match cmd {
            args::RouteCmd::Compute(args) => {
                validate::validate_route_planets(&args.planets)?;
                let mut con = crate::cli::open_db_migrating(cli.db.clone())?;
                let computed = commands::route::resolve_compute_for_tui(&mut con, args)?;

                let loaded = sw_galaxy_map_core::db::queries::load_route(&con, computed.route_id)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("Route not found after compute: id={}", computed.route_id)
                    })?;

                let mut out = build_route_show_output(&con, &loaded)?;
                out.log_lines
                    .insert(0, "Route computed successfully.".to_string());

                Ok(out)
            }

            args::RouteCmd::List {
                json: _,
                file: _,
                limit,
                status,
                from,
                to,
                wp,
                sort,
            } => {
                validate::validate_limit(*limit as i64, "list")?;
                let con = crate::cli::open_db_migrating(cli.db.clone())?;
                let items =
                    resolve_list_for_tui(&con, *limit, status.as_deref(), *from, *to, *wp, *sort)?;

                let mut out = tui_default_output();

                if items.is_empty() {
                    out.log_lines
                        .push("Route list: no routes found.".to_string());
                    return Ok(out);
                }

                out.log_lines.push("Routes:".to_string());
                out.log_lines.push(String::new());

                let len_width = items
                    .iter()
                    .map(|item| {
                        item.length_parsec
                            .map(|v| format!("{:.3} pc", v))
                            .unwrap_or_else(|| "-".to_string())
                            .len()
                    })
                    .max()
                    .unwrap_or(1);

                for (idx, item) in items.iter().enumerate() {
                    let len_txt = item
                        .length_parsec
                        .map(|v| format!("{:.3} pc", v))
                        .unwrap_or_else(|| "-".to_string());

                    out.log_lines.push(format!(
                        "  {}. {} → {} (ID: {})",
                        idx + 1,
                        item.from_name,
                        item.to_name,
                        item.route_id
                    ));

                    let status_suffix = if item.status != "ok" {
                        format!(" | status: {}", item.status)
                    } else {
                        String::new()
                    };

                    out.log_lines.push(format!(
                        "     len: {:>width$} | wp: {:>2} | det: {:>2}{}",
                        len_txt,
                        item.waypoints_count,
                        item.detours_count,
                        status_suffix,
                        width = len_width
                    ));

                    out.log_lines.push(String::new());
                }

                out.log_lines.push(String::new());
                out.log_lines
                    .push("Type a number or `option N` to open a listed route.".to_string());

                out.route_list_results = items;

                Ok(out)
            }

            args::RouteCmd::Show { route_id } => {
                validate::validate_route_id(*route_id, "show")?;
                let con = crate::cli::open_db_migrating(cli.db.clone())?;
                let data = resolve_show_for_tui(&con, *route_id)?;
                build_route_show_output(&con, &data.loaded)
            }

            _ => {
                let mut out = tui_default_output();
                out.log_lines.push(
                    "TUI rendering for this route subcommand is not implemented yet.".to_string(),
                );
                Ok(out)
            }
        },

        args::Commands::Db { cmd } => match cmd {
            args::DbCommands::Stats { top } => {
                let con = crate::cli::open_db_migrating(cli.db.clone())?;
                let s = sw_galaxy_map_core::db::queries::galaxy_stats(&con, *top)?;
                let mut out = tui_default_output();
                crate::cli::reports::build_galaxy_stats_tui(&s, *top, &mut out);
                Ok(out)
            }
            _ => {
                let mut out = tui_default_output();
                out.log_lines.push(
                    "This db subcommand is not available in TUI. Use the CLI directly.".to_string(),
                );
                Ok(out)
            }
        },

        _ => {
            let mut out = tui_default_output();
            out.log_lines
                .push("TUI rendering for this command is not implemented yet.".to_string());
            Ok(out)
        }
    }
}

#[allow(dead_code)]
pub fn route_eta_text(con: &rusqlite::Connection, loaded: &RouteLoaded) -> Option<String> {
    route_eta(con, loaded).map(|e| e.format_human())
}

pub fn route_eta(con: &rusqlite::Connection, loaded: &RouteLoaded) -> Option<RouteEtaEstimate> {
    estimate_route_eta(
        con,
        loaded,
        crate::tui::types::ETA_HYPERDRIVE_CLASS,
        crate::tui::types::ETA_REGION_BLEND,
        crate::tui::types::ETA_DETOUR_COUNT_BASE,
        crate::tui::types::ETA_SEVERITY_K,
    )
}
