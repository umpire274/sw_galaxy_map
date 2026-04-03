use anyhow::Result;
use rusqlite::Connection;
use std::fs;
use std::io::Write;

use crate::cli::color::Colors;
use crate::ui::Style;
use sw_galaxy_map_core::db::queries;
use sw_galaxy_map_core::domain::RouteListSort;
use sw_galaxy_map_core::utils::formatting::truncate_ellipsis;

use super::types::{
    RouteListEndpoint, RouteListExport, RouteListItem, RouteListOptions, RouteListTuiItem,
};

pub(crate) fn run_list(con: &Connection, opts: RouteListOptions<'_>) -> Result<()> {
    let style = Style::default();
    let c = Colors::new(&style);

    let (rows, rows_count) = queries::list_routes(
        con,
        opts.limit,
        opts.status,
        opts.from,
        opts.to,
        opts.wp,
        opts.sort,
    )?;

    if opts.json {
        let export = RouteListExport {
            routes: rows
                .into_iter()
                .map(|r| RouteListItem {
                    id: r.id,
                    from: RouteListEndpoint {
                        fid: r.from_planet_fid,
                        name: r.from_planet_name,
                    },
                    to: RouteListEndpoint {
                        fid: r.to_planet_fid,
                        name: r.to_planet_name,
                    },
                    status: r.status,
                    length_parsec: r.length,
                    iterations: r.iterations,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                    waypoints_count: r.waypoints_count,
                    detours_count: r.detours_count,
                })
                .collect(),
        };

        let s = serde_json::to_string_pretty(&export)?;

        if let Some(path) = opts.file {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent)?;
            }

            let mut f = fs::File::create(path)?;
            f.write_all(s.as_bytes())?;
            f.write_all(b"\n")?;
            eprintln!("JSON written to {}", path.display());
        } else {
            println!("{}", s);
        }

        return Ok(());
    }

    println!("{}", c.ok("Routes:"));
    if rows.is_empty() {
        println!("{}", c.dim("(none)"));
        return Ok(());
    }

    let shown = rows.len();
    let total = rows_count;

    if total == 0 {
        println!("{}", c.dim("Found 0 routes."));
        println!("{}", c.dim("(none)"));
        return Ok(());
    }

    if opts.limit > 0 && shown < total {
        println!(
            "{}",
            c.dim(format!(
                "Found {} routes (showing {} of {}, limit={}).",
                total, shown, total, opts.limit
            ))
        );
    } else {
        println!("{}", c.dim(format!("Found {} routes.", total)));
    }

    println!(
        "{:>6}  {:<26}  {:<26}  {:<8}  {:>10}  {:>6}  {:>4}  {:>4}  UPDATED",
        "ID", "FROM", "TO", "STATUS", "LENGTH", "ITERS", "WP", "DET"
    );

    for r in rows {
        let from = format!("{} [{}]", r.from_planet_name, r.from_planet_fid);
        let to = format!("{} [{}]", r.to_planet_name, r.to_planet_fid);
        let from = truncate_ellipsis(&from, 26);
        let to = truncate_ellipsis(&to, 26);

        let len_txt = r
            .length
            .map(|v| format!("{:>10.3}", v))
            .unwrap_or_else(|| format!("{:>10}", "-"));

        let it_txt = r
            .iterations
            .map(|v| format!("{:>6}", v))
            .unwrap_or_else(|| format!("{:>6}", "-"));

        let status_plain = format!("{:<8}", r.status);

        let status_txt = if r.status.eq_ignore_ascii_case("ok") {
            c.ok(&status_plain)
        } else {
            c.err(&status_plain)
        };

        let wp_plain = format!("{:>4}", r.waypoints_count);
        let det_plain = format!("{:>4}", r.detours_count);

        let wp_txt = if r.waypoints_count == 0 {
            c.dim(&wp_plain)
        } else {
            c.warn(&wp_plain)
        };

        let det_txt = if r.detours_count == 0 {
            c.dim(&det_plain)
        } else {
            c.warn(&det_plain)
        };

        let upd = r.updated_at.clone().unwrap_or_else(|| r.created_at.clone());

        println!(
            "{:>6}  {:<26}  {:<26}  {}  {}  {}  {}  {}  {}",
            r.id, from, to, status_txt, len_txt, it_txt, wp_txt, det_txt, upd
        );
    }

    Ok(())
}

pub(crate) fn resolve_list_for_tui(
    con: &Connection,
    limit: usize,
    status: Option<&str>,
    from: Option<i64>,
    to: Option<i64>,
    wp: Option<usize>,
    sort: RouteListSort,
) -> Result<Vec<RouteListTuiItem>> {
    let (rows, _rows_count) = queries::list_routes(con, limit, status, from, to, wp, sort)?;

    let items = rows
        .into_iter()
        .map(|r| RouteListTuiItem {
            route_id: r.id,
            from_name: r.from_planet_name,
            to_name: r.to_planet_name,
            status: r.status,
            length_parsec: r.length,
            waypoints_count: r.waypoints_count,
            detours_count: r.detours_count,
        })
        .collect();

    Ok(items)
}
