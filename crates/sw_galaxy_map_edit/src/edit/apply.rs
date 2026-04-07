//! Database update helpers for editable planet fields.

use anyhow::{Result, bail};
use rusqlite::{Connection, params};
use sw_galaxy_map_core::utils::normalize_text;

use crate::edit::field::{EditableField, FieldValue};

/// Applies a single-field update to a planet row.
pub fn update_single_field(
    con: &Connection,
    fid: i64,
    field: EditableField,
    value: &FieldValue,
) -> Result<()> {
    ensure_planet_exists(con, fid)?;

    match (field, value) {
        (EditableField::Planet, FieldValue::Text(text)) => {
            let normalized = normalize_text(text);

            if normalized.trim().is_empty() {
                bail!("Planet name cannot normalize to an empty value.");
            }

            con.execute(
                "UPDATE planets
                 SET Planet = ?1,
                     planet_norm = ?2
                 WHERE FID = ?3",
                params![text, normalized, fid],
            )?;
        }

        (_, FieldValue::Text(text)) => {
            let sql = format!(
                "UPDATE planets SET {} = ?1 WHERE FID = ?2",
                field.column_name()
            );
            con.execute(&sql, params![text, fid])?;
        }

        (_, FieldValue::Real { value, .. }) => {
            let sql = format!(
                "UPDATE planets SET {} = ?1 WHERE FID = ?2",
                field.column_name()
            );
            con.execute(&sql, params![value, fid])?;
        }

        (_, FieldValue::Null) => {
            if !field.nullable() {
                bail!("Field '{}' cannot be set to NULL.", field);
            }

            let sql = format!(
                "UPDATE planets SET {} = NULL WHERE FID = ?1",
                field.column_name()
            );
            con.execute(&sql, params![fid])?;
        }
    }

    Ok(())
}

fn ensure_planet_exists(con: &Connection, fid: i64) -> Result<()> {
    let exists: i64 = con.query_row(
        "SELECT COUNT(*) FROM planets WHERE FID = ?1",
        params![fid],
        |row| row.get(0),
    )?;

    if exists == 0 {
        bail!("Planet with FID {} does not exist.", fid);
    }

    Ok(())
}