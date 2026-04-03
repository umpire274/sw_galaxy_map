use crate::cli::commands::route::types::RouteListTuiItem;
use crate::tui::build_navigation_panel;
use ratatui::prelude::Line;
use sw_galaxy_map_core::model::{NearHit, PlanetSearchRow};
use sw_galaxy_map_core::routing::eta::RegionBlend;

pub(crate) const ETA_HYPERDRIVE_CLASS: f64 = 1.0;
pub(crate) const ETA_DETOUR_COUNT_BASE: f64 = 0.97;
pub(crate) const ETA_SEVERITY_K: f64 = 0.15;
pub(crate) const ETA_REGION_BLEND: RegionBlend = RegionBlend::Avg;

pub(crate) enum NavigationPanelKind {
    Empty,
    Route {
        length_parsec: Option<f64>,
        eta_text: Option<String>,
        detours: Option<usize>,
        region_text: Option<String>,
    },
    Near {
        distance_parsec: f64,
        reference_name: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct TuiCommandOutput {
    pub log_lines: Vec<String>,
    pub planet1_title: Line<'static>,
    pub planet1_lines: Vec<String>,
    pub navigation_title: Line<'static>,
    pub navigation_lines: Vec<String>,
    pub planet2_title: Line<'static>,
    pub planet2_lines: Vec<String>,
    pub search_results: Vec<PlanetSearchRow>,
    pub near_results: Vec<NearHit>,
    pub route_list_results: Vec<RouteListTuiItem>,
}

pub(crate) fn tui_default_output() -> TuiCommandOutput {
    let (navigation_title, navigation_lines) = build_navigation_panel(NavigationPanelKind::Empty);

    TuiCommandOutput {
        log_lines: Vec::new(),
        planet1_title: Line::from("Planet 1 Information"),
        planet1_lines: vec!["No data".to_string()],
        navigation_title,
        navigation_lines,
        planet2_title: Line::from("Planet 2 Information"),
        planet2_lines: vec!["No data".to_string()],
        search_results: Vec::new(),
        near_results: Vec::new(),
        route_list_results: Vec::new(),
    }
}

pub(crate) fn region_name(
    r: sw_galaxy_map_core::routing::hyperspace::GalacticRegion,
) -> &'static str {
    match r {
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::DeepCore => "Deep Core",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::CoreWorlds => "Core Worlds",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::Colonies => "Colonies",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::InnerRim => "Inner Rim",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::ExpansionRegion => {
            "Expansion Region"
        }
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::MidRim => "Mid Rim",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::HuttSpace => "Hutt Space",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::OuterRim => "Outer Rim",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::WildSpace => "Wild Space",
        sw_galaxy_map_core::routing::hyperspace::GalacticRegion::UnknownRegions => {
            "Unknown Regions"
        }
    }
}
