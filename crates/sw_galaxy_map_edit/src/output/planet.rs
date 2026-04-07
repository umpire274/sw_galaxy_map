//! Output helpers for sw_galaxy_map_edit.

use sw_galaxy_map_core::model::{Planet, PlanetSearchRow};

fn cell(opt: &Option<String>) -> &str {
    match opt.as_deref() {
        Some(s) if !s.trim().is_empty() => s,
        _ => "-",
    }
}

fn flag_cell(value: Option<i64>) -> &'static str {
    match value {
        Some(1) => "yes",
        Some(0) => "no",
        _ => "-",
    }
}

pub fn print_planet(planet: &Planet) {
    println!("Planet   : {}", planet.planet);
    println!("FID      : {}", planet.fid);
    println!("Norm     : {}", planet.planet_norm);
    println!("Region   : {}", cell(&planet.region));
    println!("Sector   : {}", cell(&planet.sector));
    println!("System   : {}", cell(&planet.system));
    println!("Grid     : {}", cell(&planet.grid));
    println!("X        : {:.3}", planet.x);
    println!("Y        : {:.3}", planet.y);
    println!("Canon    : {}", flag_cell(planet.canon));
    println!("Legends  : {}", flag_cell(planet.legends));
    println!("ZM       : {}", flag_cell(planet.zm));
    println!("Name0    : {}", cell(&planet.name0));
    println!("Name1    : {}", cell(&planet.name1));
    println!("Name2    : {}", cell(&planet.name2));

    match planet.lat {
        Some(v) => println!("Lat      : {:.6}", v),
        None => println!("Lat      : -"),
    }

    match planet.long {
        Some(v) => println!("Long     : {:.6}", v),
        None => println!("Long     : -"),
    }

    println!("Ref      : {}", cell(&planet.reference));
    println!("Status   : {}", cell(&planet.status));
    println!("CRegion  : {}", cell(&planet.c_region));
    println!("CRegionL : {}", cell(&planet.c_region_li));
}

pub fn print_search_results(rows: &[PlanetSearchRow]) {
    if rows.is_empty() {
        println!("No results found.");
        return;
    }

    println!(
        "{:>8}  {:<28}  {:<18}  {:<12}  {:<8}  {:>9}  {:>9}",
        "FID", "Planet", "Region", "Sector", "Grid", "X", "Y"
    );
    println!(
        "{:-<8}  {:-<28}  {:-<18}  {:-<12}  {:-<8}  {:-<9}  {:-<9}",
        "", "", "", "", "", "", ""
    );

    for row in rows {
        println!(
            "{:>8}  {:<28}  {:<18}  {:<12}  {:<8}  {:>9.2}  {:>9.2}",
            row.fid,
            row.name,
            cell(&row.region),
            cell(&row.sector),
            cell(&row.grid),
            row.x,
            row.y
        );
    }
}