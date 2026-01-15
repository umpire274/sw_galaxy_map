use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;

pub fn resolve_db_path(db_arg: Option<String>) -> Result<PathBuf> {
    Ok(match db_arg {
        Some(p) => PathBuf::from(p),
        None => crate::provision::paths::default_db_path()?,
    })
}

fn get_meta(con: &Connection, key: &str) -> Result<Option<String>> {
    con.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .context("Failed to read meta")
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
        println!("Status: MISSING");
        println!("Hint: run `sw_galaxy_map db init` to create it.");
        return Ok(());
    }

    let meta = fs::metadata(&db_path).context("Unable to read database file metadata")?;
    println!("Status: OK");
    println!("Size: {} bytes", meta.len());

    let con = Connection::open(&db_path)
        .with_context(|| format!("Unable to open database: {}", db_path.display()))?;

    // Meta (best-effort: missing keys are fine)
    let imported_at = get_meta(&con, "imported_at_utc")?;
    let source_id = get_meta(&con, "source_serviceItemId")?;
    let dataset_version = get_meta(&con, "dataset_version")?;
    let importer_version = get_meta(&con, "importer_version")?;
    let fts_enabled = get_meta(&con, "fts_enabled")?;

    println!();
    println!("Meta:");
    println!(
        "  imported_at_utc: {}",
        imported_at.as_deref().unwrap_or("-")
    );
    println!(
        "  source_serviceItemId: {}",
        source_id.as_deref().unwrap_or("-")
    );
    println!(
        "  dataset_version: {}",
        dataset_version.as_deref().unwrap_or("-")
    );
    println!(
        "  importer_version: {}",
        importer_version.as_deref().unwrap_or("-")
    );
    println!("  fts_enabled: {}", fts_enabled.as_deref().unwrap_or("-"));
    let fts_table = has_table(&con, "planets_fts")?;

    // Counts
    println!();
    println!("Counts:");
    println!("  planets: {}", count(&con, "planets")?);
    println!("  planet_aliases: {}", count(&con, "planet_aliases")?);
    println!("  planet_search: {}", count(&con, "planet_search")?);

    // View check
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
    println!();
    println!("FTS:");
    println!(
        "  meta.fts_enabled: {}",
        fts_enabled.as_deref().unwrap_or("-")
    );
    println!(
        "  planets_fts table: {}",
        if fts_table { "present" } else { "missing" }
    );

    if fts_table {
        println!("  planets_fts rows: {}", count(&con, "planets_fts")?);
    }

    // Mismatch hint (best-effort)
    let meta_flag = matches!(fts_enabled.as_deref(), Some("1"));
    if meta_flag && !fts_table {
        println!("  warning: meta says FTS is enabled but planets_fts table is missing");
    } else if !meta_flag && fts_table {
        println!("  warning: planets_fts exists but meta says FTS is disabled");
    }

    Ok(())
}
