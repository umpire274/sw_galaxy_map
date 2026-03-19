use crate::db::{paths, provision};
use crate::provision::arcgis;
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DbInitReport {
    pub out_path: PathBuf,
    pub overwritten_existing: bool,
    pub downloaded_features: usize,
    pub fts_enabled: bool,
}

pub fn run(out: Option<String>, force: bool) -> Result<DbInitReport> {
    let out_path: PathBuf = match out {
        Some(p) => PathBuf::from(p),
        None => paths::default_db_path()?,
    };

    paths::ensure_parent_dir(&out_path)?;
    let mut overwritten_existing = false;

    if out_path.exists() {
        if force {
            std::fs::remove_file(&out_path).with_context(|| {
                format!("Unable to remove existing database: {}", out_path.display())
            })?;
            overwritten_existing = true;
        } else if confirm_overwrite(&out_path)? {
            std::fs::remove_file(&out_path).with_context(|| {
                format!("Unable to remove existing database: {}", out_path.display())
            })?;
            overwritten_existing = true;
        } else {
            anyhow::bail!("Aborted. Existing database was not modified.");
        }
    }

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

    let mut con = rusqlite::Connection::open(&out_path)
        .with_context(|| format!("Unable to create SQLite database: {}", out_path.display()))?;

    let enable_fts = provision::has_fts5(&con);
    provision::create_schema(&con, enable_fts)?;

    let meta = provision::BuildMeta {
        imported_at_utc: chrono::Utc::now().to_rfc3339(),
        source_service_item_id: layer.service_item_id,
        dataset_version: "C2".to_string(),
        importer_version: "sw_galaxy_map-0.2.0-dev".to_string(),
    };

    provision::insert_all(&mut con, meta, &features, enable_fts)?;

    Ok(DbInitReport {
        out_path,
        overwritten_existing,
        downloaded_features: features.len(),
        fts_enabled: enable_fts,
    })
}

fn confirm_overwrite(path: &std::path::Path) -> Result<bool> {
    if !atty::is(atty::Stream::Stdin) {
        return Ok(false);
    }

    eprintln!(
        "Database already exists:
  {}
",
        path.display()
    );
    eprint!("Overwrite existing database? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let answer = input.trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}
