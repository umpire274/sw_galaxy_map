//! Set command implementation.

use anyhow::{Result, anyhow, bail};
use inquire::Confirm;

use crate::cli::SetArgs;
use crate::db::runtime::open_db;
use crate::edit::apply::update_single_field_with_audit;
use crate::edit::display::{display_new_value, display_to_option, extract_field_value};
use crate::edit::field::EditableField;
use crate::edit::parser::parse_input;
use crate::output::planet::print_planet;
use crate::output::validation::print_validation_issues;
use crate::resolve::planet::{resolve_by_fid, resolve_by_name_or_alias};
use crate::validate::field::{has_errors, validate_field_value};

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

    let issues = validate_field_value(field, &parsed_value);

    if !issues.is_empty() {
        println!();
        print_validation_issues(&issues);
    }

    if has_errors(&issues) {
        bail!("Cannot apply the change because validation failed.");
    }

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
