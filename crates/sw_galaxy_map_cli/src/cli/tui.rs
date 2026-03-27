use crate::cli::typewriter::{TypewriterConfig, TypewriterMode, TypewriterState};
use crate::cli::{NavigationPanelKind, build_navigation_panel};
use clap::Parser;
use crossterm::event::KeyEventKind;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{
    io,
    time::{Duration, Instant},
};

const PANEL_COUNT: usize = 5;
const BLINK_INTERVAL_MS: u64 = 500;
const POLL_INTERVAL_MS: u64 = 50;
const PAGE_SCROLL_STEP: u16 = 10;

/// Run the interactive TUI.
pub fn run_tui(db_arg: Option<String>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let result = run_app(&mut terminal, db_arg);

    terminal.show_cursor()?;
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// Main TUI event loop.
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    db_arg: Option<String>,
) -> io::Result<()> {
    let mut app = App {
        session_db: db_arg,
        ..App::default()
    };

    loop {
        app.update_cursor_blink();
        update_typewriter(&mut app);

        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(Duration::from_millis(POLL_INTERVAL_MS))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && handle_key(key, &mut app)
        {
            break;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionMode {
    None,
    Search,
    Near,
    RouteList,
}

/// Interactive TUI state.
struct App {
    log: Vec<String>,
    input: String,
    selected_panel: usize,

    cursor_visible: bool,
    last_blink: Instant,

    log_scroll: u16,
    planet1_scroll: u16,
    planet2_scroll: u16,
    navigation_scroll: u16,

    log_viewport_height: u16,
    log_viewport_width: u16,

    planet1_title: Line<'static>,
    planet1_lines: Vec<String>,
    navigation_title: Line<'static>,
    navigation_lines: Vec<String>,
    planet2_title: Line<'static>,
    planet2_lines: Vec<String>,

    search_results: Vec<crate::cli::PlanetSearchRow>,
    near_results: Vec<sw_galaxy_map_core::model::NearHit>,
    selection_mode: SelectionMode,

    history: Vec<String>,
    history_index: Option<usize>,

    route_list_results: Vec<crate::cli::commands::route::RouteListTuiItem>,

    session_db: Option<String>,
    typewriter: TypewriterState,
    typewriter_config: TypewriterConfig,
}

impl Default for App {
    fn default() -> Self {
        let (nav_title, nav_lines) = build_navigation_panel(NavigationPanelKind::Empty);

        Self {
            log: vec![
                "sw_galaxy_map TUI initialized.".to_string(),
                "Type a command and press Enter.".to_string(),
            ],
            input: String::new(),
            selected_panel: 0,
            cursor_visible: true,
            last_blink: Instant::now(),
            log_scroll: 0,
            planet1_scroll: 0,
            planet2_scroll: 0,
            navigation_scroll: 0,
            log_viewport_height: 0,
            log_viewport_width: 0,
            planet1_title: Line::from("Planet 1 Information"),
            planet1_lines: vec!["No data".to_string()],
            navigation_title: nav_title,
            navigation_lines: nav_lines,
            planet2_title: Line::from("Planet 2 Information"),
            planet2_lines: vec!["No data".to_string()],
            search_results: Vec::new(),
            near_results: Vec::new(),
            selection_mode: SelectionMode::None,
            history: Vec::new(),
            history_index: None,
            route_list_results: Vec::new(),
            session_db: None,
            typewriter: TypewriterState::default(),
            typewriter_config: TypewriterConfig::default(),
        }
    }
}

impl App {
    /// Update the custom cursor blink state.
    fn update_cursor_blink(&mut self) {
        if self.last_blink.elapsed() >= Duration::from_millis(BLINK_INTERVAL_MS) {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = Instant::now();
        }
    }

    /// Reset the custom cursor blink state after input activity.
    fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.last_blink = Instant::now();
    }

    /// Move focus to the next panel.
    fn next_panel(&mut self) {
        self.selected_panel = (self.selected_panel + 1) % PANEL_COUNT;
    }

    /// Move focus to the previous panel.
    fn previous_panel(&mut self) {
        self.selected_panel = if self.selected_panel == 0 {
            PANEL_COUNT - 1
        } else {
            self.selected_panel - 1
        };
    }

    fn push_history(&mut self, command: &str) {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return;
        }

        let should_push = match self.history.last() {
            Some(last) => last != trimmed,
            None => true,
        };

        if should_push {
            self.history.push(trimmed.to_string());
        }

        self.history_index = None;
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.history.len().saturating_sub(1),
            Some(0) => 0,
            Some(index) => index.saturating_sub(1),
        };

        self.history_index = Some(new_index);
        self.input = self.history[new_index].clone();
        self.reset_cursor_blink();
    }

    fn history_down(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {}
            Some(index) if index + 1 < self.history.len() => {
                let new_index = index + 1;
                self.history_index = Some(new_index);
                self.input = self.history[new_index].clone();
            }
            Some(_) => {
                self.history_index = None;
                self.input.clear();
            }
        }

        self.reset_cursor_blink();
    }

    fn reset_history_navigation(&mut self) {
        self.history_index = None;
    }

    fn clear_selectable_results(&mut self) {
        self.search_results.clear();
        self.near_results.clear();
        self.route_list_results.clear();
        self.selection_mode = SelectionMode::None;
    }
}

