pub mod edit;
pub mod find;
pub mod history;

use crate::cli::{EditCli, EditCommand};
use anyhow::Result;

pub fn run(args: EditCli) -> Result<()> {
    match args.command {
        Some(EditCommand::Find(cmd)) => find::run(cmd),
        Some(EditCommand::Edit(cmd)) => edit::run(cmd),
        Some(EditCommand::History(cmd)) => history::run(cmd),
        None => crate::interactive::wizard::run(),
    }
}