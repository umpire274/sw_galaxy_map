use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::{
    CsvOverlayOutcome, CsvOverlayRow, NormalizedPlanet, PlanetDbRow, UpsertOutcome,
};
use crate::utils::{build_planet_norm, cmp_key, same_overlay_fields, strip_roman_suffix};

/// SQLite repository for planet synchronization/import operations.
pub struct SqlitePlanetRepository<'conn> {
    conn: &'conn Connection,
    table: String,
}

impl<'conn> SqlitePlanetRepository<'conn> {
    /// Create a new SQLite planet repository.
    pub fn new(conn: &'conn Connection, table: impl Into<String>) -> Self {
        Self {
            conn,
            table: table.into(),
        }
    }

    /// Ensure that this repository table exists.
    pub fn ensure_table_exists(&self) -> Result<()> {
        if !table_exists(self.conn, &self.table)? {
            bail!("Target table '{}' does not exist", self.table);
        }

        Ok(())
    }

    /// Upsert a normalized ArcGIS planet into the target table.
    pub fn upsert_arcgis_planet(&self, planet: &NormalizedPlanet) -> Result<UpsertOutcome> {
        let existing_hash = self
            .find_existing_arcgis_hash_by_fid(planet.fid)
            .with_context(|| format!("Unable to inspect existing FID {}", planet.fid))?;

        if existing_hash.as_deref() == Some(planet.arcgis_hash.as_str()) {
            return Ok(UpsertOutcome::Skipped);
        }

        if existing_hash.is_some() {
            self.update_arcgis_planet(planet)?;
            Ok(UpsertOutcome::Updated)
        } else {
            self.insert_arcgis_planet(planet)?;
            Ok(UpsertOutcome::Inserted)
        }
    }

    /// Apply one official CSV overlay row to the target table.
    pub fn apply_csv_overlay_row(
        &self,
        row: &CsvOverlayRow,
        dry_run: bool,
    ) -> Result<CsvOverlayOutcome> {
        if let Some(existing) = self.find_exact_match(row)? {
            return self.apply_csv_match(existing, row, dry_run, false);
        }

        if let Some(existing) = self.find_suffix_match(row)? {
            return self.apply_csv_match(existing, row, dry_run, true);
        }

        if !dry_run {
            self.insert_csv_overlay_row(row)?;
        }

        Ok(CsvOverlayOutcome::Inserted)
    }

    /// Mark records not represented by the CSV overlay as deleted/skipped.
    pub fn mark_deleted_not_in_csv(
        &self,
        csv_rows: &[CsvOverlayRow],
        dry_run: bool,
    ) -> Result<(usize, usize)> {
        let db_rows = self.load_planet_rows()?;
        let mut deleted = 0usize;
        let mut skipped = 0usize;

        for db_row in db_rows {
            if exists_in_csv(&db_row, csv_rows) {
                if db_row.status.trim().is_empty() {
                    skipped += 1;
                    if !dry_run {
                        self.set_status(db_row.fid, "skipped")?;
                    }
                }
            } else {
                deleted += 1;
                if !dry_run {
                    self.set_status(db_row.fid, "deleted")?;
                }
            }
        }

        Ok((deleted, skipped))
    }

    fn apply_csv_match(
        &self,
        existing: PlanetDbRow,
        row: &CsvOverlayRow,
        dry_run: bool,
        suffix_match: bool,
    ) -> Result<CsvOverlayOutcome> {
        if same_overlay_fields(
            &existing.sector,
            &existing.region,
            &existing.grid,
            &row.sector,
            &row.region,
            &row.grid,
        ) {
            if !dry_run {
                self.set_status(existing.fid, "active")?;
            }

            Ok(CsvOverlayOutcome::Active)
        } else {
            if !dry_run {
                self.update_csv_overlay_row(existing.fid, row, "modified")?;
            }

            Ok(if suffix_match {
                CsvOverlayOutcome::ModifiedSuffix
            } else {
                CsvOverlayOutcome::ModifiedExact
            })
        }
    }

