//! PostgreSQL remote connection helpers.

use crate::db::pull::config::RemoteDbConfig;
use sqlx::postgres::PgConnectOptions;

/// Builds PostgreSQL connection options without interpolating credentials
/// into a raw DSN string.
pub fn build_postgres_connect_options(config: &RemoteDbConfig) -> anyhow::Result<PgConnectOptions> {
    Ok(PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .database(&config.database)
        .username(&config.username)
        .password(&config.password))
}
