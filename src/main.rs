use anyhow::Result;

mod cli;
mod db;
mod model;
mod normalize;

fn main() -> Result<()> {
    cli::run()
}
