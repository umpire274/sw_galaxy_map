pub mod args;
pub mod color;
pub mod commands;
pub mod export;
pub mod tui;
pub mod typewriter;

use crate::ui::{error, info, success, warning};
use anyhow::Result;
use clap::Parser;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::path::Path;
use sw_galaxy_map_core::db::db_status::{DbHealth, DbStatusReport, resolve_db_path};
use sw_galaxy_map_core::db::db_update::{ChangeKind, DbUpdateReport};
use sw_galaxy_map_core::db::migrate::MigrationReport;
use sw_galaxy_map_core::model::{NearHit, PlanetSearchRow, RouteLoaded};
use sw_galaxy_map_core::routing::eta::{RegionBlend, RouteEtaEstimate, estimate_route_eta};
use sw_galaxy_map_core::validate;

#[derive(Debug, Clone)]
pub(crate) struct TuiCommandOutput {
    pub log_lines: Vec<String>,
    pub planet1_title: Line<'static>,
    pub planet1_lines: Vec<String>,
    pub navigation_title: Line<'static>,
    pub navigation_lines: Vec<String>,
    pub planet2_title: Line<'static>,
    pub planet2_lines: Vec<String>,
    pub search_results: Vec<PlanetSearchRow>,
    pub near_results: Vec<NearHit>,
    pub route_list_results: Vec<commands::route::RouteListTuiItem>,
}

pub(crate) enum NavigationPanelKind {
    Empty,
    Route {
        length_parsec: Option<f64>,
        eta_text: Option<String>,
        detours: Option<usize>,
        region_text: Option<String>,
    },
    Near {
        distance_parsec: f64,
        reference_name: Option<String>,
    },
}

const PANEL_LABEL_WIDTH: usize = 9;
const ETA_HYPERDRIVE_CLASS: f64 = 1.0;
const ETA_DETOUR_COUNT_BASE: f64 = 0.97;
const ETA_SEVERITY_K: f64 = 0.15;
const ETA_REGION_BLEND: RegionBlend = RegionBlend::Avg;

fn panel_kv(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label:<PANEL_LABEL_WIDTH$}: {value}")
}

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();

    if cli.cmd.is_none() {
        return run_interactive_shell(cli.db.clone());
    }

    let cmd = cli
        .cmd
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing command"))?;

    run_one_shot(&cli, cmd)
}

fn print_db_init_report(report: &sw_galaxy_map_core::db::db_init::DbInitReport) {
    println!(
        "Initializing local database at: {}",
        report.out_path.display()
    );
    if report.overwritten_existing {
        info("Existing database overwritten.");
    }
    println!("Downloading data from remote service...");
    info(format!(
        "Downloaded {} features.",
        report.downloaded_features
    ));
    println!("Building SQLite database...");
    println!(
        "FTS5 enabled: {}",
        if report.fts_enabled { "yes" } else { "no" }
    );
    println!("Done.");
}

fn print_db_status_report(report: &DbStatusReport) {
    match report.health {
        DbHealth::Ok => success("Status: OK"),
        DbHealth::Missing => error("Status: MISSING"),
        DbHealth::Invalid => warning("Status: INVALID"),
    }

    info(format!("Database path: {}", report.db_path.display()));
    match report.file_size_bytes {
        Some(bytes) => info(format!("Database size: {} bytes", bytes)),
        None => warning("Database size: unavailable"),
    }

    for line in &report.lines {
        println!("{}", line);
    }
    for msg in &report.warnings {
        warning(msg);
    }
}