/// Handle one keyboard event.
/// Returns true when the TUI should exit.
fn handle_key(key: KeyEvent, app: &mut App) -> bool {
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

                match crate::cli::split_args(&command) {
                    Ok(tokens) => {
                        let mut argv: Vec<String> = Vec::with_capacity(tokens.len() + 3);
                        argv.push("sw_galaxy_map".to_string());

                        let user_passed_db = tokens.iter().any(|t| t == "--db");
                        if !user_passed_db && let Some(ref db) = app.session_db {
                            argv.push("--db".to_string());
                            argv.push(db.clone());
                        }

                        argv.extend(tokens);

                        match crate::cli::args::Cli::try_parse_from(argv) {
                            Ok(cli) => {
                                if let Some(ref cmd) = cli.cmd {
                                    match crate::cli::run_one_shot_for_tui(&cli, cmd) {
                                        Ok(out) => {
                                            let crate::cli::TuiCommandOutput {
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
                                                (false, false, false) => {
                                                    app.clear_selectable_results();
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

                                            if matches!(
                                                cmd,
                                                crate::cli::args::Commands::Near { .. }
                                            ) && app.near_results.is_empty()
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

/// Render the TUI.
fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(11),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(area);

    let top_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_layout[0]);

    let right_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(7),
            Constraint::Length(6),
            Constraint::Min(7),
        ])
        .split(top_layout[1]);

    app.log_viewport_height = top_layout[0].height.saturating_sub(2);
    app.log_viewport_width = top_layout[0].width.saturating_sub(2);

    let mut rendered_lines = app.log.clone();

    if let Some(partial) = app.typewriter.visible_partial_line() {
        rendered_lines.push(partial);
    }

    let log_text = if rendered_lines.is_empty() {
        String::new()
    } else {
        rendered_lines.join("\n")
    };

    let log = Paragraph::new(log_text)
        .block(panel_block(Line::from("Log"), app.selected_panel == 0))
        .scroll((app.log_scroll, 0))
        .wrap(Wrap { trim: false });

    let planet1 = Paragraph::new(app.planet1_lines.join("\n"))
        .block(panel_block(
            app.planet1_title.clone(),
            app.selected_panel == 1,
        ))
        .scroll((app.planet1_scroll, 0))
        .wrap(Wrap { trim: false });

    let navigation = Paragraph::new(app.navigation_lines.join("\n"))
        .block(panel_block(
            app.navigation_title.clone(),
            app.selected_panel == 2,
        ))
        .scroll((app.navigation_scroll, 0))
        .wrap(Wrap { trim: false });

    let planet2 = Paragraph::new(app.planet2_lines.join("\n"))
        .block(panel_block(
            app.planet2_title.clone(),
            app.selected_panel == 3,
        ))
        .scroll((app.planet2_scroll, 0))
        .wrap(Wrap { trim: false });

    let help = Paragraph::new(help_line_for_panel(app.selected_panel, app.selection_mode))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    let input_line = build_input_line(app, main_layout[2].width);
    let input = Paragraph::new(input_line)
        .block(panel_block(Line::from("Command"), app.selected_panel == 4))
        .wrap(Wrap { trim: false });

    f.render_widget(log, top_layout[0]);
    f.render_widget(planet1, right_layout[0]);
    f.render_widget(navigation, right_layout[1]);
    f.render_widget(planet2, right_layout[2]);
    f.render_widget(help, main_layout[1]);
    f.render_widget(input, main_layout[2]);
}

/// Build a panel block with active styling.
fn panel_block(title: Line<'_>, active: bool) -> Block<'_> {
    let display_title = if active {
        let mut spans = Vec::with_capacity(title.spans.len() + 1);
        spans.push(Span::raw("▶ "));
        spans.extend(title.spans);
        Line::from(spans)
    } else {
        title
    };

    let block = Block::default().title(display_title).borders(Borders::ALL);

    if active {
        block.border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        block
    }
}

/// Build the command input line, including the custom blinking cursor.
fn build_input_line(app: &App, width: u16) -> String {
    let inner_width = width.saturating_sub(2) as usize;
    let available = inner_width.saturating_sub(2);

    let cursor = if app.cursor_visible { "|" } else { " " };

    let input_chars: Vec<char> = app.input.chars().collect();
    let visible_input: String = if input_chars.len() > available.saturating_sub(1) {
        input_chars[input_chars.len() - available.saturating_sub(1)..]
            .iter()
            .collect()
    } else {
        input_chars.iter().collect()
    };

    let mut line = format!("> {visible_input}{cursor}");

    if line.chars().count() < inner_width {
        let padding = inner_width - line.chars().count();
        line.push_str(&" ".repeat(padding));
    }

    line
}

/// Scroll the focused panel up by one line.
fn scroll_up(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_sub(1),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_sub(1),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_sub(1),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_sub(1),
        _ => {}
    }
}

/// Scroll the focused panel down by one line.
fn scroll_down(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_add(1),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_add(1),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_add(1),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_add(1),
        _ => {}
    }
}

/// Scroll the focused panel up by one page.
fn scroll_page_up(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_sub(PAGE_SCROLL_STEP),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_sub(PAGE_SCROLL_STEP),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_sub(PAGE_SCROLL_STEP),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_sub(PAGE_SCROLL_STEP),
        _ => {}
    }
}

/// Scroll the focused panel down by one page.
fn scroll_page_down(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_add(PAGE_SCROLL_STEP),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_add(PAGE_SCROLL_STEP),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_add(PAGE_SCROLL_STEP),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_add(PAGE_SCROLL_STEP),
        _ => {}
    }
}

fn wrapped_line_count(line: &str, width: usize) -> usize {
    if width == 0 {
        return 0;
    }

    if line.is_empty() {
        return 1;
    }

    let char_count = line.chars().count();
    char_count.div_ceil(width).max(1)
}

fn force_scroll_to_bottom(app: &mut App) {
    let visible_lines = app.log_viewport_height as usize;
    let visible_width = app.log_viewport_width as usize;

    if visible_lines == 0 || visible_width == 0 {
        return;
    }

    let total_visual_lines = app
        .log
        .iter()
        .map(|line| wrapped_line_count(line, visible_width))
        .sum::<usize>();

    if total_visual_lines > visible_lines {
        app.log_scroll = (total_visual_lines - visible_lines) as u16;
    } else {
        app.log_scroll = 0;
    }
}

/// Push one line into the log and autoscroll only if needed.
fn push_log_line(app: &mut App, line: impl Into<String>) {
    flush_typewriter(app);
    app.log.push(line.into());
    force_scroll_to_bottom(app);
}

fn enqueue_log_line(app: &mut App, line: impl Into<String>) {
    enqueue_log_lines(app, vec![line.into()]);
}

/// Extend the log and autoscroll only if needed.
fn extend_log_lines(app: &mut App, lines: Vec<String>) {
    flush_typewriter(app);
    app.log.extend(lines);
    force_scroll_to_bottom(app);
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

fn enqueue_log_lines(app: &mut App, lines: Vec<String>) {
    if lines.is_empty() {
        return;
    }

    if app.typewriter_config.mode == TypewriterMode::Off
        || lines.len() > app.typewriter_config.max_animated_lines
    {
        app.log.extend(lines);
        force_scroll_to_bottom(app);
        return;
    }

    app.typewriter.enqueue_lines(lines);
}

fn flush_typewriter(app: &mut App) {
    let flushed = app.typewriter.flush_all();
    if !flushed.is_empty() {
        app.log.extend(flushed);
        force_scroll_to_bottom(app);
    }
}

fn update_typewriter(app: &mut App) {
    let completed = app.typewriter.update(app.typewriter_config);
    if !completed.is_empty() {
        app.log.extend(completed);
        force_scroll_to_bottom(app);
    }
}

fn handle_search_selection(app: &mut App, index: usize) {
    let Some(planet) = app.search_results.get(index - 1).cloned() else {
        push_log_line(
            app,
            format!("Invalid selection: option {} does not exist.", index),
        );
        return;
    };

    let (title, lines) = crate::cli::build_planet_panel(&planet, None);

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

    let con = match crate::cli::open_db_migrating(app.session_db.clone()) {
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

    let (title, lines) = crate::cli::build_near_planet_panel(&planet, Some(&aliases));

    app.planet2_title = title;
    app.planet2_lines = lines;
    app.planet2_scroll = 0;

    let (nav_title, nav_lines) =
        crate::cli::build_navigation_panel(crate::cli::NavigationPanelKind::Near {
            distance_parsec: hit.distance,
            reference_name: None,
        });
    app.navigation_title = nav_title;
    app.navigation_lines = nav_lines;
    app.navigation_scroll = 0;

    // 👉 QUESTA È LA RIGA CHIAVE
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

    let con = match crate::cli::open_db_migrating(app.session_db.clone()) {
        Ok(con) => con,
        Err(e) => {
            push_log_line(app, format!("Database error: {e:#}"));
            return;
        }
    };

    let data = match crate::cli::commands::route::resolve_show_for_tui(&con, item.route_id) {
        Ok(data) => data,
        Err(e) => {
            push_log_line(
                app,
                format!("Failed to open route {}: {e:#}", item.route_id),
            );
            return;
        }
    };

    let out = match crate::cli::build_route_show_output(&con, &data.loaded) {
        Ok(out) => out,
        Err(e) => {
            push_log_line(
                app,
                format!("Failed to render route {}: {e:#}", item.route_id),
            );
            return;
        }
    };

    let crate::cli::TuiCommandOutput {
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

fn help_line_for_panel(selected_panel: usize, selection_mode: SelectionMode) -> Line<'static> {
    let focus_name = match selected_panel {
        0 => "Log",
        1 => "Planet 1",
        2 => "Navigation",
        3 => "Planet 2",
        4 => "Command",
        _ => "Unknown",
    };

    let scroll_desc = match selected_panel {
        0 => "↑/↓ scroll log | PgUp/PgDn fast scroll",
        1..=3 => "↑/↓ scroll details | PgUp/PgDn fast scroll",
        4 => "↑ previous command | ↓ next command",
        _ => "↑/↓ scroll | PgUp/PgDn fast scroll",
    };

    let mut spans = vec![
        Span::raw("-- Focus on: "),
        Span::styled(
            focus_name,
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" --  | "),
        Span::raw(scroll_desc),
        Span::raw(" | "),
    ];

    if selected_panel == 0 {
        match selection_mode {
            SelectionMode::Search => {
                spans.push(Span::styled(
                    "type `1` or `option N` to inspect search result",
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" | "));
            }
            SelectionMode::Near => {
                spans.push(Span::styled(
                    "type `1` or `option N` to inspect nearby planet",
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" | "));
            }
            SelectionMode::RouteList => {
                spans.push(Span::styled(
                    "type `1` or `option N` to open a listed route",
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::raw(" | "));
            }
            SelectionMode::None => {}
        }
    }

    spans.push(Span::raw("Tab/Shift+Tab switch | Esc exit"));

    Line::from(spans)
}
