use anyhow::Result;
use sqlx::{Connection, PgConnection};

use crate::db::config::DbConfig;
use crate::models::{CsvOverlayOutcome, CsvOverlayRow, PlanetDbRow};
use crate::models::{NormalizedPlanet, UpsertOutcome};
use crate::utils::{cmp_key, same_overlay_fields, strip_roman_suffix};
use indicatif::ProgressBar;
use sqlx::postgres::PgConnectOptions;

pub fn build_postgres_connect_options(cfg: &DbConfig) -> Result<PgConnectOptions> {
    let username = cfg
        .username
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL username is required"))?;

    let password = cfg
        .password
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL password is required"))?;

    let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
    let port = cfg.port.unwrap_or(5432);

    let database = cfg
        .database
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("PostgreSQL database is required"))?;

    Ok(PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(username)
        .password(password)
        .database(database))
}

pub async fn test_connection(cfg: &DbConfig) -> Result<()> {
    let db_options = build_postgres_connect_options(cfg)?;
    let mut conn = PgConnection::connect_with(&db_options).await?;

    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&mut conn)
        .await?;

    println!();
    println!("Connected successfully.");
    println!("Server version: {}", version.0);

    Ok(())
}

/// Creates the initial PostgreSQL schema required by sw_galaxy_map_sync.
///
/// This is the PostgreSQL equivalent of the SQLite schema baseline used by
/// the sync pipeline.
pub async fn bootstrap_schema(cfg: &DbConfig) -> Result<()> {
    let db_options = build_postgres_connect_options(cfg)?;
    let mut conn = PgConnection::connect_with(&db_options).await?;

    crate::db::schema::postgres::create_postgres_schema(&mut conn).await?;

    println!();
    println!("PostgreSQL schema bootstrap completed.");
    println!("Created/verified tables: meta, planets, planets_unknown");

    Ok(())
}

/// Inserts or updates a normalized ArcGIS planet into the PostgreSQL `planets` table.
///
/// The upsert key is the ArcGIS `FID`.
/// If the stored `arcgis_hash` is unchanged, the row is skipped.
pub async fn upsert_known_arcgis_planet(
    conn: &mut PgConnection,
    planet: &NormalizedPlanet,
) -> Result<UpsertOutcome> {
    let existing_hash: Option<String> = sqlx::query_scalar(
        r#"
        SELECT arcgis_hash
        FROM planets
        WHERE FID = $1
        LIMIT 1
        "#,
    )
    .bind(planet.fid)
    .fetch_optional(&mut *conn)
    .await?;

    if existing_hash.as_deref() == Some(planet.arcgis_hash.as_str()) {
        return Ok(UpsertOutcome::Skipped);
    }

    sqlx::query(
        r#"
        INSERT INTO planets (
            FID,
            Planet,
            planet_norm,
            Region,
            Sector,
            System,
            Grid,
            X,
            Y,
            arcgis_hash,
            deleted,
            Canon,
            Legends,
            status,
            grid_unit
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, 0, $11, $12, $13, $14
        )
        ON CONFLICT (FID) DO UPDATE SET
            Planet = EXCLUDED.Planet,
            planet_norm = EXCLUDED.planet_norm,
            Region = EXCLUDED.Region,
            Sector = EXCLUDED.Sector,
            System = EXCLUDED.System,
            Grid = EXCLUDED.Grid,
            X = EXCLUDED.X,
            Y = EXCLUDED.Y,
            arcgis_hash = EXCLUDED.arcgis_hash,
            deleted = 0,
            Canon = EXCLUDED.Canon,
            Legends = EXCLUDED.Legends,
            status = EXCLUDED.status,
            grid_unit = EXCLUDED.grid_unit
        "#,
    )
    .bind(planet.fid)
    .bind(&planet.planet)
    .bind(&planet.planet_norm)
    .bind(&planet.region)
    .bind(&planet.sector)
    .bind(&planet.system)
    .bind(&planet.grid)
    .bind(planet.x.unwrap_or(0.0))
    .bind(planet.y.unwrap_or(0.0))
    .bind(&planet.arcgis_hash)
    .bind(planet.canon)
    .bind(planet.legends)
    .bind("active")
    .bind("pc")
    .execute(&mut *conn)
    .await?;

    Ok(if existing_hash.is_some() {
        UpsertOutcome::Updated
    } else {
        UpsertOutcome::Inserted
    })
}