    fn find_existing_arcgis_hash_by_fid(&self, fid: i64) -> Result<Option<String>> {
        let sql = format!(
            "SELECT arcgis_hash FROM {} WHERE FID = ?1 LIMIT 1",
            quote_ident(&self.table)
        );

        self.conn
            .query_row(&sql, params![fid], |row| row.get::<_, Option<String>>(0))
            .optional()
            .map(|value| value.flatten())
            .map_err(Into::into)
    }

    fn find_exact_match(&self, row: &CsvOverlayRow) -> Result<Option<PlanetDbRow>> {
        let target = cmp_key(&row.system);
        let rows = self.load_planet_rows()?;

        Ok(rows
            .into_iter()
            .find(|db_row| cmp_key(&db_row.planet) == target))
    }

    fn find_suffix_match(&self, row: &CsvOverlayRow) -> Result<Option<PlanetDbRow>> {
        let target = cmp_key(&row.system);
        let target_base = strip_roman_suffix(&target);
        let rows = self.load_planet_rows()?;

        Ok(rows.into_iter().find(|db_row| {
            let db_key = cmp_key(&db_row.planet);
            let db_base = strip_roman_suffix(&db_key);

            db_base == target || db_key == target_base || db_base == target_base
        }))
    }

    fn load_planet_rows(&self) -> Result<Vec<PlanetDbRow>> {
        let sql = format!(
            "SELECT FID,
                    COALESCE(Planet, ''),
                    COALESCE(Sector, ''),
                    COALESCE(Region, ''),
                    COALESCE(Grid, ''),
                    COALESCE(status, '')
             FROM {}
             WHERE COALESCE(deleted, 0) = 0",
            quote_ident(&self.table)
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(PlanetDbRow {
                    fid: row.get(0)?,
                    planet: row.get(1)?,
                    sector: row.get(2)?,
                    region: row.get(3)?,
                    grid: row.get(4)?,
                    status: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(rows)
    }

    fn insert_arcgis_planet(&self, planet: &NormalizedPlanet) -> Result<()> {
        let sql = format!(
            "INSERT INTO {}
                (FID, Planet, planet_norm, Region, Sector, System, Grid, X, Y,
                 arcgis_hash, deleted, Canon, Legends)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12)",
            quote_ident(&self.table)
        );

        self.conn.execute(
            &sql,
            params![
                planet.fid,
                planet.planet,
                planet.planet_norm,
                planet.region,
                planet.sector,
                planet.system,
                planet.grid,
                planet.x.map(round3),
                planet.y.map(round3),
                planet.arcgis_hash,
                planet.canon,
                planet.legends,
            ],
        )?;

        Ok(())
    }

    fn update_arcgis_planet(&self, planet: &NormalizedPlanet) -> Result<()> {
        let sql = format!(
            "UPDATE {}
             SET
                Planet = ?2,
                planet_norm = ?3,
                Region = ?4,
                Sector = ?5,
                System = ?6,
                Grid = ?7,
                X = ?8,
                Y = ?9,
                arcgis_hash = ?10,
                deleted = 0,
                Canon = ?11,
                Legends = ?12
             WHERE FID = ?1",
            quote_ident(&self.table)
        );

        self.conn.execute(
            &sql,
            params![
                planet.fid,
                planet.planet,
                planet.planet_norm,
                planet.region,
                planet.sector,
                planet.system,
                planet.grid,
                planet.x.map(round3),
                planet.y.map(round3),
                planet.arcgis_hash,
                planet.canon,
                planet.legends,
            ],
        )?;

        Ok(())
    }

    fn insert_csv_overlay_row(&self, row: &CsvOverlayRow) -> Result<()> {
        let fid = self.next_fid()?;
        let planet_norm = build_planet_norm(&row.system);
        let sql = format!(
            "INSERT INTO {}
                (FID, Planet, planet_norm, Region, Sector, System, Grid,
                 X, Y, arcgis_hash, deleted, Canon, Legends, status)
             VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                 ?8, ?9, '', 0, 1, 0, 'inserted')",
            quote_ident(&self.table)
        );

        self.conn.execute(
            &sql,
            params![
                fid,
                row.system,
                planet_norm,
                row.region,
                row.sector,
                row.system,
                row.grid,
                0.0_f64,
                0.0_f64,
            ],
        )?;

        Ok(())
    }

    fn update_csv_overlay_row(&self, fid: i64, row: &CsvOverlayRow, status: &str) -> Result<()> {
        let planet_norm = build_planet_norm(&row.system);
        let sql = format!(
            "UPDATE {}
             SET
                Planet = ?2,
                planet_norm = ?3,
                Region = ?4,
                Sector = ?5,
                System = ?6,
                Grid = ?7,
                deleted = 0,
                status = ?8
             WHERE FID = ?1",
            quote_ident(&self.table)
        );

        self.conn.execute(
            &sql,
            params![
                fid,
                row.system,
                planet_norm,
                row.region,
                row.sector,
                row.system,
                row.grid,
                status,
            ],
        )?;

        Ok(())
    }

    pub fn set_status(&self, fid: i64, status: &str) -> Result<()> {
        let deleted = i64::from(status.eq_ignore_ascii_case("deleted"));

        let sql = format!(
            "UPDATE {} SET status = ?2, deleted = ?3 WHERE FID = ?1",
            quote_ident(&self.table)
        );

        self.conn.execute(&sql, params![fid, status, deleted])?;

        Ok(())
    }

    fn next_fid(&self) -> Result<i64> {
        let sql = format!(
            "SELECT COALESCE(MAX(FID), 0) + 1 FROM {}",
            quote_ident(&self.table)
        );

        self.conn
            .query_row(&sql, [], |row| row.get(0))
            .map_err(Into::into)
    }
}

/// Ensures that the required sync tables exist without destroying existing data.
pub fn ensure_required_schema(
    conn: &Connection,
    planets_table: &str,
    unknown_table: &str,
) -> Result<()> {
    let has_planets = table_exists(conn, planets_table)?;
    let has_unknown = table_exists(conn, unknown_table)?;

    match (has_planets, has_unknown) {
        (true, true) => Ok(()),

        (false, _) => {
            let enable_fts = sw_galaxy_map_core::db::provision::has_fts5(conn);
            sw_galaxy_map_core::db::provision::create_schema(conn, enable_fts)?;

            if !table_exists(conn, unknown_table)? {
                create_table_like(conn, planets_table, unknown_table)?;
            }

            Ok(())
        }

        (true, false) => {
            create_table_like(conn, planets_table, unknown_table)?;
            Ok(())
        }
    }
}

/// Returns true when a SQLite table exists.
pub fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            params![table],
            |_| Ok(()),
        )
        .optional()?
        .is_some();

    Ok(exists)
}

