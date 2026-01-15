use anyhow::Result;
use rusqlite::Connection;

use crate::{db, normalize::normalize_text};

pub fn run(con: &Connection, planet: String) -> Result<()> {
    let pn = normalize_text(&planet);
    let p = db::get_planet_by_norm(con, &pn)?;
    let aliases = db::get_aliases(con, p.fid)?;

    println!("FID: {}", p.fid);
    println!("Planet: {}", p.planet);
    println!("planet_norm: {}", p.planet_norm);
    println!("Region: {}", p.region.as_deref().unwrap_or("-"));
    println!("Sector: {}", p.sector.as_deref().unwrap_or("-"));
    println!("System: {}", p.system.as_deref().unwrap_or("-"));
    println!("Grid: {}", p.grid.as_deref().unwrap_or("-"));
    println!("X (parsecs): {}", p.x);
    println!("Y (parsecs): {}", p.y);
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
        "Latitude: {}",
        p.lat.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
    );
    println!(
        "Longitude: {}",
        p.long.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
    );
    println!("Status: {}", p.status.as_deref().unwrap_or("-"));
    println!("Reference: {}", p.reference.as_deref().unwrap_or("-"));
    println!("Canonical Region: {}", p.c_region.as_deref().unwrap_or("-"));
    println!(
        "Canonical Region (long): {}",
        p.c_region_li.as_deref().unwrap_or("-")
    );

    println!("Name aliases:");
    println!("  name0: {}", p.name0.as_deref().unwrap_or("-"));
    println!("  name1: {}", p.name1.as_deref().unwrap_or("-"));
    println!("  name2: {}", p.name2.as_deref().unwrap_or("-"));

    if aliases.is_empty() {
        println!("Aliases: -");
    } else {
        println!("Aliases:");
        for a in aliases {
            let src = a.source.unwrap_or_else(|| "unknown".to_string());
            println!("  - {} ({})", a.alias, src);
        }
    }

    Ok(())
}
