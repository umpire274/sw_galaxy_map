//! Set command implementation.

use anyhow::{Result, anyhow, bail};
use inquire::Confirm;

use crate::cli::SetArgs;
use crate::db::runtime::open_db;
use crate::edit::apply::update_single_field_with_audit;
use crate::edit::field::{EditableField, FieldValue};
use crate::edit::parser::parse_input;
use crate::output::planet::print_planet;
use crate::resolve::planet::{resolve_by_fid, resolve_by_name_or_alias};

pub fn run(args: SetArgs) -> Result<()> {
    if args.fid.is_none() && args.planet.is_none() {
        bail!("You must provide either --fid <FID> or --planet <NAME>.");
    }

    let field = EditableField::parse(&args.field).ok_or_else(|| {
        anyhow!(
            "Unknown field '{}'. Allowed values: {}",
            args.field,
            EditableField::accepted_names().join(", ")
        )
    })?;

    let parsed_value = parse_input(field, &args.value)?;

    let mut con = open_db()?;

    let planet = if let Some(fid) = args.fid {
        resolve_by_fid(&con, fid)?
    } else if let Some(name) = args.planet.as_deref() {
        resolve_by_name_or_alias(&con, name)?
    } else {
        None
    };

    let planet = match planet {
        Some(p) => p,
        None => bail!("Planet not found."),
    };

    let old_display = extract_field_value(&planet, field);
    let new_display = display_new_value(&parsed_value);

    println!("Target planet:");
    println!();
    print_planet(&planet);
    println!();

    println!("Preview change:");
    println!("Field : {}", field);
    println!("Old   : {}", old_display);
    println!("New   : {}", new_display);
    println!("Reason: {}", display_reason(args.reason.as_deref()));
    println!();

    if !args.yes {
        let confirm = Confirm::new("Apply this change?")
            .with_default(false)
            .prompt()?;

        if !confirm {
            println!("Change discarded.");
            return Ok(());
        }
    }

    update_single_field_with_audit(
        &mut con,
        planet.fid,
        field,
        &parsed_value,
        display_to_option(&old_display),
        display_to_option(&new_display),
        normalize_optional_reason(args.reason.as_deref()),
    )?;

    let updated = resolve_by_fid(&con, planet.fid)?
        .ok_or_else(|| anyhow!("Planet disappeared after update."))?;

    println!();
    println!("Change applied successfully.");
    println!();
    print_planet(&updated);

    Ok(())
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

fn display_to_option(value: &str) -> Option<&str> {
    if value == "NULL" {
        None
    } else {
        Some(value)
    }
}

fn normalize_optional_reason(reason: Option<&str>) -> Option<&str> {
    match reason {
        Some(s) if !s.trim().is_empty() => Some(s.trim()),
        _ => None,
    }
}

fn display_reason(reason: Option<&str>) -> &str {
    match reason {
        Some(s) if !s.trim().is_empty() => s,
        _ => "-",
    }
}