//! Audit log helpers for sw_galaxy_map_edit.

use anyhow::Result;
use rusqlite::{Connection, params};

/// Ensures that the audit log table exists.
pub fn ensure_audit_table(con: &Connection) -> Result<()> {
    con.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS entity_edit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            entity_id INTEGER NOT NULL,
            field_name TEXT NOT NULL,
            old_value TEXT,
            new_value TEXT,
            edited_at TEXT NOT NULL,
            reason TEXT,
            source TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_entity_edit_log_entity
            ON entity_edit_log (entity_type, entity_id);

        CREATE INDEX IF NOT EXISTS idx_entity_edit_log_time
            ON entity_edit_log (edited_at);
        "#,
    )?;

    Ok(())
}

/// Writes a single audit log entry.
pub fn insert_audit_entry(
    con: &Connection,
    entity_type: &str,
    entity_id: i64,
    field_name: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    edited_at: &str,
    reason: Option<&str>,
    source: Option<&str>,
) -> Result<()> {
    con.execute(
        r#"
        INSERT INTO entity_edit_log (
            entity_type,
            entity_id,
            field_name,
            old_value,
            new_value,
            edited_at,
            reason,
            source
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            entity_type,
            entity_id,
            field_name,
            old_value,
            new_value,
            edited_at,
            reason,
            source
        ],
    )?;

    Ok(())
}