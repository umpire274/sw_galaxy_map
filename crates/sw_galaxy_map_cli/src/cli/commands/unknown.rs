use crate::cli::args::UnknownCmd;
use crate::ui::{info, warning};
use anyhow::Result;
use rusqlite::Connection;
use sw_galaxy_map_core::db::queries::{
    count_unknown_planets, list_unknown_planets_paginated, near_planets_for_unknown_id,
};
use sw_galaxy_map_core::validate;

fn col_width<T: AsRef<str>>(items: &[T], min: usize) -> usize {
    items
        .iter()
        .map(|s| s.as_ref().len())
        .max()
        .unwrap_or(min)
        .max(min)
}

/// Formats an optional coordinate for CLI output.
fn fmt_coord(coord: Option<f64>) -> String {
    match coord {
        Some(value) => format!("{value:>10.2}"),
        None => format!("{:>10}", "-"),
    }
}

/// Runs unknown-planet commands.
pub fn run(con: &Connection, cmd: &UnknownCmd) -> Result<()> {
    match cmd {
        UnknownCmd::List { page, page_size } => {
            if *page == 0 {
                anyhow::bail!("--page must be greater than 0");
            }

            if *page_size == 0 {
                anyhow::bail!("--page-size must be greater than 0");
            }

            let total = count_unknown_planets(con)?;

            if total == 0 {
                println!("No unknown planets found.");
                return Ok(());
            }

            let total_pages = if *page_size == 0 {
                0
            } else {
                (total + (*page_size as i64) - 1) / (*page_size as i64)
            };
            let planets = list_unknown_planets_paginated(con, *page, *page_size)?;

            if planets.is_empty() {
                println!(
                    "No results for page {}. Total available pages: {}.",
                    page, total_pages
                );
                return Ok(());
            }

            println!(
                "Unknown planets - page {}/{} (page size: {}, total items: {})",
                page, total_pages, page_size, total
            );
            println!();

            for p in planets {
                println!(
                    "#{:>4} | fid={:<6} | {:<30} | x={} | y={} | reviewed={} | promoted={}",
                    p.id,
                    p.fid
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    p.planet,
                    fmt_coord(p.x),
                    fmt_coord(p.y),
                    if p.reviewed != 0 { "yes" } else { "no" },
                    if p.promoted != 0 { "yes" } else { "no" },
                );
            }

            Ok(())
        }
        UnknownCmd::Search { id, near, limit } => run_search(con, *id, *near, *limit),
    }
}

/// Searches known planets near an unknown planet using squared-distance SQL.
fn run_search(con: &Connection, id: i64, near: f64, limit: i64) -> Result<()> {
    if near <= 0.0 {
        anyhow::bail!("--near must be greater than 0.");
    }
    validate::validate_limit(limit, "unknown search")?;

    let (unknown, rows) = near_planets_for_unknown_id(con, id, near, limit)?;
    let origin_fid = unknown
        .fid
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string());

    let (origin_x, origin_y) = match (unknown.x, unknown.y) {
        (Some(x), Some(y)) => (x, y),
        _ => anyhow::bail!(
            "Unknown planet {} has no coordinates (x/y). Cannot perform proximity search.",
            unknown.id
        ),
    };

    println!(
        "Origin: {} (ID={}, FID={}, X={:.3}, Y={:.3})",
        unknown.planet, unknown.id, origin_fid, origin_x, origin_y
    );
    println!("Radius: {:.3} parsecs", near);
    println!("Limit: {}", limit);
    println!();

    if rows.is_empty() {
        warning(format!(
            "No known planets found within {:.3} parsecs of unknown ID {}.",
            near, id
        ));
        return Ok(());
    }

    info(format!(
        "Found {} known planet(s) near unknown ID {}:",
        rows.len(),
        id
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
