use crate::cli::args;
use crate::cli::commands::route::resolve_show_for_tui;
use crate::cli::db_runtime::open_db_migrating;
use crate::cli::shell::split_args;
use crate::tui::app::{App, SelectionMode};
use crate::tui::bridge::run_one_shot_for_tui;
use crate::tui::log::{
    enqueue_log_line, enqueue_log_lines, extend_log_lines, flush_typewriter,
    force_scroll_to_bottom, push_log_line, scroll_down, scroll_page_down, scroll_page_up,
    scroll_up,
};
use crate::tui::{
    NavigationPanelKind, TuiCommandOutput, build_navigation_panel, build_near_planet_panel,
    build_planet_panel, build_route_show_output,
};
use clap::Parser;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

/// Handle one keyboard event.
/// Returns true when the TUI should exit.
pub(crate) fn handle_key(key: KeyEvent, app: &mut App) -> bool {
    match key.code {
        KeyCode::Esc => return true,

        KeyCode::Tab => {
            app.next_panel();
            app.reset_cursor_blink();
        }

        KeyCode::BackTab => {
            app.previous_panel();
            app.reset_cursor_blink();
        }

        KeyCode::Backspace => {
            app.reset_history_navigation();
            app.input.pop();
            app.reset_cursor_blink();
        }

        KeyCode::Enter => {
            let command = app.input.trim().to_string();
            flush_typewriter(app);

            if let Some(rest) = command.strip_prefix(':') {
                let cmd = rest.trim().to_ascii_lowercase();

                match cmd.as_str() {
                    "q" | "quit" | "exit" | "x" => {
                        push_log_line(app, "Exiting...");
                        return true;
                    }
                    "help" => {
                        push_log_line(app, "System commands:");
                        push_log_line(app, "  :q | :quit | :exit | :x   Exit application");
                        push_log_line(app, "  :help                     Show this help");
                    }
                    _ => {
                        push_log_line(app, format!("Unknown system command: :{}", cmd));
                    }
                }

                app.input.clear();
                app.reset_cursor_blink();
                return false;
            }

            if !command.is_empty() {
                app.push_history(&command);
                push_log_line(app, format!("> {command}"));

                if let Some(index) = parse_selection(&command) {
                    handle_selection(app, index);
                    app.input.clear();
                    app.reset_cursor_blink();
                    return false;
                }

                match split_args(&command) {
                    Ok(tokens) => {
                        let mut argv: Vec<String> = Vec::with_capacity(tokens.len() + 3);
                        argv.push("sw_galaxy_map".to_string());

                        let user_passed_db = tokens.iter().any(|t| t == "--db");
                        if !user_passed_db && let Some(ref db) = app.session_db {
                            argv.push("--db".to_string());
                            argv.push(db.clone());
                        }

                        argv.extend(tokens);

                        match args::Cli::try_parse_from(argv) {
                            Ok(cli) => {
                                if let Some(ref cmd) = cli.cmd {
                                    match run_one_shot_for_tui(&cli, cmd) {
                                        Ok(out) => {
                                            let TuiCommandOutput {
                                                log_lines,
                                                planet1_title,
                                                planet1_lines,
                                                navigation_title,
                                                navigation_lines,
                                                planet2_title,
                                                planet2_lines,
                                                search_results,
                                                near_results,
                                                route_list_results,
                                            } = out;

                                            enqueue_log_lines(app, log_lines);

                                            let has_search_results = !search_results.is_empty();
                                            let has_near_results = !near_results.is_empty();
                                            let has_route_list_results =
                                                !route_list_results.is_empty();

                                            match (
                                                has_search_results,
                                                has_near_results,
                                                has_route_list_results,
                                            ) {
                                                (true, false, false) => {
                                                    app.search_results = search_results;
                                                    app.near_results.clear();
                                                    app.route_list_results.clear();
                                                    app.selection_mode = SelectionMode::Search;
                                                }
                                                (false, true, false) => {
                                                    app.near_results = near_results;
                                                    app.search_results.clear();
                                                    app.route_list_results.clear();
                                                    app.selection_mode = SelectionMode::Near;
                                                }
                                                (false, false, true) => {
                                                    app.route_list_results = route_list_results;
                                                    app.search_results.clear();
                                                    app.near_results.clear();
                                                    app.selection_mode = SelectionMode::RouteList;
                                                }
                                                _ => {
                                                    app.clear_selectable_results();
                                                }
                                            }

                                            app.planet1_title = planet1_title;
                                            app.planet1_lines = planet1_lines;

                                            app.navigation_title = navigation_title;
                                            app.navigation_lines = navigation_lines;
                                            app.navigation_scroll = 0;

                                            app.planet2_title = planet2_title;
                                            app.planet2_lines = planet2_lines;

                                            force_scroll_to_bottom(app);

                                            app.planet1_scroll = 0;
                                            app.planet2_scroll = 0;

                                            if matches!(cmd, args::Commands::Near { .. })
                                                && app.near_results.is_empty()
                                                && app.planet2_lines.len() > 1
                                                && app.planet2_lines[0] != "No data"
                                            {
                                                app.selected_panel = 3;
                                                app.reset_cursor_blink();
                                            }
                                        }
                                        Err(e) => {
                                            push_log_line(app, format!("Error: {e:#}"));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                push_log_line(app, e.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        push_log_line(app, format!("Parse error: {e:#}"));
                    }
                }
            }

            app.input.clear();
            app.reset_cursor_blink();
        }

        KeyCode::Char(c) => {
            app.reset_history_navigation();
            app.input.push(c);
            app.reset_cursor_blink();
        }

        KeyCode::Up => {
            if app.selected_panel == 4 {
                app.history_up();
            } else {
                scroll_up(app);
            }
        }

        KeyCode::Down => {
            if app.selected_panel == 4 {
                app.history_down();
            } else {
                scroll_down(app);
            }
        }

        KeyCode::PageUp => scroll_page_up(app),
        KeyCode::PageDown => scroll_page_down(app),

        _ => {}
    }

    false
}

fn parse_selection(command: &str) -> Option<usize> {
    let trimmed = command.trim();

    if let Ok(n) = trimmed.parse::<usize>() {
        return Some(n);
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("option ")
        && let Ok(n) = rest.trim().parse::<usize>()
    {
        return Some(n);
    }

    None
}

fn handle_search_selection(app: &mut App, index: usize) {
    let Some(planet) = app.search_results.get(index - 1).cloned() else {
        push_log_line(
            app,
            format!("Invalid selection: option {} does not exist.", index),
        );
        return;
    };

    let (title, lines) = build_planet_panel(&planet, None);

    app.planet1_title = title;
    app.planet1_lines = lines;
    app.planet1_scroll = 0;

    app.planet2_title = Line::from("Planet 2 Information");
    app.planet2_lines = vec!["No data".to_string()];
    app.planet2_scroll = 0;

    enqueue_log_line(app, format!("Selected result {}: {}", index, planet.name));
}

fn handle_near_selection(app: &mut App, index: usize) {
    let Some(hit) = app.near_results.get(index - 1).cloned() else {
        push_log_line(
            app,
            format!("Invalid selection: option {} does not exist.", index),
        );
        return;
    };

    let con = match open_db_migrating(app.session_db.clone()) {
        Ok(con) => con,
        Err(e) => {
            push_log_line(app, format!("Database error: {e:#}"));
            return;
        }
    };

    let (planet, aliases) = match crate::cli::commands::info::resolve_by_fid(&con, hit.fid) {
        Ok(data) => data,
        Err(e) => {
            push_log_line(app, format!("Failed to load nearby planet details: {e:#}"));
            return;
        }
    };

    let (title, lines) = build_near_planet_panel(&planet, Some(&aliases));

    app.planet2_title = title;
    app.planet2_lines = lines;
    app.planet2_scroll = 0;

    let (nav_title, nav_lines) = build_navigation_panel(NavigationPanelKind::Near {
        distance_parsec: hit.distance,
        reference_name: None,
    });
    app.navigation_title = nav_title;
    app.navigation_lines = nav_lines;
    app.navigation_scroll = 0;

    app.selected_panel = 3;

    enqueue_log_line(
        app,
        format!("Selected nearby planet {}: {}", index, planet.name),
    );
}

fn handle_route_list_selection(app: &mut App, index: usize) {
    let Some(item) = app.route_list_results.get(index - 1).cloned() else {
        push_log_line(
            app,
            format!("Invalid selection: option {} does not exist.", index),
        );
        return;
    };

    let con = match open_db_migrating(app.session_db.clone()) {
        Ok(con) => con,
        Err(e) => {
            push_log_line(app, format!("Database error: {e:#}"));
            return;
        }
    };

    let data = match resolve_show_for_tui(&con, item.route_id) {
        Ok(data) => data,
        Err(e) => {
            push_log_line(
                app,
                format!("Failed to open route {}: {e:#}", item.route_id),
            );
            return;
        }
    };

    let out = match build_route_show_output(&con, &data.loaded) {
        Ok(out) => out,
        Err(e) => {
            push_log_line(
                app,
                format!("Failed to render route {}: {e:#}", item.route_id),
            );
            return;
        }
    };

    let TuiCommandOutput {
        log_lines,
        planet1_title,
        planet1_lines,
        navigation_title,
        navigation_lines,
        planet2_title,
        planet2_lines,
        ..
    } = out;

    extend_log_lines(app, log_lines);
    app.clear_selectable_results();

    app.planet1_title = planet1_title;
    app.planet1_lines = planet1_lines;
    app.navigation_title = navigation_title;
    app.navigation_lines = navigation_lines;
    app.navigation_scroll = 0;
    app.planet2_title = planet2_title;
    app.planet2_lines = planet2_lines;

    app.planet1_scroll = 0;
    app.planet2_scroll = 0;
    app.selected_panel = 0;
    app.reset_cursor_blink();

    enqueue_log_line(
        app,
        format!(
            "Opened route {}: {} → {}",
            item.route_id, item.from_name, item.to_name
        ),
    );
}

fn handle_selection(app: &mut App, index: usize) {
    if index == 0 {
        push_log_line(app, "Invalid selection: use a number starting from 1.");
        return;
    }

    match app.selection_mode {
        SelectionMode::Search => handle_search_selection(app, index),
        SelectionMode::Near => handle_near_selection(app, index),
        SelectionMode::RouteList => handle_route_list_selection(app, index),
        SelectionMode::None => {
            push_log_line(app, "No selectable results available.");
        }
    }
}
