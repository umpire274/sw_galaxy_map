use crate::ui;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Transaction};

const SCHEMA_VERSION: i64 = 5;

fn column_exists(tx: &Transaction<'_>, table: &str, col: &str) -> Result<bool> {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = tx.prepare(&sql)?;
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
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?;

    match s {
        None => Ok(None),
        Some(v) => Ok(Some(v.parse::<i64>().with_context(|| {
            format!(
                "Invalid integer value in meta table for key '{}': '{}'",
                key, v
            )
        })?)),
    }
}

fn meta_upsert(tx: &Transaction<'_>, key: &str, value: &str) -> Result<()> {
    tx.execute(
        r#"
        INSERT INTO meta(key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        (key, value),
    )?;
    Ok(())
}

fn m_to_v4(tx: &Transaction<'_>) -> Result<()> {
    // v0.4.0 additions
    if !column_exists(tx, "planets", "deleted")? {
        tx.execute_batch(
            r#"
            ALTER TABLE planets
            ADD COLUMN deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0,1));
            "#,
        )
        .context("Failed to add planets.deleted")?;
    }

    if !column_exists(tx, "planets", "arcgis_hash")? {
        tx.execute_batch(
            r#"
            ALTER TABLE planets
            ADD COLUMN arcgis_hash TEXT NOT NULL DEFAULT '';
            "#,
        )
        .context("Failed to add planets.arcgis_hash")?;
    }

    Ok(())
}

fn m_to_v5(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        r#"
        -- =========================
        -- WAYPOINTS (catalog)
        -- =========================
        CREATE TABLE IF NOT EXISTS waypoints (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL,
            name_norm  TEXT NOT NULL,
            x          REAL NOT NULL,
            y          REAL NOT NULL,
            kind       TEXT NOT NULL DEFAULT 'manual',
            note       TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_name_norm
          ON waypoints(name_norm);

        CREATE INDEX IF NOT EXISTS idx_waypoints_xy
          ON waypoints(x, y);

        CREATE TRIGGER IF NOT EXISTS trg_waypoints_updated_at
        AFTER UPDATE ON waypoints
        FOR EACH ROW
        BEGIN
          UPDATE waypoints SET updated_at = datetime('now') WHERE id = OLD.id;
        END;
        "#,
    )
    .context("Failed to migrate schema to v5 (create waypoints table)")?;

    Ok(())
}

/// Run schema migrations up to SCHEMA_VERSION.
/// Idempotent and safe to call on every startup/open.
pub fn run(con: &mut Connection) -> Result<()> {
    con.query_row("SELECT 1 FROM meta LIMIT 1", [], |r| r.get::<_, i32>(0))
        .context("Database schema is missing required table: meta")?;

    let current = meta_get_i64(con, "schema_version")?.unwrap_or(0);

    if current >= SCHEMA_VERSION {
        return Ok(());
    }

    ui::info(format!(
        "Database schema upgrade required (current: v{}, target: v{})",
        current, SCHEMA_VERSION
    ));

    let tx = con
        .transaction()
        .context("Failed to start migration transaction")?;

    // Incremental migrations
    if current < 4 {
        ui::info("Applying migration: v3 → v4 (planets metadata)");
        m_to_v4(&tx)?;
        let new_schema_version = "4";
        meta_upsert(&tx, "schema_version", new_schema_version).context(format!(
            "Failed to update meta.schema_version to {}",
            new_schema_version
        ))?;
        ui::success("Migration v3 → v4 completed");
    }

    if current < 5 {
        ui::info("Applying migration: v4 → v5 (waypoints catalog)");
        m_to_v5(&tx)?;
        let new_schema_version = "5";
        meta_upsert(&tx, "schema_version", new_schema_version).context(format!(
            "Failed to update meta.schema_version to {}",
            new_schema_version
        ))?;
        ui::success("Migration v4 → v5 completed");
    }

    tx.commit().context("Failed to commit migration")?;

    ui::info("Database schema successfully updated");

    Ok(())
}
