//! Fields command implementation.

use anyhow::Result;

use crate::edit::field::EditableField;

pub fn run() -> Result<()> {
    println!(
        "{:<12}  {:<10}  {:<6}  {:<8}  Description",
        "Field", "DB column", "Type", "Nullable"
    );
    println!(
        "{:-<12}  {:-<10}  {:-<6}  {:-<8}  {:-<1}",
        "", "", "", "", ""
    );

    for field in EditableField::all() {
        println!(
            "{:<12}  {:<10}  {:<6}  {:<8}  {}",
            field.cli_name(),
            field.column_name(),
            field.value_kind().as_str(),
            yes_no(field.nullable()),
            field.description()
        );
    }

    println!();
    println!("Notes:");
    println!("- Use an empty string to write NULL on nullable fields.");
    println!("- The 'planet' field also updates 'planet_norm'.");
    println!("- Accepted alias: 'ref' can be used for 'reference'.");

    Ok(())
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
