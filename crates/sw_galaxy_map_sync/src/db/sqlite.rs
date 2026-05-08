use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};

use crate::models::{NormalizedPlanet, UpsertOutcome};

/// SQLite repository for ArcGIS planet imports.
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
        if !self.table_exists()? {
            bail!("Target table '{}' does not exist", self.table);
        }

        Ok(())
    }

    /// Create a table with the same column names/types as another table if missing.
    ///
    /// This is used for `planets_unknown`. We intentionally do not rely on
    /// `ON CONFLICT`, so the cloned table does not need to preserve indexes
    /// or primary-key constraints.
    pub fn create_like_if_missing(
        conn: &'conn Connection,
        source_table: &str,
        target_table: &str,
    ) -> Result<Self> {
        let repo = Self::new(conn, target_table);

        if repo.table_exists()? {
            return Ok(repo);
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

        Ok(repo)
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
            self.update_planet(planet)?;
            Ok(UpsertOutcome::Updated)
        } else {
            self.insert_planet(planet)?;
            Ok(UpsertOutcome::Inserted)
        }
    }

    fn table_exists(&self) -> Result<bool> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
                params![self.table],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        Ok(exists)
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

    fn insert_planet(&self, planet: &NormalizedPlanet) -> Result<()> {
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

    fn update_planet(&self, planet: &NormalizedPlanet) -> Result<()> {
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
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// Ensures that the required sync tables exist.
///
/// If either `planets` or `planets_unknown` is missing, the canonical schema
/// from `sw_galaxy_map_core` is created.
pub fn ensure_required_schema(
    conn: &Connection,
    planets_table: &str,
    unknown_table: &str,
) -> Result<()> {
    let has_planets = table_exists(conn, planets_table)?;
    let has_unknown = table_exists(conn, unknown_table)?;

    if has_planets && has_unknown {
        return Ok(());
    }

    let enable_fts = sw_galaxy_map_core::db::provision::has_fts5(conn);
    sw_galaxy_map_core::db::provision::create_schema(conn, enable_fts)?;

    Ok(())
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
