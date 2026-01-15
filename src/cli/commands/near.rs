use crate::ui::{info, warning};
use crate::{db, normalize::normalize_text};
use anyhow::Result;
use rusqlite::Connection;

pub fn run(
    con: &Connection,
    r: f64,
    planet: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    limit: i64,
) -> Result<()> {
    let rows = if let Some(planet_name) = planet {
        let pn = normalize_text(&planet_name);
        let p = db::get_planet_by_norm(con, &pn)?;

        println!(
            "Center: {} (X={}, Y={}), radius={} parsecs",
            p.planet, p.x, p.y, r
        );

        db::near_planets_excluding_fid(con, p.fid, p.x, p.y, r, limit)?
    } else {
        let x = x.ok_or_else(|| anyhow::anyhow!("You must specify --x if --planet is not used"))?;
        let y = y.ok_or_else(|| anyhow::anyhow!("You must specify --y if --planet is not used"))?;
        db::near_planets(con, x, y, r, limit)?
    };

    if rows.is_empty() {
        warning(format!(
            "No planets found within a radius of {} parsecs.",
            r
        ));
    } else {
        println!();
        info(format!("Found the following planets around {} parsecs:", r));
        println!();
        println!("FID\tPlanet\tX\tY\tDistance(parsecs)");
        for hit in rows {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                hit.fid, hit.planet, hit.x, hit.y, hit.distance
            );
        }
    }

    Ok(())
}
