use anyhow::{Result, bail};
use clap::Parser;
use sw_galaxy_map_sync::cli::{ArcgisCommand, Cli, Commands, CsvCommand, DbCommands};
use sw_galaxy_map_sync::db::config::load_db_config;
use sw_galaxy_map_sync::db::postgres;
use sw_galaxy_map_sync::pipeline::arcgis_import::{
    ArcgisFetchOptions, ArcgisImportOptions, fetch_arcgis_to_file, import_arcgis_to_sqlite,
};
use sw_galaxy_map_sync::pipeline::csv_overlay::{CsvOverlayOptions, apply_csv_overlay};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Arcgis(command) => match command {
            ArcgisCommand::Fetch {
                out,
                page_size,
                pretty,
            } => fetch_arcgis_to_file(&ArcgisFetchOptions {
                out,
                page_size,
                pretty,
            }),

            ArcgisCommand::Import {
                db,
                table,
                unknown_table,
                page_size,
                dry_run,
            } => {
                let result = import_arcgis_to_sqlite(&ArcgisImportOptions {
                    db,
                    table,
                    unknown_table,
                    page_size,
                    dry_run,
                })?;

                println!();
                println!("ArcGIS import completed.");
                println!("Fetched          : {}", result.fetched);
                println!("Known records    : {}", result.known);
                println!("Unknown records  : {}", result.unknown);
                println!("Known inserted   : {}", result.inserted);
                println!("Known updated    : {}", result.updated);
                println!("Known skipped    : {}", result.skipped);
                println!("Unknown inserted : {}", result.unknown_inserted);
                println!("Unknown updated  : {}", result.unknown_updated);
                println!("Unknown skipped  : {}", result.unknown_skipped);
                println!("Dry run          : {}", result.dry_run);

                Ok(())
            }
        },
        Commands::Csv(command) => match command {
            CsvCommand::Overlay {
                db,
                csv,
                table,
                unknown_table,
                format,
                delimiter,
                mark_deleted,
                dry_run,
            } => {
                let delimiter = parse_delimiter(delimiter)?;
                let stats = apply_csv_overlay(&CsvOverlayOptions {
                    db,
                    csv,
                    table,
                    unknown_table,
                    format,
                    delimiter,
                    dry_run,
                    mark_deleted,
                })?;

                println!();
                println!("CSV overlay completed.");
                println!("Rows read        : {}", stats.rows_read);
                println!("Inserted         : {}", stats.inserted);
                println!("Active           : {}", stats.active);
                println!("Modified exact   : {}", stats.modified_exact);
                println!("Modified suffix  : {}", stats.modified_suffix);
                println!("Deleted          : {}", stats.deleted);
                println!("Skipped          : {}", stats.skipped);
                println!("Dry run          : {}", stats.dry_run);

                Ok(())
            }
        },
        Commands::Db(command) => match command {
            DbCommands::Test { db_config } => {
                let cfg = load_db_config(&db_config)?;
                postgres::test_connection(&cfg).await
            }
        },
    }
}

fn parse_delimiter(value: Option<String>) -> Result<Option<u8>> {
    let Some(value) = value else {
        return Ok(None);
    };

    let mut chars = value.chars();
    let Some(ch) = chars.next() else {
        bail!("CSV delimiter cannot be empty");
    };

    if chars.next().is_some() {
        bail!("CSV delimiter must be a single character");
    }

    if !ch.is_ascii() {
        bail!("CSV delimiter must be an ASCII character");
    }

    Ok(Some(ch as u8))
}
