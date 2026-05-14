use anyhow::Result;
use sqlx::{Connection, PgConnection};

use crate::db::config::DbConfig;
use crate::models::{NormalizedPlanet, UpsertOutcome};

pub(crate) fn build_postgres_url(cfg: &DbConfig) -> Result<String> {
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

    Ok(format!(
        "postgres://{username}:{password}@{host}:{port}/{database}"
    ))
}

pub async fn test_connection(cfg: &DbConfig) -> Result<()> {
    let url = build_postgres_url(cfg)?;

    let mut conn = PgConnection::connect(&url).await?;

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
    let url = build_postgres_url(cfg)?;
    let mut conn = PgConnection::connect(&url).await?;

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
