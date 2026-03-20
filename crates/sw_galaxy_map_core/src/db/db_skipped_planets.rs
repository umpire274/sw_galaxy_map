use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use serde_json;

#[derive(Debug, Serialize)]
struct SkippedPlanetRow {
    id: i64,
    fid: Option<i64>,
    planet: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    reason: String,
}

pub fn run(con: &mut Connection) -> Result<()> {
    let mut stmt = con
        .prepare(
            r#"
            SELECT id, fid, planet, x, y, reason
            FROM planets_unknown
            ORDER BY id
            "#,
        )
        .context("Failed to query skipped planets table")?;

    let rows = stmt
        .query_map([], |r| {
            Ok(SkippedPlanetRow {
                id: r.get(0)?,
                fid: r.get(1)?,
                planet: r.get(2)?,
                x: r.get(3)?,
                y: r.get(4)?,
                reason: r.get(5)?,
            })
        })
        .context("Failed to read skipped planets rows")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }

    let json =
        serde_json::to_string_pretty(&out).context("Failed to encode skipped planets JSON")?;
    println!("{json}");

    Ok(())
}
