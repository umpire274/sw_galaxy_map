use rusqlite::{Result as SqlResult, Row};

use crate::utils::wiki::fandom_planet_url;

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

#[derive(Debug)]
pub struct Waypoint {
    pub id: i64,
    pub name: String,
    pub name_norm: String,
    pub x: f64,
    pub y: f64,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl Waypoint {
    pub fn fmt_short(&self) -> String {
        format!(
            "#{:<4} {:<24} ({:>10.3}, {:>10.3}) kind={}",
            self.id, self.name, self.x, self.y, self.kind
        )
    }
}

#[derive(Debug)]
pub struct WaypointPlanetLink {
    pub waypoint_id: i64,
    pub planet_fid: i64,
    pub role: String,
    pub distance: Option<f64>,
}

impl Planet {
    pub fn from_row(r: &Row<'_>) -> SqlResult<Self> {
        Ok(Self {
            fid: r.get("fid")?,
            planet: r.get("planet")?,
            planet_norm: r.get("planet_norm")?,
            region: r.get("region")?,
            sector: r.get("sector")?,
            system: r.get("system")?,
            grid: r.get("grid")?,
            x: r.get("x")?,
            y: r.get("y")?,
            canon: r.get("canon")?,
            legends: r.get("legends")?,
            zm: r.get("zm")?,
            name0: r.get("name0")?,
            name1: r.get("name1")?,
            name2: r.get("name2")?,
            lat: r.get("lat")?,
            long: r.get("long")?,
            reference: r.get("reference")?,
            status: r.get("status")?,
            c_region: r.get("c_region")?,
            c_region_li: r.get("c_region_li")?,
        })
    }

    pub fn info_planet_url(&self) -> String {
        fandom_planet_url(&self.planet)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RouteRow {
    pub id: i64,
    pub from_planet_fid: i64,
    pub to_planet_fid: i64,
    pub from_planet_name: String,
    pub to_planet_name: String,
    pub algo_version: String,
    pub options_json: String,
    pub length: Option<f64>,
    pub iterations: Option<i64>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RouteWaypointRow {
    pub seq: i64,
    pub x: f64,
    pub y: f64,
    pub waypoint_id: Option<i64>,
    // NEW (nullable because start/end may have no waypoint_id)
    pub waypoint_name: Option<String>,
    pub waypoint_kind: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RouteDetourRow {
    pub idx: i64,

    pub iteration: i64,
    pub segment_index: i64,

    pub obstacle_id: i64,
    pub obstacle_name: String,
    pub obstacle_x: f64,
    pub obstacle_y: f64,
    pub obstacle_radius: f64,

    pub closest_t: f64,
    pub closest_qx: f64,
    pub closest_qy: f64,
    pub closest_dist: f64,

    pub offset_used: f64,

    pub wp_x: f64,
    pub wp_y: f64,
    pub waypoint_id: Option<i64>,

    pub score_base: f64,
    pub score_turn: f64,
    pub score_back: f64,
    pub score_proximity: f64,
    pub score_total: f64,
}

#[derive(Debug, Clone)]
pub struct RouteLoaded {
    pub route: RouteRow,
    pub waypoints: Vec<RouteWaypointRow>,
    pub detours: Vec<RouteDetourRow>,
}
