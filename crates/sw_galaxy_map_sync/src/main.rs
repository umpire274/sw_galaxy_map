use anyhow::{Result, bail};
use clap::Parser;
use sw_galaxy_map_sync::cli::{
    ArcgisCommand, Cli, Commands, ConvertCommands, CoordinateUnitArg, CsvCommand, DbCommands,
    DbDriverArg,
};
use sw_galaxy_map_sync::db::config::load_db_config;
use sw_galaxy_map_sync::db::postgres;
use sw_galaxy_map_sync::pipeline::arcgis_import::{
    ArcgisFetchOptions, ArcgisImportOptions, fetch_arcgis_to_file, import_arcgis_to_postgres,
    import_arcgis_to_sqlite,
};
use sw_galaxy_map_sync::pipeline::convert_coordinates::{
    ConvertCoordinatesOptions, print_convert_coordinates_summary, rollback_coordinates_postgres,
};
use sw_galaxy_map_sync::pipeline::convert_coordinates::{
    convert_coordinates_postgres, convert_coordinates_sqlite, print_convert_rollback_summary,
    rollback_coordinates_sqlite,
};
use sw_galaxy_map_sync::pipeline::csv_overlay::{
    CsvOverlayOptions, apply_csv_overlay, apply_csv_overlay_postgres,
};

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
                driver,
                db,
                db_config,
                table,
                unknown_table,
                page_size,
                dry_run,
            } => match driver {
                DbDriverArg::Sqlite => {
                    let db =
                        db.ok_or_else(|| anyhow::anyhow!("--db is required when --driver sqlite"))?;

                    let result = tokio::task::spawn_blocking(move || {
                        import_arcgis_to_sqlite(&ArcgisImportOptions {
                            db,
                            table,
                            unknown_table,
                            page_size,
                            dry_run,
                        })
                    })
                    .await??;

                    println!();
                    println!("ArcGIS import completed.");
                    println!("Backend          : sqlite");
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

                DbDriverArg::Postgres => {
                    let db_config = db_config.ok_or_else(|| {
                        anyhow::anyhow!("--db-config is required when --driver postgres")
                    })?;

                    let cfg = load_db_config(&db_config)?;

                    let result =
                        import_arcgis_to_postgres(&cfg, table, unknown_table, page_size, dry_run)
                            .await?;

                    println!();
                    println!("ArcGIS import completed.");
                    println!("Backend          : postgres");
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

                DbDriverArg::Mysql => {
                    bail!("MySQL backend is not implemented yet");
                }
            },
        },

        Commands::Csv(command) => match command {
            CsvCommand::Overlay {
                driver,
                db,
                db_config,
                csv,
                table,
                unknown_table,
                format,
                delimiter,
                mark_deleted,
                dry_run,
            } => match driver {
                DbDriverArg::Sqlite => {
                    let delimiter = parse_delimiter(delimiter)?;
                    let stats = apply_csv_overlay(&CsvOverlayOptions {
                        driver,
                        db,
                        db_config,
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
                DbDriverArg::Postgres => {
                    let delimiter = parse_delimiter(delimiter)?;
                    let stats = apply_csv_overlay_postgres(&CsvOverlayOptions {
                        driver,
                        db,
                        db_config,
                        csv,
                        table,
                        unknown_table,
                        format,
                        delimiter,
                        dry_run,
                        mark_deleted,
                    })
                    .await?;

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

                DbDriverArg::Mysql => {
                    bail!("MySQL backend is not implemented yet");
                }
            },
        },

        Commands::Db(command) => match command {
            DbCommands::Test { db_config } => {
                let cfg = load_db_config(&db_config)?;
                postgres::test_connection(&cfg).await
            }

            DbCommands::Bootstrap {
                driver,
                db,
                db_config,
            } => match driver {
                DbDriverArg::Sqlite => {
                    let db =
                        db.ok_or_else(|| anyhow::anyhow!("--db is required when --driver sqlite"))?;

                    let conn = rusqlite::Connection::open(&db).map_err(|err| {
                        anyhow::anyhow!("Unable to open SQLite DB {}: {err}", db.display())
                    })?;

                    sw_galaxy_map_sync::db::schema::sqlite::create_sqlite_schema(&conn)?;

                    println!("SQLite schema bootstrap completed.");
                    println!("Database: {}", db.display());

                    Ok(())
                }

                DbDriverArg::Postgres => {
                    let db_config = db_config.ok_or_else(|| {
                        anyhow::anyhow!("--db-config is required when --driver postgres")
                    })?;

                    let cfg = load_db_config(&db_config)?;

                    postgres::bootstrap_schema(&cfg).await
                }

                DbDriverArg::Mysql => {
                    bail!("MySQL backend is not implemented yet");
                }
            },
        },

        Commands::Convert { command } => match command {
            ConvertCommands::Coordinates {
                driver,
                db,
                db_config,
                to,
                dry_run,
            } => {
                let options = ConvertCoordinatesOptions {
                    driver,
                    db,
                    db_config,
                    to,
                    dry_run,
                };

                match driver {
                    DbDriverArg::Sqlite => {
                        let stats = convert_coordinates_sqlite(&options)?;
                        print_convert_coordinates_summary(&stats);
                        Ok(())
                    }

                    DbDriverArg::Postgres => {
                        let stats = convert_coordinates_postgres(&options).await?;
                        print_convert_coordinates_summary(&stats);
                        Ok(())
                    }

                    DbDriverArg::Mysql => {
                        anyhow::bail!("MySQL backend is not implemented yet")
                    }
                }
            }

            ConvertCommands::Rollback {
                driver,
                db,
                db_config,
                dry_run,
            } => {
                let options = ConvertCoordinatesOptions {
                    driver,
                    db,
                    db_config,
                    to: CoordinateUnitArg::Pc, // non usato dal rollback
                    dry_run,
                };

                match driver {
                    DbDriverArg::Sqlite => {
                        let stats = rollback_coordinates_sqlite(&options)?;
                        print_convert_rollback_summary(&stats);
                        Ok(())
                    }

                    DbDriverArg::Postgres => {
                        let stats = rollback_coordinates_postgres(&options).await?;
                        print_convert_rollback_summary(&stats);
                        Ok(())
                    }

                    DbDriverArg::Mysql => {
                        bail!("MySQL backend is not implemented yet")
                    }
                }
            }
        },
    }
}

fn parse_delimiter(value: Option<char>) -> Result<Option<u8>> {
    let Some(ch) = value else {
        return Ok(None);
    };

    if !ch.is_ascii() {
        bail!("CSV delimiter must be an ASCII character");
    }

    Ok(Some(ch as u8))
}
