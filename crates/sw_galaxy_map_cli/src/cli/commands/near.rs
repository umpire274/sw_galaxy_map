use crate::ui::{info, warning};
use anyhow::Result;
use rusqlite::Connection;
use sw_galaxy_map_core::db::queries::{
    find_planet_for_info, get_unknown_planet_by_fid, near_planets, near_planets_excluding_fid,
};
use sw_galaxy_map_core::model::{NearHit, PlanetSearchRow};
use sw_galaxy_map_core::utils::normalize_text;

fn col_width<T: AsRef<str>>(items: &[T], min: usize) -> usize {
    items
        .iter()
        .map(|s| s.as_ref().len())
        .max()
        .unwrap_or(min)
        .max(min)
}

/// Hint shown to users about negative number parsing by clap.
fn print_negative_hint() {
    println!("Tip: for negative coordinates use --x=-190 / --y=-190 (with '=')\n");
}

#[derive(Debug, Clone)]
pub(crate) enum NearReference {
    Planet(PlanetSearchRow),
    Coordinates { x: f64, y: f64 },
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve(
    con: &Connection,
    range: f64,
    unknown: bool,
    fid: Option<i64>,
    planet: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    limit: i64,
) -> Result<(NearReference, Vec<NearHit>)> {
    if unknown {
        let fid = fid.ok_or_else(|| anyhow::anyhow!("--fid is required with --unknown"))?;
        let unknown_planet = get_unknown_planet_by_fid(con, fid)?
            .ok_or_else(|| anyhow::anyhow!("No unknown planet found for fid {}", fid))?;
        let x = unknown_planet.x;
        let y = unknown_planet.y;
        let name = unknown_planet.planet;

        let (origin_x, origin_y) = match (x, y) {
            (Some(x), Some(y)) => (x, y),
            _ => anyhow::bail!("Center planet has no coordinates (x/y)."),
        };

        let reference_fid = unknown_planet.fid.unwrap_or(fid);

        let reference = NearReference::Planet(PlanetSearchRow {
            fid: reference_fid,
            name,
            region: None,
            sector: None,
            system: None,
            grid: None,
            x: origin_x,
            y: origin_y,
            canon: false,
            legends: false,
        });

        let rows = near_planets(con, origin_x, origin_y, range, limit)?;
        return Ok((reference, rows));
    }

    if let Some(planet_name) = planet {
        let pn = normalize_text(&planet_name);
        let p = match find_planet_for_info(con, &pn)? {
            Some(p) => p,
            None => anyhow::bail!("No planet found matching '{}'", planet_name),
        };

        let reference = NearReference::Planet(PlanetSearchRow {
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
        });

        let rows = near_planets_excluding_fid(con, p.fid, p.x, p.y, range, limit)?;
        return Ok((reference, rows));
    }

    let x = x.ok_or_else(|| {
        anyhow::anyhow!(
            "You must specify --x if --planet is not used.\n\
             Tip: for negative coordinates use --x=-190 (with '=')"
        )
    })?;
    let y = y.ok_or_else(|| {
        anyhow::anyhow!(
            "You must specify --y if --planet is not used.\n\
             Tip: for negative coordinates use --y=-190 (with '=')"
        )
    })?;

    let reference = NearReference::Coordinates { x, y };
    let rows = near_planets(con, x, y, range, limit)?;
    Ok((reference, rows))
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    con: &Connection,
    r: f64,
    unknown: bool,
    fid: Option<i64>,
    planet: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    limit: i64,
) -> Result<()> {
    let (reference, rows) = resolve(con, r, unknown, fid, planet, x, y, limit)?;

    match &reference {
        NearReference::Planet(p) => {
            println!("Center: {} (X={:.3}, Y={:.3})", p.name, p.x, p.y);
        }
        NearReference::Coordinates { x, y } => {
            println!("Center: (X={:.3}, Y={:.3})", x, y);
        }
    }
    println!("Radius: {:.3} parsecs", r);
    println!("Limit: {}", limit);
    println!();

    if rows.is_empty() {
        warning(format!(
            "No planets found within a radius of {:.3} parsecs.",
            r
        ));
        print_negative_hint();
        return Ok(());
    }

    info(format!(
        "Found the following planets within {:.3} parsecs:",
        r
    ));
    println!();

    let fid_w: usize = 6;

    let name_vals: Vec<&str> = rows.iter().map(|p| p.planet.as_str()).collect();
    let name_w = col_width(&name_vals, "Planet".len().max(10));

    let x_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.x)).collect();
    let y_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.y)).collect();
    let d_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.distance)).collect();

    let x_w = col_width(&x_vals, "X (pc)".len());
    let y_w = col_width(&y_vals, "Y (pc)".len());
    let d_w = col_width(&d_vals, "Distance (pc)".len());

    println!(
        "{fid:>fid_w$}   {name:<name_w$}  {x:<x_w$}  {y:<y_w$}  {d:<d_w$}",
        fid = "FID",
        name = "Planet",
        x = "X (pc)",
        y = "Y (pc)",
        d = "Distance (pc)",
    );
    println!(
        "{:-<fid_w$}   {:-<name_w$}  {:-<x_w$}  {:-<y_w$}  {:-<d_w$}",
        "", "", "", "", ""
    );

    for p in rows {
        println!(
            "{fid:>fid_w$}   {name:<name_w$}  {x:>x_w$}  {y:>y_w$}  {d:>d_w$}",
            fid = p.fid,
            name = p.planet,
            x = format!("{:.3}", p.x),
            y = format!("{:.3}", p.y),
            d = format!("{:.3}", p.distance),
        );
    }

    Ok(())
}
