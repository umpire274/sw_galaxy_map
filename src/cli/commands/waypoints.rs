use crate::cli::args::WaypointCmd;
use anyhow::{Result, bail};
use rusqlite::Connection;

use crate::db::queries;
use crate::normalize::normalize_text;
use crate::ui;

// se ce l'hai; altrimenti usa println!

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

            let n = queries::delete_waypoint(con, *id)?;
            if n == 0 {
                bail!("Waypoint not deleted (not found): id={}", id);
            }

            ui::success("Waypoint deleted");
            Ok(())
        }
    }
}
