use crate::model::RouteOptionsJson;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ExplainExport {
    pub route: ExplainRouteMeta,
    pub options: Option<RouteOptionsJson>,
    pub detours: Vec<ExplainDetour>,
    pub note: ExplainNote,
}

#[derive(Debug, Serialize)]
pub struct ExplainRouteMeta {
    pub id: i64,
    pub from: ExplainEndpoint,
    pub to: ExplainEndpoint,
    pub status: String,
    pub length_parsec: Option<f64>,
    pub iterations: Option<i64>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainEndpoint {
    pub fid: i64,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ExplainDetour {
    pub idx: i64,
    pub iteration: usize,
    pub segment_index: usize,

    pub obstacle: ExplainObstacle,
    pub closest: ExplainClosest,

    pub offset_used: f64,
    pub waypoint: ExplainWaypoint,

    pub score: ExplainScore,

    pub tries_used: Option<i64>,
    pub tries_exhausted: bool,

    pub dominant_penalty: ExplainDominantPenalty,
    pub decision_drivers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainObstacle {
    pub id: i64,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Debug, Serialize)]
pub struct ExplainClosest {
    pub t: f64,
    pub qx: f64,
    pub qy: f64,
    pub dist: f64,
    pub required: f64,
    pub violated_by: f64,
}

#[derive(Debug, Serialize)]
pub struct ExplainWaypoint {
    pub x: f64,
    pub y: f64,
    pub computed_waypoint_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ExplainScore {
    pub base: f64,
    pub turn: f64,
    pub back: f64,
    pub proximity: f64,
    pub total: f64,
}

#[derive(Debug, Serialize)]
pub struct ExplainDominantPenalty {
    pub component: String,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct ExplainNote {
    pub text: String,
    pub units: String,
}
