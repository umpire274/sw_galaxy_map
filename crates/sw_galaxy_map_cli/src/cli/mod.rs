pub mod args;
pub mod color;
pub mod commands;
pub mod export;

use crate::ui::{error, info, success, warning};
use anyhow::Result;
use clap::Parser;
use std::path::Path;
use sw_galaxy_map_core::db::db_status::{DbHealth, DbStatusReport, resolve_db_path};
use sw_galaxy_map_core::db::db_update::{ChangeKind, DbUpdateReport};
use sw_galaxy_map_core::db::migrate::MigrationReport;
use sw_galaxy_map_core::validate;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    println!();

    // One-shot CLI
    if let Some(cmd) = &cli.cmd {
        return run_one_shot(&cli, cmd);
    }

    // Default: interactive CLI
    run_interactive_shell(cli.db.clone())
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
            r,
            unknown,
            fid,
            planet,
            x,
            y,
            limit,
        } => {
            validate::validate_near(*unknown, fid, planet, x, y)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::near::run(&con, *r, *unknown, *fid, planet.clone(), *x, *y, *limit)
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
    use std::io::{self, Write};

    println!(
        "{}",
        format_args!(
            "Interactive mode ({}). Type 'help' or 'exit'.",
            env!("CARGO_PKG_VERSION").to_string()
        )
    );
    println!(
        "Tip: commands are the same as one-shot CLI (e.g. `search scarif`, `route show 42`).\n"
    );

    // Default DB for the session (user can still pass --db per command)
    let mut session_db = db_arg;

    loop {
        print!("sw_galaxy_map> ");
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Built-in REPL commands (start with ':')
        if let Some(rest) = line.strip_prefix(':') {
            let rest = rest.trim();
            if rest == "exit" || rest == "quit" {
                break;
            }
            if rest == "help" {
                println!("REPL commands:");
                println!("  :help                 Show this help");
                println!("  :exit | :quit         Exit interactive mode");
                println!("  :db <path>            Set default DB path for this session");
                println!("  :db                   Show current default DB");
                println!();
                continue;
            }
            if rest == "db" {
                println!(
                    "Default DB: {}",
                    session_db.as_deref().unwrap_or("<auto-resolved (default)>")
                );
                continue;
            }
            if let Some(path) = rest.strip_prefix("db ") {
                let path = path.trim();
                if path.is_empty() {
                    println!("Usage: :db <path>");
                } else {
                    session_db = Some(path.to_string());
                    println!("Default DB set to: {}", path);
                }
                continue;
            }

            println!("Unknown REPL command: :{}", rest);
            continue;
        }

        if line == "exit" || line == "quit" {
            break;
        }
        if line == "help" {
            println!("Try a normal command, e.g.:");
            println!("  search scarif");
            println!("  info coruscant");
            println!("  near --planet coruscant -r 50");
            println!("  route show 42");
            println!("  unknown list");
            println!("Or REPL help: :help\n");
            continue;
        }

        // 1) Split into tokens
        let tokens = match split_args(line) {
            Ok(t) => t,
            Err(e) => {
                println!("Parse error: {:#}", e);
                continue;
            }
        };

        // 2) Build argv for clap: ["sw_galaxy_map", <tokens...>]
        let mut argv: Vec<String> = Vec::with_capacity(tokens.len() + 2);
        argv.push("sw_galaxy_map".to_string());

        // If the user didn't pass --db explicitly, inject session default
        let user_passed_db = tokens.iter().any(|t| t == "--db");
        if !user_passed_db && let Some(ref db) = session_db {
            argv.push("--db".to_string());
            argv.push(db.clone());
        }

        argv.extend(tokens);

        // 3) Parse with clap
        match args::Cli::try_parse_from(argv) {
            Ok(cli) => {
                // No subcommand (should be rare): ignore
                let Some(ref cmd) = cli.cmd else {
                    continue;
                };

                // 4) Execute using the same dispatcher
                if let Err(e) = run_one_shot(&cli, cmd) {
                    println!("Error: {:#}", e);
                }

                println!();
            }
            Err(e) => {
                // Clap pretty error (unknown cmd, wrong args, etc.)
                // Use println so it shows in the REPL output buffer
                println!("{}", e);
                println!();
            }
        }
    }

    Ok(())
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
