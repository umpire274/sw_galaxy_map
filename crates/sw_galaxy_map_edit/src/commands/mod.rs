//! Command dispatch for sw_galaxy_map_edit.

pub mod edit;
pub mod find;

use crate::cli::{EditCli, EditCommand};
use anyhow::Result;

pub fn run(args: EditCli) -> Result<()> {
    match args.command {
        Some(EditCommand::Find(cmd)) => find::run(cmd),
        Some(EditCommand::Edit(cmd)) => edit::run(cmd),

        None => {
            // 👇 fallback: wizard interattivo
            crate::interactive::wizard::run()
        }
    }
}
