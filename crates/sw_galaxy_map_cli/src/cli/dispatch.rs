use crate::cli::{
    args, commands, open_db_migrating, open_db_raw, print_db_init_report, print_db_status_report,
    print_db_update_report, print_galaxy_stats, print_migration_report,
};
use crate::ui::{info, success};
use sw_galaxy_map_core::validate;

pub(crate) fn run_one_shot(cli: &args::Cli, cmd: &args::Commands) -> anyhow::Result<()> {
    match cmd {
        args::Commands::Db { cmd } => match cmd {
            args::DbCommands::Init { out, force } => {
                let report = sw_galaxy_map_core::db::db_init::run(out.clone(), *force)?;
                print_db_init_report(&report);
                Ok(())
            }

            args::DbCommands::Status => {
                let report = sw_galaxy_map_core::db::db_status::run(cli.db.clone())?;
                print_db_status_report(&report);
                Ok(())
            }

            args::DbCommands::Update {
                prune,
                dry_run,
                stats,
                stats_limit,
            } => {
                let mut con = open_db_migrating(cli.db.clone())?;
                let report = sw_galaxy_map_core::db::db_update::run(
                    &mut con,
                    *prune,
                    *dry_run,
                    *stats,
                    *stats_limit,
                )?;
                print_db_update_report(&report);
                Ok(())
            }

            args::DbCommands::SkippedPlanets => {
                let mut con = open_db_migrating(cli.db.clone())?;
                sw_galaxy_map_core::db::db_skipped_planets::run(&mut con)
            }

            args::DbCommands::Migrate { dry_run } => {
                // IMPORTANT: do not auto-migrate before running migrate
                let mut con = open_db_raw(cli.db.clone())?;
                let report = sw_galaxy_map_core::db::migrate::run(&mut con, *dry_run, true)?;
                print_migration_report(&report);
                Ok(())
            }

            args::DbCommands::RebuildSearch => {
                let mut con = open_db_migrating(cli.db.clone())?;
                info("Rebuilding planet_search and FTS indexes...");
                sw_galaxy_map_core::db::provision::rebuild_search_indexes(&mut con)?;
                success("planet_search and FTS indexes rebuilt successfully.");
                Ok(())
            }

            args::DbCommands::Stats { top } => {
                let con = open_db_migrating(cli.db.clone())?;
                let s = sw_galaxy_map_core::db::queries::galaxy_stats(&con, *top)?;
                print_galaxy_stats(&s, *top);
                Ok(())
            }

            args::DbCommands::Sync {
                csv,
                table,
                delimiter,
                dry_run,
                mark_deleted,
                report,
            } => {
                let csv_path = sw_galaxy_map_sync::resolve_csv_path(csv)?;

                let delimiter_byte = delimiter
                    .to_string()
                    .as_bytes()
                    .first()
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Invalid delimiter"))?;

                let mut con = open_db_migrating(cli.db.clone())?;

                info(format!("Syncing from CSV: {}", csv_path.display()));

                let opts = sw_galaxy_map_sync::SyncOptions {
                    csv: csv_path,
                    table: table.clone(),
                    delimiter: delimiter_byte,
                    dry_run: *dry_run,
                    mark_deleted: *mark_deleted,
                    report_path: report.clone(),
                };

                let result = sw_galaxy_map_sync::run_sync(&mut con, &opts)?;

                println!();
                info("Sync summary:");
                println!("  Inserted         : {}", result.stats.inserted);
                println!("  Updated exact    : {}", result.stats.updated_exact);
                println!("  Updated suffix   : {}", result.stats.updated_suffix);
                println!("  Invalid CSV rows : {}", result.stats.invalid_csv_rows);
                println!("  Marked invalid   : {}", result.stats.invalid_marked);
                println!("  Skipped DB       : {}", result.stats.skipped_db);
                println!("  Logically deleted: {}", result.stats.deleted_logically);

                if !*dry_run {
                    println!();
                    info("Rebuilding planet_search and FTS indexes...");
                    sw_galaxy_map_core::db::provision::rebuild_search_indexes(&mut con)?;
                    success("Sync complete. Search indexes rebuilt.");
                } else {
                    success("Dry run complete. No changes written.");
                }

                Ok(())
            }

            args::DbCommands::Backup(args) => commands::db::backup::run(args),

            args::DbCommands::Export(args) => commands::db::export::run(args),
        },

        args::Commands::Search {
            query,
            region,
            sector,
            grid,
            status,
            canon,
            legends,
            fuzzy,
            limit,
        } => {
            let filter = sw_galaxy_map_core::model::SearchFilter {
                query: query.clone(),
                region: region.clone(),
                sector: sector.clone(),
                grid: grid.clone(),
                status: status.clone(),
                canon: if *canon { Some(true) } else { None },
                legends: if *legends { Some(true) } else { None },
                fuzzy: *fuzzy,
                limit: *limit,
            };
            validate::validate_search(&filter)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::search::run(&con, filter)
        }

        args::Commands::Info { planet } => {
            let con = open_db_migrating(cli.db.clone())?;
            commands::info::run(&con, planet.clone())
        }

        args::Commands::Near {
            range,
            unknown,
            fid,
            planet,
            x,
            y,
            limit,
        } => {
            validate::validate_near(*unknown, fid, planet, x, y)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::near::run(&con, *range, *unknown, *fid, planet.clone(), *x, *y, *limit)
        }

        args::Commands::Waypoint { cmd } => {
            let mut con = open_db_migrating(cli.db.clone())?;
            commands::waypoints::run_waypoint(&mut con, cmd)
        }

        args::Commands::Route { cmd } => {
            let mut con = open_db_migrating(cli.db.clone())?;
            commands::route::run(&mut con, cmd)
        }

        args::Commands::Unknown { cmd } => {
            let con = open_db_migrating(cli.db.clone())?;
            commands::unknown::run(&con, cmd)
        }
    }
}
