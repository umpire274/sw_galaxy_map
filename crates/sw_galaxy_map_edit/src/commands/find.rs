//! Find command implementation.

use crate::cli::FindArgs;
use crate::db::runtime::open_db;
use crate::output::planet::{print_planet, print_search_results};
use crate::resolve::planet::{resolve_single, search};
use anyhow::Result;

pub fn run(args: FindArgs) -> Result<()> {
    let con = open_db()?;

    if let Some(planet) = resolve_single(&con, &args.query)? {
        print_planet(&planet);
        return Ok(());
    }

    let rows = search(&con, &args.query, 20)?;
    print_search_results(&rows);

    Ok(())
}
