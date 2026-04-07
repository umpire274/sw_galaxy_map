//! History command implementation.

use crate::audit::history::load_entity_history;
use crate::cli::HistoryArgs;
use crate::db::runtime::open_db;
use crate::output::history::print_history_rows;
use crate::resolve::planet::{resolve_by_fid, resolve_by_name_or_alias};
use anyhow::{Result, bail};

pub fn run(args: HistoryArgs) -> Result<()> {
    if args.fid.is_none() && args.planet.is_none() {
        bail!("You must provide either --fid <FID> or --planet <NAME>.");
    }

    let con = open_db()?;

    let planet = if let Some(fid) = args.fid {
        resolve_by_fid(&con, fid)?
    } else if let Some(name) = args.planet.as_deref() {
        resolve_by_name_or_alias(&con, name)?
    } else {
        None
    };

    let planet = match planet {
        Some(p) => p,
        None => bail!("Planet not found."),
    };

    let rows = load_entity_history(&con, "planet", planet.fid, args.limit)?;

    println!("History for {} (FID: {})", planet.planet, planet.fid);
    println!();

    print_history_rows(&rows);

    Ok(())
}
