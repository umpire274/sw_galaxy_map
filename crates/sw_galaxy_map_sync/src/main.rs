use anyhow::Result;
use clap::Parser;
use sw_galaxy_map_sync::cli::{ArcgisCommand, Cli, Commands};
use sw_galaxy_map_sync::pipeline::arcgis_import::{
    ArcgisFetchOptions, ArcgisImportOptions, fetch_arcgis_to_file, import_arcgis_to_sqlite,
};

fn main() -> Result<()> {
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
    }
}
