use anyhow::{Context, Result};
use rusqlite::Connection;

pub fn open_db(path: &str) -> Result<Connection> {
    let con = Connection::open(path).with_context(|| format!("Unable to open database: {path}"))?;
    Ok(con)
}

pub fn has_table(con: &Connection, table: &str) -> Result<bool> {
    let n: i64 = con.query_row(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table' AND name = ?1
        "#,
        [table],
        |r| r.get(0),
    )?;
    Ok(n > 0)
}
