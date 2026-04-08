pub(crate) mod app;
pub(crate) mod bridge;
pub(crate) mod input;
pub(crate) mod log;
pub(crate) mod panels;
pub(crate) mod render;
pub(crate) mod runtime;
pub(crate) mod types;

pub use runtime::run_tui;
pub(crate) use runtime::{tui_log_only, tui_only_cli_message};

pub(crate) use panels::{
    build_navigation_panel, build_near_planet_panel, build_planet_panel, build_route_show_output,
};

pub(crate) use types::{NavigationPanelKind, TuiCommandOutput, region_name, tui_default_output};
