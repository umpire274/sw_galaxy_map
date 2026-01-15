use anyhow::Result;
use rusqlite::Connection;

use crate::{db, normalize::normalize_text};

pub fn run(con: &Connection, query: String, limit: i64) -> Result<()> {
    let qn = normalize_text(&query);
    let rows = db::search_planets(con, &qn, limit)?;

    if rows.is_empty() {
        println!("No results found for: {query}");
    } else {
        for (fid, name) in rows {
            println!("{fid}\t{name}");
        }
    }

    Ok(())
}
