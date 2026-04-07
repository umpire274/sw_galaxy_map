//! Command-line argument definitions for sw_galaxy_map_edit.

use clap::{Args, Parser, Subcommand};

/// Command-line editor for maintaining planet records in sw_galaxy_map.
#[derive(Debug, Parser)]
#[command(
    name = "sw_galaxy_map_edit",
    version,
    about = "Edit and review planet records stored in the local sw_galaxy_map database"
)]
pub struct EditCli {
    #[command(subcommand)]
    pub command: Option<EditCommand>,
}

#[derive(Debug, Subcommand)]
pub enum EditCommand {
    /// Find a planet by name, alias, or FID.
    Find(FindArgs),

    /// Open a planet editing session.
    Edit(EditArgs),

    /// Show edit history for a planet.
    History(HistoryArgs),
}

#[derive(Debug, Args)]
pub struct FindArgs {
    /// Planet name, alias, or numeric FID.
    pub query: String,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    /// Planet FID.
    #[arg(long)]
    pub fid: Option<i64>,

    /// Exact planet name.
    #[arg(long)]
    pub planet: Option<String>,

    /// Start the guided interactive editor.
    #[arg(long)]
    pub interactive: bool,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Planet FID.
    #[arg(long)]
    pub fid: Option<i64>,

    /// Exact planet name or alias.
    #[arg(long)]
    pub planet: Option<String>,

    /// Maximum number of history rows to display.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}