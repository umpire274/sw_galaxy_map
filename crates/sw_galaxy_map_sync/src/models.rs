use serde::Deserialize;

/// Minimal DB row shape used for matching.
#[derive(Debug)]
pub struct DbPlanetRow {
    pub fid: i64,
    pub planet: String,
    pub sector: String,
    pub region: String,
    pub grid: String,
}

/// Minimal DB row for audit phase.
#[derive(Debug)]
pub struct DbRow {
    pub fid: i64,
    pub planet: String,
    pub sector: String,
    pub region: String,
    pub grid: String,
    pub status: String,
}

/// Default values used when inserting a new planets row from the official CSV.
#[derive(Debug, Clone)]
pub struct InsertDefaults {
    pub x: f64,
    pub y: f64,
    pub canon: i64,
    pub legends: i64,
    pub arcgis_hash: String,
}

impl Default for InsertDefaults {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            canon: 1,
            legends: 0,
            arcgis_hash: String::new(),
        }
    }
}

/// Type-safe report classification.
#[derive(Debug, Clone, Copy)]
pub enum ReportKind {
    Inserted,
    Modified,
    Active,
    Invalid,
    Skipped,
    Deleted,
}

/// One row in the final report.
#[derive(Debug, Clone)]
pub struct ReportRow {
    pub fid: Option<i64>,
    pub planet: String,
    pub sector: String,
    pub region: String,
    pub grid: String,
}

/// Deserialize one official CSV row.
#[derive(Debug, Deserialize)]
pub struct OfficialRow {
    #[serde(rename = "system")]
    pub system: String,

    #[serde(rename = "sector")]
    pub sector: Option<String>,

    #[serde(rename = "region")]
    pub region: Option<String>,

    #[serde(rename = "grid")]
    pub grid: Option<String>,
}

/// Internal normalized row used during synchronization.
#[derive(Debug, Clone)]
pub struct SyncRow {
    pub system: String,
    pub sector: String,
    pub region: String,
    pub grid: String,
}

/// Summary counters for the sync run.
#[derive(Debug, Default, Clone)]
pub struct SyncStats {
    pub inserted: usize,
    pub updated_exact: usize,
    pub updated_suffix: usize,
    pub invalid_csv_rows: usize,
    pub invalid_marked: usize,
    pub skipped_db: usize,
    pub deleted_logically: usize,
}

/// Report grouped by status.
#[derive(Debug, Default)]
pub struct SyncReport {
    pub inserted: Vec<ReportRow>,
    pub modified: Vec<ReportRow>,
    pub active: Vec<ReportRow>,
    pub invalid: Vec<ReportRow>,
    pub skipped: Vec<ReportRow>,
    pub deleted: Vec<ReportRow>,
}
