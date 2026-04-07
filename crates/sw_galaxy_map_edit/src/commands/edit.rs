//! Edit command implementation.

use crate::cli::EditArgs;
use crate::db::runtime::open_db;
use crate::interactive::wizard;
use crate::output::planet::print_planet;
use crate::resolve::planet::{resolve_by_fid, resolve_by_name_or_alias};
use anyhow::{Result, bail};

pub fn run(args: EditArgs) -> Result<()> {
    if args.interactive {
        return wizard::run();
    }

    if args.fid.is_none() && args.planet.is_none() {
        bail!("You must provide either --fid <FID> or --planet <NAME>, or use --interactive.");
    }

    let con = open_db()?;

    let planet = if let Some(fid) = args.fid {
        resolve_by_fid(&con, fid)?
    } else if let Some(name) = args.planet.as_deref() {
        resolve_by_name_or_alias(&con, name)?
    } else {
        None
    };

    match planet {
        Some(planet) => {
            println!("Editing target resolved:");
            println!();
            print_planet(&planet);
            Ok(())
        }
        None => {
            bail!("Planet not found.");
        }
    }
}
