use crate::cli::args::WaypointCmd;
use crate::cli::color::Colors;
use crate::db::queries;
use crate::model::Planet;
use crate::ui;
use crate::ui::Style;
use crate::utils::formatting::truncate_ellipsis;
use crate::utils::normalize::normalize_text;

use anyhow::{Result, bail};
use rusqlite::Connection;

// Resolve planet by name/alias (normalized)
fn resolve_planet_for_waypoint(con: &Connection, input: &str) -> Result<Planet> {
    let norm = normalize_text(input);

    match queries::find_planet_for_info(con, &norm)? {
        Some(p) => Ok(p),
        None => bail!("Planet not found: {}", input),
    }
}

pub fn run_waypoint(con: &mut Connection, cmd: &WaypointCmd) -> Result<()> {
    match cmd {
        WaypointCmd::Add {
            name,
            x,
            y,
            kind,
            note,
        } => {
            let name_norm = normalize_text(name);

            // Avoid duplicates (friendly)
            if let Some(existing) = queries::find_waypoint_by_norm(con, &name_norm)? {
                bail!(
                    "Waypoint already exists: '{}' (id={}, name_norm='{}')",
                    existing.name,
                    existing.id,
                    existing.name_norm
                );
            }

            let id =
                queries::insert_waypoint(con, name, &name_norm, *x, *y, kind, note.as_deref())?;
            ui::info(format!("Waypoint created: id={} name='{}'", id, name));
            Ok(())
        }

        WaypointCmd::List { limit, offset } => run_list(con, *limit, *offset),

        WaypointCmd::Show { key } => run_show(con, key),

        WaypointCmd::Delete { id } => {
            // Optional: show what you're deleting
            if let Some(w) = queries::find_waypoint_by_id(con, *id)? {
                ui::warning(format!("Deleting waypoint: {}", w.fmt_short()));
            } else {
                bail!("Waypoint not found: id={}", id);
            }

            // Before deleting waypoint, remove links (even if ON DELETE CASCADE should handle it)
            let _ = queries::delete_waypoint_links(con, *id)?;
            let n = queries::delete_waypoint(con, *id)?;
            if n == 0 {
                bail!("Waypoint not deleted (not found): id={}", id);
            }

            ui::success("Waypoint deleted");
            Ok(())
        }

        WaypointCmd::Link {
            waypoint_id,
            planet,
            role,
            distance,
        } => {
            // Ensure waypoint exists
            let Some(wp) = queries::find_waypoint_by_id(con, *waypoint_id)? else {
                bail!("Waypoint not found: id={}", waypoint_id);
            };

            // Resolve planet (name or alias)
            let p = resolve_planet_for_waypoint(con, planet)?;
            queries::link_waypoint_to_planet(con, wp.id, p.fid, role, *distance)?;

            ui::success(format!(
                "Linked waypoint id={} to planet '{}' (fid={}) role={}",
                wp.id, p.planet, p.fid, role
            ));

            Ok(())
        }

        WaypointCmd::Links { waypoint_id } => run_waypoint_links(con, *waypoint_id),

        WaypointCmd::ForPlanet {
            planet,
            role,
            limit,
            offset,
        } => {
            let p = resolve_planet_for_waypoint(con, planet)?;

            let wps =
                queries::list_waypoints_for_planet(con, p.fid, role.as_deref(), *limit, *offset)?;

            ui::info(format!(
                "Waypoints for planet '{}' (fid={})",
                p.planet, p.fid
            ));

            if wps.is_empty() {
                println!("(none)");
                return Ok(());
            }

            for w in wps {
                println!("{}", w.fmt_short());
            }

            Ok(())
        }

        WaypointCmd::Unlink {
            waypoint_id,
            planet,
        } => {
            let p = resolve_planet_for_waypoint(con, planet)?;
            let n = queries::unlink_waypoint_from_planet(con, *waypoint_id, p.fid)?;
            if n == 0 {
                bail!(
                    "No link found for waypoint_id={} planet_fid={}",
                    waypoint_id,
                    p.fid
                );
            }
            ui::success("Link removed");
            Ok(())
        }

        WaypointCmd::Prune {
            dry_run,
            include_linked,
        } => run_waypoint_prune(con, *dry_run, *include_linked),
    }
}

