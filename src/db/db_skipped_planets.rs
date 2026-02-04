use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::db::db_update::META_SKIPPED_PLANETS_JSON;

pub fn run(con: &mut Connection) -> Result<()> {
    let skipped_json: Option<String> = con
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [META_SKIPPED_PLANETS_JSON],
            |r| r.get(0),
        )
        .optional()
        .context("Failed to read skipped planets metadata")?;

    match skipped_json {
        Some(json) => {
            println!("{json}");
        }
        None => {
            println!("[]");
        }
    }

    Ok(())
}
