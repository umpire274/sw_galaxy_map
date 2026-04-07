use crate::cli::commands::route::types::RouteListTuiItem;
use crate::cli::typewriter::{TypewriterConfig, TypewriterState};
use crate::tui::{NavigationPanelKind, build_navigation_panel};
use ratatui::text::Line;
use std::time::{Duration, Instant};
use sw_galaxy_map_core::model::{NearHit, PlanetSearchRow};

const PANEL_COUNT: usize = 5;
const BLINK_INTERVAL_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionMode {
    None,
    Search,
    Near,
    RouteList,
}

/// Interactive TUI state.
pub(crate) struct App {
    pub log: Vec<String>,
    pub input: String,
    pub selected_panel: usize,

    pub cursor_visible: bool,
    pub last_blink: Instant,

    pub log_scroll: u16,
    pub planet1_scroll: u16,
    pub planet2_scroll: u16,
    pub navigation_scroll: u16,

    pub log_viewport_height: u16,
    pub log_viewport_width: u16,

    pub planet1_title: Line<'static>,
    pub planet1_lines: Vec<String>,
    pub navigation_title: Line<'static>,
    pub navigation_lines: Vec<String>,
    pub planet2_title: Line<'static>,
    pub planet2_lines: Vec<String>,

    pub search_results: Vec<PlanetSearchRow>,
    pub near_results: Vec<NearHit>,
    pub selection_mode: SelectionMode,

    pub history: Vec<String>,
    pub history_index: Option<usize>,

    pub route_list_results: Vec<RouteListTuiItem>,

    pub session_db: Option<String>,
    pub typewriter: TypewriterState,
    pub typewriter_config: TypewriterConfig,
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
    pub(crate) fn update_cursor_blink(&mut self) {
        if self.last_blink.elapsed() >= Duration::from_millis(BLINK_INTERVAL_MS) {
            self.cursor_visible = !self.cursor_visible;
            self.last_blink = Instant::now();
        }
    }

    /// Reset the custom cursor blink state after input activity.
    pub(crate) fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.last_blink = Instant::now();
    }

    /// Move focus to the next panel.
    pub(crate) fn next_panel(&mut self) {
        self.selected_panel = (self.selected_panel + 1) % PANEL_COUNT;
    }

    /// Move focus to the previous panel.
    pub(crate) fn previous_panel(&mut self) {
        self.selected_panel = if self.selected_panel == 0 {
            PANEL_COUNT - 1
        } else {
            self.selected_panel - 1
        };
    }

    pub(crate) fn push_history(&mut self, command: &str) {
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

    pub(crate) fn history_up(&mut self) {
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

    pub(crate) fn history_down(&mut self) {
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

    pub(crate) fn reset_history_navigation(&mut self) {
        self.history_index = None;
    }

    pub(crate) fn clear_selectable_results(&mut self) {
        self.search_results.clear();
        self.near_results.clear();
        self.route_list_results.clear();
        self.selection_mode = SelectionMode::None;
    }
}
