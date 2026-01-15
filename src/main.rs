use anyhow::Result;
use clap::{Parser, Subcommand};

mod db;
mod model;
mod normalize;

use db::*;
use normalize::normalize_text;

#[derive(Parser)]
#[command(
    name = "sw_galaxy_map",
    version,
    about = "CLI to query the Star Wars galaxy map (SQLite)"
)]
struct Cli {
    /// Path to the SQLite database
    #[arg(long, default_value = "res/sw_planets.sqlite")]
    db: String,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let con = open_db(&cli.db)?;

    println!();
    match cli.cmd {
        Commands::Search { query, limit } => {
            let qn = normalize_text(&query);
            let rows = search_planets(&con, &qn, limit)?;
            if rows.is_empty() {
                println!("No results found for: {query}");
            } else {
                for (fid, name) in rows {
                    println!("{fid}\t{name}");
                }
            }
        }

        Commands::Info { planet } => {
            let pn = normalize_text(&planet);
            let p = get_planet_by_norm(&con, &pn)?;
            let aliases = get_aliases(&con, p.fid)?;

            println!("FID: {}", p.fid);
            println!("Planet: {}", p.planet);
            println!("Region: {}", p.region.as_deref().unwrap_or("-"));
            println!("Sector: {}", p.sector.as_deref().unwrap_or("-"));
            println!("System: {}", p.system.as_deref().unwrap_or("-"));
            println!("Grid: {}", p.grid.as_deref().unwrap_or("-"));
            println!("X (parsec): {}", p.x);
            println!("Y (parsec): {}", p.y);
            println!(
                "Canon: {}",
                p.canon.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
            );
            println!(
                "Legends: {}",
                p.legends
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".into())
            );
            println!(
                "zm: {}",
                p.zm.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
            );
            println!(
                "lat: {}",
                p.lat.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
            );
            println!(
                "long: {}",
                p.long.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
            );
            println!("status: {}", p.status.as_deref().unwrap_or("-"));
            println!("ref: {}", p.reference.as_deref().unwrap_or("-"));
            println!("CRegion: {}", p.c_region.as_deref().unwrap_or("-"));
            println!("CRegion_li: {}", p.c_region_li.as_deref().unwrap_or("-"));

            if aliases.is_empty() {
                println!("Aliases: -");
            } else {
                println!("Aliases:");
                for a in aliases {
                    let src = a.source.unwrap_or_else(|| "unknown".to_string());
                    println!("  - {} ({})", a.alias, src);
                }
            }
            println!("planet_norm: {}", p.planet_norm);
            println!("name0: {}", p.name0.as_deref().unwrap_or("-"));
            println!("name1: {}", p.name1.as_deref().unwrap_or("-"));
            println!("name2: {}", p.name2.as_deref().unwrap_or("-"));
        }

        Commands::Near {
            r,
            planet,
            x,
            y,
            limit,
        } => {
            let rows = if let Some(planet_name) = planet {
                let pn = normalize_text(&planet_name);
                let p = get_planet_by_norm(&con, &pn)?;

                // >>> ADD THESE TWO LINES <<<
                println!(
                    "Center: {} (X={}, Y={}), radius={} parsecs",
                    p.planet, p.x, p.y, r
                );
                println!();

                near_planets_excluding_fid(&con, p.fid, p.x, p.y, r, limit)?
            } else {
                let x = x.expect("You must specify --x if --planet is not used");
                let y = y.expect("You must specify --y if --planet is not used");
                near_planets(&con, x, y, r, limit)?
            };

            if rows.is_empty() {
                println!("No planets found within a radius of {} parsecs.", r);
            } else {
                println!("FID\tPlanet\tX\tY\tDistance(parsecs)");
                for hit in rows {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        hit.fid, hit.planet, hit.x, hit.y, hit.distance
                    );
                }
            }
        }
    }

    Ok(())
}
