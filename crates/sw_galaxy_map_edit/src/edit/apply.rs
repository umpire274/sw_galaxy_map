//! Database update helpers for editable planet fields.

use anyhow::{Result, bail};
use chrono::Utc;
use rusqlite::{Connection, params};
use sw_galaxy_map_core::utils::normalize_text;

use crate::audit::log::{AuditEntry, insert_audit_entry};
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

            refresh_planet_search_artifacts(&tx, fid)?;
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

    let field_name = field.to_string();

    let entry = AuditEntry {
        entity_type: "planet",
        entity_id: fid,
        field_name: &field_name,
        old_value,
        new_value,
        edited_at: &edited_at,
        reason,
        source: Some("sw_galaxy_map_edit"),
    };

    insert_audit_entry(&tx, &entry)?;

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

fn refresh_planet_search_artifacts(con: &rusqlite::Connection, fid: i64) -> Result<()> {
    refresh_planet_search_row(con, fid)?;

    if table_exists(con, "planets_fts")? {
        refresh_planets_fts_row(con, fid)?;
    }

    Ok(())
}

fn refresh_planet_search_row(con: &rusqlite::Connection, fid: i64) -> Result<()> {
    con.execute("DELETE FROM planet_search WHERE fid = ?1", params![fid])?;

    con.execute(
        r#"
        INSERT INTO planet_search (
            fid,
            name,
            name_norm,
            region,
            sector,
            system,
            grid,
            x,
            y,
            canon,
            legends,
            zm,
            deleted
        )
        SELECT
            p.FID,
            p.Planet,
            p.planet_norm,
            p.Region,
            p.Sector,
            p.System,
            p.Grid,
            p.X,
            p.Y,
            p.Canon,
            p.Legends,
            p.zm,
            p.deleted
        FROM planets p
        WHERE p.FID = ?1
        "#,
        params![fid],
    )?;

    Ok(())
}

fn refresh_planets_fts_row(con: &rusqlite::Connection, fid: i64) -> Result<()> {
    con.execute("DELETE FROM planets_fts WHERE fid = ?1", params![fid])?;

    con.execute(
        r#"
        INSERT INTO planets_fts (
            fid,
            planet,
            planet_norm,
            region,
            sector,
            system,
            grid
        )
        SELECT
            p.FID,
            p.Planet,
            p.planet_norm,
            COALESCE(p.Region, ''),
            COALESCE(p.Sector, ''),
            COALESCE(p.System, ''),
            COALESCE(p.Grid, '')
        FROM planets p
        WHERE p.FID = ?1
        "#,
        params![fid],
    )?;

    Ok(())
}

fn table_exists(con: &rusqlite::Connection, table_name: &str) -> Result<bool> {
    let exists: i64 = con.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table_name],
        |row| row.get(0),
    )?;

    Ok(exists > 0)
}
