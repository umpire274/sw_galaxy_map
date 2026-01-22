use sw_galaxy_map::cli;
use sw_galaxy_map::gui;
use sw_galaxy_map::ui::error;

use anyhow::Result;

fn main() -> Result<()> {
    // If no extra args are provided, run GUI.
    // args_os().len() includes the executable name as the first argument.
    if std::env::args_os().len() <= 1 {
        return gui::run();
    }

    if let Err(e) = cli::run() {
        error(format!("{:#}", e));
        std::process::exit(1);
    }
    println!();

    Ok(())
}
