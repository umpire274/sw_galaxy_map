pub mod args;
pub mod color;
pub mod commands;
pub mod export;
pub mod validate;

use crate::db::db_status::resolve_db_path;
use crate::ui::warning;
use anyhow::Result;
use clap::Parser;
use std::path::Path;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    println!();

    match &cli.cmd {
        args::Commands::Db { cmd } => match cmd {
            args::DbCommands::Init { out, force } => crate::db::db_init::run(out.clone(), *force),

            args::DbCommands::Status => {
                // Se db_status::run usa resolve_db_path e fa solo inspect, va benissimo così.
                crate::db::db_status::run(cli.db.clone())
            }

            args::DbCommands::Update {
                prune,
                dry_run,
                stats,
                stats_limit,
            } => {
                let mut con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
                crate::db::db_update::run(&mut con, *prune, *dry_run, *stats, *stats_limit)
            }

            args::DbCommands::Migrate { dry_run } => {
                // open_db_for_commands NON deve auto-migrare in questo caso
                let mut con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
                crate::db::migrate::run(&mut con, *dry_run, true)
            }
        },

        args::Commands::Search { query, limit } => {
            validate::validate_search(query, *limit)?;
            let con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
            commands::search::run(&con, query.clone(), *limit)
        }

        args::Commands::Info { planet } => {
            let con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
            commands::info::run(&con, planet.clone())
        }

        args::Commands::Near {
            r,
            planet,
            x,
            y,
            limit,
        } => {
            validate::validate_near(planet, x, y)?;
            let con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
            commands::near::run(&con, *r, planet.clone(), *x, *y, *limit)
        }

        args::Commands::Waypoint { cmd } => {
            let con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
            commands::waypoints::run_waypoint(&con, cmd)
        }

        args::Commands::Route { cmd } => {
            // route::run muta perché scrive su DB (compute, clear, prune, ecc.)
            let mut con = open_db_for_commands(cli.db.clone(), &cli.cmd)?;
            commands::route::run(&mut con, cmd)
        }
    }
}

fn open_db_for_commands(
    db_arg: Option<String>,
    cmd: &args::Commands,
) -> Result<rusqlite::Connection> {
    let db_path = resolve_db_path(db_arg)?;
    ensure_db_ready(&db_path)?;

    let mut con = crate::db::open_db(&db_path.to_string_lossy())?;

    let skip_migration = matches!(
        cmd,
        args::Commands::Db {
            cmd: args::DbCommands::Migrate { .. }
        }
    );

    if !skip_migration {
        crate::db::migrate::run(&mut con, false, false)?;
    }

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

    crate::db::db_init::run(Some(db_path.to_string_lossy().to_string()), false)
}
