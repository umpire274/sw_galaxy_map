//! Database export command.

use anyhow::{Result, bail};
use rusqlite::Connection;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use sw_galaxy_map_core::db::db_status::resolve_db_path;

use crate::cli::args::DbExportArgs;
use crate::cli::commands::db::utils::human_size;

/// Tables that can be exported through the CLI.
///
/// This whitelist prevents arbitrary SQL table access from user input.
const ALLOWED_TABLES: &[&str] = &[
    "planets",
    "planets_unknown",
    "planet_aliases",
    "planet_search",
    "routes",
    "route_waypoints",
    "route_detours",
    "entity_edit_log",
];

/// Exports a database table to CSV or JSON.
pub fn run(db_override: Option<String>, args: &DbExportArgs) -> Result<()> {
    let table = args.table.trim();

    if table.is_empty() {
        bail!("Table name cannot be empty.");
    }

    if !ALLOWED_TABLES.contains(&table) {
        bail!(
            "Table '{}' is not exportable. Allowed tables: {}",
            table,
            ALLOWED_TABLES.join(", ")
        );
    }

    let db_path = resolve_db_path(db_override)?;
    if !db_path.exists() {
        bail!("Database file not found: {}", db_path.display());
    }

    let con = Connection::open(&db_path)?;

    let dest_dir = match &args.output {
        Some(path) => path.clone(),
        None => prompt_destination_directory()?,
    };
    validate_destination_directory(&dest_dir)?;

    let output_path = build_output_path(&dest_dir, table, args.csv, args.json)?;

    if args.csv {
        export_csv(&con, table, &output_path)?;
        println!("CSV export completed successfully.");
    } else if args.json {
        export_json(&con, table, &output_path)?;
        println!("JSON export completed successfully.");
    } else {
        bail!("You must specify either --csv or --json.");
    }

    let size = std::fs::metadata(&output_path)?.len();

    println!("Size    : {}", human_size(size));
    println!("Saved to: {}", output_path.display());

    Ok(())
}

/// Prompts the user for the destination directory.
fn prompt_destination_directory() -> Result<PathBuf> {
    println!("Enter the destination directory for the export file.");
    println!();

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

/// Ensures that the destination directory exists and is a directory.
fn validate_destination_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("Destination directory does not exist: {}", path.display());
    }

    if !path.is_dir() {
        bail!("Destination path is not a directory: {}", path.display());
    }

    Ok(())
}

/// Builds the final export file path using table name, timestamp, and format.
fn build_output_path(dest_dir: &Path, table: &str, csv: bool, json: bool) -> Result<PathBuf> {
    let ext = if csv {
        "csv"
    } else if json {
        "json"
    } else {
        bail!("You must specify either --csv or --json.");
    };

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let file_name = format!("{}_{}.{}", table, timestamp, ext);

    Ok(dest_dir.join(file_name))
}

/// Exports the selected table as CSV.
fn export_csv(con: &Connection, table: &str, output_path: &Path) -> Result<()> {
    let sql = format!("SELECT * FROM {table}");
    let mut stmt = con.prepare(&sql)?;
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let file = File::create(output_path)?;
    let mut writer = csv::Writer::from_writer(file);

    writer.write_record(&columns)?;

    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let mut record = Vec::with_capacity(columns.len());

        for idx in 0..columns.len() {
            record.push(sqlite_value_to_string(row, idx)?);
        }

        writer.write_record(&record)?;
    }

    writer.flush()?;
    Ok(())
}

/// Exports the selected table as JSON.
fn export_json(con: &Connection, table: &str, output_path: &Path) -> Result<()> {
    let sql = format!("SELECT * FROM {table}");
    let mut stmt = con.prepare(&sql)?;
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let mut rows = stmt.query([])?;
    let mut out = Vec::<Value>::new();

    while let Some(row) = rows.next()? {
        let mut obj = Map::new();

        for (idx, col) in columns.iter().enumerate() {
            obj.insert(col.clone(), sqlite_value_to_json(row, idx)?);
        }

        out.push(Value::Object(obj));
    }

    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &out)?;

    Ok(())
}

/// Converts a SQLite value into a CSV-safe string representation.
fn sqlite_value_to_string(row: &rusqlite::Row<'_>, idx: usize) -> Result<String> {
    let value = row.get_ref(idx)?;

    let text = match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(v) => v.to_string(),
        ValueRef::Real(v) => v.to_string(),
        ValueRef::Text(v) => String::from_utf8_lossy(v).to_string(),
        ValueRef::Blob(v) => format!("<blob:{} bytes>", v.len()),
    };

    Ok(text)
}

/// Converts a SQLite value into a JSON value.
fn sqlite_value_to_json(row: &rusqlite::Row<'_>, idx: usize) -> Result<Value> {
    let value = row.get_ref(idx)?;

    let json = match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::from(v),
        ValueRef::Real(v) => Value::from(v),
        ValueRef::Text(v) => Value::from(String::from_utf8_lossy(v).to_string()),
        ValueRef::Blob(v) => Value::from(format!("<blob:{} bytes>", v.len())),
    };

    Ok(json)
}
