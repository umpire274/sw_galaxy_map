use crate::ui::info;
use anyhow::Result;
use rusqlite::Connection;
use sw_galaxy_map_core::db::queries::{find_planet_for_info, get_aliases};
use sw_galaxy_map_core::model::PlanetSearchRow;
use sw_galaxy_map_core::utils::normalize_text;

const LABEL_W: usize = 24;

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "-".into())
}

fn opt_str(v: Option<&str>) -> &str {
    v.unwrap_or("-")
}

pub(crate) fn resolve(con: &Connection, planet: &str) -> Result<(PlanetSearchRow, Vec<String>)> {
    let pn = normalize_text(planet);
    let p = match find_planet_for_info(con, &pn)? {
        Some(p) => p,
        None => anyhow::bail!("No planet found matching '{}'", planet),
    };

    let aliases = get_aliases(con, p.fid)?
        .into_iter()
        .map(|a| {
            let src = a.source.unwrap_or_else(|| "unknown".to_string());
            format!("{} ({})", a.alias, src)
        })
        .collect();

    let row = PlanetSearchRow {
        fid: p.fid,
        name: p.planet,
        region: p.region,
        sector: p.sector,
        system: p.system,
        grid: p.grid,
        x: p.x,
        y: p.y,
        canon: p.canon.is_some(),
        legends: p.legends.is_some(),
    };

    Ok((row, aliases))
}

pub(crate) fn resolve_by_fid(con: &Connection, fid: i64) -> Result<(PlanetSearchRow, Vec<String>)> {
    let p = sw_galaxy_map_core::db::queries::get_planet_by_fid(con, fid)?
        .ok_or_else(|| anyhow::anyhow!("No planet found with fid {}", fid))?;

    let aliases = get_aliases(con, p.fid)?
        .into_iter()
        .map(|a| {
            let src = a.source.unwrap_or_else(|| "unknown".to_string());
            format!("{} ({})", a.alias, src)
        })
        .collect();

    let row = PlanetSearchRow {
        fid: p.fid,
        name: p.planet,
        region: p.region,
        sector: p.sector,
        system: p.system,
        grid: p.grid,
        x: p.x,
        y: p.y,
        canon: p.canon.is_some(),
        legends: p.legends.is_some(),
    };

    Ok((row, aliases))
}

pub fn run(con: &Connection, planet: String) -> Result<()> {
    let pn = normalize_text(&planet);
    let p = match find_planet_for_info(con, &pn)? {
        Some(p) => p,
        None => anyhow::bail!("No planet found matching '{}'", planet),
    };

    let aliases = get_aliases(con, p.fid)?;

    info("Planet Information");
    println!();

    println!("{:<LABEL_W$}: {}", "FID", p.fid);
    println!("{:<LABEL_W$}: {}", "Planet", p.planet);
    println!("{:<LABEL_W$}: {}", "planet_norm", p.planet_norm);

    println!("{:<LABEL_W$}: {}", "Region", opt_str(p.region.as_deref()));
    println!("{:<LABEL_W$}: {}", "Sector", opt_str(p.sector.as_deref()));
    println!("{:<LABEL_W$}: {}", "System", opt_str(p.system.as_deref()));
    println!("{:<LABEL_W$}: {}", "Grid", opt_str(p.grid.as_deref()));

    println!("{:<LABEL_W$}: {}", "X (parsecs)", p.x);
    println!("{:<LABEL_W$}: {}", "Y (parsecs)", p.y);

    println!("{:<LABEL_W$}: {}", "Canon", opt(p.canon));
    println!("{:<LABEL_W$}: {}", "Legends", opt(p.legends));
    println!("{:<LABEL_W$}: {}", "zm", opt(p.zm));
    println!("{:<LABEL_W$}: {}", "Latitude", opt(p.lat));
    println!("{:<LABEL_W$}: {}", "Longitude", opt(p.long));

    println!("{:<LABEL_W$}: {}", "Status", opt_str(p.status.as_deref()));
    println!(
        "{:<LABEL_W$}: {}",
        "Reference",
        opt_str(p.reference.as_deref())
    );
    println!(
        "{:<LABEL_W$}: {}",
        "Canonical Region",
        opt_str(p.c_region.as_deref())
    );
    println!(
        "{:<LABEL_W$}: {}",
        "Canonical Region (long)",
        opt_str(p.c_region_li.as_deref())
    );

    let label_w_new = LABEL_W - 3;
    println!();
    println!("Name aliases:");
    println!(
        "{:>2} {:<label_w_new$}: {}",
        "-",
        "name0",
        opt_str(p.name0.as_deref())
    );
    println!(
        "{:>2} {:<label_w_new$}: {}",
        "-",
        "name1",
        opt_str(p.name1.as_deref())
    );
    println!(
        "{:>2} {:<label_w_new$}: {}",
        "-",
        "name2",
        opt_str(p.name2.as_deref())
    );

    println!();
    if aliases.is_empty() {
        println!("Aliases: -");
    } else {
        println!("Aliases:");
        for a in aliases {
            let src = a.source.as_deref().unwrap_or("unknown");
            println!("  - {:<label_w_new$} ({})", a.alias, src);
        }
    }

    println!();
    println!("{:<LABEL_W$}: {}", "Info URL", p.info_planet_url());

    Ok(())
}
