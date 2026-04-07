//! Database update helpers for editable planet fields.

use anyhow::{Result, bail};
use chrono::Utc;
use rusqlite::{Connection, params};
use sw_galaxy_map_core::utils::normalize_text;

use crate::audit::log::insert_audit_entry;
use crate::edit::field::{EditableField, FieldValue};

/// Applies a single-field update to a planet row and writes an audit entry.
pub fn update_single_field_with_audit(
    con: &mut Connection,
    fid: i64,
    field: EditableField,
    value: &FieldValue,
    old_value: Option<&str>,
    new_value: Option<&str>,
    reason: Option<&str>,
) -> Result<()> {
    ensure_planet_exists(con, fid)?;

    let tx = con.transaction()?;

    match (field, value) {
        (EditableField::Planet, FieldValue::Text(text)) => {
            let normalized = normalize_text(text);

            if normalized.trim().is_empty() {
                bail!("Planet name cannot normalize to an empty value.");
            }

            tx.execute(
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
            tx.execute(&sql, params![text, fid])?;
        }

        (_, FieldValue::Real { value, .. }) => {
            let sql = format!(
                "UPDATE planets SET {} = ?1 WHERE FID = ?2",
                field.column_name()
            );
            tx.execute(&sql, params![value, fid])?;
        }

        (_, FieldValue::Null) => {
            if !field.nullable() {
                bail!("Field '{}' cannot be set to NULL.", field);
            }

            let sql = format!(
                "UPDATE planets SET {} = NULL WHERE FID = ?1",
                field.column_name()
            );
            tx.execute(&sql, params![fid])?;
        }
    }

    let edited_at = Utc::now().to_rfc3339();

    insert_audit_entry(
        &tx,
        "planet",          // 👈 entity_type fisso per ora
        fid,
        &field.to_string(),
        old_value,
        new_value,
        &edited_at,
        reason,
        Some("sw_galaxy_map_edit"),
    )?;
    
    tx.commit()?;
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