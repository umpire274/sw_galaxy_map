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
