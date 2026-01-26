use crate::ui;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Transaction};

const START_SCHEMA_VERSION: i64 = 3;
const LATEST_SCHEMA_VERSION: i64 = 10;

struct MigrationStep {
    from: i64,
    to: i64,
    label: &'static str,
    apply: fn(&Transaction<'_>) -> Result<()>,
}

fn migration_steps() -> &'static [MigrationStep] {
    &[
        MigrationStep {
            from: 3,
            to: 4,
            label: "planets metadata",
            apply: m_to_v4,
        },
        MigrationStep {
            from: 4,
            to: 5,
            label: "waypoints catalog",
            apply: m_to_v5,
        },
        MigrationStep {
            from: 5,
            to: 6,
            label: "waypoint↔planet links + fingerprint",
            apply: m_to_v6,
        },
        MigrationStep {
            from: 6,
            to: 7,
            label: "routes persistence",
            apply: m_to_v7,
        },
        MigrationStep {
            from: 7,
            to: 8,
            label: "route upsert + updated_at",
            apply: m_to_v8,
        },
        MigrationStep {
            from: 8,
            to: 9,
            label: "route_detours tries_used + tries_exhausted",
            apply: m_to_v9,
        },
        MigrationStep {
            from: 9,
            to: 10,
            label: "routes status index",
            apply: m_to_v10,
        },
    ]
}

fn set_schema_version(tx: &Transaction<'_>, v: i64) -> Result<()> {
    let v_str = v.to_string();
    meta_upsert(tx, "schema_version", &v_str)
        .with_context(|| format!("Failed to update meta.schema_version to {}", v_str))?;
    Ok(())
}

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

fn m_to_v6(tx: &Transaction<'_>) -> Result<()> {
    // 1) waypoints.fingerprint (ALTER TABLE must be conditional in SQLite)
    if !column_exists(tx, "waypoints", "fingerprint")? {
        tx.execute_batch(
            r#"
            ALTER TABLE waypoints ADD COLUMN fingerprint TEXT;
            "#,
        )
        .context("Failed to add waypoints.fingerprint")?;
    }

    // 2) Index for fingerprint (unique only when present)
    tx.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_fingerprint
          ON waypoints(fingerprint)
          WHERE fingerprint IS NOT NULL;
        "#,
    )
    .context("Failed to create idx_waypoints_fingerprint")?;

    // 3) N:N relation table waypoints <-> planets
    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS waypoint_planets (
          waypoint_id INTEGER NOT NULL,
          planet_fid  INTEGER NOT NULL,
          role        TEXT NOT NULL DEFAULT 'anchor', -- anchor/avoid/near/cluster_member
          distance    REAL,
          PRIMARY KEY (waypoint_id, planet_fid),
          FOREIGN KEY (waypoint_id) REFERENCES waypoints(id) ON DELETE CASCADE,
          FOREIGN KEY (planet_fid)  REFERENCES planets(FID)  ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_wp_planets_planet
          ON waypoint_planets(planet_fid);

        CREATE INDEX IF NOT EXISTS idx_wp_planets_waypoint
          ON waypoint_planets(waypoint_id);

        CREATE INDEX IF NOT EXISTS idx_wp_planets_role
          ON waypoint_planets(role);
        "#,
    )
    .context("Failed to create waypoint_planets relation table/indexes")?;

    Ok(())
}

