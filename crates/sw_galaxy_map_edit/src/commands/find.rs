//! Find command implementation.

use crate::cli::FindArgs;
use anyhow::Result;

pub fn run(args: FindArgs) -> Result<()> {
    println!("find command not implemented yet");
    println!("query: {}", args.query);
    Ok(())
}