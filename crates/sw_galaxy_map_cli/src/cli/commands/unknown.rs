use crate::cli::args::UnknownCmd;
use crate::ui::{info, warning};
use anyhow::Result;
use rusqlite::Connection;
use sw_galaxy_map_core::db::queries::{list_unknown_planets, near_planets_for_unknown_fid};
use sw_galaxy_map_core::validate;

fn col_width<T: AsRef<str>>(items: &[T], min: usize) -> usize {
    items
        .iter()
        .map(|s| s.as_ref().len())
        .max()
        .unwrap_or(min)
        .max(min)
}

fn opt_name(name: &Option<String>) -> &str {
    name.as_deref().unwrap_or("-")
}

fn opt_reason(reason: &Option<String>) -> &str {
    reason.as_deref().unwrap_or("-")
}

fn opt_coord(coord: Option<f64>) -> String {
    coord
        .map(|v| format!("{v:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

/// Runs unknown-planet commands.
pub fn run(con: &Connection, cmd: &UnknownCmd) -> Result<()> {
    match cmd {
        UnknownCmd::List { limit } => run_list(con, *limit),
        UnknownCmd::Search { fid, near, limit } => run_search(con, *fid, *near, *limit),
    }
}

/// Lists rows from `planets_unknown`.
fn run_list(con: &Connection, limit: i64) -> Result<()> {
    validate::validate_limit(limit, "unknown list")?;

    let rows = list_unknown_planets(con, limit)?;
    if rows.is_empty() {
        warning("No rows found in planets_unknown.");
        return Ok(());
    }

    info(format!(
        "Found {} unclassified planet(s) in planets_unknown:",
        rows.len()
    ));
    println!();

    let fid_w = 6usize;
    let name_vals: Vec<&str> = rows.iter().map(|p| opt_name(&p.planet)).collect();
    let x_vals: Vec<String> = rows.iter().map(|p| opt_coord(p.x)).collect();
    let y_vals: Vec<String> = rows.iter().map(|p| opt_coord(p.y)).collect();
    let reason_vals: Vec<&str> = rows.iter().map(|p| opt_reason(&p.reason)).collect();

    let name_w = col_width(&name_vals, "Planet".len().max(12));
    let x_w = col_width(&x_vals, "X (pc)".len());
    let y_w = col_width(&y_vals, "Y (pc)".len());
    let reason_w = col_width(&reason_vals, "Reason".len().max(12));

    println!(
        "{fid:>fid_w$}   {name:<name_w$}  {x:<x_w$}  {y:<y_w$}  {reason:<reason_w$}",
        fid = "FID",
        name = "Planet",
        x = "X (pc)",
        y = "Y (pc)",
        reason = "Reason",
    );
    println!(
        "{:-<fid_w$}   {:-<name_w$}  {:-<x_w$}  {:-<y_w$}  {:-<reason_w$}",
        "", "", "", "", ""
    );

    for row in rows {
        println!(
            "{fid:>fid_w$}   {name:<name_w$}  {x:>x_w$}  {y:>y_w$}  {reason:<reason_w$}",
            fid = row.fid,
            name = opt_name(&row.planet),
            x = opt_coord(row.x),
            y = opt_coord(row.y),
            reason = opt_reason(&row.reason),
        );
    }

    Ok(())
}

/// Searches known planets near an unknown planet using squared-distance SQL.
fn run_search(con: &Connection, fid: i64, near: f64, limit: i64) -> Result<()> {
    if near <= 0.0 {
        anyhow::bail!("--near must be greater than 0.");
    }
    validate::validate_limit(limit, "unknown search")?;

    let (unknown, rows) = near_planets_for_unknown_fid(con, fid, near, limit)?;
    let origin_name = unknown
        .planet
        .clone()
        .unwrap_or_else(|| format!("(unknown fid {fid})"));
    let origin_x = unknown
        .x
        .ok_or_else(|| anyhow::anyhow!("Missing X coordinate for fid {fid}."))?;
    let origin_y = unknown
        .y
        .ok_or_else(|| anyhow::anyhow!("Missing Y coordinate for fid {fid}."))?;

    println!(
        "Origin: {} (FID={}, X={:.3}, Y={:.3})",
        origin_name, fid, origin_x, origin_y
    );
    println!("Radius: {:.3} parsecs", near);
    println!("Limit: {}", limit);
    println!();

    if rows.is_empty() {
        warning(format!(
            "No known planets found within {:.3} parsecs of unknown FID {}.",
            near, fid
        ));
        return Ok(());
    }

    info(format!(
        "Found {} known planet(s) near unknown FID {}:",
        rows.len(),
        fid
    ));
    println!();

    let fid_w = 6usize;
    let name_vals: Vec<&str> = rows.iter().map(|p| p.planet.as_str()).collect();
    let x_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.x)).collect();
    let y_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.y)).collect();
    let d_vals: Vec<String> = rows.iter().map(|p| format!("{:.3}", p.distance)).collect();

    let name_w = col_width(&name_vals, "Planet".len().max(10));
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

    for row in rows {
        println!(
            "{fid:>fid_w$}   {name:<name_w$}  {x:>x_w$}  {y:>y_w$}  {d:>d_w$}",
            fid = row.fid,
            name = row.planet,
            x = format!("{:.3}", row.x),
            y = format!("{:.3}", row.y),
            d = format!("{:.3}", row.distance),
        );
    }

    Ok(())
}
