use sw_galaxy_map::cli;
use sw_galaxy_map::ui::error;

fn main() {
    if let Err(e) = cli::run() {
        error(format!("{:#}", e));
        std::process::exit(1);
    }
    println!();
}
