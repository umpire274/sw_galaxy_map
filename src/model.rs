use anyhow::Result;
use rusqlite::Row;

#[derive(Debug)]
pub struct Planet {
    pub fid: i64,
    pub planet: String,
    pub planet_norm: String,
    pub region: Option<String>,
    pub sector: Option<String>,
    pub system: Option<String>,
    pub grid: Option<String>,
    pub x: f64,
    pub y: f64,
    pub canon: Option<i64>,
    pub legends: Option<i64>,
    pub zm: Option<i64>,
    pub name0: Option<String>,
    pub name1: Option<String>,
    pub name2: Option<String>,
    pub lat: Option<f64>,
    pub long: Option<f64>,
    pub reference: Option<String>,
    pub status: Option<String>,
    pub c_region: Option<String>,
    pub c_region_li: Option<String>,
}

#[derive(Debug)]
pub struct AliasRow {
    pub alias: String,
    pub source: Option<String>,
}

#[derive(Debug)]
pub struct NearHit {
    pub fid: i64,
    pub planet: String,
    pub x: f64,
    pub y: f64,
    pub distance: f64,
}

impl Planet {
    pub fn from_row(r: &Row<'_>) -> Result<Self> {
        Ok(Self {
            fid: r.get(0)?,
            planet: r.get(1)?,
            planet_norm: r.get(2)?,
            region: r.get(3)?,
            sector: r.get(4)?,
            system: r.get(5)?,
            grid: r.get(6)?,
            x: r.get(7)?,
            y: r.get(8)?,
            canon: r.get(9)?,
            legends: r.get(10)?,
            zm: r.get(11)?,
            name0: r.get(12)?,
            name1: r.get(13)?,
            name2: r.get(14)?,
            lat: r.get(15)?,
            long: r.get(16)?,
            reference: r.get(17)?,
            status: r.get(18)?,
            c_region: r.get(19)?,
            c_region_li: r.get(20)?,
        })
    }
}
