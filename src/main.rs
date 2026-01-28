use sw_galaxy_map::cli;
use sw_galaxy_map::ui::error;

use anyhow::Result;

fn main() -> Result<()> {
    if let Err(e) = cli::run() {
        error(format!("{:#}", e));
        std::process::exit(1);
    }
    println!();

    Ok(())
}
