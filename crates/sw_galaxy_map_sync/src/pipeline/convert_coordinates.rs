use crate::cli::{CoordinateUnitArg, DbDriverArg};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConvertCoordinatesOptions {
    pub driver: DbDriverArg,
    pub db: Option<PathBuf>,
    pub db_config: Option<PathBuf>,
    pub to: CoordinateUnitArg,
    pub dry_run: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ConvertCoordinatesStats {
    pub rows_checked: usize,
    pub rows_converted: usize,
    pub rows_backed_up: usize,
    pub from_unit: String,
    pub to_unit: String,
    pub dry_run: bool,
}

pub const PC_TO_LY: f64 = 3.26156;

pub fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};

pub fn convert_coordinates_sqlite(
    options: &ConvertCoordinatesOptions,
) -> Result<ConvertCoordinatesStats> {
    let db_path = options
        .db
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--db is required when --driver sqlite"))?;

    let mut conn = Connection::open(db_path)
        .with_context(|| format!("Unable to open SQLite DB: {}", db_path.display()))?;

    let target_unit = match options.to {
        CoordinateUnitArg::Pc => "pc",
        CoordinateUnitArg::Ly => "ly",
    };

    let current_unit: String = conn.query_row(
        "SELECT grid_unit FROM planets WHERE grid_unit IS NOT NULL LIMIT 1",
        [],
        |row| row.get(0),
    )?;

    if current_unit.eq_ignore_ascii_case(target_unit) {
        bail!("Coordinates are already stored in '{target_unit}'. Nothing to convert.");
    }

    let rows_checked: i64 = conn.query_row("SELECT COUNT(*) FROM planets", [], |row| row.get(0))?;

    if options.dry_run {
        return Ok(ConvertCoordinatesStats {
            rows_checked: rows_checked as usize,
            rows_converted: rows_checked as usize,
            rows_backed_up: 0,
            from_unit: current_unit,
            to_unit: target_unit.to_string(),
            dry_run: true,
        });
    }

    let factor = match (current_unit.as_str(), target_unit) {
        ("pc", "ly") => PC_TO_LY,
        ("ly", "pc") => 1.0 / PC_TO_LY,
        _ => bail!(
            "Unsupported coordinate conversion: '{}' -> '{}'",
            current_unit,
            target_unit
        ),
    };

    let backup_timestamp = chrono::Utc::now().to_rfc3339();

    let tx = conn.transaction()?;

    tx.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS planets_coordinates_backup (
            fid INTEGER NOT NULL,
            X REAL NOT NULL,
            Y REAL NOT NULL,
            grid_unit TEXT NOT NULL,
            backup_timestamp TEXT NOT NULL
        );
        "#,
    )?;

    let backed_up = tx.execute(
        r#"
        INSERT INTO planets_coordinates_backup (
            fid,
            X,
            Y,
            grid_unit,
            backup_timestamp
        )
        SELECT
            FID,
            X,
            Y,
            grid_unit,
            ?1
        FROM planets
        "#,
        params![backup_timestamp],
    )?;

    let converted = tx.execute(
        r#"
        UPDATE planets
        SET
            X = ROUND(X * ?1, 2),
            Y = ROUND(Y * ?1, 2),
            grid_unit = ?2
        "#,
        params![factor, target_unit],
    )?;

    tx.commit()?;

    Ok(ConvertCoordinatesStats {
        rows_checked: rows_checked as usize,
        rows_converted: converted,
        rows_backed_up: backed_up,
        from_unit: current_unit,
        to_unit: target_unit.to_string(),
        dry_run: false,
    })
}

pub async fn convert_coordinates_postgres(
    _options: &ConvertCoordinatesOptions,
) -> anyhow::Result<ConvertCoordinatesStats> {
    anyhow::bail!("PostgreSQL coordinate conversion is not implemented yet")
}

pub fn print_convert_coordinates_summary(stats: &ConvertCoordinatesStats) {
    println!();
    println!("Coordinate conversion completed.");
    println!("Rows checked     : {}", stats.rows_checked);
    println!("Rows converted   : {}", stats.rows_converted);
    println!("Rows backed up   : {}", stats.rows_backed_up);
    println!("From unit        : {}", stats.from_unit);
    println!("To unit          : {}", stats.to_unit);
    println!("Dry run          : {}", stats.dry_run);
}