fn run_list(con: &Connection, limit: usize, offset: usize) -> Result<()> {
    let style = Style::default();
    let c = Colors::new(&style);

    let (items, total) = queries::list_waypoints(con, limit, offset)?;
    let has_orphan_links = items
        .iter()
        .any(|w| w.links_count > 0 && w.routes_count == 0);

    println!("{}", c.ok("Waypoints:"));

    if total == 0 {
        println!("{}", c.dim("Found 0 waypoints."));
        println!("{}", c.dim("(none)"));
        return Ok(());
    }

    let shown = items.len();
    if limit > 0 && shown < total {
        println!(
            "{}",
            c.dim(format!(
                "Found {} waypoints (showing {} of {}, limit={}).",
                total, shown, total, limit
            ))
        );
    } else {
        println!("{}", c.dim(format!("Found {} waypoints.", total)));
    }

    println!(
        "{:>6}  {:<26}  {:<10}  {:>10}  {:>10}  {:>6}",
        "ID", "NAME", "KIND", "X", "Y", "LINKS"
    );

    for w in items {
        let id_txt = format!("{:>6}", w.waypoint.id);

        let name_raw = truncate_ellipsis(&w.waypoint.name, 26);
        let name_txt = format!("{:<26}", name_raw);

        let kind_raw = truncate_ellipsis(&w.waypoint.kind, 10);
        let kind_txt = format!("{:<10}", kind_raw);

        let x_txt = format!("{:>10.3}", w.waypoint.x);
        let y_txt = format!("{:>10.3}", w.waypoint.y);

        let links_plain_5 = format!("{:>5}", w.links_count);

        // colorize padded count (keeps width visually)
        let links_count_txt = if w.links_count == 0 {
            c.dim(&links_plain_5)
        } else {
            c.warn(&links_plain_5)
        };

        // orphan marker: has links but no routes
        let orphan = w.links_count > 0 && w.routes_count == 0;

        // IMPORTANT: don't format/pad AFTER adding colors, just concatenate
        let links_txt = if orphan {
            format!("{}{}", links_count_txt, c.red_alert("*")) // red star
        } else {
            format!("{} ", links_count_txt) // keep width = 6
        };

        println!(
            "{}  {}  {}  {}  {}  {}",
            id_txt, name_txt, kind_txt, x_txt, y_txt, links_txt
        );
    }

    if has_orphan_links {
        println!(
            "{}",
            c.dim("* = linked to planets but not used by any route")
        );
    }

    Ok(())
}

fn run_show(con: &Connection, key: &String) -> Result<()> {
    let style = Style::default();
    let c = Colors::new(&style);

    let wp = if let Ok(id) = key.parse::<i64>() {
        queries::find_waypoint_by_id(con, id)?
    } else {
        let norm = normalize_text(key);
        queries::find_waypoint_by_norm(con, &norm)?
    };

    let Some(w) = wp else {
        bail!("Waypoint not found: {}", key);
    };

    println!("{}", c.ok("Waypoint details:"));
    println!();

    let pairs = vec![
        ("ID", w.id.to_string()),
        ("Name", w.name.clone()),
        ("Name norm", w.name_norm.clone()),
        ("X", format!("{:.3}", w.x)),
        ("Y", format!("{:.3}", w.y)),
        ("Kind", w.kind.clone()),
        (
            "Fingerprint",
            w.fingerprint.clone().unwrap_or_else(|| "-".into()),
        ),
        ("Note", w.note.clone().unwrap_or_else(|| "-".into())),
        ("Created at", w.created_at.clone()),
        (
            "Updated at",
            w.updated_at.clone().unwrap_or_else(|| "-".into()),
        ),
    ];

    crate::utils::formatting::print_kv_block_colored_keys(&pairs, |s| c.dim(s));
    Ok(())
}

