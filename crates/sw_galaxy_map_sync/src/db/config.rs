use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbDriver {
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    pub driver: DbDriver,

    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,

    pub sqlite_path: Option<String>,

    #[serde(default)]
    pub ssl: bool,
}

/// Loads a database configuration from a JSON file.
pub fn load_db_config(path: &Path) -> anyhow::Result<DbConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Unable to read DB config file: {}", path.display()))?;

    let config = serde_json::from_str::<DbConfig>(&content)
        .with_context(|| format!("Invalid DB config JSON: {}", path.display()))?;

    Ok(config)
}
