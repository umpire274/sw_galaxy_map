use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sw_galaxy_map",
    version,
    about = "CLI to query the Star Wars galaxy map (SQLite)"
)]
pub struct Cli {
    /// Path to the SQLite database
    #[arg(long, default_value = "res/sw_planets.sqlite")]
    pub db: String,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search planets by text (uses FTS if available, otherwise LIKE)
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },

    /// Print all available information about a planet
    Info { planet: String },

    /// Find nearby planets within a radius (parsecs) using Euclidean distance on X/Y
    Near {
        /// Radius (parsecs)
        #[arg(long)]
        r: f64,

        /// Center the search around a planet (by name)
        #[arg(long)]
        planet: Option<String>,

        /// X coordinate (parsecs), if --planet is not used
        #[arg(long)]
        x: Option<f64>,

        /// Y coordinate (parsecs), if --planet is not used
        #[arg(long)]
        y: Option<f64>,

        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
}
