//! Entry point for the sw_galaxy_map_edit binary.

mod audit;
mod cli;
mod commands;
mod db;
mod edit;
mod interactive;
mod output;
mod resolve;
mod validate;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::EditCli::parse();
    commands::run(args)
}
