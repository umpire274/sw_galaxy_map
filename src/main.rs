use anyhow::Result;

mod cli;
mod db;
mod model;
mod normalize;
mod provision;

fn main() -> Result<()> {
    cli::run()
}
