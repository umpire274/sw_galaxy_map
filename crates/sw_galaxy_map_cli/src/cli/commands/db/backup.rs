use anyhow::{Context, Result, bail};
use chrono::Local;
use rusqlite::{Connection, backup::Backup};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use sw_galaxy_map_core::db::db_status::resolve_db_path;

use crate::cli::args::DbBackupArgs;
use crate::cli::commands::db::utils::human_size;

/// Creates a consistent backup copy of the current SQLite database.
pub fn run(db_override: Option<String>, args: &DbBackupArgs) -> Result<()> {
    let db_path = resolve_db_path(db_override)?;

    if !db_path.exists() {
        bail!("Database file not found: {}", db_path.display());
    }

    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_name = format!("sw_galaxy_map-{}.sqlite", timestamp);

    println!("Current database : {}", db_path.display());
    println!("Backup file name : {}", backup_name);
    println!();

    let dest_dir = match &args.output {
        Some(path) => PathBuf::from(path),
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

    let src = Connection::open(&db_path)
        .with_context(|| format!("Failed to open source database '{}'", db_path.display()))?;

    let mut dst = Connection::open(&dest_file).with_context(|| {
        format!(
            "Failed to create destination database '{}'",
            dest_file.display()
        )
    })?;

    {
        let backup = Backup::new(&src, &mut dst)
            .with_context(|| "Failed to initialize SQLite backup".to_string())?;

        backup
            .step(-1)
            .with_context(|| "SQLite backup step failed".to_string())?;
    }

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
