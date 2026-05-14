use anyhow::Result;
use sqlx::{Connection, PgConnection};

use crate::db::config::DbConfig;

pub async fn test_connection(cfg: &DbConfig) -> Result<()> {
    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        cfg.username.as_deref().unwrap_or("postgres"),
        cfg.password.as_deref().unwrap_or(""),
        cfg.host.as_deref().unwrap_or("127.0.0.1"),
        cfg.port.unwrap_or(5432),
        cfg.database.as_deref().unwrap_or("postgres"),
    );

    let mut conn = PgConnection::connect(&url).await?;

    let version: (String,) = sqlx::query_as("SELECT version()")
        .fetch_one(&mut conn)
        .await?;

    println!("Connected successfully.");
    println!("Server version: {}", version.0);

    Ok(())
}
