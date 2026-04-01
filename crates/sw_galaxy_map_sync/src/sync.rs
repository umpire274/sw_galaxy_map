use anyhow::Result;
use csv::ReaderBuilder;
use rusqlite::{Connection, params};

use crate::db::{
    find_exact_match, find_suffix_match, insert_planet_row, set_status, update_planet_row,
};
use crate::models::{
    DbRow, InsertDefaults, OfficialRow, ReportKind, ReportRow, SyncReport, SyncRow, SyncStats,
};
use crate::report::push_report_row;
use crate::utils::{cmp_key, is_exact_match, normalize_field, strip_roman_suffix};
use std::collections::HashSet;
use std::path::PathBuf;

/// Load and normalize all CSV rows.
pub fn load_csv(path: &PathBuf, delimiter: u8) -> Result<Vec<SyncRow>> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_path(path)?;

    let mut rows = Vec::new();

    for record in rdr.deserialize::<OfficialRow>() {
        let raw = record?;
        rows.push(SyncRow {
            system: normalize_field(&raw.system),
            sector: normalize_field(raw.sector.as_deref().unwrap_or("")),
            region: normalize_field(raw.region.as_deref().unwrap_or("")),
            grid: normalize_field(raw.grid.as_deref().unwrap_or("")),
        });
    }

    Ok(rows)
}

/// Build a ReportRow from SyncRow.
pub fn make_report_row(fid: Option<i64>, row: &SyncRow) -> ReportRow {
    ReportRow {
        fid,
        planet: row.system.clone(),
        sector: row.sector.clone(),
        region: row.region.clone(),
        grid: row.grid.clone(),
    }
}

/// Synchronize one CSV row into the target table.
pub fn sync_row(
    conn: &Connection,
    table: &str,
    row: &SyncRow,
    defaults: &InsertDefaults,
    dry_run: bool,
    stats: &mut SyncStats,
    report: &mut SyncReport,
) -> Result<()> {
    if let Some(existing) = find_exact_match(conn, table, &row.system)? {
        if is_exact_match(&existing, row) {
            if !dry_run {
                set_status(conn, table, existing.fid, "active")?;

                push_report_row(
                    report,
                    make_report_row(Some(existing.fid), row),
                    ReportKind::Active,
                );
            }
        } else {
            if !dry_run {
                update_planet_row(conn, table, existing.fid, row, "modified")?;

                push_report_row(
                    report,
                    make_report_row(Some(existing.fid), row),
                    ReportKind::Modified,
                );
            }
            stats.updated_exact += 1;
        }
        return Ok(());
    }

    if let Some(existing) = find_suffix_match(conn, table, row)? {
        if is_exact_match(&existing, row) {
            if !dry_run {
                set_status(conn, table, existing.fid, "active")?;

                push_report_row(
                    report,
                    make_report_row(Some(existing.fid), row),
                    ReportKind::Active,
                );
            }
        } else {
            if !dry_run {
                update_planet_row(conn, table, existing.fid, row, "modified")?;

                push_report_row(
                    report,
                    make_report_row(Some(existing.fid), row),
                    ReportKind::Modified,
                );
            }
            stats.updated_suffix += 1;
        }
        return Ok(());
    }

    if !dry_run {
        insert_planet_row(conn, table, row, defaults)?;

        push_report_row(report, make_report_row(None, row), ReportKind::Inserted);
    }

    stats.inserted += 1;

    Ok(())
}

/// Check if a DB row exists in CSV using multiple strategies.
pub fn exists_in_csv(
    db: &DbRow,
    name_set: &HashSet<String>,
    key_set: &HashSet<(String, String, String)>,
) -> bool {
    let name = cmp_key(&db.planet);

    if name_set.contains(&name) {
        return true;
    }

    let base = strip_roman_suffix(&name);
    if name_set.contains(&base) {
        return true;
    }

    let key = (cmp_key(&db.sector), cmp_key(&db.region), cmp_key(&db.grid));

    key_set.contains(&key)
}

