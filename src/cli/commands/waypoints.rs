use crate::cli::args::WaypointCmd;
use anyhow::{Result, bail};
use rusqlite::Connection;

use crate::db::queries;
use crate::model::Planet;
use crate::normalize::normalize_text;
use crate::ui;

// se ce l'hai; altrimenti usa println!
fn resolve_planet_for_waypoint(con: &Connection, input: &str) -> Result<Planet> {
    let norm = normalize_text(input);

    match queries::find_planet_for_info(con, &norm)? {
        Some(p) => Ok(p),
        None => bail!("Planet not found: {}", input),
    }
}

pub fn run_waypoint(con: &Connection, cmd: &WaypointCmd) -> Result<()> {
    match cmd {
        WaypointCmd::Add {
            name,
            x,
            y,
            kind,
            note,
        } => {
            let name_norm = normalize_text(name);

            // Evita duplicati (friendly)
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

        WaypointCmd::List { limit, offset } => {
            let items = queries::list_waypoints(con, *limit, *offset)?;

            if items.is_empty() {
                ui::info("No waypoints found");
                return Ok(());
            }

            ui::info(format!("Waypoints (limit={}, offset={}):", limit, offset));
            for w in items {
                println!("{}", w.fmt_short());
            }
            Ok(())
        }

        WaypointCmd::Show { key } => {
            let wp = if let Ok(id) = key.parse::<i64>() {
                queries::find_waypoint_by_id(con, id)?
            } else {
                let norm = normalize_text(key);
                queries::find_waypoint_by_norm(con, &norm)?
            };

            let Some(w) = wp else {
                bail!("Waypoint not found: {}", key);
            };

            ui::info("Waypoint");
            println!();
            println!("ID: {}", w.id);
            println!("Name: {}", w.name);
            println!("Name norm: {}", w.name_norm);
            println!("X: {}", w.x);
            println!("Y: {}", w.y);
            println!("Kind: {}", w.kind);
            println!("Fingerprint: {}", w.fingerprint.as_deref().unwrap_or("-"));
            println!("Note: {}", w.note.as_deref().unwrap_or("-"));
            println!("Created at: {}", w.created_at);
            println!("Updated at: {}", w.updated_at.as_deref().unwrap_or("-"));

            Ok(())
        }

        WaypointCmd::Delete { id } => {
            // Optional: show what you're deleting
            if let Some(w) = queries::find_waypoint_by_id(con, *id)? {
                ui::warning(format!("Deleting waypoint: {}", w.fmt_short()));
            } else {
                bail!("Waypoint not found: id={}", id);
            }

            // prima di cancellare waypoint, elimina link (anche se ON DELETE CASCADE dovrebbe farlo)
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
            // verifica waypoint esiste
            let Some(wp) = queries::find_waypoint_by_id(con, *waypoint_id)? else {
                bail!("Waypoint not found: id={}", waypoint_id);
            };

            // risolvi pianeta (nome o alias)
            let p = resolve_planet_for_waypoint(con, planet)?;
            queries::link_waypoint_to_planet(con, wp.id, p.fid, role, *distance)?;

            ui::success(format!(
                "Linked waypoint id={} to planet '{}' (fid={}) role={}",
                wp.id, p.planet, p.fid, role
            ));

            Ok(())
        }

        WaypointCmd::Links { waypoint_id } => {
            let Some(wp) = queries::find_waypoint_by_id(con, *waypoint_id)? else {
                bail!("Waypoint not found: id={}", waypoint_id);
            };

            let links = queries::list_links_for_waypoint(con, wp.id)?;
            ui::info(format!("Links for waypoint: {}", wp.fmt_short()));

            if links.is_empty() {
                println!("(none)");
                return Ok(());
            }

            for l in links {
                // Mostra anche il nome pianeta (facoltativo ma utile)
                // Riusa la tua get_planet_by_fid se ce l’hai, altrimenti implementala.
                println!(
                    "- waypoint_id={} planet_fid={} role={} distance={}",
                    l.waypoint_id,
                    l.planet_fid,
                    l.role,
                    l.distance
                        .map(|v| format!("{:.3}", v))
                        .unwrap_or_else(|| "-".into())
                );
            }

            Ok(())
        }

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
    }
}
