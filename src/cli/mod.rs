pub mod args;
pub mod commands;

use anyhow::Result;
use clap::Parser;

use crate::db;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    let con = db::open_db(&cli.db)?;

    match cli.cmd {
        args::Commands::Search { query, limit } => commands::search::run(&con, query, limit),
        args::Commands::Info { planet } => commands::info::run(&con, planet),
        args::Commands::Near {
            r,
            planet,
            x,
            y,
            limit,
        } => commands::near::run(&con, r, planet, x, y, limit),
    }
}
