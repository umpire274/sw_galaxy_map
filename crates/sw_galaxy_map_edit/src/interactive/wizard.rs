//! Guided interactive editor.

use anyhow::{Result, anyhow, bail};
use inquire::{Confirm, Select, Text};

use crate::db::runtime::open_db;
use crate::edit::apply::update_single_field_with_audit;
use crate::edit::field::{EditableField, FieldValue};
use crate::edit::parser::parse_input;
use crate::output::planet::print_planet;
use crate::resolve::planet::{resolve_by_fid, resolve_single, search};
use crate::output::validation::print_validation_issues;
use crate::validate::field::{has_errors, validate_field_value};

/// Starts the interactive editing wizard.
pub fn run() -> Result<()> {
    println!("sw_galaxy_map_edit interactive mode");
    println!();

    let mut con = open_db()?;

    let query = Text::new("Planet name, alias, or FID:")
        .with_help_message("Example: Coruscant, Tatooine, or 1234")
        .prompt()?;

    println!();

    let planet = match resolve_single(&con, &query) {
        Ok(Some(p)) => p,
        Ok(None) => resolve_from_search(&con, &query)?,
        Err(_) => resolve_from_search(&con, &query)?,
    };

    println!("Selected planet:");
    println!();
    print_planet(&planet);
    println!();

    let field = Select::new("Select field to edit:", EditableField::all().to_vec()).prompt()?;

    let help = if field.nullable() {
        "Leave empty to write NULL."
    } else {
        "This field cannot be empty."
    };

    let raw_value = Text::new("New value:")
        .with_help_message(help)
        .prompt()?;

    let parsed_value = parse_input(field, &raw_value)?;

    let issues = validate_field_value(field, &parsed_value);

    if !issues.is_empty() {
        println!();
        print_validation_issues(&issues);
    }

    if has_errors(&issues) {
        anyhow::bail!("Cannot apply the change because validation failed.");
    }

    let old_display = extract_field_value(&planet, field);
    let new_display = display_new_value(&parsed_value);

    println!();
    println!("Preview change:");
    println!("Field : {}", field);
    println!("Old   : {}", old_display);
    println!("New   : {}", new_display);
    println!();

    let reason_raw = Text::new("Reason for change:")
        .with_help_message("Describe why this field is being edited")
        .prompt()?;

    let reason = normalize_optional_text(&reason_raw);

    let confirm = Confirm::new("Apply this change?")
        .with_default(false)
        .prompt()?;

    if !confirm {
        println!("Change discarded.");
        return Ok(());
    }

    update_single_field_with_audit(
        &mut con,
        planet.fid,
        field,
        &parsed_value,
        display_to_option(&old_display),
        display_to_option(&new_display),
        reason.as_deref(),
    )?;

    let updated = resolve_by_fid(&con, planet.fid)?
        .ok_or_else(|| anyhow!("Planet disappeared after update."))?;

    println!();
    println!("Change applied successfully.");
    println!();
    print_planet(&updated);

    Ok(())
}

fn resolve_from_search(
    con: &rusqlite::Connection,
    query: &str,
) -> Result<sw_galaxy_map_core::model::Planet> {
    let results = search(con, query, 20)?;

    if results.is_empty() {
        bail!("No planets found.");
    }

    let options: Vec<String> = results.iter().map(format_search_option).collect();

    let selection = Select::new("Multiple matches found. Select one:", options).prompt()?;

    let index = results
        .iter()
        .position(|r| format_search_option(r) == selection)
        .ok_or_else(|| anyhow!("Selection mismatch."))?;

    let fid = results[index].fid;

    resolve_by_fid(con, fid)?
        .ok_or_else(|| anyhow!("Failed to load selected planet."))
}

fn format_search_option(row: &sw_galaxy_map_core::model::PlanetSearchRow) -> String {
    let grid = row.grid.as_deref().unwrap_or("-");
    format!("{} (FID: {}, Grid: {})", row.name, row.fid, grid)
}

fn extract_field_value(p: &sw_galaxy_map_core::model::Planet, field: EditableField) -> String {
    match field {
        EditableField::Planet => p.planet.clone(),
        EditableField::Region => opt_text(&p.region),
        EditableField::Sector => opt_text(&p.sector),
        EditableField::System => opt_text(&p.system),
        EditableField::Grid => opt_text(&p.grid),
        EditableField::X => format!("{:.3}", p.x),
        EditableField::Y => format!("{:.3}", p.y),
        EditableField::Lat => p
            .lat
            .map(|v| format!("{:.6}", v))
            .unwrap_or_else(|| "NULL".to_string()),
        EditableField::Long => p
            .long
            .map(|v| format!("{:.6}", v))
            .unwrap_or_else(|| "NULL".to_string()),
        EditableField::Status => opt_text(&p.status),
        EditableField::Reference => opt_text(&p.reference),
    }
}

fn display_new_value(value: &FieldValue) -> String {
    match value {
        FieldValue::Text(s) => s.clone(),
        FieldValue::Real { raw, .. } => raw.clone(),
        FieldValue::Null => "NULL".to_string(),
    }
}

fn opt_text(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "NULL".to_string())
}

fn normalize_optional_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn display_to_option(value: &str) -> Option<&str> {
    if value == "NULL" {
        None
    } else {
        Some(value)
    }
}