pub mod args;
pub mod color;
pub mod commands;
pub mod export;
pub mod validate;

use crate::db::db_status::resolve_db_path;
use crate::gui;
use crate::ui::warning;
use anyhow::{Result, bail};
use clap::Parser;
use std::path::Path;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    println!();

    // Enforce: --gui must be used without subcommands
    if cli.gui && cli.cmd.is_some() {
        bail!(
            "'--gui' cannot be used together with subcommands. Run either '--gui' or a CLI command."
        );
    }

    // 1) GUI explicit
    if cli.gui {
        return run_gui();
    }

    // 2) One-shot CLI
    if let Some(cmd) = &cli.cmd {
        return run_one_shot(&cli, cmd);
    }

    // 3) Default: interactive CLI
    run_interactive_shell(cli.db.clone())
}

fn run_one_shot(cli: &args::Cli, cmd: &args::Commands) -> Result<()> {
    match cmd {
        args::Commands::Db { cmd } => match cmd {
            args::DbCommands::Init { out, force } => crate::db::db_init::run(out.clone(), *force),

            args::DbCommands::Status => crate::db::db_status::run(cli.db.clone()),

            args::DbCommands::Update {
                prune,
                dry_run,
                stats,
                stats_limit,
            } => {
                let mut con = open_db_migrating(cli.db.clone())?;
                crate::db::db_update::run(&mut con, *prune, *dry_run, *stats, *stats_limit)
            }

            args::DbCommands::Migrate { dry_run } => {
                // IMPORTANT: do not auto-migrate before running migrate
                let mut con = open_db_raw(cli.db.clone())?;
                crate::db::migrate::run(&mut con, *dry_run, true)
            }
        },

        args::Commands::Search { query, limit } => {
            validate::validate_search(query, *limit)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::search::run(&con, query.clone(), *limit)
        }

        args::Commands::Info { planet } => {
            let con = open_db_migrating(cli.db.clone())?;
            commands::info::run(&con, planet.clone())
        }

        args::Commands::Near {
            r,
            planet,
            x,
            y,
            limit,
        } => {
            validate::validate_near(planet, x, y)?;
            let con = open_db_migrating(cli.db.clone())?;
            commands::near::run(&con, *r, planet.clone(), *x, *y, *limit)
        }

        args::Commands::Waypoint { cmd } => {
            let mut con = open_db_migrating(cli.db.clone())?;
            commands::waypoints::run_waypoint(&mut con, cmd)
        }

        args::Commands::Route { cmd } => {
            let mut con = open_db_migrating(cli.db.clone())?;
            commands::route::run(&mut con, cmd)
        }
    }
}

fn run_gui() -> Result<()> {
    // Execute the GUI
    gui::run()
}

fn run_interactive_shell(db_arg: Option<String>) -> Result<()> {
    use std::io::{self, Write};

    println!("Interactive mode (v0.8.0). Type 'help' or 'exit'.");
    println!(
        "Tip: commands are the same as one-shot CLI (e.g. `search scarif`, `route show 42`).\n"
    );

    // Default DB for the session (user can still pass --db per command)
    let mut session_db = db_arg;

    loop {
        print!("sw_galaxy_map> ");
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Built-in REPL commands (start with ':')
        if let Some(rest) = line.strip_prefix(':') {
            let rest = rest.trim();
            if rest == "exit" || rest == "quit" {
                break;
            }
            if rest == "help" {
                println!("REPL commands:");
                println!("  :help                 Show this help");
                println!("  :exit | :quit         Exit interactive mode");
                println!("  :db <path>            Set default DB path for this session");
                println!("  :db                   Show current default DB");
                println!();
                continue;
            }
            if rest == "db" {
                println!(
                    "Default DB: {}",
                    session_db.as_deref().unwrap_or("<auto-resolved (default)>")
                );
                continue;
            }
            if let Some(path) = rest.strip_prefix("db ") {
                let path = path.trim();
                if path.is_empty() {
                    println!("Usage: :db <path>");
                } else {
                    session_db = Some(path.to_string());
                    println!("Default DB set to: {}", path);
                }
                continue;
            }

            println!("Unknown REPL command: :{}", rest);
            continue;
        }

        if line == "exit" || line == "quit" {
            break;
        }
        if line == "help" {
            println!("Try a normal command, e.g.:");
            println!("  search scarif");
            println!("  info coruscant");
            println!("  near --planet coruscant -r 50");
            println!("  route show 42");
            println!("Or REPL help: :help\n");
            continue;
        }

        // 1) Split into tokens
        let tokens = match split_args(line) {
            Ok(t) => t,
            Err(e) => {
                println!("Parse error: {:#}", e);
                continue;
            }
        };

        // 2) Build argv for clap: ["sw_galaxy_map", <tokens...>]
        let mut argv: Vec<String> = Vec::with_capacity(tokens.len() + 2);
        argv.push("sw_galaxy_map".to_string());

        // If the user didn't pass --db explicitly, inject session default
        let user_passed_db = tokens.iter().any(|t| t == "--db");
        if !user_passed_db && let Some(ref db) = session_db {
            argv.push("--db".to_string());
            argv.push(db.clone());
        }

        argv.extend(tokens);

        // 3) Parse with clap
        match args::Cli::try_parse_from(argv) {
            Ok(cli) => {
                // Enforce same rule as non-interactive: --gui not allowed here
                if cli.gui {
                    println!(
                        "Error: '--gui' is not available inside interactive mode. Use ':exit' then run `sw_galaxy_map --gui`."
                    );
                    continue;
                }

                // No subcommand (should be rare): ignore
                let Some(ref cmd) = cli.cmd else {
                    continue;
                };

                // 4) Execute using the same dispatcher
                if let Err(e) = run_one_shot(&cli, cmd) {
                    println!("Error: {:#}", e);
                }

                println!();
            }
            Err(e) => {
                // Clap pretty error (unknown cmd, wrong args, etc.)
                // Use println so it shows in the REPL output buffer
                println!("{}", e);
                println!();
            }
        }
    }

    Ok(())
}

fn split_args(line: &str) -> anyhow::Result<Vec<String>> {
    Ok(shell_words::split(line)?)
}

fn open_db_raw(db_arg: Option<String>) -> Result<rusqlite::Connection> {
    let db_path = resolve_db_path(db_arg)?;
    ensure_db_ready(&db_path)?;
    crate::db::open_db(&db_path.to_string_lossy())
}

fn open_db_migrating(db_arg: Option<String>) -> Result<rusqlite::Connection> {
    let mut con = open_db_raw(db_arg)?;
    crate::db::migrate::run(&mut con, false, false)?;
    Ok(con)
}

fn ensure_db_ready(db_path: &Path) -> Result<()> {
    if db_path.exists() {
        return Ok(());
    }

    println!();
    warning(format!(
        "Local database not found at: {}\nInitializing it now (this may take a moment)...",
        db_path.display()
    ));

    crate::db::db_init::run(Some(db_path.to_string_lossy().to_string()), false)
}
