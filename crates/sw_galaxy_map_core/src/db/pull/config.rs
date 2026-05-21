//! Remote database configuration for local pull workflows.

use std::path::Path;

/// Supported remote database drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDbDriver {
    /// PostgreSQL remote canonical database.
    Postgres,
}

/// Remote database connection configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDbConfig {
    /// Remote backend driver.
    pub driver: RemoteDbDriver,

    /// Remote database host.
    pub host: String,

    /// Remote database port.
    pub port: u16,

    /// Remote database name.
    pub database: String,

    /// Remote database username.
    pub username: String,

    /// Remote database password.
    pub password: String,
}

impl RemoteDbConfig {
    /// Loads a remote database configuration from a JSON file.
    pub fn from_json_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|err| {
            anyhow::anyhow!("Unable to read remote DB config {}: {err}", path.display())
        })?;

        let raw: RawRemoteDbConfig = serde_json::from_str(&content).map_err(|err| {
            anyhow::anyhow!("Invalid remote DB config JSON {}: {err}", path.display())
        })?;

        raw.try_into()
    }
}

#[derive(Debug, serde::Deserialize)]
struct RawRemoteDbConfig {
    driver: Option<String>,
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
}

impl TryFrom<RawRemoteDbConfig> for RemoteDbConfig {
    type Error = anyhow::Error;

    fn try_from(value: RawRemoteDbConfig) -> Result<Self, Self::Error> {
        let driver = match value.driver.as_deref().unwrap_or("postgres") {
            "postgres" | "postgresql" => RemoteDbDriver::Postgres,
            other => anyhow::bail!("Unsupported remote DB driver: {other}"),
        };

        Ok(Self {
            driver,
            host: value.host,
            port: value.port,
            database: value.database,
            username: value.username,
            password: value.password,
        })
    }
}