fn m_to_v7(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        r#"
        -- =========================
        -- ROUTES (computed runs)
        -- =========================
        CREATE TABLE IF NOT EXISTS routes (
          id              INTEGER PRIMARY KEY AUTOINCREMENT,
          from_planet_fid INTEGER NOT NULL,
          to_planet_fid   INTEGER NOT NULL,
          algo_version    TEXT NOT NULL,
          options_json    TEXT NOT NULL,
          length          REAL,
          iterations      INTEGER,
          status          TEXT NOT NULL DEFAULT 'ok' CHECK(status IN ('ok','failed')),
          error           TEXT,
          created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
          FOREIGN KEY(from_planet_fid) REFERENCES planets(FID),
          FOREIGN KEY(to_planet_fid)   REFERENCES planets(FID)
        );

        CREATE INDEX IF NOT EXISTS idx_routes_from_to
          ON routes(from_planet_fid, to_planet_fid, created_at);

        -- =========================
        -- ROUTE WAYPOINTS (polyline)
        -- =========================
        CREATE TABLE IF NOT EXISTS route_waypoints (
          route_id     INTEGER NOT NULL,
          seq          INTEGER NOT NULL,
          x            REAL NOT NULL,
          y            REAL NOT NULL,
          waypoint_id  INTEGER,
          PRIMARY KEY(route_id, seq),
          FOREIGN KEY(route_id) REFERENCES routes(id) ON DELETE CASCADE,
          FOREIGN KEY(waypoint_id) REFERENCES waypoints(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_route_waypoints_route
          ON route_waypoints(route_id);

        -- =========================
        -- ROUTE DETOURS (decisions + score)
        -- =========================
        CREATE TABLE IF NOT EXISTS route_detours (
          route_id        INTEGER NOT NULL,
          idx             INTEGER NOT NULL,

          iteration       INTEGER NOT NULL,
          segment_index   INTEGER NOT NULL,

          obstacle_id     INTEGER NOT NULL,
          obstacle_x      REAL NOT NULL,
          obstacle_y      REAL NOT NULL,
          obstacle_radius REAL NOT NULL,

          closest_t       REAL NOT NULL,
          closest_qx      REAL NOT NULL,
          closest_qy      REAL NOT NULL,
          closest_dist    REAL NOT NULL,

          offset_used     REAL NOT NULL,

          wp_x            REAL NOT NULL,
          wp_y            REAL NOT NULL,
          waypoint_id     INTEGER,

          score_base      REAL NOT NULL,
          score_turn      REAL NOT NULL,
          score_back      REAL NOT NULL,
          score_proximity REAL NOT NULL,
          score_total     REAL NOT NULL,

          PRIMARY KEY(route_id, idx),
          FOREIGN KEY(route_id) REFERENCES routes(id) ON DELETE CASCADE,
          FOREIGN KEY(waypoint_id) REFERENCES waypoints(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_route_detours_route
          ON route_detours(route_id);
        "#,
    )
    .context("Failed to migrate schema to v7 (routes persistence tables)")?;

    Ok(())
}

fn m_to_v8(tx: &Transaction<'_>) -> Result<()> {
    // 1) Add routes.updated_at (idempotent)
    if !column_exists(tx, "routes", "updated_at")? {
        tx.execute_batch(
            r#"
            ALTER TABLE routes
            ADD COLUMN updated_at TEXT;
            "#,
        )
        .context("Failed to add routes.updated_at")?;
    }

    // 2) Ensure uniqueness for (from,to)
    // If duplicates exist, this will fail: in that case we should deduplicate first.
    tx.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS ux_routes_from_to
        ON routes(from_planet_fid, to_planet_fid);
        "#,
    )
    .context("Failed to create unique index ux_routes_from_to on routes(from,to)")?;

    Ok(())
}

fn m_to_v9(tx: &Transaction<'_>) -> Result<()> {
    // No changes in v9 yet
    if !column_exists(tx, "route_detours", "tries_used")? {
        tx.execute_batch(
            r#"
            ALTER TABLE route_detours
            ADD COLUMN tries_used INTEGER;
            "#,
        )
        .context("Failed to add route_detours.tries_used")?;
    }

    if !column_exists(tx, "route_detours", "tries_exhausted")? {
        tx.execute_batch(
            r#"
            ALTER TABLE route_detours ADD COLUMN tries_exhausted INTEGER NOT NULL DEFAULT 0;
            "#,
        )
        .context("Failed to add route_detours.tries_exhausted")?;
    }

    Ok(())
}

fn m_to_v10(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_routes_status ON routes(status);
        "#,
    )
    .context("Failed to migrate schema to v10 (creation idx_routes_status index)")?;

    Ok(())
}

/// Run schema migrations up to SCHEMA_VERSION.
/// Idempotent and safe to call on every startup/open.
pub fn run(con: &mut Connection, dry_run: bool, emit_noop: bool) -> Result<()> {
    con.query_row("SELECT 1 FROM meta LIMIT 1", [], |r| r.get::<_, i32>(0))
        .context("Database schema is missing required table: meta")?;

    let current = meta_get_i64(con, "schema_version")?.unwrap_or(0);

    if current >= LATEST_SCHEMA_VERSION {
        if emit_noop {
            ui::info(format!("Database schema already up-to-date (v{})", current));
        }
        return Ok(());
    }

    ui::info(format!(
        "Database schema upgrade required (current: v{}, target: v{})",
        current, LATEST_SCHEMA_VERSION
    ));

    let tx = con
        .transaction()
        .context("Failed to start migration transaction")?;

    let steps = migration_steps();
    let latest = steps
        .iter()
        .map(|s| s.to)
        .max()
        .unwrap_or(START_SCHEMA_VERSION);
    let mut cur = current.max(START_SCHEMA_VERSION);

    let mut applied = 0usize;

    while cur < latest {
        let next = cur + 1;

        let Some(step) = steps.iter().find(|s| s.from == cur && s.to == next) else {
            anyhow::bail!(
                "No migration step found for v{} → v{}. Available steps: {}",
                cur,
                next,
                steps
                    .iter()
                    .map(|s| format!("v{}→v{}", s.from, s.to))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        };

        ui::info(format!(
            "Applying migration: v{} → v{} ({})",
            step.from, step.to, step.label
        ));

        if dry_run {
            ui::warning("DRY-RUN: no changes will be applied");
        } else {
            (step.apply)(&tx)?;
            set_schema_version(&tx, step.to)?;
        }

        ui::success(format!("Migration v{} → v{} completed", step.from, step.to));

        applied += 1;
        cur = step.to;
    }

    if dry_run {
        ui::info(format!(
            "Dry-run completed: {applied} migration(s) would be applied."
        ));
        // niente commit
        return Ok(());
    }

    tx.commit().context("Failed to commit migration")?;
    ui::info(format!(
        "Database schema successfully updated (applied {applied} migration(s))."
    ));

    Ok(())
}
