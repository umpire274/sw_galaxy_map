pub mod args;
pub mod color;
pub mod commands;
pub mod export;
pub mod tui;

use crate::ui::{error, info, success, warning};
use anyhow::Result;
use clap::Parser;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::path::Path;
use sw_galaxy_map_core::db::db_status::{DbHealth, DbStatusReport, resolve_db_path};
use sw_galaxy_map_core::db::db_update::{ChangeKind, DbUpdateReport};
use sw_galaxy_map_core::db::migrate::MigrationReport;
use sw_galaxy_map_core::db::queries::search_planets;
use sw_galaxy_map_core::model::{NearHit, PlanetSearchRow};
use sw_galaxy_map_core::utils::normalize_text;
use sw_galaxy_map_core::validate;

#[derive(Debug, Clone)]
pub(crate) struct TuiCommandOutput {
    pub log_lines: Vec<String>,
    pub planet1_title: Line<'static>,
    pub planet1_lines: Vec<String>,
    pub planet2_title: Line<'static>,
    pub planet2_lines: Vec<String>,
    pub search_results: Vec<PlanetSearchRow>,
    pub near_results: Vec<NearHit>,
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
    TuiCommandOutput {
        log_lines: Vec::new(),
        planet1_title: Line::from("Planet 1 Information"),
        planet1_lines: vec!["No data".to_string()],
        planet2_title: Line::from("Planet 2 Information"),
        planet2_lines: vec!["No data".to_string()],
        search_results: Vec::new(),
        near_results: Vec::new(),
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
        },

        args::Commands::Search { query, limit } => {
            validate::validate_search(query, *limit)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::search::run(&con, query.clone(), *limit)
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
        format!("Region: {}", tui_cell(&p.region)),
        format!("Sector: {}", tui_cell(&p.sector)),
        format!("System: {}", tui_cell(&p.system)),
        format!("Grid: {}", tui_cell(&p.grid)),
        format!("X: {:.2}", p.x),
        format!("Y: {:.2}", p.y),
        format!("Canon: {}", if p.canon { "Yes" } else { "No" }),
        format!("Legends: {}", if p.legends { "Yes" } else { "No" }),
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
    distance: f64,
    aliases: Option<&[String]>,
) -> (Line<'static>, Vec<String>) {
    let title = build_planet_title(planet);

    let mut lines = vec![
        format!("Distance: {:.2} pc", distance),
        format!("Region: {}", tui_cell(&planet.region)),
        format!("Sector: {}", tui_cell(&planet.sector)),
        format!("System: {}", tui_cell(&planet.system)),
        format!("Grid: {}", tui_cell(&planet.grid)),
        format!("X: {:.2}", planet.x),
        format!("Y: {:.2}", planet.y),
        format!("Canon: {}", if planet.canon { "Yes" } else { "No" }),
        format!("Legends: {}", if planet.legends { "Yes" } else { "No" }),
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

pub(crate) fn run_one_shot_for_tui(
    cli: &args::Cli,
    cmd: &args::Commands,
) -> Result<TuiCommandOutput> {
    match cmd {
        args::Commands::Search { query, limit } => {
            validate::validate_search(query, *limit)?;
            let con = open_db_migrating(cli.db.clone())?;
            let qn = normalize_text(query);
            let rows = search_planets(&con, &qn, *limit)?;

            let mut out = tui_default_output();

            if rows.is_empty() {
                out.log_lines
                    .push(format!("Search result for \"{}\": no planets found", query));
                return Ok(out);
            }

            if rows.len() == 1 {
                let planet = &rows[0];
                let (title, lines) = build_planet_panel(planet, None);

                out.log_lines
                    .push(format!("Search result for \"{}\": 1 planet found", query));
                out.log_lines
                    .push(format!("Displaying result: {}", planet.name));

                out.planet1_title = title;
                out.planet1_lines = lines;

                return Ok(out);
            }

            out.log_lines.push(format!(
                "Search result for \"{}\": {} planets found",
                query,
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
                let (title2, lines2) =
                    build_near_planet_panel(&planet, hit.distance, Some(&aliases));

                out.planet2_title = title2;
                out.planet2_lines = lines2;
            } else {
                out.near_results = hits;
            }

            Ok(out)
        }

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
