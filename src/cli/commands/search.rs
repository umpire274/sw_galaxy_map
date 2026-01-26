use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries::search_planets;
use crate::normalize::normalize_text;
use crate::ui::warning;

/// Return `-` if the value is None or empty/whitespace.
fn cell(opt: &Option<String>) -> &str {
    match opt.as_deref() {
        Some(s) if !s.trim().is_empty() => s,
        _ => "-",
    }
}

fn col_width_from_strs(items: &[&str], min: usize) -> usize {
    items.iter().map(|s| s.len()).max().unwrap_or(min).max(min)
}

pub fn run(con: &Connection, query: String, limit: i64) -> Result<()> {
    let qn = normalize_text(&query);
    let rows = search_planets(con, &qn, limit)?;

    if rows.is_empty() {
        warning(format!("No results found for: {query}"));
        return Ok(());
    }

    // --- Compute column widths (monospace-friendly)
    // Keep FID width stable; others adapt to content with sensible minimums.
    let fid_w: usize = 6;

    let name_vals: Vec<&str> = rows.iter().map(|p| p.name.as_str()).collect();
    let region_vals: Vec<&str> = rows.iter().map(|p| cell(&p.region)).collect();
    let sector_vals: Vec<&str> = rows.iter().map(|p| cell(&p.sector)).collect();
    let system_vals: Vec<&str> = rows.iter().map(|p| cell(&p.system)).collect();
    let grid_vals: Vec<&str> = rows.iter().map(|p| cell(&p.grid)).collect();

    let name_w = col_width_from_strs(&name_vals, "Planet".len().max(12));
    let region_w = col_width_from_strs(&region_vals, "Region".len().max(10));
    let sector_w = col_width_from_strs(&sector_vals, "Sector".len().max(10));
    let system_w = col_width_from_strs(&system_vals, "System".len().max(10));
    let grid_w = col_width_from_strs(&grid_vals, "Grid".len().max(6));

    // --- Header
    println!(
        "{fid:>fid_w$}   {name:<name_w$}  {region:<region_w$}  {sector:<sector_w$}  {system:<system_w$}  {grid:<grid_w$}",
        fid = "FID",
        name = "Planet",
        region = "Region",
        sector = "Sector",
        system = "System",
        grid = "Grid",
    );

    println!(
        "{:-<fid_w$}   {:-<name_w$}  {:-<region_w$}  {:-<sector_w$}  {:-<system_w$}  {:-<grid_w$}",
        "", "", "", "", "", ""
    );

    // --- Rows
    for p in &rows {
        println!(
            "{fid:>fid_w$}   {name:<name_w$}  {region:<region_w$}  {sector:<sector_w$}  {system:<system_w$}  {grid:<grid_w$}",
            fid = p.fid,
            name = p.name,
            region = cell(&p.region),
            sector = cell(&p.sector),
            system = cell(&p.system),
            grid = cell(&p.grid),
        );
    }

    Ok(())
}