/// Mark DB records as deleted or skipped after comparing them against the CSV.
pub fn mark_deleted_not_in_csv(
    conn: &Connection,
    table: &str,
    csv_rows: &[SyncRow],
    dry_run: bool,
    stats: &mut SyncStats,
    report: &mut SyncReport,
) -> Result<()> {
    let mut name_set = HashSet::new();
    let mut key_set = HashSet::new();

    for row in csv_rows {
        name_set.insert(cmp_key(&row.system));
        key_set.insert((
            cmp_key(&row.sector),
            cmp_key(&row.region),
            cmp_key(&row.grid),
        ));
    }

    let sql = format!(
        "SELECT FID,
                Planet,
                COALESCE(Sector, ''),
                COALESCE(Region, ''),
                COALESCE(Grid, ''),
                COALESCE(status, '')
         FROM {table}"
    );

    let mut stmt = conn.prepare(&sql)?;
    let mut query_rows = stmt.query([])?;

    let mut db_rows: Vec<DbRow> = Vec::new();

    while let Some(row) = query_rows.next()? {
        db_rows.push(DbRow {
            fid: row.get(0)?,
            planet: row.get(1)?,
            sector: row.get(2)?,
            region: row.get(3)?,
            grid: row.get(4)?,
            status: row.get(5)?,
        });
    }

    let check_pb = crate::utils::make_progress_bar(db_rows.len() as u64, "Check deleted")?;

    let mut to_delete: Vec<DbRow> = Vec::new();
    let mut to_skip: Vec<DbRow> = Vec::new();

    for db in db_rows {
        if exists_in_csv(&db, &name_set, &key_set) {
            if db.status.trim().is_empty() {
                to_skip.push(db);
            }
        } else {
            to_delete.push(db);
        }

        check_pb.inc(1);
    }

    check_pb.finish_with_message("Check deleted completed");

    stats.deleted_logically = to_delete.len();
    stats.skipped_db = to_skip.len();

    if !dry_run {
        let delete_sql = format!("UPDATE {table} SET status = 'deleted' WHERE FID = ?1");
        for db in &to_delete {
            conn.execute(&delete_sql, params![db.fid])?;
        }

        let skip_sql = format!("UPDATE {table} SET status = 'skipped' WHERE FID = ?1");
        for db in &to_skip {
            conn.execute(&skip_sql, params![db.fid])?;
        }
    }

    for db in to_delete {
        push_report_row(
            report,
            ReportRow {
                fid: Some(db.fid),
                planet: db.planet,
                sector: db.sector,
                region: db.region,
                grid: db.grid,
            },
            ReportKind::Deleted,
        );
    }

    for db in to_skip {
        push_report_row(
            report,
            ReportRow {
                fid: Some(db.fid),
                planet: db.planet,
                sector: db.sector,
                region: db.region,
                grid: db.grid,
            },
            ReportKind::Skipped,
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DbRow;

    /// Test CSV existence by exact name.
    #[test]
    fn exists_in_csv_matches_exact_name() {
        let db = DbRow {
            fid: 1,
            planet: "Coruscant".to_string(),
            sector: "Corusca".to_string(),
            region: "Core Worlds".to_string(),
            grid: "L-9".to_string(),
            status: "".to_string(),
        };

        let mut names = HashSet::new();
        let mut keys = HashSet::new();

        names.insert(cmp_key("Coruscant"));
        keys.insert((cmp_key("X"), cmp_key("Y"), cmp_key("Z")));

        assert!(exists_in_csv(&db, &names, &keys));
    }

    /// Test CSV existence by Roman suffix normalization.
    #[test]
    fn exists_in_csv_matches_roman_suffix() {
        let db = DbRow {
            fid: 2,
            planet: "Yavin IV".to_string(),
            sector: "Gordian Reach".to_string(),
            region: "Outer Rim Territories".to_string(),
            grid: "P-17".to_string(),
            status: "".to_string(),
        };

        let mut names = HashSet::new();
        let keys = HashSet::new();

        names.insert(cmp_key("Yavin"));

        assert!(exists_in_csv(&db, &names, &keys));
    }

    /// Test CSV existence by sector-region-grid triple.
    #[test]
    fn exists_in_csv_matches_by_location() {
        let db = DbRow {
            fid: 3,
            planet: "Yavin Prime".to_string(),
            sector: "Gordian Reach".to_string(),
            region: "Outer Rim Territories".to_string(),
            grid: "P-17".to_string(),
            status: "".to_string(),
        };

        let names = HashSet::new();
        let mut keys = HashSet::new();

        keys.insert((
            cmp_key("Gordian Reach"),
            cmp_key("Outer Rim Territories"),
            cmp_key("P-17"),
        ));

        assert!(exists_in_csv(&db, &names, &keys));
    }
}
