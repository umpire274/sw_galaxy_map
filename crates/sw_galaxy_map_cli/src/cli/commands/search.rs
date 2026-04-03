use anyhow::Result;
use rusqlite::Connection;

use crate::ui::{info, warning};
use sw_galaxy_map_core::db::queries::{fuzzy_search_filtered, search_planets_filtered};
use sw_galaxy_map_core::model::SearchFilter;
use sw_galaxy_map_core::utils::normalize_text;

/// Max Levenshtein distance for fuzzy matching.
const FUZZY_MAX_DISTANCE: usize = 3;

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

/// Build a human-readable description of the active search criteria.
fn describe_filter(filter: &SearchFilter) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(q) = filter.query.as_deref() {
        parts.push(format!("\"{}\"", q));
    }
    if let Some(r) = filter.region.as_deref() {
        parts.push(format!("region={}", r));
    }
    if let Some(s) = filter.sector.as_deref() {
        parts.push(format!("sector={}", s));
    }
    if let Some(g) = filter.grid.as_deref() {
        parts.push(format!("grid={}", g));
    }
    if let Some(st) = filter.status.as_deref() {
        parts.push(format!("status={}", st));
    }
    if filter.canon == Some(true) {
        parts.push("canon".to_string());
    }
    if filter.legends == Some(true) {
        parts.push("legends".to_string());
    }
    if filter.fuzzy {
        parts.push("fuzzy".to_string());
    }

    if parts.is_empty() {
        "(no criteria)".to_string()
    } else {
        parts.join(", ")
    }
}

fn print_table(rows: &[sw_galaxy_map_core::model::PlanetSearchRow]) {
    let fid_w: usize = 8;

    let name_vals: Vec<&str> = rows.iter().map(|p| p.name.as_str()).collect();
    let region_vals: Vec<&str> = rows.iter().map(|p| cell(&p.region)).collect();
    let sector_vals: Vec<&str> = rows.iter().map(|p| cell(&p.sector)).collect();
    let system_vals: Vec<&str> = rows.iter().map(|p| cell(&p.system)).collect();
    let grid_vals: Vec<&str> = rows.iter().map(|p| cell(&p.grid)).collect();
    let status_vals: Vec<&str> = rows.iter().map(|p| cell(&p.status)).collect();

    let x_vals: Vec<String> = rows.iter().map(|p| format!("{:.2}", p.x)).collect();
    let y_vals: Vec<String> = rows.iter().map(|p| format!("{:.2}", p.y)).collect();

    let x_refs: Vec<&str> = x_vals.iter().map(String::as_str).collect();
    let y_refs: Vec<&str> = y_vals.iter().map(String::as_str).collect();

    let name_w = col_width_from_strs(&name_vals, "Planet".len().max(12));
    let region_w = col_width_from_strs(&region_vals, "Region".len().max(10));
    let sector_w = col_width_from_strs(&sector_vals, "Sector".len().max(10));
    let system_w = col_width_from_strs(&system_vals, "System".len().max(10));
    let grid_w = col_width_from_strs(&grid_vals, "Grid".len().max(6));
    let status_w = col_width_from_strs(&status_vals, "Status".len().max(8));
    let x_w = col_width_from_strs(&x_refs, "X".len().max(8));
    let y_w = col_width_from_strs(&y_refs, "Y".len().max(8));

    println!(
        "{fid:>fid_w$}   {name:<name_w$}  {region:<region_w$}  {sector:<sector_w$}  {system:<system_w$}  {grid:<grid_w$}  {status:<status_w$}  {x:>x_w$}  {y:>y_w$}",
        fid = "FID",
        name = "Planet",
        region = "Region",
        sector = "Sector",
        system = "System",
        grid = "Grid",
        status = "Status",
        x = "X",
        y = "Y",
    );

    println!(
        "{:-<fid_w$}   {:-<name_w$}  {:-<region_w$}  {:-<sector_w$}  {:-<system_w$}  {:-<grid_w$}  {:-<status_w$}  {:-<x_w$}  {:-<y_w$}",
        "", "", "", "", "", "", "", "", ""
    );

    for p in rows {
        println!(
            "{fid:>fid_w$}   {name:<name_w$}  {region:<region_w$}  {sector:<sector_w$}  {system:<system_w$}  {grid:<grid_w$}  {status:<status_w$}  {x:>x_w$}  {y:>y_w$}",
            fid = p.fid,
            name = p.name,
            region = cell(&p.region),
            sector = cell(&p.sector),
            system = cell(&p.system),
            grid = cell(&p.grid),
            status = cell(&p.status),
            x = format!("{:.2}", p.x),
            y = format!("{:.2}", p.y),
        );
    }
}

pub fn run(con: &Connection, filter: SearchFilter) -> Result<()> {
    let description = describe_filter(&filter);

    // --- Explicit fuzzy mode: skip exact search, go straight to fuzzy ---
    if filter.fuzzy {
        let query_text = filter.query.as_deref().unwrap_or("");
        if query_text.trim().is_empty() {
            warning("--fuzzy requires a text query");
            return Ok(());
        }

        let qn = normalize_text(query_text);
        let rows = fuzzy_search_filtered(con, &qn, FUZZY_MAX_DISTANCE, &filter)?;

        if rows.is_empty() {
            warning(format!(
                "No fuzzy matches found for: {} (max distance: {})",
                description, FUZZY_MAX_DISTANCE
            ));
            return Ok(());
        }

        info(format!("Fuzzy search results for: {}", description));
        println!();
        print_table(&rows);

        println!("\n{} fuzzy match(es) for: {}", rows.len(), description);

        return Ok(());
    }

    // --- Standard exact search ---
    let rows = search_planets_filtered(con, &filter)?;

    if rows.is_empty() {
        warning(format!("No results found for: {}", description));

        // --- Automatic "Did you mean?" suggestion ---
        if let Some(query_text) = filter.query.as_deref().filter(|s| !s.trim().is_empty()) {
            let qn = normalize_text(query_text);
            let hits = fuzzy_search_filtered(con, &qn, FUZZY_MAX_DISTANCE, &filter)?;

            if !hits.is_empty() {
                println!();
                info("Did you mean?");
                for hit in &hits {
                    println!("  - {}", hit.name);
                }
                println!();
                println!("Tip: use --fuzzy to search with typo tolerance.");
            }
        }

        return Ok(());
    }

    print_table(&rows);
    println!("\n{} result(s) for: {}", rows.len(), description);

    Ok(())
}
