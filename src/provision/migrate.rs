use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

const SCHEMA_VERSION: i64 = 4;

fn column_exists(con: &Connection, table: &str, col: &str) -> Result<bool> {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = con.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?; // PRAGMA table_info: 1 = name
        if name.eq_ignore_ascii_case(col) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn meta_get_i64(con: &Connection, key: &str) -> Result<Option<i64>> {
    let s: Option<String> = con
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .optional()?;

    match s {
        None => Ok(None),
        Some(v) => {
            let n = v.parse::<i64>().with_context(|| {
                format!(
                    "Invalid integer value in meta table for key '{}': '{}'",
                    key, v
                )
            })?;
            Ok(Some(n))
        }
    }
}

fn meta_upsert(con: &Connection, key: &str, value: &str) -> Result<()> {
    con.execute(
        r#"
        INSERT INTO meta(key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        (key, value),
    )?;
    Ok(())
}

/// Run schema migrations up to SCHEMA_VERSION.
/// Idempotent and safe to call on every startup/open.
pub fn run(con: &mut Connection) -> Result<()> {
    // meta table should exist for your DBs; still, keep a nice error if not.
    con.query_row("SELECT 1 FROM meta LIMIT 1", [], |r| r.get::<_, i32>(0))
        .context("Database schema is missing required table: meta")?;

    let current = meta_get_i64(con, "schema_version")?.unwrap_or(0);

    if current >= SCHEMA_VERSION {
        return Ok(());
    }

    // One transaction for migration
    let tx = con
        .transaction()
        .context("Failed to start migration transaction")?;

    // v0.4.0 additions
    if !column_exists(&tx, "planets", "deleted")? {
        tx.execute_batch(
            r#"
            ALTER TABLE planets
            ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0,1));
            "#,
        )
        .context("Failed to add planets.deleted")?;
    }

    if !column_exists(&tx, "planets", "arcgis_hash")? {
        tx.execute_batch(
            r#"
            ALTER TABLE planets
            ADD COLUMN arcgis_hash TEXT NOT NULL DEFAULT '';
            "#,
        )
        .context("Failed to add planets.arcgis_hash")?;
    }

    meta_upsert(&tx, "schema_version", &SCHEMA_VERSION.to_string())
        .context("Failed to update meta.schema_version")?;

    tx.commit().context("Failed to commit migration")?;
    Ok(())
}