/// Creates `target_table` with the same column names and SQLite column types
/// as `source_table`.
pub fn create_table_like(conn: &Connection, source_table: &str, target_table: &str) -> Result<()> {
    if table_exists(conn, target_table)? {
        return Ok(());
    }

    let columns = table_columns(conn, source_table)
        .with_context(|| format!("Unable to inspect source table '{source_table}'"))?;

    if columns.is_empty() {
        bail!("Source table '{source_table}' does not exist or has no columns");
    }

    let column_sql = columns
        .iter()
        .map(|column| {
            let ty = if column.ty.trim().is_empty() {
                "TEXT"
            } else {
                column.ty.as_str()
            };

            format!("{} {}", quote_ident(&column.name), ty)
        })
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        quote_ident(target_table),
        column_sql
    );

    conn.execute(&sql, [])?;

    Ok(())
}

fn exists_in_csv(db_row: &PlanetDbRow, csv_rows: &[CsvOverlayRow]) -> bool {
    let db_name = cmp_key(&db_row.planet);
    let db_base = strip_roman_suffix(&db_name);

    csv_rows.iter().any(|row| {
        let csv_name = cmp_key(&row.system);
        let csv_base = strip_roman_suffix(&csv_name);

        db_name == csv_name || db_base == csv_name || db_name == csv_base || db_base == csv_base
    })
}

#[derive(Debug)]
struct TableColumn {
    name: String,
    ty: String,
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<TableColumn>> {
    let sql = format!("PRAGMA table_info({})", quote_ident(table));
    let mut stmt = conn.prepare(&sql)?;

    let columns = stmt
        .query_map([], |row| {
            Ok(TableColumn {
                name: row.get(1)?,
                ty: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(columns)
}

fn quote_ident(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
