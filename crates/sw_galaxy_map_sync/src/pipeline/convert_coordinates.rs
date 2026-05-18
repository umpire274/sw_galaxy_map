use crate::cli::{CoordinateUnitArg, DbDriverArg};
use crate::db::config::load_db_config;
use crate::db::postgres::build_postgres_connect_options;
use crate::progress::spinner;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use sqlx::{Connection as SqlxConnection, PgConnection};
use std::io::{self, Write};
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

#[derive(Debug, Clone)]
struct CoordinateBackupEntry {
    index: usize,
    backup_timestamp: String,
    rows: usize,
    grid_unit: String,
}

#[derive(Debug, Default, Clone)]
pub struct ConvertRollbackStats {
    pub rows_restored: usize,
    pub backup_timestamp: String,
    pub grid_unit: String,
    pub dry_run: bool,
}

pub const PC_TO_LY: f64 = 3.26156;

pub fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

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

    println!();
    let pb = spinner("Converting SQLite coordinates...");

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

    pb.finish_with_message("SQLite coordinate conversion completed.");
    println!();

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
    options: &ConvertCoordinatesOptions,
) -> Result<ConvertCoordinatesStats> {
    let db_config = options
        .db_config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--db-config is required when --driver postgres"))?;

    let cfg = load_db_config(db_config)?;
    let connect_options = build_postgres_connect_options(&cfg)?;
    let mut conn = PgConnection::connect_with(&connect_options).await?;

    let target_unit = match options.to {
        CoordinateUnitArg::Pc => "pc",
        CoordinateUnitArg::Ly => "ly",
    };

    let current_unit: String = sqlx::query_scalar(
        r#"
        SELECT grid_unit
        FROM planets
        WHERE grid_unit IS NOT NULL
        LIMIT 1
        "#,
    )
    .fetch_one(&mut conn)
    .await?;

    if current_unit.eq_ignore_ascii_case(target_unit) {
        bail!("Coordinates are already stored in '{target_unit}'. Nothing to convert.");
    }

    let rows_checked: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM planets")
        .fetch_one(&mut conn)
        .await?;

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

    let mut tx = conn.begin().await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS planets_coordinates_backup (
            fid BIGINT NOT NULL,
            x DOUBLE PRECISION NOT NULL,
            y DOUBLE PRECISION NOT NULL,
            grid_unit TEXT NOT NULL,
            backup_timestamp TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    let backed_up = sqlx::query(
        r#"
        INSERT INTO planets_coordinates_backup (
            fid,
            x,
            y,
            grid_unit,
            backup_timestamp
        )
        SELECT
            FID,
            X,
            Y,
            grid_unit,
            $1
        FROM planets
        "#,
    )
    .bind(&backup_timestamp)
    .execute(&mut *tx)
    .await?
    .rows_affected() as usize;

    println!();
    let spinner = spinner("Converting PostgreSQL coordinates...");

    let converted = sqlx::query(
        r#"
        UPDATE planets
        SET
            X = ROUND((X * $1)::numeric, 2)::double precision,
            Y = ROUND((Y * $1)::numeric, 2)::double precision,
            grid_unit = $2
        "#,
    )
    .bind(factor)
    .bind(target_unit)
    .execute(&mut *tx)
    .await?
    .rows_affected() as usize;

    spinner.finish_with_message("PostgreSQL coordinate conversion completed.");
    println!();

    tx.commit().await?;

    Ok(ConvertCoordinatesStats {
        rows_checked: rows_checked as usize,
        rows_converted: converted,
        rows_backed_up: backed_up,
        from_unit: current_unit,
        to_unit: target_unit.to_string(),
        dry_run: false,
    })
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

pub fn print_convert_rollback_summary(stats: &ConvertRollbackStats) {
    println!();
    println!("Coordinate rollback completed.");
    println!("Backup timestamp : {}", stats.backup_timestamp);
    println!("Rows restored    : {}", stats.rows_restored);
    println!("Grid unit        : {}", stats.grid_unit);
    println!("Dry run          : {}", stats.dry_run);
}

pub fn rollback_coordinates_sqlite(
    options: &ConvertCoordinatesOptions,
) -> Result<ConvertRollbackStats> {
    let db_path = options
        .db
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--db is required when --driver sqlite"))?;

    let mut conn = Connection::open(db_path)
        .with_context(|| format!("Unable to open SQLite DB: {}", db_path.display()))?;

    ensure_backup_table_exists_sqlite(&conn)?;

    let backups = load_coordinate_backups_sqlite(&conn)?;

    if backups.is_empty() {
        bail!("No coordinate backups found.");
    }

    println!();
    println!("Available coordinate backups:");
    println!();

    for backup in &backups {
        println!(
            "[{}] {} - {} rows - {}",
            backup.index, backup.backup_timestamp, backup.rows, backup.grid_unit
        );
    }

    println!();
    print!("Select backup to restore: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selected_index: usize = input.trim().parse().context("Invalid backup selection")?;

    let selected = backups
        .iter()
        .find(|entry| entry.index == selected_index)
        .ok_or_else(|| anyhow::anyhow!("Selected backup does not exist"))?;

    println!();
    println!(
        "This will restore {} coordinate rows from backup timestamp {}.",
        selected.rows, selected.backup_timestamp
    );
    println!("Target grid_unit: {}", selected.grid_unit);
    print!("Continue? [y/N]: ");
    io::stdout().flush()?;

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;

    if !confirm.trim().eq_ignore_ascii_case("y") {
        bail!("Rollback cancelled.");
    }

    if options.dry_run {
        return Ok(ConvertRollbackStats {
            rows_restored: selected.rows,
            backup_timestamp: selected.backup_timestamp.clone(),
            grid_unit: selected.grid_unit.clone(),
            dry_run: true,
        });
    }

    println!();
    let pb = spinner("Rolling back SQLite coordinates...");

    let tx = conn.transaction()?;

    let restored = tx.execute(
        r#"
        UPDATE planets
        SET
            X = (
                SELECT b.X
                FROM planets_coordinates_backup b
                WHERE b.fid = planets.FID
                  AND b.backup_timestamp = ?1
            ),
            Y = (
                SELECT b.Y
                FROM planets_coordinates_backup b
                WHERE b.fid = planets.FID
                  AND b.backup_timestamp = ?1
            ),
            grid_unit = (
                SELECT b.grid_unit
                FROM planets_coordinates_backup b
                WHERE b.fid = planets.FID
                  AND b.backup_timestamp = ?1
            )
        WHERE EXISTS (
            SELECT 1
            FROM planets_coordinates_backup b
            WHERE b.fid = planets.FID
              AND b.backup_timestamp = ?1
        )
        "#,
        params![selected.backup_timestamp],
    )?;

    tx.commit()?;

    pb.finish_with_message("SQLite coordinate rollback completed.");
    println!();

    Ok(ConvertRollbackStats {
        rows_restored: restored,
        backup_timestamp: selected.backup_timestamp.clone(),
        grid_unit: selected.grid_unit.clone(),
        dry_run: false,
    })
}

pub async fn rollback_coordinates_postgres(
    options: &ConvertCoordinatesOptions,
) -> Result<ConvertRollbackStats> {
    let db_config = options
        .db_config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--db-config is required when --driver postgres"))?;

    let cfg = load_db_config(db_config)?;
    let connect_options = build_postgres_connect_options(&cfg)?;

    let mut conn = PgConnection::connect_with(&connect_options).await?;

    ensure_backup_table_exists_postgres(&mut conn).await?;

    let backups = load_coordinate_backups_postgres(&mut conn).await?;

    if backups.is_empty() {
        bail!("No coordinate backups found.");
    }

    println!();
    println!("Available coordinate backups:");
    println!();

    for backup in &backups {
        println!(
            "[{}] {} - {} rows - {}",
            backup.index, backup.backup_timestamp, backup.rows, backup.grid_unit
        );
    }

    println!();
    print!("Select backup to restore: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selected_index: usize = input.trim().parse().context("Invalid backup selection")?;

    let selected = backups
        .iter()
        .find(|entry| entry.index == selected_index)
        .ok_or_else(|| anyhow::anyhow!("Selected backup does not exist"))?;

    println!();
    println!(
        "This will restore {} coordinate rows from backup timestamp {}.",
        selected.rows, selected.backup_timestamp
    );
    println!("Target grid_unit: {}", selected.grid_unit);

    print!("Continue? [y/N]: ");
    io::stdout().flush()?;

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;

    if !confirm.trim().eq_ignore_ascii_case("y") {
        bail!("Rollback cancelled.");
    }

    if options.dry_run {
        return Ok(ConvertRollbackStats {
            rows_restored: selected.rows,
            backup_timestamp: selected.backup_timestamp.clone(),
            grid_unit: selected.grid_unit.clone(),
            dry_run: true,
        });
    }

    let mut tx = conn.begin().await?;

    println!();
    let spinner = spinner("Rolling back PostgreSQL coordinates...");

    let restored = sqlx::query(
        r#"
        UPDATE planets
        SET
            X = b.X,
            Y = b.Y,
            grid_unit = b.grid_unit
        FROM planets_coordinates_backup b
        WHERE planets.FID = b.fid
          AND b.backup_timestamp = $1
        "#,
    )
    .bind(&selected.backup_timestamp)
    .execute(&mut *tx)
    .await?
    .rows_affected() as usize;

    spinner.finish_with_message("PostgreSQL coordinate rollback completed.");
    println!();

    tx.commit().await?;

    Ok(ConvertRollbackStats {
        rows_restored: restored,
        backup_timestamp: selected.backup_timestamp.clone(),
        grid_unit: selected.grid_unit.clone(),
        dry_run: false,
    })
}

fn ensure_backup_table_exists_sqlite(conn: &Connection) -> Result<()> {
    let exists: i64 = conn.query_row(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table'
          AND name = 'planets_coordinates_backup'
        "#,
        [],
        |row| row.get(0),
    )?;

    if exists == 0 {
        bail!("Backup table 'planets_coordinates_backup' does not exist.");
    }

    Ok(())
}

async fn ensure_backup_table_exists_postgres(conn: &mut PgConnection) -> Result<()> {
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_name = 'planets_coordinates_backup'
        )
        "#,
    )
    .fetch_one(conn)
    .await?;

    if !exists {
        bail!("Backup table 'planets_coordinates_backup' does not exist.");
    }

    Ok(())
}

fn coordinate_backup_entries_from_rows(
    rows: Vec<(String, i64, String)>,
) -> Vec<CoordinateBackupEntry> {
    rows.into_iter()
        .enumerate()
        .map(
            |(idx, (backup_timestamp, rows_count, grid_unit))| CoordinateBackupEntry {
                index: idx + 1,
                backup_timestamp,
                rows: rows_count as usize,
                grid_unit,
            },
        )
        .collect()
}

fn load_coordinate_backups_sqlite(conn: &Connection) -> anyhow::Result<Vec<CoordinateBackupEntry>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT backup_timestamp, COUNT(*) AS rows_count, COALESCE(MIN(grid_unit), '') AS grid_unit
        FROM planets_coordinates_backup
        GROUP BY backup_timestamp
        ORDER BY backup_timestamp DESC
        "#,
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(coordinate_backup_entries_from_rows(rows))
}

async fn load_coordinate_backups_postgres(
    conn: &mut PgConnection,
) -> anyhow::Result<Vec<CoordinateBackupEntry>> {
    let rows = sqlx::query_as::<_, (String, i64, String)>(
        r#"
        SELECT backup_timestamp, COUNT(*) AS rows_count, COALESCE(MIN(grid_unit), '') AS grid_unit
        FROM planets_coordinates_backup
        GROUP BY backup_timestamp
        ORDER BY backup_timestamp DESC
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(coordinate_backup_entries_from_rows(rows))
}
