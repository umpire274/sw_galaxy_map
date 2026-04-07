//! Entry point for the sw_galaxy_map_edit binary.

mod cli;
mod commands;
mod interactive;
mod db;
mod output;
mod resolve;
mod edit;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::EditCli::parse();
    commands::run(args)
}