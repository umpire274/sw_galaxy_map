use crate::utils::normalize::normalize_text;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub struct BuildMeta {
    pub imported_at_utc: String,
    pub source_service_item_id: String,
    pub dataset_version: String,
    pub importer_version: String,
}

#[derive(Debug, Clone)]
struct SearchRow {
    planet_fid: i64,
    planet: String,
    planet_norm: String,
    aliases: Option<String>,
    aliases_norm: Option<String>,
    search_text: String,
    search_norm: String,
}

pub fn create_schema(con: &Connection, enable_fts: bool) -> Result<()> {
    con.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;

        -- =========================
        -- META
        -- =========================
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- =========================
        -- CORE
        -- =========================
        DROP TABLE IF EXISTS planets;

        CREATE TABLE planets (
            FID         INTEGER PRIMARY KEY,
            Planet      TEXT NOT NULL,
            planet_norm TEXT NOT NULL,

            Region      TEXT,
            Sector      TEXT,
            System      TEXT,
            Grid        TEXT,

            X           REAL NOT NULL,
            Y           REAL NOT NULL,

            -- v0.4.0
            arcgis_hash TEXT NOT NULL,
            deleted     INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0,1)),

            Canon       INTEGER,
            Legends     INTEGER,
            zm          INTEGER,

            name0       TEXT,
            name1       TEXT,
            name2       TEXT,

            lat         REAL,
            long        REAL,

            ref         TEXT,
            status      TEXT,

            CRegion     TEXT,
            CRegion_li  TEXT,

            CHECK (Canon   IS NULL OR Canon   IN (0, 1)),
            CHECK (Legends IS NULL OR Legends IN (0, 1))
        );

        CREATE INDEX IF NOT EXISTS idx_planets             ON planets(Planet,Region,Sector,System,X,Y);
        CREATE INDEX IF NOT EXISTS idx_planets_planet      ON planets(Planet);
        CREATE INDEX IF NOT EXISTS idx_planets_planet_norm ON planets(planet_norm);
        CREATE INDEX IF NOT EXISTS idx_planets_region      ON planets(Region);
        CREATE INDEX IF NOT EXISTS idx_planets_sector      ON planets(Sector);
        CREATE INDEX IF NOT EXISTS idx_planets_system      ON planets(System);
        CREATE INDEX IF NOT EXISTS idx_planets_grid        ON planets(Grid);
        CREATE INDEX IF NOT EXISTS idx_planets_xy          ON planets(X, Y);

        -- =========================
        -- WAYPOINTS (catalog)
        -- =========================
        DROP TABLE IF EXISTS waypoints;

        CREATE TABLE waypoints (
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

        CREATE UNIQUE INDEX IF NOT EXISTS idx_waypoints_name_norm ON waypoints(name_norm);
        CREATE INDEX IF NOT EXISTS idx_waypoints_xy ON waypoints(x, y);

        CREATE TRIGGER IF NOT EXISTS trg_waypoints_updated_at
        AFTER UPDATE ON waypoints
        FOR EACH ROW
        BEGIN
            UPDATE waypoints SET updated_at = datetime('now') WHERE id = OLD.id;
        END;

        -- =========================
        -- ALIASES (N per planet)
        -- =========================
        DROP TABLE IF EXISTS planet_aliases;

        CREATE TABLE planet_aliases (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            planet_fid  INTEGER NOT NULL,
            alias       TEXT NOT NULL,
            alias_norm  TEXT NOT NULL,
            source      TEXT, -- name0/name1/name2/manual
            UNIQUE(planet_fid, alias_norm),
            FOREIGN KEY (planet_fid) REFERENCES planets(FID) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_alias_norm      ON planet_aliases(alias_norm);
        CREATE INDEX IF NOT EXISTS idx_alias_planetfid ON planet_aliases(planet_fid);

        -- =========================
        -- DENORMALIZED SEARCH TABLE
        -- =========================
        DROP TABLE IF EXISTS planet_search;

        CREATE TABLE planet_search (
            planet_fid    INTEGER PRIMARY KEY,
            planet        TEXT NOT NULL,
            planet_norm   TEXT NOT NULL,
            aliases       TEXT,          -- alias raw concatenati
            aliases_norm  TEXT,          -- alias norm concatenati
            search_text   TEXT NOT NULL, -- raw lower (debug/LIKE)
            search_norm   TEXT NOT NULL, -- normalizzato (preferibile)
            FOREIGN KEY (planet_fid) REFERENCES planets(FID) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_search_planet_norm ON planet_search(planet_norm);
        CREATE INDEX IF NOT EXISTS idx_search_norm        ON planet_search(search_norm);

        -- =========================
        -- CLEAN VIEW
        -- =========================
        DROP VIEW IF EXISTS v_planets_clean;

        CREATE VIEW v_planets_clean AS
        SELECT
            p.FID,
            p.Planet,
            p.Region,
            p.Sector,
            p.System,
            p.Grid,
            p.X AS x_parsec,
            p.Y AS y_parsec,
            p.Canon,
            p.Legends,
            p.status,
            p.ref
        FROM planets p
        WHERE p.deleted = 0
        ORDER BY p.Planet COLLATE NOCASE;

        -- =========================
        -- OPTIONAL FTS5
        -- =========================
        "#,
    )?;

    if enable_fts {
        con.execute_batch(
            r#"
            DROP TABLE IF EXISTS planets_fts;

            CREATE VIRTUAL TABLE planets_fts USING fts5(
                planet_fid UNINDEXED,
                search_norm,
                tokenize = 'unicode61'
            );
            "#,
        )?;
    } else {
        // Ensure a clean state if FTS isn't available
        con.execute_batch("DROP TABLE IF EXISTS planets_fts;")?;
    }

    Ok(())
}

fn build_search_row(tx: &Transaction<'_>, fid: i64) -> Result<Option<SearchRow>> {
    let row = tx
        .query_row(
            r#"
            SELECT
                p.FID,
                p.Planet,
                p.planet_norm,
                group_concat(a.alias, ' | ') AS aliases,
                group_concat(a.alias_norm, ' ') AS aliases_norm
            FROM planets p
            LEFT JOIN planet_aliases a ON a.planet_fid = p.FID
            WHERE p.FID = ? AND p.deleted = 0
            GROUP BY p.FID
            "#,
            [fid],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?;

    let Some((planet_fid, planet, planet_norm, aliases, aliases_norm)) = row else {
        return Ok(None);
    };

    let search_text = format!("{} {}", planet, aliases.clone().unwrap_or_default())
        .trim()
        .to_lowercase();

    let search_norm = format!(
        "{} {}",
        planet_norm,
        aliases_norm.clone().unwrap_or_default()
    )
    .trim()
    .to_string();

    Ok(Some(SearchRow {
        planet_fid,
        planet,
        planet_norm,
        aliases,
        aliases_norm,
        search_text,
        search_norm,
    }))
}

fn compute_arcgis_hash(a: &Value) -> String {
    // Build a canonical JSON string by extracting fields in a fixed order.
    // This avoids hash changes due to map ordering or unrelated ArcGIS properties.
    let keys = [
        "FID",
        "Planet",
        "Region",
        "Sector",
        "System",
        "Grid",
        "X",
        "Y",
        "Canon",
        "Legends",
        "zm",
        "name0",
        "name1",
        "name2",
        "lat",
        "long",
        "ref",
        "status",
        "CRegion",
        "CRegion_li",
    ];

    let mut s = String::new();
    for k in keys {
        s.push_str(k);
        s.push('=');

        // Normalize values:
        // - null/missing -> empty
        // - strings -> trimmed
        // - numbers -> stable string
        match a.get(k) {
            None | Some(Value::Null) => {}
            Some(Value::String(v)) => s.push_str(v.trim()),
            Some(Value::Number(n)) => s.push_str(&n.to_string()),
            Some(Value::Bool(b)) => s.push_str(if *b { "1" } else { "0" }),
            Some(other) => s.push_str(&other.to_string()),
        }

        s.push('\n');
    }

    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

fn rebuild_planet_search(tx: &Transaction<'_>) -> Result<()> {
    tx.execute("DELETE FROM planet_search", [])?;

    let mut ins = tx.prepare(
        r#"
        INSERT INTO planet_search(
            planet_fid, planet, planet_norm, aliases, aliases_norm, search_text, search_norm
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(planet_fid) DO UPDATE SET
            planet=excluded.planet,
            planet_norm=excluded.planet_norm,
            aliases=excluded.aliases,
            aliases_norm=excluded.aliases_norm,
            search_text=excluded.search_text,
            search_norm=excluded.search_norm
        "#,
    )?;

    let mut q = tx.prepare("SELECT FID FROM planets WHERE deleted = 0")?;
    let fids = q.query_map([], |r| r.get::<_, i64>(0))?;

    for fid in fids {
        let fid = fid?;
        if let Some(sr) = build_search_row(tx, fid)? {
            ins.execute(rusqlite::params![
                sr.planet_fid,
                sr.planet,
                sr.planet_norm,
                sr.aliases,
                sr.aliases_norm,
                sr.search_text,
                sr.search_norm
            ])?;
        }
    }

    Ok(())
}

fn meta_upsert(con: &Connection, key: &str, value: &str) -> Result<()> {
    con.execute(
        r#"
        INSERT INTO meta(key, value) VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value=excluded.value
        "#,
        params![key, value],
    )?;
    Ok(())
}

pub fn insert_all(
    con: &mut Connection,
    meta: BuildMeta,
    rows: &[Value],
    enable_fts: bool,
) -> Result<()> {
    let tx = con.transaction()?;

    meta_upsert(&tx, "imported_at_utc", &meta.imported_at_utc)?;
    meta_upsert(&tx, "source_serviceItemId", &meta.source_service_item_id)?;
    meta_upsert(&tx, "dataset_version", &meta.dataset_version)?;
    meta_upsert(&tx, "importer_version", &meta.importer_version)?;
    meta_upsert(&tx, "fts_enabled", if enable_fts { "1" } else { "0" })?;
    meta_upsert(&tx, "schema_version", "4")?;

    {
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO planets(
                FID, Planet, planet_norm, Region, Sector, System, Grid,
                X, Y,
                arcgis_hash, deleted,
                Canon, Legends, zm,
                name0, name1, name2,
                lat, long, ref, status, CRegion, CRegion_li
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                ?8, ?9,
                ?10, 0,
                ?11, ?12, ?13,
                ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22
            )
            "#,
        )?;

        let mut stmt_alias = tx.prepare(
            r#"
            INSERT OR IGNORE INTO planet_aliases(planet_fid, alias, alias_norm, source)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )?;

        for a in rows {
            let fid = a
                .get("FID")
                .and_then(|v| v.as_i64())
                .context("Missing FID")?;

            let planet = a
                .get("Planet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            let x = a.get("X").and_then(|v| v.as_f64()).context("Missing X")?;
            let y = a.get("Y").and_then(|v| v.as_f64()).context("Missing Y")?;

            if planet.is_empty() {
                continue;
            }

            let planet_norm = normalize_text(&planet);

            let get_s = |k: &str| a.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
            let get_i = |k: &str| a.get(k).and_then(|v| v.as_i64());
            let get_f = |k: &str| a.get(k).and_then(|v| v.as_f64());
            let arcgis_hash = compute_arcgis_hash(a);

            stmt.execute(params![
                fid,
                planet,
                planet_norm,
                get_s("Region"),
                get_s("Sector"),
                get_s("System"),
                get_s("Grid"),
                x,
                y,
                arcgis_hash,
                get_i("Canon"),
                get_i("Legends"),
                get_i("zm"),
                get_s("name0"),
                get_s("name1"),
                get_s("name2"),
                get_f("lat"),
                get_f("long"),
                get_s("ref"),
                get_s("status"),
                get_s("CRegion"),
                get_s("CRegion_li"),
            ])?;

            for (src, key) in [("name0", "name0"), ("name1", "name1"), ("name2", "name2")] {
                if let Some(val) = a.get(key).and_then(|v| v.as_str()) {
                    let val = val.trim();
                    if !val.is_empty() {
                        stmt_alias.execute(params![fid, val, normalize_text(val), src])?;
                    }
                }
            }
        }
    } // stmt dropped

    // Build denormalized search table (planet_search)
    rebuild_planet_search(&tx)?;
    if enable_fts {
        rebuild_planets_fts(&tx)?;
    }

    tx.commit()?;
    Ok(())
}

pub fn has_fts5(con: &Connection) -> bool {
    // Best-effort detection: try to create a tiny FTS5 virtual table.
    // If the SQLite build lacks FTS5, this will error.
    let ddl = r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS __fts5_test USING fts5(x);
        DROP TABLE __fts5_test;
    "#;

    con.execute_batch(ddl).is_ok()
}

fn rebuild_planets_fts(tx: &Transaction<'_>) -> Result<()> {
    // If the table doesn't exist, this will error.
    // We'll call it only when enable_fts == true.
    tx.execute("DELETE FROM planets_fts", [])?;
    tx.execute(
        r#"
        INSERT INTO planets_fts(planet_fid, search_norm)
        SELECT s.planet_fid, s.search_norm
        FROM planet_search s
        JOIN planets p ON p.FID = s.planet_fid
        WHERE p.deleted = 0
        "#,
        [],
    )?;
    Ok(())
}

pub(crate) fn rebuild_planet_search_public(tx: &Transaction<'_>) -> Result<()> {
    rebuild_planet_search(tx)
}

pub(crate) fn rebuild_planets_fts_if_enabled(tx: &Transaction<'_>) -> Result<()> {
    let enabled: Option<String> = tx
        .query_row(
            "SELECT value FROM meta WHERE key = 'fts_enabled'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if matches!(enabled.as_deref(), Some("1")) {
        rebuild_planets_fts(tx)?;
    }
    Ok(())
}

pub(crate) fn meta_upsert_public(con: &Connection, key: &str, value: &str) -> Result<()> {
    meta_upsert(con, key, value)
}
