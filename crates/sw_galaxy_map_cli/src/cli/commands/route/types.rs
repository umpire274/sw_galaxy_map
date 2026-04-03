use serde::Serialize;
use std::path::Path;
use sw_galaxy_map_core::domain::RouteListSort;
use sw_galaxy_map_core::model::RouteLoaded;

#[derive(Debug)]
pub(crate) struct RouteComputeTuiData {
    pub route_id: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct RouteShowTuiData {
    pub loaded: RouteLoaded,
}

#[derive(Debug, Serialize)]
pub(crate) struct RouteListExport {
    pub routes: Vec<RouteListItem>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RouteListItem {
    pub id: i64,
    pub from: RouteListEndpoint,
    pub to: RouteListEndpoint,
    pub status: String,
    pub length_parsec: Option<f64>,
    pub iterations: Option<i64>,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub waypoints_count: i64,
    pub detours_count: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct RouteListEndpoint {
    pub fid: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RouteListTuiItem {
    pub route_id: i64,
    pub from_name: String,
    pub to_name: String,
    pub status: String,
    pub length_parsec: Option<f64>,
    pub waypoints_count: i64,
    pub detours_count: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct RouteListOptions<'a> {
    pub json: bool,
    pub file: Option<&'a Path>,
    pub limit: usize,
    pub status: Option<&'a str>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub wp: Option<usize>,
    pub sort: RouteListSort,
}
