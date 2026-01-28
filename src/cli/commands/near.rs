use crate::db::queries::{find_planet_for_info, near_planets, near_planets_excluding_fid};
use crate::ui::{info, warning};
use crate::utils::normalize::normalize_text;
use anyhow::Result;
use rusqlite::Connection;

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
    // This is the convention we decided for this project:
    // use `--x=-190` / `--y=-190` for negative values.
    println!("Tip: for negative coordinates use --x=-190 / --y=-190 (with '=')\n");
}

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
        let p = match find_planet_for_info(con, &pn)? {
            Some(p) => p,
            None => anyhow::bail!("No planet found matching '{}'", planet_name),
        };

        println!("Center: {} (X={:.3}, Y={:.3})", p.planet, p.x, p.y);
        println!("Radius: {:.3} parsecs", r);
        println!("Limit: {}", limit);
        println!();

        near_planets_excluding_fid(con, p.fid, p.x, p.y, r, limit)?
    } else {
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

        println!("Center: (X={:.3}, Y={:.3})", x, y);
        println!("Radius: {:.3} parsecs", r);
        println!("Limit: {}", limit);
        println!();

        near_planets(con, x, y, r, limit)?
    };

    if rows.is_empty() {
        warning(format!(
            "No planets found within a radius of {:.3} parsecs.",
            r
        ));
        // Still show the hint, since "near" is where users commonly discover the issue.
        print_negative_hint();
        return Ok(());
    }

    info(format!(
        "Found the following planets within {:.3} parsecs:",
        r
    ));
    println!();

    // --- Column widths (monospace-friendly)
    let fid_w: usize = 6;

    let name_vals: Vec<&str> = rows.iter().map(|p| p.planet.as_str()).collect();
    let name_w = col_width(&name_vals, "Planet".len().max(10));

    let x_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.x)).collect();
    let y_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.y)).collect();
    let d_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.distance)).collect();

    let x_w = col_width(&x_vals, "X (pc)".len());
    let y_w = col_width(&y_vals, "Y (pc)".len());
    let d_w = col_width(&d_vals, "Distance (pc)".len());

    // --- Header
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

    // --- Rows
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
