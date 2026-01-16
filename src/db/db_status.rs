use crate::ui::{error, success, warning};
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;

pub fn resolve_db_path(db_arg: Option<String>) -> Result<PathBuf> {
    Ok(match db_arg {
        Some(p) => PathBuf::from(p),
        None => crate::db::paths::default_db_path()?,
    })
}

fn get_meta(con: &Connection, key: &str) -> Result<Option<String>> {
    con.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .with_context(|| format!("Failed to read meta key: {}", key))
}

fn count(con: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {}", table);
    let n: i64 = con
        .query_row(&sql, [], |r| r.get(0))
        .with_context(|| format!("Failed to count rows in table: {}", table))?;
    Ok(n)
}

fn has_view(con: &Connection, name: &str) -> Result<bool> {
    let n: i64 = con.query_row(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'view' AND name = ?1
        "#,
        [name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

fn has_table(con: &Connection, name: &str) -> Result<bool> {
    let n: i64 = con.query_row(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table' AND name = ?1
        "#,
        [name],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}

pub fn run(db_arg: Option<String>) -> Result<()> {
    let db_path = resolve_db_path(db_arg)?;

    println!("Database path: {}", db_path.display());

    if !db_path.exists() {
        error("Status: MISSING");
        println!("Hint: run `sw_galaxy_map db init` to create it.");
        return Ok(());
    }

    let meta_fs = fs::metadata(&db_path).context("Unable to read database file metadata")?;
    success("Status: OK");
    println!("Size: {} bytes", meta_fs.len());

    let con = Connection::open(&db_path)
        .with_context(|| format!("Unable to open database: {}", db_path.display()))?;

    // If meta table is missing, this isn't a valid DB for our app (or it's corrupted).
    let meta_table_ok = has_table(&con, "meta")?;
    if !meta_table_ok {
        warning("Warning: table 'meta' is missing (database not initialized or schema is invalid)");
        return Ok(());
    }

    // --- Meta (ordered, best-effort)
    println!();
    println!("Meta:");
    for k in [
        "schema_version",
        "imported_at_utc",
        "last_update_utc",
        "source_serviceItemId",
        "source_currentVersion",
        "source_maxRecordCount",
        "source_lastEditDate",
        "source_schemaLastEditDate",
        "source_dataLastEditDate",
        "dataset_version",
        "importer_version",
        "update_mode",
        "prune_used",
        "fts_enabled",
    ] {
        if let Some(v) = get_meta(&con, k)? {
            match k {
                "source_lastEditDate" => print_epoch_millis_iso(&con, "source_lastEditDate")?,
                "source_schemaLastEditDate" => {
                    print_epoch_millis_iso(&con, "source_schemaLastEditDate")?
                }
                "source_dataLastEditDate" => {
                    print_epoch_millis_iso(&con, "source_dataLastEditDate")?
                }
                _ => {
                    println!("  {}: {}", k, v);
                }
            }
        }
    }

    // --- Counts
    println!();
    println!("Counts:");
    let planets_total = count(&con, "planets")?;
    println!("  planets: {}", planets_total);

    // If 'deleted' column exists, show active vs deleted breakdown.
    // We detect by trying a query; if it fails, we just skip the breakdown.
    let active = con
        .query_row("SELECT COUNT(*) FROM planets WHERE deleted = 0", [], |r| {
            r.get::<_, i64>(0)
        })
        .optional();
    match active {
        Ok(Some(active_n)) => {
            let deleted_n = planets_total - active_n;
            println!("  active_planets (deleted=0): {}", active_n);
            println!("  deleted_planets (deleted=1): {}", deleted_n);
        }
        _ => {
            // older schema: no 'deleted' column or query failed; ignore
        }
    }

    // Related tables (may not exist in partial/old DBs)
    if has_table(&con, "planet_aliases")? {
        println!("  planet_aliases: {}", count(&con, "planet_aliases")?);
    } else {
        println!("  planet_aliases: -");
    }

    if has_table(&con, "planet_search")? {
        println!("  planet_search: {}", count(&con, "planet_search")?);
    } else {
        println!("  planet_search: -");
    }

    // --- Schema checks
    println!();
    println!("Schema:");
    println!(
        "  v_planets_clean: {}",
        if has_view(&con, "v_planets_clean")? {
            "present"
        } else {
            "missing"
        }
    );

    // --- FTS checks
    println!();
    println!("FTS:");
    let fts_enabled = get_meta(&con, "fts_enabled")?;
    let meta_flag = matches!(fts_enabled.as_deref(), Some("1"));
    println!(
        "  meta.fts_enabled: {}",
        fts_enabled.as_deref().unwrap_or("-")
    );

    let fts_table = has_table(&con, "planets_fts")?;
    println!(
        "  planets_fts table: {}",
        if fts_table { "present" } else { "missing" }
    );

    if fts_table {
        println!("  planets_fts rows: {}", count(&con, "planets_fts")?);
    }

    if meta_flag && !fts_table {
        warning("warning: meta says FTS is enabled but planets_fts table is missing");
    } else if !meta_flag && fts_table {
        warning("warning: planets_fts exists but meta says FTS is disabled");
    }

    Ok(())
}

fn print_epoch_millis_iso(con: &Connection, key: &str) -> Result<()> {
    if let Some(ms) = get_meta(con, key)?
        && let Ok(ms) = ms.parse::<i64>()
        && let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
    {
        println!("  {}_iso: {}", key, dt.to_rfc3339());
    }
    Ok(())
}
