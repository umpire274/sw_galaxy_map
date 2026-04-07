//! Audit history query helpers.

use anyhow::Result;
use rusqlite::{Connection, params};

/// One audit history row loaded from `entity_edit_log`.
#[derive(Debug, Clone)]
pub struct EntityHistoryRow {
    pub id: i64,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub edited_at: String,
    pub reason: Option<String>,
    pub source: Option<String>,
}

/// Loads recent history rows for a given entity.
pub fn load_entity_history(
    con: &Connection,
    entity_type: &str,
    entity_id: i64,
    limit: usize,
) -> Result<Vec<EntityHistoryRow>> {
    let mut stmt = con.prepare(
        r#"
        SELECT
            id,
            field_name,
            old_value,
            new_value,
            edited_at,
            reason,
            source
        FROM entity_edit_log
        WHERE entity_type = ?1
          AND entity_id = ?2
        ORDER BY id DESC
        LIMIT ?3
        "#,
    )?;

    let rows = stmt.query_map(params![entity_type, entity_id, limit as i64], |row| {
        Ok(EntityHistoryRow {
            id: row.get(0)?,
            field_name: row.get(1)?,
            old_value: row.get(2)?,
            new_value: row.get(3)?,
            edited_at: row.get(4)?,
            reason: row.get(5)?,
            source: row.get(6)?,
        })
    })?;

    let collected = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(collected)
}