/// Inserts or updates an unnamed ArcGIS planet record into the PostgreSQL `planets_unknown` table.
///
/// The upsert key is the ArcGIS `FID`.
/// If the stored `arcgis_hash` is unchanged, the row is skipped.
pub async fn upsert_unknown_arcgis_planet(
    conn: &mut PgConnection,
    planet: &NormalizedPlanet,
) -> Result<UpsertOutcome> {
    let existing_hash: Option<String> = sqlx::query_scalar(
        r#"
        SELECT arcgis_hash
        FROM planets_unknown
        WHERE FID = $1
        LIMIT 1
        "#,
    )
    .bind(planet.fid)
    .fetch_optional(&mut *conn)
    .await?;

    if existing_hash.as_deref() == Some(planet.arcgis_hash.as_str()) {
        return Ok(UpsertOutcome::Skipped);
    }

    sqlx::query(
        r#"
        INSERT INTO planets_unknown (
            FID,
            Planet,
            planet_norm,
            Region,
            Sector,
            System,
            Grid,
            X,
            Y,
            arcgis_hash,
            deleted,
            Canon,
            Legends,
            status,
            grid_unit
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, 0, $11, $12, $13, $14
        )
        ON CONFLICT (FID) DO UPDATE SET
            Planet = EXCLUDED.Planet,
            planet_norm = EXCLUDED.planet_norm,
            Region = EXCLUDED.Region,
            Sector = EXCLUDED.Sector,
            System = EXCLUDED.System,
            Grid = EXCLUDED.Grid,
            X = EXCLUDED.X,
            Y = EXCLUDED.Y,
            arcgis_hash = EXCLUDED.arcgis_hash,
            deleted = 0,
            Canon = EXCLUDED.Canon,
            Legends = EXCLUDED.Legends,
            status = EXCLUDED.status,
            grid_unit = EXCLUDED.grid_unit
        "#,
    )
    .bind(planet.fid)
    .bind(&planet.planet)
    .bind(&planet.planet_norm)
    .bind(&planet.region)
    .bind(&planet.sector)
    .bind(&planet.system)
    .bind(&planet.grid)
    .bind(planet.x.unwrap_or(0.0))
    .bind(planet.y.unwrap_or(0.0))
    .bind(&planet.arcgis_hash)
    .bind(planet.canon)
    .bind(planet.legends)
    .bind("unknown")
    .bind("pc")
    .execute(&mut *conn)
    .await?;

    Ok(if existing_hash.is_some() {
        UpsertOutcome::Updated
    } else {
        UpsertOutcome::Inserted
    })
}

pub async fn upsert_meta_struct<T>(conn: &mut PgConnection, key: &str, value: &T) -> Result<()>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value)?;

    sqlx::query(
        r#"
        INSERT INTO meta (key, value)
        VALUES ($1, $2)
        ON CONFLICT (key) DO UPDATE SET
            value = EXCLUDED.value
        "#,
    )
    .bind(key)
    .bind(json)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

pub async fn apply_csv_overlay_row_postgres(
    conn: &mut PgConnection,
    row: &CsvOverlayRow,
    dry_run: bool,
    progress: Option<&ProgressBar>,
) -> Result<CsvOverlayOutcome> {
    let exact = find_postgres_planet_exact(conn, &row.system).await?;

    let existing = match exact {
        Some(db_row) => Some((db_row, false)),
        None => find_postgres_planet_suffix(conn, &row.system)
            .await?
            .map(|db_row| (db_row, true)),
    };

    let outcome = match existing {
        Some((db_row, suffix_match)) => {
            if same_overlay_fields(
                &db_row.sector,
                &db_row.region,
                &db_row.grid,
                &row.sector,
                &row.region,
                &row.grid,
            ) {
                if !dry_run {
                    set_postgres_status(conn, db_row.fid, "active").await?
                };
                CsvOverlayOutcome::Active
            } else {
                if !dry_run {
                    update_postgres_overlay_fields(conn, db_row.fid, row).await?
                };

                if suffix_match {
                    CsvOverlayOutcome::ModifiedSuffix
                } else {
                    CsvOverlayOutcome::ModifiedExact
                }
            }
        }
        None => {
            if !dry_run {
                insert_postgres_curated_only_row(conn, row).await?
            };
            CsvOverlayOutcome::Inserted
        }
    };

    if let Some(pb) = progress {
        pb.inc(1);
    }

    Ok(outcome)
}

pub async fn mark_deleted_not_in_csv_postgres(
    conn: &mut PgConnection,
    csv_rows: &[CsvOverlayRow],
    dry_run: bool,
    progress: Option<&ProgressBar>,
) -> Result<(usize, usize)> {
    let db_rows = load_postgres_planet_rows(conn).await?;

    let mut deleted = 0usize;
    let mut skipped = 0usize;

    for db_row in db_rows {
        if postgres_exists_in_csv(&db_row, csv_rows) {
            if db_row.status.trim().is_empty() {
                skipped += 1;

                if !dry_run {
                    set_postgres_status(conn, db_row.fid, "skipped").await?;
                }
            }
        } else {
            deleted += 1;

            if !dry_run {
                set_postgres_status(conn, db_row.fid, "deleted").await?;
            }
        }

        if let Some(pb) = progress {
            pb.inc(1);
        }
    }

    Ok((deleted, skipped))
}

