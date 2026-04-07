//! Find command implementation.

use crate::cli::FindArgs;
use crate::db::runtime::open_db;
use crate::output::planet::{print_planet, print_search_results};
use crate::resolve::planet::{resolve_by_fid, resolve_by_name_or_alias, search};
use anyhow::Result;

pub fn run(args: FindArgs) -> Result<()> {
    let con = open_db()?;
    let query = args.query.trim();

    if let Ok(fid) = query.parse::<i64>() {
        if let Some(planet) = resolve_by_fid(&con, fid)? {
            print_planet(&planet);
            return Ok(());
        }
    }

    if let Some(planet) = resolve_by_name_or_alias(&con, query)? {
        print_planet(&planet);
        return Ok(());
    }

    let rows = search(&con, query, 20)?;
    print_search_results(&rows);

    Ok(())
}