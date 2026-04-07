pub mod args;
pub mod color;
pub mod commands;
pub(crate) mod db_runtime;
pub(crate) mod dispatch;
pub mod export;
pub(crate) mod reports;
pub(crate) mod shell;
pub mod typewriter;

pub(crate) use crate::cli::db_runtime::{open_db_migrating, open_db_raw};
use crate::cli::dispatch::run_one_shot;
pub(crate) use crate::cli::reports::{
    print_db_init_report, print_db_status_report, print_db_update_report, print_galaxy_stats,
    print_migration_report,
};
use crate::cli::shell::run_interactive_shell;
use anyhow::Result;
use clap::Parser;

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