pub fn run_waypoint_links(con: &Connection, waypoint_id: i64) -> Result<()> {
    let style = Style::default();
    let c = Colors::new(&style);

    // Header: show waypoint if exists
    let wp = queries::find_waypoint_by_id(con, waypoint_id)?;
    println!("{}", c.ok("Waypoint links:"));
    if let Some(wp) = wp {
        println!("{}", c.dim(format!("Waypoint #{} — {}", wp.id, wp.name)));
    } else {
        println!("{}", c.dim(format!("Waypoint #{}", waypoint_id)));
    }

    let rows = queries::list_waypoint_links(con, waypoint_id)?;
    println!("{}", c.dim(format!("Found {} links.", rows.len())));

    if rows.is_empty() {
        println!("{}", c.dim("(none)"));
    } else {
        println!(
            "{:>8}  {:<26}  {:<10}  {:>10}",
            "FID", "PLANET", "ROLE", "DIST"
        );

        for r in rows {
            let fid_txt = format!("{:>8}", r.planet_fid);

            let planet_raw =
                truncate_ellipsis(&format!("{} [{}]", r.planet_name, r.planet_fid), 26);
            let planet_txt = format!("{:<26}", planet_raw);

            let role_raw = truncate_ellipsis(r.role.trim(), 10);
            let role_txt = format!("{:<10}", role_raw);

            let dist_txt = match r.distance {
                Some(d) => format!("{:>10.3}", d),
                None => format!("{:>10}", "-"),
            };

            println!("{}  {}  {}  {}", fid_txt, planet_txt, role_txt, dist_txt);
        }
    }

    // Associated routes
    let routes = queries::list_routes_for_waypoint(con, waypoint_id)?;
    println!();
    println!("{}", c.ok("Associated routes:"));
    println!("{}", c.dim(format!("Found {} routes.", routes.len())));

    if routes.is_empty() {
        println!("{}", c.dim("(none)"));
        return Ok(());
    }

    println!(
        "{:>6}  {:<26}  {:<26}  {:<8}  {:>10}  {:>4}  UPDATED",
        "ID", "FROM", "TO", "STATUS", "LENGTH", "N"
    );

    for r in routes {
        let from = truncate_ellipsis(
            &format!("{} [{}]", r.from_planet_name, r.from_planet_fid),
            26,
        );
        let to = truncate_ellipsis(&format!("{} [{}]", r.to_planet_name, r.to_planet_fid), 26);

        // ANSI-safe: pad first, then colorize
        let status_plain = format!("{:<8}", r.status);
        let status_txt = if r.status.eq_ignore_ascii_case("ok") {
            c.ok(&status_plain)
        } else {
            c.err(&status_plain)
        };

        let len_txt = r
            .length
            .map(|v| format!("{:>10.3}", v))
            .unwrap_or_else(|| format!("{:>10}", "-"));

        let occ_txt = format!("{:>4}", r.occurrences);

        println!(
            "{:>6}  {:<26}  {:<26}  {}  {}  {}  {}",
            r.id, from, to, status_txt, len_txt, occ_txt, r.updated_at
        );
    }

    Ok(())
}

