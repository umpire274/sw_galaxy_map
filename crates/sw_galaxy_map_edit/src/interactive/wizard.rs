//! Guided interactive editor.

use anyhow::{Result, bail};
use inquire::{Select, Text, Confirm};

use crate::db::runtime::open_db;
use crate::output::planet::print_planet;
use crate::resolve::planet::{resolve_single, search};

/// Entry point for interactive wizard.
pub fn run() -> Result<()> {
    println!("🧭 sw_galaxy_map_edit interactive mode");
    println!();

    let con = open_db()?;

    // STEP 1: query input
    let query = Text::new("Planet name, alias, or FID:")
        .with_help_message("Example: Coruscant, Tatooine, or 1234")
        .prompt()?;

    println!();

    // STEP 2: resolve
    let planet = match resolve_single(&con, &query) {
        Ok(Some(p)) => p,
        Ok(None) => {
            // fallback search
            let results = search(&con, &query, 20)?;

            if results.is_empty() {
                bail!("No planets found.");
            }

            // STEP 3: multi-select
            let options: Vec<String> = results
                .iter()
                .map(|r| format!("{} (FID: {}, Grid: {})", r.name, r.fid, r.grid.clone().unwrap_or_default()))
                .collect();

            let selection = Select::new("Multiple matches found. Select one:", options).prompt()?;

            let index = results
                .iter()
                .position(|r| {
                    let label = format!("{} (FID: {}, Grid: {})", r.name, r.fid, r.grid.clone().unwrap_or_default());
                    label == selection
                })
                .ok_or_else(|| anyhow::anyhow!("Selection mismatch"))?;

            let fid = results[index].fid;
            crate::resolve::planet::resolve_by_fid(&con, fid)?
                .ok_or_else(|| anyhow::anyhow!("Failed to load selected planet"))?
        }
        Err(e) => {
            // multiple match error → fallback search
            let results = search(&con, &query, 20)?;

            if results.is_empty() {
                return Err(e);
            }

            let options: Vec<String> = results
                .iter()
                .map(|r| format!("{} (FID: {}, Grid: {})", r.name, r.fid, r.grid.clone().unwrap_or_default()))
                .collect();

            let selection = Select::new("Multiple matches found. Select one:", options).prompt()?;

            let index = results
                .iter()
                .position(|r| {
                    let label = format!("{} (FID: {}, Grid: {})", r.name, r.fid, r.grid.clone().unwrap_or_default());
                    label == selection
                })
                .ok_or_else(|| anyhow::anyhow!("Selection mismatch"))?;

            let fid = results[index].fid;
            crate::resolve::planet::resolve_by_fid(&con, fid)?
                .ok_or_else(|| anyhow::anyhow!("Failed to load selected planet"))?
        }
    };

    println!();
    println!("Selected planet:");
    println!();
    print_planet(&planet);

    // STEP 4: choose field
    let fields = vec![
        "planet",
        "region",
        "sector",
        "system",
        "grid",
        "x",
        "y",
        "lat",
        "long",
        "status",
        "reference",
    ];

    let field = Select::new("Select field to edit:", fields).prompt()?;

    // STEP 5: input new value
    let new_value = Text::new("New value:")
        .with_help_message("Leave empty to set NULL (where allowed)")
        .prompt()?;

    println!();

    // STEP 6: preview (semplice)
    println!("Preview change:");
    println!("Field : {}", field);
    println!("Old   : {}", extract_field_value(&planet, &field));
    println!("New   : {}", if new_value.trim().is_empty() { "NULL" } else { &new_value });

    println!();

    // STEP 7: confirm
    let confirm = Confirm::new("Apply this change?")
        .with_default(false)
        .prompt()?;

    if !confirm {
        println!("Change discarded.");
        return Ok(());
    }

    println!("(Simulation only) Change accepted but not yet persisted.");

    Ok(())
}

/// Extracts a field value as string for preview.
fn extract_field_value(p: &sw_galaxy_map_core::model::Planet, field: &str) -> String {
    match field {
        "planet" => p.planet.clone(),
        "region" => p.region.clone().unwrap_or_default(),
        "sector" => p.sector.clone().unwrap_or_default(),
        "system" => p.system.clone().unwrap_or_default(),
        "grid" => p.grid.clone().unwrap_or_default(),
        "x" => format!("{:.3}", p.x),
        "y" => format!("{:.3}", p.y),
        "lat" => p.lat.map(|v| format!("{:.6}", v)).unwrap_or_default(),
        "long" => p.long.map(|v| format!("{:.6}", v)).unwrap_or_default(),
        "status" => p.status.clone().unwrap_or_default(),
        "reference" => p.reference.clone().unwrap_or_default(),
        _ => "-".to_string(),
    }
}