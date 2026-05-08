use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Normalized planet record produced from ArcGIS attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedPlanet {
    pub fid: i64,
    pub planet: String,
    pub planet_norm: String,
    pub region: String,
    pub sector: String,
    pub system: String,
    pub grid: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub arcgis_hash: String,
    pub canon: i64,
    pub legends: i64,
    pub raw: Value,
}

/// Classified ArcGIS record.
///
/// Records with a valid planet name are imported into the main `planets`
/// table. Records without a planet name are preserved in `planets_unknown`
/// instead of being discarded.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "record", rename_all = "snake_case")]
pub enum ArcgisRecord {
    Known(NormalizedPlanet),
    Unknown(NormalizedPlanet),
}

impl ArcgisRecord {
    /// Returns the normalized planet payload.
    pub fn planet(&self) -> &NormalizedPlanet {
        match self {
            ArcgisRecord::Known(planet) | ArcgisRecord::Unknown(planet) => planet,
        }
    }

    /// Returns true when the record contains a usable planet name.
    pub fn is_known(&self) -> bool {
        matches!(self, ArcgisRecord::Known(_))
    }
}

/// Normalized ArcGIS fetch result split by classification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArcgisDataset {
    pub known: Vec<NormalizedPlanet>,
    pub unknown: Vec<NormalizedPlanet>,
}

impl ArcgisDataset {
    /// Total number of records.
    pub fn len(&self) -> usize {
        self.known.len() + self.unknown.len()
    }

    /// Returns true when there are no records.
    pub fn is_empty(&self) -> bool {
        self.known.is_empty() && self.unknown.is_empty()
    }
}

/// Result of an ArcGIS import operation.
#[derive(Debug, Clone, Default)]
pub struct ArcgisImportResult {
    pub fetched: usize,
    pub known: usize,
    pub unknown: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub unknown_inserted: usize,
    pub unknown_updated: usize,
    pub unknown_skipped: usize,
    pub dry_run: bool,
}

/// Classification returned by the SQLite upsert layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Inserted,
    Updated,
    Skipped,
}

/// Supported CSV overlay input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvOverlayFormat {
    /// Auto-detect the format from CSV headers.
    Auto,
    /// Official Disney-style curated CSV: system;sector;region;grid.
    Official,
    /// Full ArcGIS/export CSV containing the planets table columns.
    Full,
}

impl std::str::FromStr for CsvOverlayFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "official" => Ok(Self::Official),
            "full" => Ok(Self::Full),
            other => anyhow::bail!("Unsupported CSV overlay format: {other}"),
        }
    }
}

/// One normalized row read from a CSV overlay source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvOverlayRow {
    pub system: String,
    pub sector: String,
    pub region: String,
    pub grid: String,
}

/// One database row used by the CSV overlay matching phase.
#[derive(Debug, Clone)]
pub struct PlanetDbRow {
    pub fid: i64,
    pub planet: String,
    pub sector: String,
    pub region: String,
    pub grid: String,
    pub status: String,
}

/// Result classification for a single CSV overlay row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvOverlayOutcome {
    Inserted,
    Active,
    ModifiedExact,
    ModifiedSuffix,
}

/// Aggregate statistics produced by the CSV overlay pipeline.
#[derive(Debug, Clone, Default)]
pub struct CsvOverlayStats {
    pub rows_read: usize,
    pub inserted: usize,
    pub active: usize,
    pub modified_exact: usize,
    pub modified_suffix: usize,
    pub deleted: usize,
    pub skipped: usize,
    pub dry_run: bool,
}
