//! Display helpers for editable field values.

use crate::edit::field::{EditableField, FieldValue};
use sw_galaxy_map_core::model::Planet;

/// Extracts the current display value of a field from a planet record.
pub fn extract_field_value(p: &Planet, field: EditableField) -> String {
    match field {
        EditableField::Planet => p.planet.clone(),
        EditableField::Region => opt_text(&p.region),
        EditableField::Sector => opt_text(&p.sector),
        EditableField::System => opt_text(&p.system),
        EditableField::Grid => opt_text(&p.grid),
        EditableField::X => format!("{:.3}", p.x),
        EditableField::Y => format!("{:.3}", p.y),
        EditableField::Lat => format_optional_real(p.lat, 6),
        EditableField::Long => format_optional_real(p.long, 6),
        EditableField::Status => opt_text(&p.status),
        EditableField::Reference => opt_text(&p.reference),
    }
}

/// Converts a parsed new value to its display representation.
pub fn display_new_value(value: &FieldValue) -> String {
    match value {
        FieldValue::Text(s) => s.clone(),
        FieldValue::Real { raw, .. } => raw.clone(),
        FieldValue::Null => "NULL".to_string(),
    }
}

/// Converts a displayed value to an optional audit value.
pub fn display_to_option(value: &str) -> Option<&str> {
    if value == "NULL" { None } else { Some(value) }
}

fn opt_text(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| "NULL".to_string())
}

fn format_optional_real(value: Option<f64>, decimals: usize) -> String {
    match value {
        Some(v) => format!("{:.*}", decimals, v),
        None => "NULL".to_string(),
    }
}
