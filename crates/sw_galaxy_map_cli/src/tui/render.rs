use crate::tui::app::{App, SelectionMode};
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the TUI.
pub(crate) fn ui(f: &mut Frame, app: &mut App) {
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
