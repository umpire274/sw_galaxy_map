//! Guided interactive editor.

use anyhow::Result;
use inquire::Text;

pub fn run() -> Result<()> {
    println!("sw_galaxy_map_edit interactive mode");
    println!();

    let query = Text::new("Planet name, alias, or FID:")
        .with_help_message("Example: Coruscant, Tatooine, or 1234")
        .prompt()?;

    println!();
    println!("You entered: {}", query);
    println!("Interactive editing flow not implemented yet.");

    Ok(())
}