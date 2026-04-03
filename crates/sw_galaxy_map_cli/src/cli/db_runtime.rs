use crate::cli::print_db_init_report;
use crate::ui::warning;
use std::path::Path;
use sw_galaxy_map_core::db::db_status::resolve_db_path;

pub(crate) fn open_db_raw(db_arg: Option<String>) -> anyhow::Result<rusqlite::Connection> {
    let db_path = resolve_db_path(db_arg)?;
    ensure_db_ready(&db_path)?;
    sw_galaxy_map_core::db::open_db(&db_path.to_string_lossy())
}

pub(crate) fn open_db_migrating(db_arg: Option<String>) -> anyhow::Result<rusqlite::Connection> {
    let mut con = open_db_raw(db_arg)?;
    let _ = sw_galaxy_map_core::db::migrate::run(&mut con, false, false)?;
    Ok(con)
}

fn ensure_db_ready(db_path: &Path) -> anyhow::Result<()> {
    if db_path.exists() {
        return Ok(());
    }

    println!();
    warning(format!(
        "Local database not found at: {}\nInitializing it now (this may take a moment)...",
        db_path.display()
    ));

    let report =
        sw_galaxy_map_core::db::db_init::run(Some(db_path.to_string_lossy().to_string()), false)?;
    print_db_init_report(&report);
    Ok(())
}