fn print_db_update_report(report: &DbUpdateReport) {
    info("Fetching data from remote service...");
    info(format!(
        "Downloaded {} features. Comparing with local database...",
        report.downloaded_features
    ));
    if report.dry_run {
        warning("DRY-RUN mode enabled: no changes will be written");
        if report.prune {
            warning("Prune requested in dry-run: this will be reported as 'would prune'");
        }
    }

    if report.dry_run {
        success("Dry-run completed (no changes written)");
    } else {
        success("Update completed");
    }

    info(format!("inserted: {}", report.summary.inserted));
    info(format!("updated: {}", report.summary.updated));
    info(format!("revived: {}", report.summary.revived));
    info(format!("unchanged: {}", report.summary.unchanged));
    info(format!("marked deleted: {}", report.summary.marked_deleted));

    if report.prune {
        if report.dry_run {
            info(format!("would prune: {}", report.summary.pruned));
        } else {
            info(format!("pruned: {}", report.summary.pruned));
        }
    }

    if report.summary.skipped > 0 {
        warning(format!("skipped invalid rows: {}", report.summary.skipped));
        info(format!(
            "  missing Planet: {}",
            report.summary.skipped_missing_planet
        ));
        info(format!("  missing X: {}", report.summary.skipped_missing_x));
        info(format!("  missing Y: {}", report.summary.skipped_missing_y));
    }

    if let Some(stats) = &report.stats {
        fn kind_label(k: ChangeKind) -> &'static str {
            match k {
                ChangeKind::Inserted => "inserted",
                ChangeKind::Updated => "updated",
                ChangeKind::Revived => "revived",
                ChangeKind::MarkedDeleted => "marked deleted",
            }
        }
        println!();
        info("Stats:");
        for (kind, rows) in [
            (ChangeKind::Inserted, &stats.top_inserted),
            (ChangeKind::Updated, &stats.top_updated),
            (ChangeKind::Revived, &stats.top_revived),
            (ChangeKind::MarkedDeleted, &stats.top_marked_deleted),
        ] {
            info(format!("  Top {} {}:", rows.len(), kind_label(kind)));
            if rows.is_empty() {
                info("    (none)");
            } else {
                for e in rows {
                    if let Some(p) = &e.planet {
                        info(format!("    FID={} | {}", e.fid, p));
                    } else {
                        info(format!("    FID={}", e.fid));
                    }
                }
            }
        }
        info(format!(
            "  First {} changed FIDs:",
            stats.first_changed.len()
        ));
        if stats.first_changed.is_empty() {
            info("    (none)");
        } else {
            for e in &stats.first_changed {
                let planet = e
                    .planet
                    .as_ref()
                    .map(|p| format!(" | {}", p))
                    .unwrap_or_default();
                info(format!(
                    "    FID={} | {}{}",
                    e.fid,
                    kind_label(e.kind),
                    planet
                ));
            }
        }
    }
}

fn print_migration_report(report: &MigrationReport) {
    if report.noop {
        info(format!(
            "Database schema already up-to-date (v{})",
            report.current_version
        ));
        return;
    }
    info(format!(
        "Database schema upgrade required (current: v{}, target: v{})",
        report.current_version, report.target_version
    ));
    if report.dry_run {
        warning("DRY-RUN: no changes will be applied");
    }
    for step in &report.applied {
        info(format!(
            "Applying migration: v{} → v{} ({})",
            step.from, step.to, step.label
        ));
        success(format!("Migration v{} → v{} completed", step.from, step.to));
    }
    if report.dry_run {
        info(format!(
            "Dry-run completed: {} migration(s) would be applied.",
            report.applied.len()
        ));
    } else {
        info(format!(
            "Database schema successfully updated (applied {} migration(s)).",
            report.applied.len()
        ));
    }
}

fn tui_default_output() -> TuiCommandOutput {
    let (navigation_title, navigation_lines) = build_navigation_panel(NavigationPanelKind::Empty);

    TuiCommandOutput {
        log_lines: Vec::new(),
        planet1_title: Line::from("Planet 1 Information"),
        planet1_lines: vec!["No data".to_string()],
        navigation_title,
        navigation_lines,
        planet2_title: Line::from("Planet 2 Information"),
        planet2_lines: vec!["No data".to_string()],
        search_results: Vec::new(),
        near_results: Vec::new(),
        route_list_results: Vec::new(),
    }
}

fn tui_cell(opt: &Option<String>) -> &str {
    match opt.as_deref() {
        Some(s) if !s.trim().is_empty() => s,
        _ => "-",
    }
}

