//! Parsing and validation helpers for editable field values.

use anyhow::{Result, bail};

use crate::edit::field::{EditableField, FieldValue};

/// Parses the raw input entered by the user into a typed field value.
pub fn parse_input(field: EditableField, raw: &str) -> Result<FieldValue> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        if field.nullable() {
            return Ok(FieldValue::Null);
        }

        bail!("Field '{}' cannot be set to NULL.", field);
    }

    match field {
        EditableField::Planet => {
            let value = trimmed.to_string();
            if value.is_empty() {
                bail!("Planet name cannot be empty.");
            }
            Ok(FieldValue::Text(value))
        }

        EditableField::Region
        | EditableField::Sector
        | EditableField::System
        | EditableField::Grid
        | EditableField::Status
        | EditableField::Reference => Ok(FieldValue::Text(trimmed.to_string())),

        EditableField::X | EditableField::Y | EditableField::Lat | EditableField::Long => {
            let value: f64 = trimmed.parse()?;
            Ok(FieldValue::Real {
                value,
                raw: trimmed.to_string(),
            })
        }
    }
}