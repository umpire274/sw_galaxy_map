use crate::model::AliasRow;
use anyhow::Result;
use rusqlite::{Connection, params};

/// Returns all aliases for the given planet FID ordered by source and alias.
pub fn get_aliases(con: &Connection, fid: i64) -> Result<Vec<AliasRow>> {
    let mut stmt = con.prepare(
        r#"
        SELECT alias, source
        FROM planet_aliases
        WHERE planet_fid = ?1
        ORDER BY source, alias
        "#,
    )?;

    let rows = stmt
        .query_map(params![fid], |r| {
            Ok(AliasRow {
                alias: r.get(0)?,
                source: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}
