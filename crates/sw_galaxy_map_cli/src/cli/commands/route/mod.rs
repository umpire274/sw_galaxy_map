mod cleanup;
mod compute;
pub(crate) mod explain;
pub(crate) mod list;
mod show;
pub(crate) mod types;

use cleanup::{run_clear, run_prune};
use compute::run_compute;
use list::run_list;
use show::{run_last, run_show};
use types::RouteListOptions;

pub(crate) use compute::resolve_compute_for_tui;
pub(crate) use explain::{RegionBlend, compute_eta_summary, run_explain};
pub(crate) use show::resolve_show_for_tui;

use crate::cli::args::RouteCmd;

use anyhow::Result;
use rusqlite::Connection;
use sw_galaxy_map_core::validate;

// ETA model defaults (not exposed to CLI yet)
pub fn run(con: &mut Connection, cmd: &RouteCmd) -> Result<()> {
    match cmd {
        RouteCmd::Compute(args) => {
            validate::validate_route_planets(&args.planets)?;
        }
        RouteCmd::Show { route_id } => {
            validate::validate_route_id(*route_id, "show")?;
        }
        RouteCmd::Explain(args) => {
            validate::validate_route_id(args.route_id, "explain")?;
        }
        RouteCmd::Last { from, to } => {
            validate::validate_route_compute(from, to)?;
        }
        RouteCmd::List { limit, .. } => {
            validate::validate_limit(*limit as i64, "list")?;
        }
        _ => {}
    }

    match cmd {
        RouteCmd::Compute(args) => run_compute(con, args),
        RouteCmd::Show { route_id } => run_show(con, *route_id),
        RouteCmd::Explain(args) => run_explain(con, args),
        RouteCmd::Clear { yes } => run_clear(con, *yes),
        RouteCmd::Prune => run_prune(con),
        RouteCmd::Last { from, to } => run_last(con, from, to),
        RouteCmd::List {
            json,
            file,
            limit,
            status,
            from,
            to,
            wp,
            sort,
        } => {
            let opts = RouteListOptions {
                json: *json,
                file: file.as_deref(),
                limit: *limit,
                status: status.as_deref(),
                from: *from,
                to: *to,
                wp: *wp,
                sort: *sort,
            };

            run_list(con, opts)
        }
    }
}
