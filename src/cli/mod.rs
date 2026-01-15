pub mod args;
pub mod commands;

use crate::provision::db_status::resolve_db_path;
use crate::ui::warning;
use anyhow::Result;
use clap::Parser;
use std::path::Path;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    println!();

    match cli.cmd {
        args::Commands::Db { cmd } => match cmd {
            args::DbCommands::Init { out, force } => crate::provision::db_init::run(out, force),
            args::DbCommands::Status => crate::provision::db_status::run(cli.db),
        },

        args::Commands::Search { query, limit } => {
            let con = open_db_for_commands(cli.db)?;
            commands::search::run(&con, query, limit)
        }

        args::Commands::Info { planet } => {
            let con = open_db_for_commands(cli.db)?;
            commands::info::run(&con, planet)
        }

        args::Commands::Near {
            r,
            planet,
            x,
            y,
            limit,
        } => {
            let con = open_db_for_commands(cli.db)?;
            commands::near::run(&con, r, planet, x, y, limit)
        }
    }
}

fn open_db_for_commands(db_arg: Option<String>) -> Result<rusqlite::Connection> {
    let db_path = resolve_db_path(db_arg)?;
    ensure_db_ready(&db_path)?;

    crate::db::open_db(&db_path.to_string_lossy())
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

    crate::provision::db_init::run(Some(db_path.to_string_lossy().to_string()), false)
}
