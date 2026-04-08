//! Database backup command.

use anyhow::{Context, Result, bail};
use chrono::Local;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::cli::args::DbBackupArgs;
use crate::cli::commands::db::utils::human_size;
use sw_galaxy_map_core::db::db_status::resolve_db_path;

/// Creates a physical backup copy of the current SQLite database.
pub fn run(args: &DbBackupArgs) -> Result<()> {
    let db_path = resolve_db_path(None)?;

    if !db_path.exists() {
        bail!("Database file not found: {}", db_path.display());
    }

    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_name = format!("sw_galaxy_map-{}.sqlite", timestamp);

    println!("Current database : {}", db_path.display());
    println!("Backup file name : {}", backup_name);
    println!();

    let dest_dir = match &args.output {
        Some(path) => path.clone(),
        None => prompt_destination_directory()?,
    };

    if !dest_dir.exists() {
        bail!(
            "Destination directory does not exist: {}",
            dest_dir.display()
        );
    }

    if !dest_dir.is_dir() {
        bail!(
            "Destination path is not a directory: {}",
            dest_dir.display()
        );
    }

    let dest_file = dest_dir.join(backup_name);

    fs::copy(&db_path, &dest_file).with_context(|| {
        format!(
            "Failed to copy database from '{}' to '{}'",
            db_path.display(),
            dest_file.display()
        )
    })?;

    let size = fs::metadata(&dest_file)?.len();
    println!("Backup created successfully.");
    println!("Size    : {}", human_size(size));
    println!("Saved to: {}", dest_file.display());

    Ok(())
}

/// Prompts the user for the destination directory.
fn prompt_destination_directory() -> Result<PathBuf> {
    print!("Destination directory: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("Destination directory cannot be empty.");
    }

    Ok(PathBuf::from(trimmed))
}
