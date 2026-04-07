use crate::tui::app::App;
use crate::tui::input::handle_key;
use crate::tui::log::update_typewriter;
use crate::tui::render::ui;
use crossterm::event::KeyEventKind;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};

const POLL_INTERVAL_MS: u64 = 50;

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
