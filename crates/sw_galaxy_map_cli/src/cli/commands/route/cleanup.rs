use anyhow::{Result, bail};
use rusqlite::Connection;
use std::io::{self, Write};

use crate::cli::color::Colors;
use crate::ui::Style;

fn confirm_destructive(action: &str) -> Result<bool> {
    let style = Style::default();
    let c = Colors::new(&style);

    eprintln!("{}", c.warn("⚠️  DESTRUCTIVE OPERATION"));
    eprintln!("{}", c.warn(action));
    println!();
    eprintln!("Type YES to continue, or anything else to abort.");
    eprint!("> ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("YES"))
}

pub(crate) fn run_clear(con: &mut Connection, yes: bool) -> Result<()> {
    let style = Style::default();
    let c = Colors::new(&style);

    if !yes {
        let action =
            "This will DELETE ALL routes, route waypoints, and route detours from the database.";

        if !confirm_destructive(action)? {
            bail!("Aborted by user.");
        }
    }

    let tx = con.transaction()?;

    let detours_deleted = tx.execute("DELETE FROM route_detours", [])?;
    let waypoints_deleted = tx.execute("DELETE FROM route_waypoints", [])?;
    let routes_deleted = tx.execute("DELETE FROM routes", [])?;

    tx.commit()?;

    println!("{}", c.ok("Routes cleared:"));
    println!(
        "  route_detours:   {}",
        c.warn(format!("{} rows deleted", detours_deleted))
    );
    println!(
        "  route_waypoints: {}",
        c.warn(format!("{} rows deleted", waypoints_deleted))
    );
    println!(
        "  routes:          {}",
        c.warn(format!("{} rows deleted", routes_deleted))
    );

    Ok(())
}

pub(crate) fn run_prune(con: &mut Connection) -> Result<()> {
    let style = Style::default();
    let c = Colors::new(&style);

    let tx = con.transaction()?;

    let detours_deleted = tx.execute(
        r#"
        DELETE FROM route_detours
        WHERE route_id NOT IN (SELECT id FROM routes)
        "#,
        [],
    )?;

    let waypoints_deleted = tx.execute(
        r#"
        DELETE FROM route_waypoints
        WHERE route_id NOT IN (SELECT id FROM routes)
        "#,
        [],
    )?;

    tx.commit()?;

    println!("{}", c.ok("Prune completed:"));

    let detours_txt = format!("{} orphan rows deleted", detours_deleted);
    let waypoints_txt = format!("{} orphan rows deleted", waypoints_deleted);

    let detours_out = if detours_deleted == 0 {
        c.dim(detours_txt)
    } else {
        c.warn(detours_txt)
    };
    let waypoints_out = if waypoints_deleted == 0 {
        c.dim(waypoints_txt)
    } else {
        c.warn(waypoints_txt)
    };

    println!("  route_detours:   {}", detours_out);
    println!("  route_waypoints: {}", waypoints_out);

    Ok(())
}
