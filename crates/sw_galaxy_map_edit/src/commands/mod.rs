pub mod edit;
pub mod fields;
pub mod find;
pub mod history;
pub mod set;

use crate::cli::{EditCli, EditCommand};
use anyhow::Result;

pub fn run(args: EditCli) -> Result<()> {
    match args.command {
        Some(EditCommand::Find(cmd)) => find::run(cmd),
        Some(EditCommand::Edit(cmd)) => edit::run(cmd),
        Some(EditCommand::History(cmd)) => history::run(cmd),
        Some(EditCommand::Set(cmd)) => set::run(cmd),
        Some(EditCommand::Fields) => fields::run(),
        None => crate::interactive::wizard::run(),
    }
}