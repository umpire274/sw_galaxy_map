use anyhow::Result;
use sw_galaxy_map_cli::ui::error;

fn main() -> Result<()> {
    if let Err(e) = sw_galaxy_map_cli::cli::run() {
        error(format!("{:#}", e));
        std::process::exit(1);
    }
    println!();
    Ok(())
}