fn run_one_shot(cli: &args::Cli, cmd: &args::Commands) -> Result<()> {
    match cmd {
        args::Commands::Db { cmd } => match cmd {
            args::DbCommands::Init { out, force } => {
                let report = sw_galaxy_map_core::db::db_init::run(out.clone(), *force)?;
                print_db_init_report(&report);
                Ok(())
            }

            args::DbCommands::Status => {
                let report = sw_galaxy_map_core::db::db_status::run(cli.db.clone())?;
                print_db_status_report(&report);
                Ok(())
            }

            args::DbCommands::Update {
                prune,
                dry_run,
                stats,
                stats_limit,
            } => {
                let mut con = open_db_migrating(cli.db.clone())?;
                let report = sw_galaxy_map_core::db::db_update::run(
                    &mut con,
                    *prune,
                    *dry_run,
                    *stats,
                    *stats_limit,
                )?;
                print_db_update_report(&report);
                Ok(())
            }

            args::DbCommands::SkippedPlanets => {
                let mut con = open_db_migrating(cli.db.clone())?;
                sw_galaxy_map_core::db::db_skipped_planets::run(&mut con)
            }

            args::DbCommands::Migrate { dry_run } => {
                // IMPORTANT: do not auto-migrate before running migrate
                let mut con = open_db_raw(cli.db.clone())?;
                let report = sw_galaxy_map_core::db::migrate::run(&mut con, *dry_run, true)?;
                print_migration_report(&report);
                Ok(())
            }

            args::DbCommands::RebuildSearch => {
                let mut con = open_db_migrating(cli.db.clone())?;
                info("Rebuilding planet_search and FTS indexes...");
                sw_galaxy_map_core::db::provision::rebuild_search_indexes(&mut con)?;
                success("planet_search and FTS indexes rebuilt successfully.");
                Ok(())
            }

            args::DbCommands::Stats { top } => {
                let con = open_db_migrating(cli.db.clone())?;
                let s = sw_galaxy_map_core::db::queries::galaxy_stats(&con, *top)?;
                print_galaxy_stats(&s, *top);
                Ok(())
            }

            args::DbCommands::Sync {
                csv,
                table,
                delimiter,
                dry_run,
                mark_deleted,
                report,
            } => {
                let csv_path = sw_galaxy_map_sync::resolve_csv_path(csv)?;

                let delimiter_byte = delimiter
                    .to_string()
                    .as_bytes()
                    .first()
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Invalid delimiter"))?;

                let mut con = open_db_migrating(cli.db.clone())?;

                info(format!("Syncing from CSV: {}", csv_path.display()));

                let opts = sw_galaxy_map_sync::SyncOptions {
                    csv: csv_path,
                    table: table.clone(),
                    delimiter: delimiter_byte,
                    dry_run: *dry_run,
                    mark_deleted: *mark_deleted,
                    report_path: report.clone(),
                };

                let result = sw_galaxy_map_sync::run_sync(&mut con, &opts)?;

                println!();
                info("Sync summary:");
                println!("  Inserted         : {}", result.stats.inserted);
                println!("  Updated exact    : {}", result.stats.updated_exact);
                println!("  Updated suffix   : {}", result.stats.updated_suffix);
                println!("  Invalid CSV rows : {}", result.stats.invalid_csv_rows);
                println!("  Marked invalid   : {}", result.stats.invalid_marked);
                println!("  Skipped DB       : {}", result.stats.skipped_db);
                println!("  Logically deleted: {}", result.stats.deleted_logically);

                if !*dry_run {
                    println!();
                    info("Rebuilding planet_search and FTS indexes...");
                    sw_galaxy_map_core::db::provision::rebuild_search_indexes(&mut con)?;
                    success("Sync complete. Search indexes rebuilt.");
                } else {
                    success("Dry run complete. No changes written.");
                }

                Ok(())
            }
        },

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
            let con = open_db_migrating(cli.db.clone())?;
            commands::search::run(&con, filter)
        }

        args::Commands::Info { planet } => {
            let con = open_db_migrating(cli.db.clone())?;
            commands::info::run(&con, planet.clone())
        }

        args::Commands::Near {
            range,
            unknown,
            fid,
            planet,
            x,
            y,
            limit,
        } => {
            validate::validate_near(*unknown, fid, planet, x, y)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::near::run(&con, *range, *unknown, *fid, planet.clone(), *x, *y, *limit)
        }

        args::Commands::Waypoint { cmd } => {
            let mut con = open_db_migrating(cli.db.clone())?;
            commands::waypoints::run_waypoint(&mut con, cmd)
        }

        args::Commands::Route { cmd } => {
            let mut con = open_db_migrating(cli.db.clone())?;
            commands::route::run(&mut con, cmd)
        }

        args::Commands::Unknown { cmd } => {
            let con = open_db_migrating(cli.db.clone())?;
            commands::unknown::run(&con, cmd)
        }
    }
}

fn run_interactive_shell(db_arg: Option<String>) -> Result<()> {
    tui::run_tui(db_arg).map_err(Into::into)
}

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

