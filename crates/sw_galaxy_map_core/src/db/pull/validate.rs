//! Remote database validation helpers.

use crate::db::pull::config::RemoteDbConfig;
use crate::db::pull::postgres::build_postgres_connect_options;
use sqlx::{Connection as SqlxConnection, PgConnection};

/// Summary of a remote canonical database validation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteValidationReport {
    /// Whether the remote database passed all mandatory checks.
    pub is_valid: bool,

    /// Number of records found in the remote `planets` table.
    pub planets_count: i64,

    /// Number of records found in the remote `planets_unknown` table.
    pub planets_unknown_count: i64,

    /// List of required tables that were found.
    pub found_tables: Vec<String>,

    /// List of required tables that were missing.
    pub missing_tables: Vec<String>,

    /// Human-readable validation messages.
    pub messages: Vec<String>,
}

impl RemoteValidationReport {
    /// Creates an empty validation report.
    pub fn empty() -> Self {
        Self {
            is_valid: false,
            planets_count: 0,
            planets_unknown_count: 0,
            found_tables: Vec::new(),
            missing_tables: Vec::new(),
            messages: Vec::new(),
        }
    }
}

/// Required remote tables for a canonical galaxy database.
///
/// This list intentionally contains only the minimum set required for
/// local pull workflows.
pub const REQUIRED_REMOTE_TABLES: &[&str] =
    &["meta", "planets", "planets_unknown", "planet_search"];

/// Validates that the configured remote PostgreSQL database is usable as a
/// canonical source for local pull workflows.
pub async fn validate_remote_database(
    config: &RemoteDbConfig,
) -> anyhow::Result<RemoteValidationReport> {
    let options = build_postgres_connect_options(config)?;
    let mut conn = PgConnection::connect_with(&options).await?;

    let mut report = RemoteValidationReport::empty();

    let available_tables = load_remote_table_names(&mut conn).await?;

    for required in REQUIRED_REMOTE_TABLES {
        if available_tables.iter().any(|table| table == required) {
            report.found_tables.push((*required).to_string());
        } else {
            report.missing_tables.push((*required).to_string());
        }
    }

    if !report.missing_tables.is_empty() {
        report.messages.push(format!(
            "Missing required remote tables: {}",
            report.missing_tables.join(", ")
        ));

        report.is_valid = false;
        return Ok(report);
    }

    report.planets_count = count_remote_table_rows(&mut conn, "planets").await?;
    report.planets_unknown_count = count_remote_table_rows(&mut conn, "planets_unknown").await?;

    if report.planets_count == 0 {
        report
            .messages
            .push("Remote table 'planets' is empty.".to_string());
    }

    if report.planets_count < 1000 {
        report.messages.push(format!(
            "Remote table 'planets' has only {} rows; expected a populated canonical dataset.",
            report.planets_count
        ));
    }

    if report.planets_unknown_count == 0 {
        report
            .messages
            .push("Remote table 'planets_unknown' is empty.".to_string());
    }

    report.is_valid = report.messages.is_empty();

    if report.is_valid {
        report.messages.push(format!(
            "Remote database validation passed: {} planets, {} unknown records.",
            report.planets_count, report.planets_unknown_count
        ));
    }

    Ok(report)
}

async fn load_remote_table_names(conn: &mut PgConnection) -> anyhow::Result<Vec<String>> {
    let tables = sqlx::query_scalar::<_, String>(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema = 'public'
          AND table_type = 'BASE TABLE'
        ORDER BY table_name
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(tables)
}

async fn count_remote_table_rows(conn: &mut PgConnection, table: &str) -> anyhow::Result<i64> {
    let sql = match table {
        "planets" => "SELECT COUNT(*) FROM planets",
        "planets_unknown" => "SELECT COUNT(*) FROM planets_unknown",
        other => anyhow::bail!("Unsupported remote count table: {other}"),
    };

    let count = sqlx::query_scalar::<_, i64>(sql).fetch_one(conn).await?;

    Ok(count)
}
