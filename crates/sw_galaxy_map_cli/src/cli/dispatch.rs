use crate::cli::reports::{
    print_local_pull_preparation_report, print_pull_diff_report, print_pull_update_plan,
};
use crate::cli::{
    args, commands, open_db_migrating, open_db_raw, print_db_init_report, print_db_status_report,
    print_db_update_report, print_galaxy_stats, print_migration_report,
};
use crate::ui::{info, success};
use std::path::PathBuf;
use sw_galaxy_map_core::db::pull::config::RemoteDbConfig;
use sw_galaxy_map_core::db::pull::diff::diff_local_with_remote;
use sw_galaxy_map_core::db::pull::update::PullUpdatePlan;
use sw_galaxy_map_core::db::pull::validate::validate_remote_database;
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

            args::DbCommands::Sync { table, dry_run, .. } => {
                const DEFAULT_ARCGIS_PAGE_SIZE: i64 = 2000;
                const DEFAULT_UNKNOWN_TABLE: &str = "planets_unknown";

                info("Running ArcGIS ingestion sync...");

                let db_path = cli
                    .db
                    .clone()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow::anyhow!("Database path is required"))?;

                let result = sw_galaxy_map_sync::pipeline::arcgis_import::import_arcgis_to_sqlite(
                    &sw_galaxy_map_sync::pipeline::arcgis_import::ArcgisImportOptions {
                        db: db_path,
                        table: table.clone(),
                        unknown_table: DEFAULT_UNKNOWN_TABLE.to_string(),
                        page_size: DEFAULT_ARCGIS_PAGE_SIZE,
                        dry_run: *dry_run,
                    },
                )?;

                println!();
                info("ArcGIS sync summary:");
                println!("  Fetched          : {}", result.fetched);
                println!("  Known records    : {}", result.known);
                println!("  Unknown records  : {}", result.unknown);
                println!("  Known inserted   : {}", result.inserted);
                println!("  Known updated    : {}", result.updated);
                println!("  Known skipped    : {}", result.skipped);
                println!("  Unknown inserted : {}", result.unknown_inserted);
                println!("  Unknown updated  : {}", result.unknown_updated);
                println!("  Unknown skipped  : {}", result.unknown_skipped);

                if !*dry_run {
                    let mut con = open_db_migrating(cli.db.clone())?;

                    println!();
                    info("Rebuilding planet_search and FTS indexes...");
                    sw_galaxy_map_core::db::provision::rebuild_search_indexes(&mut con)?;
                    success("ArcGIS sync complete. Search indexes rebuilt.");
                } else {
                    success("Dry run complete. No changes written.");
                }

                Ok(())
            }

            args::DbCommands::Backup(args) => commands::db::backup::run(cli.db.clone(), args),

            args::DbCommands::Export(args) => commands::db::export::run(cli.db.clone(), args),

            args::DbCommands::Pull {
                db,
                remote_config,
                dry_run,
                show_suspicious,
            } => {
                let remote_config = RemoteDbConfig::from_json_file(&remote_config)?;

                let runtime = tokio::runtime::Runtime::new()?;

                let validation =
                    runtime.block_on(async { validate_remote_database(&remote_config).await })?;

                if !validation.is_valid {
                    println!();
                    println!("Remote database validation failed.");

                    for message in &validation.messages {
                        println!("  - {message}");
                    }

                    anyhow::bail!("Remote database is not valid for local pull operations.");
                }
                println!();
                println!("Remote database validation passed.");
                println!("Remote planets        : {}", validation.planets_count);
                println!(
                    "Remote unknown records: {}",
                    validation.planets_unknown_count
                );

                if *dry_run {
                    let report = runtime
                        .block_on(async { diff_local_with_remote(&db, &remote_config).await })?;

                    print_pull_diff_report(&report, *show_suspicious);
                    let plan = PullUpdatePlan::from_diff(&report);
                    print_pull_update_plan(&plan);
                } else {
                    let report = runtime
                        .block_on(async { diff_local_with_remote(&db, &remote_config).await })?;
                    let plan = PullUpdatePlan::from_diff(&report);

                    print_pull_update_plan(&plan);

                    if !plan.can_apply {
                        anyhow::bail!(
                            "Local pull cannot be applied safely. Resolve blocking reasons first."
                        );
                    }

                    let backup_id =
                        format!("local_pull_{}", chrono::Utc::now().format("%Y%m%dT%H%M%SZ"));

                    let conn = rusqlite::Connection::open(&db)?;

                    let preparation =
                        sw_galaxy_map_core::db::pull::staging::prepare_local_pull_staging(
                            &conn, &backup_id, false,
                        )?;

                    print_local_pull_preparation_report(&preparation, &backup_id);

                    anyhow::bail!("Real local pull update is not implemented yet.");
                }

                Ok(())
            }
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
