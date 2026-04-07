use crate::tui::app::App;

const PAGE_SCROLL_STEP: u16 = 10;

pub(crate) fn scroll_up(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_sub(1),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_sub(1),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_sub(1),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_sub(1),
        _ => {}
    }
}

pub(crate) fn scroll_down(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_add(1),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_add(1),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_add(1),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_add(1),
        _ => {}
    }
}

pub(crate) fn scroll_page_up(app: &mut App) {
    match app.selected_panel {
        0 => app.log_scroll = app.log_scroll.saturating_sub(PAGE_SCROLL_STEP),
        1 => app.planet1_scroll = app.planet1_scroll.saturating_sub(PAGE_SCROLL_STEP),
        2 => app.navigation_scroll = app.navigation_scroll.saturating_sub(PAGE_SCROLL_STEP),
        3 => app.planet2_scroll = app.planet2_scroll.saturating_sub(PAGE_SCROLL_STEP),
        _ => {}
    }
}

pub(crate) fn scroll_page_down(app: &mut App) {
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

pub(crate) fn force_scroll_to_bottom(app: &mut App) {
    let visible_lines = app.log_viewport_height as usize;
    let visible_width = app.log_viewport_width as usize;

    if visible_lines == 0 || visible_width == 0 {
        return;
    }

    let mut total_visual_lines = app
        .log
        .iter()
        .map(|line| wrapped_line_count(line, visible_width))
        .sum::<usize>();

    if let Some(partial) = app.typewriter.visible_partial_line() {
        total_visual_lines += wrapped_line_count(&partial, visible_width);
    }

    if total_visual_lines > visible_lines {
        app.log_scroll = (total_visual_lines - visible_lines) as u16;
    } else {
        app.log_scroll = 0;
    }
}

pub(crate) fn push_log_line(app: &mut App, line: impl Into<String>) {
    flush_typewriter(app);
    app.log.push(line.into());
    force_scroll_to_bottom(app);
}

pub(crate) fn enqueue_log_line(app: &mut App, line: impl Into<String>) {
    enqueue_log_lines(app, vec![line.into()]);
}

pub(crate) fn extend_log_lines(app: &mut App, lines: Vec<String>) {
    flush_typewriter(app);
    app.log.extend(lines);
    force_scroll_to_bottom(app);
}

pub(crate) fn enqueue_log_lines(app: &mut App, lines: Vec<String>) {
    if lines.is_empty() {
        return;
    }

    if app.typewriter_config.mode == crate::cli::typewriter::TypewriterMode::Off
        || lines.len() > app.typewriter_config.max_animated_lines
    {
        app.log.extend(lines);
        force_scroll_to_bottom(app);
        return;
    }

    app.typewriter.enqueue_lines(lines);
}

pub(crate) fn flush_typewriter(app: &mut App) {
    let flushed = app.typewriter.flush_all();
    if !flushed.is_empty() {
        app.log.extend(flushed);
        force_scroll_to_bottom(app);
    }
}

pub(crate) fn update_typewriter(app: &mut App) {
    let completed = app.typewriter.update(app.typewriter_config);
    if !completed.is_empty() {
        app.log.extend(completed);
        force_scroll_to_bottom(app);
    }
}
