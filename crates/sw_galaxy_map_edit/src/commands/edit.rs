//! Edit command implementation.

use crate::cli::EditArgs;
use anyhow::{Result, bail};

pub fn run(args: EditArgs) -> Result<()> {
    if args.fid.is_none() && args.planet.is_none() {
        bail!("You must provide either --fid <FID> or --planet <NAME>.");
    }

    println!("edit command not implemented yet");
    println!("fid         : {:?}", args.fid);
    println!("planet      : {:?}", args.planet);
    println!("interactive : {}", args.interactive);

    Ok(())
}