pub async fn count_postgres_planet_rows(conn: &mut PgConnection) -> Result<usize> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM planets
        WHERE deleted = 0
        "#,
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(count as usize)
}

async fn find_postgres_planet_exact(
    conn: &mut PgConnection,
    system: &str,
) -> Result<Option<PlanetDbRow>> {
    let key = cmp_key(system);

    let row = sqlx::query_as::<_, PlanetDbRow>(
        r#"
        SELECT
            FID AS fid,
            Planet AS planet,
            COALESCE(Sector, '') AS sector,
            COALESCE(Region, '') AS region,
            COALESCE(Grid, '') AS grid,
            COALESCE(status, '') AS status
        FROM planets
        WHERE deleted = 0
          AND planet_norm = $1
        LIMIT 1
        "#,
    )
    .bind(key)
    .fetch_optional(&mut *conn)
    .await?;

    Ok(row)
}

async fn find_postgres_planet_suffix(
    conn: &mut PgConnection,
    system: &str,
) -> Result<Option<PlanetDbRow>> {
    let target_base = strip_roman_suffix(&cmp_key(system));

    let rows = load_postgres_planet_rows(conn).await?;

    let found = rows.into_iter().find(|row| {
        let db_name = cmp_key(&row.planet);
        let db_base = strip_roman_suffix(&db_name);

        db_name == target_base || db_base == target_base
    });

    Ok(found)
}

async fn load_postgres_planet_rows(conn: &mut PgConnection) -> Result<Vec<PlanetDbRow>> {
    let rows = sqlx::query_as::<_, PlanetDbRow>(
        r#"
        SELECT
            FID AS fid,
            Planet AS planet,
            COALESCE(Sector, '') AS sector,
            COALESCE(Region, '') AS region,
            COALESCE(Grid, '') AS grid,
            COALESCE(status, '') AS status
        FROM planets
        WHERE deleted = 0
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(rows)
}

fn postgres_exists_in_csv(db_row: &PlanetDbRow, csv_rows: &[CsvOverlayRow]) -> bool {
    let db_name = cmp_key(&db_row.planet);
    let db_base = strip_roman_suffix(&db_name);

    csv_rows.iter().any(|row| {
        let csv_name = cmp_key(&row.system);
        let csv_base = strip_roman_suffix(&csv_name);

        db_name == csv_name || db_base == csv_name || db_name == csv_base || db_base == csv_base
    })
}

async fn set_postgres_status(conn: &mut PgConnection, fid: i64, status: &str) -> Result<()> {
    let deleted = i32::from(status.eq_ignore_ascii_case("deleted"));

    sqlx::query(
        r#"
        UPDATE planets
        SET status = $2,
            deleted = $3
        WHERE FID = $1
        "#,
    )
    .bind(fid)
    .bind(status)
    .bind(deleted)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

async fn update_postgres_overlay_fields(
    conn: &mut PgConnection,
    fid: i64,
    row: &CsvOverlayRow,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE planets
        SET
            Sector = $2,
            Region = $3,
            Grid = $4,
            status = $5,
            deleted = 0
        WHERE FID = $1
        "#,
    )
    .bind(fid)
    .bind(&row.sector)
    .bind(&row.region)
    .bind(&row.grid)
    .bind("modified")
    .execute(&mut *conn)
    .await?;

    Ok(())
}

async fn insert_postgres_curated_only_row(
    conn: &mut PgConnection,
    row: &CsvOverlayRow,
) -> Result<()> {
    let fid = next_negative_postgres_fid(conn).await?;
    let planet_norm = cmp_key(&row.system);

    sqlx::query(
        r#"
        INSERT INTO planets (
            FID,
            Planet,
            planet_norm,
            Region,
            Sector,
            System,
            Grid,
            X,
            Y,
            arcgis_hash,
            deleted,
            Canon,
            Legends,
            status,
            grid_unit
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            0.0, 0.0, '',
            0, 1, 0, 'inserted', 'pc'
        )
        "#,
    )
    .bind(fid)
    .bind(&row.system)
    .bind(&planet_norm)
    .bind(&row.region)
    .bind(&row.sector)
    .bind(&row.system)
    .bind(&row.grid)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

async fn next_negative_postgres_fid(conn: &mut PgConnection) -> Result<i64> {
    let min_fid: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT MIN(FID)
        FROM planets
        WHERE FID < 0
        "#,
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(min_fid.unwrap_or(0) - 1)
}