pub fn run_waypoint_prune(con: &mut Connection, dry_run: bool, include_linked: bool) -> Result<()> {
    use anyhow::Context;

    let style = Style::default();
    let c = Colors::new(&style);

    #[derive(Debug)]
    struct Candidate {
        id: i64,
        name: String,
        updated: String,
        links_count: i64,
    }

    // 1) Load candidates in a scope so stmt is dropped before transaction
    let candidates: Vec<Candidate> = {
        let sql = if include_linked {
            r#"
            SELECT
              w.id,
              w.name,
              COALESCE(w.updated_at, w.created_at) AS updated,
              (SELECT COUNT(*) FROM waypoint_planets wp WHERE wp.waypoint_id = w.id) AS links_count
            FROM waypoints w
            WHERE
              w.kind = 'computed'
              AND NOT EXISTS (SELECT 1 FROM route_waypoints rw WHERE rw.waypoint_id = w.id)
            ORDER BY updated DESC, w.id DESC
            "#
        } else {
            r#"
            SELECT
              w.id,
              w.name,
              COALESCE(w.updated_at, w.created_at) AS updated,
              (SELECT COUNT(*) FROM waypoint_planets wp WHERE wp.waypoint_id = w.id) AS links_count
            FROM waypoints w
            WHERE
              w.kind = 'computed'
              AND NOT EXISTS (SELECT 1 FROM route_waypoints rw WHERE rw.waypoint_id = w.id)
              AND NOT EXISTS (SELECT 1 FROM waypoint_planets wp WHERE wp.waypoint_id = w.id)
            ORDER BY updated DESC, w.id DESC
            "#
        };

        let mut stmt = con.prepare(sql)?;
        stmt.query_map([], |row| {
            Ok(Candidate {
                id: row.get(0)?,
                name: row.get(1)?,
                updated: row.get(2)?,
                links_count: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };

    println!("{}", c.ok("Waypoint prune:"));
    println!(
        "{}",
        c.dim(format!(
            "Mode: {}{}",
            if include_linked {
                "include-linked"
            } else {
                "safe"
            },
            if dry_run { " (dry-run)" } else { "" }
        ))
    );
    println!(
        "{}",
        c.dim(format!(
            "Found {} orphan computed waypoints.",
            candidates.len()
        ))
    );

    if candidates.is_empty() {
        println!("{}", c.dim("(nothing to prune)"));
        return Ok(());
    }

    // Preview
    println!("{:>6}  {:<26}  {:>5}  UPDATED", "ID", "NAME", "LINKS");
    for w in candidates.iter().take(30) {
        let name = truncate_ellipsis(&w.name, 26);
        let links_plain = format!("{:>5}", w.links_count);
        let links_txt = if w.links_count == 0 {
            c.dim(&links_plain)
        } else {
            c.warn(&links_plain)
        };
        println!("{:>6}  {:<26}  {}  {}", w.id, name, links_txt, w.updated);
    }
    if candidates.len() > 30 {
        println!(
            "{}",
            c.dim(format!("(showing first 30; total={})", candidates.len()))
        );
    }

    if dry_run {
        return Ok(());
    }

    // 2) Transaction
    let tx = con.transaction().context("Failed to start transaction")?;

    // If include_linked, remove links first for the candidate set (avoid FK issues).
    // We keep it deterministic using the same predicate.
    if include_linked {
        tx.execute(
            r#"
            DELETE FROM waypoint_planets
            WHERE waypoint_id IN (
              SELECT w.id
              FROM waypoints w
              WHERE
                w.kind = 'computed'
                AND NOT EXISTS (SELECT 1 FROM route_waypoints rw WHERE rw.waypoint_id = w.id)
            )
            "#,
            [],
        )?;
    }

    // Delete waypoints with same predicate (safe vs include_linked differs only by link condition)
    let deleted = if include_linked {
        tx.execute(
            r#"
            DELETE FROM waypoints
            WHERE
              kind = 'computed'
              AND NOT EXISTS (SELECT 1 FROM route_waypoints rw WHERE rw.waypoint_id = waypoints.id)
            "#,
            [],
        )?
    } else {
        tx.execute(
            r#"
            DELETE FROM waypoints
            WHERE
              kind = 'computed'
              AND NOT EXISTS (SELECT 1 FROM route_waypoints rw WHERE rw.waypoint_id = waypoints.id)
              AND NOT EXISTS (SELECT 1 FROM waypoint_planets wp WHERE wp.waypoint_id = waypoints.id)
            "#,
            [],
        )?
    };

    tx.commit().context("Failed to commit transaction")?;

    println!("{}", c.ok(format!("Pruned {} waypoints.", deleted)));
    Ok(())
}