pub(crate) fn build_near_planet_panel(
    planet: &PlanetSearchRow,
    aliases: Option<&[String]>,
) -> (Line<'static>, Vec<String>) {
    let title = build_planet_title(planet);

    let mut lines = vec![
        panel_kv("Region", tui_cell(&planet.region)),
        panel_kv("Sector", tui_cell(&planet.sector)),
        panel_kv("System", tui_cell(&planet.system)),
        panel_kv("Grid", tui_cell(&planet.grid)),
        panel_kv("X", format!("{:.2}", planet.x)),
        panel_kv("Y", format!("{:.2}", planet.y)),
        panel_kv("Canon", if planet.canon { "Yes" } else { "No" }),
        panel_kv("Legends", if planet.legends { "Yes" } else { "No" }),
        panel_kv("Status", tui_cell(&planet.status)),
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

pub(crate) fn build_route_show_output(
    con: &rusqlite::Connection,
    loaded: &sw_galaxy_map_core::model::RouteLoaded,
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

    let eta_estimate = estimate_route_eta(
        con,
        loaded,
        ETA_HYPERDRIVE_CLASS,
        ETA_REGION_BLEND,
        ETA_DETOUR_COUNT_BASE,
        ETA_SEVERITY_K,
    );

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

    // --- Waypoint-by-waypoint with segment/cumulative distances ---
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

    // --- ETA breakdown in log ---
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

    // --- Detour summary ---
    out.log_lines.push(String::new());
    out.log_lines
        .push(format!("Detours: {}", loaded.detours.len()));

    if !loaded.detours.is_empty() {
        // Aggregate stats
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

pub(crate) fn run_one_shot_for_tui(
    cli: &args::Cli,
    cmd: &args::Commands,
) -> Result<TuiCommandOutput> {
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
            let con = open_db_migrating(cli.db.clone())?;

            let mut out = tui_default_output();
            let query_label = query.as_deref().unwrap_or("(filter)");

            // --- Explicit fuzzy mode: resolve and show as selectable results ---
            if filter.fuzzy {
                if let Some(qn) = query
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

                    return Ok(out);
                } else {
                    out.log_lines
                        .push("--fuzzy requires a text query".to_string());
                    return Ok(out);
                }
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
            let con = open_db_migrating(cli.db.clone())?;
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
            let con = open_db_migrating(cli.db.clone())?;

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
                let mut con = open_db_migrating(cli.db.clone())?;
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
                let con = open_db_migrating(cli.db.clone())?;
                let items = commands::route::resolve_list_for_tui(
                    &con,
                    *limit,
                    status.as_deref(),
                    *from,
                    *to,
                    *wp,
                    *sort,
                )?;

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
                let con = open_db_migrating(cli.db.clone())?;
                let data = commands::route::resolve_show_for_tui(&con, *route_id)?;
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
                let con = open_db_migrating(cli.db.clone())?;
                let s = sw_galaxy_map_core::db::queries::galaxy_stats(&con, *top)?;
                let mut out = tui_default_output();
                build_galaxy_stats_tui(&s, *top, &mut out);
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

fn split_args(line: &str) -> Result<Vec<String>> {
    Ok(shell_words::split(line)?)
}

fn open_db_raw(db_arg: Option<String>) -> Result<rusqlite::Connection> {
    let db_path = resolve_db_path(db_arg)?;
    ensure_db_ready(&db_path)?;
    sw_galaxy_map_core::db::open_db(&db_path.to_string_lossy())
}

fn open_db_migrating(db_arg: Option<String>) -> Result<rusqlite::Connection> {
    let mut con = open_db_raw(db_arg)?;
    let _ = sw_galaxy_map_core::db::migrate::run(&mut con, false, false)?;
    Ok(con)
}

fn ensure_db_ready(db_path: &Path) -> Result<()> {
    if db_path.exists() {
        return Ok(());
    }

    println!();
    warning(format!(
        "Local database not found at: {}\nInitializing it now (this may take a moment)...",
        db_path.display()
    ));

    let report =
        sw_galaxy_map_core::db::db_init::run(Some(db_path.to_string_lossy().to_string()), false)?;
    print_db_init_report(&report);
    Ok(())
}

pub fn route_eta_text(con: &rusqlite::Connection, loaded: &RouteLoaded) -> Option<String> {
    route_eta(con, loaded).map(|e| e.format_human())
}

pub fn route_eta(con: &rusqlite::Connection, loaded: &RouteLoaded) -> Option<RouteEtaEstimate> {
    estimate_route_eta(
        con,
        loaded,
        ETA_HYPERDRIVE_CLASS,
        ETA_REGION_BLEND,
        ETA_DETOUR_COUNT_BASE,
        ETA_SEVERITY_K,
    )
}

fn region_name(r: sw_galaxy_map_core::routing::hyperspace::GalacticRegion) -> &'static str {
    match r {
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::DeepCore => "Deep Core",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::CoreWorlds => "Core Worlds",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::Colonies => "Colonies",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::InnerRim => "Inner Rim",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::ExpansionRegion => {
            "Expansion Region"
        }
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::MidRim => "Mid Rim",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::HuttSpace => "Hutt Space",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::OuterRim => "Outer Rim",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::WildSpace => "Wild Space",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::UnknownRegions => {
            "Unknown Regions"
        }
    }
}

// ---------------------------------------------------------------------------
// Galaxy statistics rendering (v0.15.0)
// ---------------------------------------------------------------------------

fn pct(n: i64, total: i64) -> String {
    if total > 0 {
        format!("{:5.1}%", n as f64 / total as f64 * 100.0)
    } else {
        "    -".to_string()
    }
}

fn print_galaxy_stats(s: &sw_galaxy_map_core::model::GalaxyStats, top: usize) {
    println!("Galaxy Statistics");
    println!("=================");
    println!();
    println!("Planets: {} total", s.total_planets);
    println!();
    println!("  By status:");
    println!(
        "    active     : {:>6}  ({})",
        s.status_active,
        pct(s.status_active, s.total_planets)
    );
    println!(
        "    inserted   : {:>6}  ({})",
        s.status_inserted,
        pct(s.status_inserted, s.total_planets)
    );
    println!(
        "    modified   : {:>6}  ({})",
        s.status_modified,
        pct(s.status_modified, s.total_planets)
    );
    println!(
        "    skipped    : {:>6}  ({})",
        s.status_skipped,
        pct(s.status_skipped, s.total_planets)
    );
    println!(
        "    deleted    : {:>6}  ({})",
        s.status_deleted,
        pct(s.status_deleted, s.total_planets)
    );
    println!(
        "    invalid    : {:>6}  ({})",
        s.status_invalid,
        pct(s.status_invalid, s.total_planets)
    );
    if s.status_null > 0 {
        println!(
            "    (no status): {:>6}  ({})",
            s.status_null,
            pct(s.status_null, s.total_planets)
        );
    }

    let active_total = s.total_planets - s.status_deleted - s.status_skipped - s.status_invalid;

    println!();
    println!("  Canon / Legends (active planets only):");
    println!(
        "    Canon      : {:>6}  ({})",
        s.canon_count,
        pct(s.canon_count, active_total)
    );
    println!(
        "    Legends    : {:>6}  ({})",
        s.legends_count,
        pct(s.legends_count, active_total)
    );
    println!(
        "    Both       : {:>6}  ({})",
        s.both_count,
        pct(s.both_count, active_total)
    );
    println!(
        "    Neither    : {:>6}  ({})",
        s.neither_count,
        pct(s.neither_count, active_total)
    );

    if !s.top_regions.is_empty() {
        println!();
        println!("  Top {} regions:", top);
        let name_w = s
            .top_regions
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_regions.iter().enumerate() {
            println!(
                "    {:>2}. {:<name_w$} : {:>5}  ({})",
                i + 1,
                name,
                cnt,
                pct(*cnt, active_total)
            );
        }
    }

    if !s.top_sectors.is_empty() {
        println!();
        println!("  Top {} sectors:", top);
        let name_w = s
            .top_sectors
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_sectors.iter().enumerate() {
            println!("    {:>2}. {:<name_w$} : {:>5}", i + 1, name, cnt);
        }
    }

    println!();
    println!("  Grid coverage: {} distinct cells", s.distinct_grids);
    if !s.top_grids.is_empty() {
        let top_str: Vec<String> = s
            .top_grids
            .iter()
            .take(5)
            .map(|(g, c)| format!("{} ({})", g, c))
            .collect();
        println!("    Most populated: {}", top_str.join(", "));
    }

    if s.total_routes > 0 {
        println!();
        println!("Routes: {}", s.total_routes);
        println!("  ok       : {}", s.routes_ok);
        println!("  failed   : {}", s.routes_failed);
        if s.total_route_length > 0.0 {
            println!("  Total length : {:.1} parsec", s.total_route_length);
        }
        if s.avg_detours_per_route > 0.0 {
            println!("  Avg detours  : {:.1} per route", s.avg_detours_per_route);
        }
    }
}

fn build_galaxy_stats_tui(
    s: &sw_galaxy_map_core::model::GalaxyStats,
    top: usize,
    out: &mut TuiCommandOutput,
) {
    let l = &mut out.log_lines;

    l.push("Galaxy Statistics".to_string());
    l.push("=================".to_string());
    l.push(String::new());
    l.push(format!("Planets: {} total", s.total_planets));
    l.push(String::new());

    l.push("  By status:".to_string());
    l.push(format!(
        "    active     : {:>6}  ({})",
        s.status_active,
        pct(s.status_active, s.total_planets)
    ));
    l.push(format!(
        "    inserted   : {:>6}  ({})",
        s.status_inserted,
        pct(s.status_inserted, s.total_planets)
    ));
    l.push(format!(
        "    modified   : {:>6}  ({})",
        s.status_modified,
        pct(s.status_modified, s.total_planets)
    ));
    l.push(format!(
        "    skipped    : {:>6}  ({})",
        s.status_skipped,
        pct(s.status_skipped, s.total_planets)
    ));
    l.push(format!(
        "    deleted    : {:>6}  ({})",
        s.status_deleted,
        pct(s.status_deleted, s.total_planets)
    ));
    l.push(format!(
        "    invalid    : {:>6}  ({})",
        s.status_invalid,
        pct(s.status_invalid, s.total_planets)
    ));
    if s.status_null > 0 {
        l.push(format!(
            "    (no status): {:>6}  ({})",
            s.status_null,
            pct(s.status_null, s.total_planets)
        ));
    }

    let active_total = s.total_planets - s.status_deleted - s.status_skipped - s.status_invalid;

    l.push(String::new());
    l.push("  Canon / Legends (active planets only):".to_string());
    l.push(format!(
        "    Canon      : {:>6}  ({})",
        s.canon_count,
        pct(s.canon_count, active_total)
    ));
    l.push(format!(
        "    Legends    : {:>6}  ({})",
        s.legends_count,
        pct(s.legends_count, active_total)
    ));
    l.push(format!(
        "    Both       : {:>6}  ({})",
        s.both_count,
        pct(s.both_count, active_total)
    ));
    l.push(format!(
        "    Neither    : {:>6}  ({})",
        s.neither_count,
        pct(s.neither_count, active_total)
    ));

    if !s.top_regions.is_empty() {
        l.push(String::new());
        l.push(format!("  Top {} regions:", top));
        let name_w = s
            .top_regions
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_regions.iter().enumerate() {
            l.push(format!(
                "    {:>2}. {:<name_w$} : {:>5}  ({})",
                i + 1,
                name,
                cnt,
                pct(*cnt, active_total)
            ));
        }
    }

    if !s.top_sectors.is_empty() {
        l.push(String::new());
        l.push(format!("  Top {} sectors:", top));
        let name_w = s
            .top_sectors
            .iter()
            .map(|(n, _)| n.len())
            .max()
            .unwrap_or(10)
            .max(10);
        for (i, (name, cnt)) in s.top_sectors.iter().enumerate() {
            l.push(format!("    {:>2}. {:<name_w$} : {:>5}", i + 1, name, cnt));
        }
    }

    l.push(String::new());
    l.push(format!(
        "  Grid coverage: {} distinct cells",
        s.distinct_grids
    ));
    if !s.top_grids.is_empty() {
        let top_str: Vec<String> = s
            .top_grids
            .iter()
            .take(5)
            .map(|(g, c)| format!("{} ({})", g, c))
            .collect();
        l.push(format!("    Most populated: {}", top_str.join(", ")));
    }

    if s.total_routes > 0 {
        l.push(String::new());
        l.push(format!("Routes: {}", s.total_routes));
        l.push(format!("  ok       : {}", s.routes_ok));
        l.push(format!("  failed   : {}", s.routes_failed));
        if s.total_route_length > 0.0 {
            l.push(format!(
                "  Total length : {:.1} parsec",
                s.total_route_length
            ));
        }
        if s.avg_detours_per_route > 0.0 {
            l.push(format!(
                "  Avg detours  : {:.1} per route",
                s.avg_detours_per_route
            ));
        }
    }
}
