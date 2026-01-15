use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use crate::provision::{arcgis, build_sqlite, paths};

pub fn run(out: Option<String>, force: bool) -> Result<()> {
    let out_path: PathBuf = match out {
        Some(p) => PathBuf::from(p),
        None => paths::default_db_path()?,
    };

    paths::ensure_parent_dir(&out_path)?;

    if out_path.exists() {
        if force {
            std::fs::remove_file(&out_path).with_context(|| {
                format!("Unable to remove existing database: {}", out_path.display())
            })?;
        } else if confirm_overwrite(&out_path)? {
            std::fs::remove_file(&out_path).with_context(|| {
                format!("Unable to remove existing database: {}", out_path.display())
            })?;
        } else {
            eprintln!("Aborted. Existing database was not modified.");
            return Ok(());
        }
    }

    println!("Initializing local database at: {}", out_path.display());
    println!("Downloading data from remote service...");

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("Unable to create HTTP client")?;

    let layer = arcgis::fetch_layer_info(&client)?;
    let page_size = if layer.max_record_count > 0 {
        layer.max_record_count
    } else {
        2000
    };

    let features = arcgis::fetch_all_features(&client, page_size)?;
    println!("Downloaded {} features.", features.len());

    let mut con = rusqlite::Connection::open(&out_path)
        .with_context(|| format!("Unable to create SQLite database: {}", out_path.display()))?;

    let enable_fts = build_sqlite::has_fts5(&con);
    build_sqlite::create_schema(&con, enable_fts)?;

    let meta = build_sqlite::BuildMeta {
        imported_at_utc: chrono::Utc::now().to_rfc3339(),
        source_service_item_id: layer.service_item_id,
        dataset_version: "C2".to_string(),
        importer_version: "sw_galaxy_map-0.2.0-dev".to_string(),
    };

    println!("Building SQLite database...");
    build_sqlite::insert_all(&mut con, meta, &features, enable_fts)?;

    println!("FTS5 enabled: {}", if enable_fts { "yes" } else { "no" });
    println!("Done.");
    Ok(())
}

fn confirm_overwrite(path: &std::path::Path) -> Result<bool> {
    // Prompt solo se stdin è TTY
    if !atty::is(atty::Stream::Stdin) {
        return Ok(false);
    }

    eprintln!("Database already exists:\n  {}\n", path.display());
    eprint!("Overwrite existing database? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let answer = input.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}
