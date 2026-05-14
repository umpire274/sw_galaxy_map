use anyhow::Result;
use serde_json::json;
use sqlx::{Connection, Executor, PgConnection};

use crate::db::config::DbConfig;

fn build_postgres_url(cfg: &DbConfig) -> Result<String> {
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

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS planets (
            FID BIGINT PRIMARY KEY,
            Planet TEXT NOT NULL DEFAULT '',
            planet_norm TEXT NOT NULL DEFAULT '',
            Region TEXT NOT NULL DEFAULT '',
            Sector TEXT NOT NULL DEFAULT '',
            System TEXT NOT NULL DEFAULT '',
            Grid TEXT NOT NULL DEFAULT '',
            X DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            Y DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            arcgis_hash TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            Canon INTEGER NOT NULL DEFAULT 0,
            Legends INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT ''
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS planets_unknown (
            FID BIGINT PRIMARY KEY,
            Planet TEXT NOT NULL DEFAULT '',
            planet_norm TEXT NOT NULL DEFAULT '',
            Region TEXT NOT NULL DEFAULT '',
            Sector TEXT NOT NULL DEFAULT '',
            System TEXT NOT NULL DEFAULT '',
            Grid TEXT NOT NULL DEFAULT '',
            X DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            Y DOUBLE PRECISION NOT NULL DEFAULT 0.0,
            arcgis_hash TEXT NOT NULL DEFAULT '',
            deleted INTEGER NOT NULL DEFAULT 0,
            Canon INTEGER NOT NULL DEFAULT 0,
            Legends INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT ''
        )
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_planets_planet_norm
        ON planets (planet_norm)
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_planets_grid
        ON planets (Grid)
        "#,
    )
    .await?;

    conn.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_planets_deleted
        ON planets (deleted)
        "#,
    )
    .await?;

    let meta = json!({
        "crate_version": env!("CARGO_PKG_VERSION"),
        "operation": "postgres_schema_bootstrap",
        "completed_at_utc": chrono::Utc::now().to_rfc3339(),
        "driver": "postgres",
        "created_tables": [
            "meta",
            "planets",
            "planets_unknown"
        ],
        "created_indexes": [
            "idx_planets_planet_norm",
            "idx_planets_grid",
            "idx_planets_deleted"
        ]
    });

    let meta_json = serde_json::to_string_pretty(&meta)?;

    sqlx::query(
        r#"
        INSERT INTO meta (key, value)
        VALUES ($1, $2)
        ON CONFLICT (key) DO UPDATE SET
            value = EXCLUDED.value
        "#,
    )
    .bind("sync.schema.last_bootstrap")
    .bind(meta_json)
    .execute(&mut conn)
    .await?;

    println!("PostgreSQL schema bootstrap completed.");
    println!("Created/verified tables: meta, planets, planets_unknown");

    Ok(())
}
