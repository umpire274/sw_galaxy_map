use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbHealth {
    Ok,
    Missing,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct DbStatusReport {
    pub db_path: PathBuf,
    pub health: DbHealth,
    pub file_size_bytes: Option<u64>,
    pub lines: Vec<String>,
    pub warnings: Vec<String>,
}

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

fn push_kv(lines: &mut Vec<String>, label: &str, value: impl std::fmt::Display) {
    lines.push(format!("  {}: {}", label, value));
}

pub fn run(db_arg: Option<String>) -> Result<DbStatusReport> {
    let db_path = resolve_db_path(db_arg)?;
    let mut lines = vec![format!("Database path: {}", db_path.display())];
    let mut warnings = Vec::new();

    if !db_path.exists() {
        warnings.push("Hint: run `sw_galaxy_map db init` to create it.".to_string());
        return Ok(DbStatusReport {
            db_path,
            health: DbHealth::Missing,
            file_size_bytes: None,
            lines,
            warnings,
        });
    }

    let meta_fs = fs::metadata(&db_path).context("Unable to read database file metadata")?;
    let file_size_bytes = meta_fs.len();
    lines.push(format!("Size: {} bytes", file_size_bytes));

    let con = Connection::open(&db_path)
        .with_context(|| format!("Unable to open database: {}", db_path.display()))?;

    let meta_table_ok = has_table(&con, "meta")?;
    if !meta_table_ok {
        warnings.push(
            "Warning: table 'meta' is missing (database not initialized or schema is invalid)"
                .to_string(),
        );
        return Ok(DbStatusReport {
            db_path,
            health: DbHealth::Invalid,
            file_size_bytes: Some(file_size_bytes),
            lines,
            warnings,
        });
    }

    lines.push(String::new());
    lines.push("Meta:".to_string());
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
                "source_lastEditDate" | "source_schemaLastEditDate" | "source_dataLastEditDate" => {
                    if let Some((label, value)) = epoch_millis_iso(&con, k)? {
                        push_kv(&mut lines, &label, value);
                    }
                }
                _ => push_kv(&mut lines, k, v),
            }
        }
    }

    lines.push(String::new());
    lines.push("Counts:".to_string());
    let planets_total = count(&con, "planets")?;
    push_kv(&mut lines, "planets", planets_total);

    let active = con
        .query_row("SELECT COUNT(*) FROM planets WHERE deleted = 0", [], |r| {
            r.get::<_, i64>(0)
        })
        .optional();
    if let Ok(Some(active_n)) = active {
        let deleted_n = planets_total - active_n;
        push_kv(&mut lines, "active_planets (deleted=0)", active_n);
        push_kv(&mut lines, "deleted_planets (deleted=1)", deleted_n);
    }

    if has_table(&con, "planets_unknown")? {
        push_kv(
            &mut lines,
            "planets_unknown",
            count(&con, "planets_unknown")?,
        );
    } else {
        push_kv(&mut lines, "planets_unknown", "-");
    }

    if has_table(&con, "planet_aliases")? {
        push_kv(&mut lines, "planet_aliases", count(&con, "planet_aliases")?);
    } else {
        push_kv(&mut lines, "planet_aliases", "-");
    }

    if has_table(&con, "planet_search")? {
        push_kv(&mut lines, "planet_search", count(&con, "planet_search")?);
    } else {
        push_kv(&mut lines, "planet_search", "-");
    }

    lines.push(String::new());
    lines.push("Schema:".to_string());
    push_kv(
        &mut lines,
        "v_planets_clean",
        if has_view(&con, "v_planets_clean")? {
            "present"
        } else {
            "missing"
        },
    );

    lines.push(String::new());
    lines.push("FTS:".to_string());
    let fts_enabled = get_meta(&con, "fts_enabled")?;
    let meta_flag = matches!(fts_enabled.as_deref(), Some("1"));
    push_kv(
        &mut lines,
        "meta.fts_enabled",
        fts_enabled.as_deref().unwrap_or("-"),
    );

    let fts_table = has_table(&con, "planets_fts")?;
    push_kv(
        &mut lines,
        "planets_fts table",
        if fts_table { "present" } else { "missing" },
    );

    if fts_table {
        push_kv(&mut lines, "planets_fts rows", count(&con, "planets_fts")?);
    }

    if meta_flag && !fts_table {
        warnings
            .push("warning: meta says FTS is enabled but planets_fts table is missing".to_string());
    } else if !meta_flag && fts_table {
        warnings.push("warning: planets_fts exists but meta says FTS is disabled".to_string());
    }

    Ok(DbStatusReport {
        db_path,
        health: DbHealth::Ok,
        file_size_bytes: Some(file_size_bytes),
        lines,
        warnings,
    })
}

fn epoch_millis_iso(con: &Connection, key: &str) -> Result<Option<(String, String)>> {
    if let Some(ms) = get_meta(con, key)?
        && let Ok(ms) = ms.parse::<i64>()
        && let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
    {
        return Ok(Some((format!("{}_iso", key), dt.to_rfc3339())));
    }
    Ok(None)
}
