// src/gui/app.rs
//
// 0.7.2: GUI "console" mode + HELP popup
// - A single command box accepts the same CLI commands (e.g. `route compute ...`).
// - Commands are executed by spawning the current executable with arguments.
// - stdout/stderr are captured and appended to the GUI output panel.
// - JSON output is auto-detected and can be exported via the existing Export JSON button.
// - NEW: Help popup that runs `--help`, `route --help`, etc. and renders output in a scrollable window.

use chrono::Local;
use eframe::egui;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

// Clipboard helper (Copy/Cut/Paste)
use arboard::Clipboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpTopic {
    General,
    Route,
    Waypoint,
    Search,
    Info,
    Near,
}

impl HelpTopic {
    fn label(self) -> &'static str {
        match self {
            HelpTopic::General => "General",
            HelpTopic::Route => "route",
            HelpTopic::Waypoint => "waypoint",
            HelpTopic::Search => "search",
            HelpTopic::Info => "info",
            HelpTopic::Near => "near",
        }
    }

    fn argv(self) -> Vec<String> {
        match self {
            HelpTopic::General => vec!["--help".to_string()],
            HelpTopic::Route => vec![self.label().to_string(), "--help".to_string()],
            HelpTopic::Waypoint => vec![self.label().to_string(), "--help".to_string()],
            HelpTopic::Search => vec![self.label().to_string(), "--help".to_string()],
            HelpTopic::Info => vec![self.label().to_string(), "--help".to_string()],
            HelpTopic::Near => vec![self.label().to_string(), "--help".to_string()],
        }
    }
}

pub struct NavicomputerApp {
    // Command line entered by the user (CLI-compatible)
    command: String,

    // Command history (most recent at end)
    history: Vec<String>,
    history_pos: Option<usize>,

    // Text output (human readable)
    output: String,

    // JSON payload ready for export (stringified)
    last_json: Option<String>,

    // System/status message shown in the status area
    status: String,

    // UI state
    error: Option<String>,
    running: bool,

    cmd_saved_sel: Option<egui::text::CCursorRange>,
    out_saved_sel: Option<egui::text::CCursorRange>,

    // Bootstrap
    boot_lines: Vec<&'static str>,
    boot_step: usize,
    boot_next: Option<Instant>,

    // DB connection status (best-effort)
    db_connected: bool,
    db_tooltip: String,

    // Status TTL
    status_deadline: Option<Instant>,
    ready_status: &'static str,

    // HELP popup state
    show_help: bool,
    help_topic: HelpTopic,
    help_text: String,
    help_loading: bool,
    help_last_loaded_at: Option<Instant>,
}

impl NavicomputerApp {
    fn prevalidate_cli_tokens(tokens: &[String]) -> anyhow::Result<()> {
        use crate::cli::validate;

        if tokens.is_empty() {
            anyhow::bail!("No command provided.");
        }

        match tokens[0].as_str() {
            "near" => {
                // Parse minimale: cerchiamo --planet, --x, --y
                let mut planet: Option<String> = None;
                let mut x: Option<f64> = None;
                let mut y: Option<f64> = None;

                let mut i = 1usize;
                while i < tokens.len() {
                    let t = tokens[i].as_str();
                    if t == "--planet" && i + 1 < tokens.len() {
                        planet = Some(tokens[i + 1].clone());
                        i += 2;
                        continue;
                    }
                    if let Some(v) = t.strip_prefix("--planet=") {
                        planet = Some(v.to_string());
                        i += 1;
                        continue;
                    }

                    if t == "--x" && i + 1 < tokens.len() {
                        x = tokens[i + 1].parse::<f64>().ok();
                        i += 2;
                        continue;
                    }
                    if let Some(v) = t.strip_prefix("--x=") {
                        x = v.parse::<f64>().ok();
                        i += 1;
                        continue;
                    }

                    if t == "--y" && i + 1 < tokens.len() {
                        y = tokens[i + 1].parse::<f64>().ok();
                        i += 2;
                        continue;
                    }
                    if let Some(v) = t.strip_prefix("--y=") {
                        y = v.parse::<f64>().ok();
                        i += 1;
                        continue;
                    }

                    i += 1;
                }

                validate::validate_near(&planet, &x, &y)?;
            }

            "search" => {
                // search <query> [--limit N] (adatta se la tua CLI è diversa)
                // prendiamo il primo argomento non-opzione come query.
                let mut query: Option<String> = None;
                let mut limit: i64 = 20;

                let mut i = 1usize;
                while i < tokens.len() {
                    let t = tokens[i].as_str();
                    if t == "--limit" && i + 1 < tokens.len() {
                        limit = tokens[i + 1].parse().unwrap_or(limit);
                        i += 2;
                        continue;
                    }
                    if let Some(v) = t.strip_prefix("--limit=") {
                        limit = v.parse().unwrap_or(limit);
                        i += 1;
                        continue;
                    }
                    if !t.starts_with('-') && query.is_none() {
                        query = Some(tokens[i].clone());
                    }
                    i += 1;
                }

                validate::validate_search(query.as_deref().unwrap_or(""), limit)?;
            }

            "route" => {
                // route compute <from> <to>, route show <id>, route explain <id>...
                if tokens.len() >= 2 {
                    match tokens[1].as_str() {
                        "compute" => {
                            let from = tokens.get(2).map(|s| s.as_str()).unwrap_or("");
                            let to = tokens.get(3).map(|s| s.as_str()).unwrap_or("");
                            validate::validate_route_compute(from, to)?;
                        }
                        "show" => {
                            let id = tokens
                                .get(2)
                                .and_then(|s| s.parse::<i64>().ok())
                                .unwrap_or(0);
                            validate::validate_route_id(id, "show")?;
                        }
                        "explain" => {
                            let id = tokens
                                .get(2)
                                .and_then(|s| s.parse::<i64>().ok())
                                .unwrap_or(0);
                            validate::validate_route_id(id, "explain")?;
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let boot_lines = vec![
            "Boot sequence initiated...",
            "Power grid: nominal. Core systems: online.",
            "Navigation stack: ready. Awaiting command input.",
        ];

        let status = boot_lines
            .first()
            .copied()
            .unwrap_or("Navicomputer ready.")
            .to_string();

        let (db_connected, db_tooltip) = Self::probe_db();

        Self {
            command: String::new(),
            history: Vec::new(),
            history_pos: None,
            output: String::new(),
            last_json: None,
            status,
            error: None,
            running: false,
            cmd_saved_sel: None,
            out_saved_sel: None,
            boot_lines,
            boot_step: 0,
            boot_next: Some(Instant::now() + Duration::from_millis(300)),
            db_connected,
            db_tooltip,
            ready_status: "Navicomputer ready. All systems are online.",
            status_deadline: None,

            show_help: false,
            help_topic: HelpTopic::General,
            help_text: String::new(),
            help_loading: false,
            help_last_loaded_at: None,
        }
    }

    fn probe_db() -> (bool, String) {
        // Best-effort DB probe, without doing provisioning.
        match crate::db::db_status::resolve_db_path(None) {
            Ok(path) => {
                if !path.exists() {
                    return (false, format!("SQLite: not found\n{}", path.display()));
                }

                match crate::db::core::open_db(&path.to_string_lossy()) {
                    Ok(_) => (true, format!("SQLite: connected\n{}", path.display())),
                    Err(e) => (false, format!("SQLite: error\n{}\n{:#}", path.display(), e)),
                }
            }
            Err(e) => (false, format!("SQLite: error\n{:#}", e)),
        }
    }

    fn app_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_deadline = None;
    }

    fn set_status_ttl(&mut self, msg: impl Into<String>, ttl: Duration) {
        self.status = msg.into();
        self.status_deadline = Some(Instant::now() + ttl);
    }

    fn tick_status_deadline(&mut self) {
        if let Some(deadline) = self.status_deadline
            && Instant::now() >= deadline
        {
            self.status = self.ready_status.to_string();
            self.status_deadline = None;
        }
    }

    fn tick_bootstrap(&mut self, ctx: &egui::Context) {
        let Some(next) = self.boot_next else {
            return;
        };

        let now = Instant::now();
        if now >= next {
            let next_step = self.boot_step + 1;

            if next_step < self.boot_lines.len() {
                self.boot_step = next_step;
                self.status = self.boot_lines[self.boot_step].to_string();
                self.boot_next = Some(now + Duration::from_millis(300));
            } else {
                self.boot_next = None;
                self.status = self.ready_status.to_string();
            }
        }

        if self.boot_next.is_some() {
            ctx.request_repaint();
        }
    }

    fn can_run(&self) -> bool {
        !self.running && !self.command.trim().is_empty()
    }

    fn can_clear(&self) -> bool {
        !self.command.is_empty()
            || !self.output.is_empty()
            || self.last_json.is_some()
            || self.error.is_some()
            || self.running
    }

    fn clear_all(&mut self) {
        self.command.clear();
        self.output.clear();
        self.last_json = None;
        self.error = None;
        self.history_pos = None;
        if self.boot_next.is_none() {
            self.status = self.ready_status.to_string();
        }
    }

    fn default_export_filename(&self) -> String {
        let ts = Local::now().format("%Y%m%d-%H%M%S");
        format!("navicomputer-export-{}.json", ts)
    }

    fn export_json(&mut self) {
        let Some(json) = self.last_json.clone() else {
            return;
        };

        let suggested = self.default_export_filename();

        let path: Option<PathBuf> = rfd::FileDialog::new()
            .set_title("Export command output (JSON)")
            .set_file_name(&suggested)
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(path) = path {
            if let Err(e) = std::fs::write(&path, format!("{}\n", json)) {
                self.error = Some(format!("Failed to write file: {} ({})", path.display(), e));
                self.set_status_ttl("Export failed.", Duration::from_secs(6));
            } else {
                self.error = None;
                self.set_status_ttl(
                    format!("Export completed: {}", path.display()),
                    Duration::from_secs(5),
                );
            }
        } else {
            self.set_status_ttl("Export cancelled.", Duration::from_secs(3));
        }
    }

    fn push_output_line(&mut self, s: &str) {
        self.output.push_str(s);
        if !s.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn current_exe() -> Result<PathBuf, String> {
        std::env::current_exe().map_err(|e| format!("Failed to locate executable: {e}"))
    }

    fn run_exe_capture(&self, argv: &[String]) -> Result<(String, String, i32), String> {
        let exe = Self::current_exe()?;
        let res = Command::new(exe).args(argv).output();

        match res {
            Ok(r) => {
                let out = String::from_utf8_lossy(&r.stdout).to_string();
                let err = String::from_utf8_lossy(&r.stderr).to_string();
                let code = r.status.code().unwrap_or(1);
                Ok((out, err, code))
            }
            Err(e) => Err(format!("Failed to execute command: {e}")),
        }
    }

    fn append_non_empty(buf: &mut String, s: &str) {
        if !s.trim().is_empty() {
            buf.push_str(s);
            if !s.ends_with('\n') {
                buf.push('\n');
            }
        }
    }

    fn run_command(&mut self) {
        let line = self.command.trim().to_string();
        if line.is_empty() {
            self.set_status_ttl("No command provided.", Duration::from_secs(3));
            return;
        }

        // Clear previous console output so each command starts from the top.
        self.output.clear();
        self.last_json = None;

        // History
        if self.history.last().map(|s| s.as_str()) != Some(line.as_str()) {
            self.history.push(line.clone());
        }
        self.history_pos = None;

        let tokens = match shell_words::split(&line) {
            Ok(t) => t,
            Err(e) => {
                self.push_output_line(&format!("❌ Parse error: {e}"));
                self.set_status_ttl("Command parse error.", Duration::from_secs(4));
                return;
            }
        };

        // Pre-validation (GUI-friendly): show consistent messages before spawning the exe.
        if let Err(e) = Self::prevalidate_cli_tokens(&tokens) {
            self.running = false;
            self.output.clear();
            self.push_output_line(&format!("> {line}"));
            self.push_output_line(&format!("❌ {e}"));
            self.set_status_ttl("Validation error.", Duration::from_secs(5));
            return;
        }

        self.running = true;
        self.error = None;
        self.set_status("Running command...");

        let (out, err, code) = match self.run_exe_capture(&tokens) {
            Ok(t) => t,
            Err(e) => {
                self.running = false;
                self.error = Some(e);
                self.set_status_ttl("Execution error.", Duration::from_secs(6));
                return;
            }
        };

        self.running = false;

        // Append to GUI console output
        self.push_output_line(&format!("> {line}"));

        NavicomputerApp::append_non_empty(&mut self.output, &out);
        NavicomputerApp::append_non_empty(&mut self.output, &err);

        // JSON auto-detect: if stdout is valid JSON, cache it for export
        let out_trim = out.trim();
        if !out_trim.is_empty()
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(out_trim)
        {
            self.last_json =
                Some(serde_json::to_string_pretty(&v).unwrap_or_else(|_| out_trim.to_string()));
        }

        if code == 0 {
            self.set_status_ttl("Done.", Duration::from_secs(3));
        } else {
            self.set_status_ttl(format!("Exited with code {code}."), Duration::from_secs(5));
        }
    }

    fn handle_history_keys(&mut self, ctx: &egui::Context, cmd_id: egui::Id) {
        let focused = ctx.memory(|m| m.focused()) == Some(cmd_id);
        if !focused {
            return;
        }

        let (up, down) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if self.history.is_empty() {
            return;
        }

        if up {
            let new_pos = match self.history_pos {
                None => self.history.len().saturating_sub(1),
                Some(0) => 0,
                Some(p) => p.saturating_sub(1),
            };
            self.history_pos = Some(new_pos);
            self.command = self.history[new_pos].clone();
        } else if down && let Some(p) = self.history_pos {
            let next = p + 1;
            if next >= self.history.len() {
                self.history_pos = None;
                self.command.clear();
            } else {
                self.history_pos = Some(next);
                self.command = self.history[next].clone();
            }
        }
    }

    fn db_status_indicator(&self, ui: &mut egui::Ui) {
        let base = if self.db_connected {
            egui::Color32::from_rgb(0, 170, 0)
        } else {
            egui::Color32::from_rgb(200, 40, 40)
        };

        let r = 6.0;
        let size = egui::vec2(r * 2.0 + 2.0, r * 2.0 + 2.0);
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
        let center = rect.center();

        let painter = ui.painter();
        let shadow = egui::Color32::from_black_alpha(80);
        painter.circle_filled(center + egui::vec2(1.2, 1.2), r + 0.3, shadow);
        painter.circle_filled(center, r, base);
        let highlight = egui::Color32::from_white_alpha(110);
        painter.circle_filled(
            center + egui::vec2(-r * 0.35, -r * 0.35),
            r * 0.45,
            highlight,
        );
        painter.circle_stroke(center, r, egui::Stroke::new(1.0, egui::Color32::BLACK));

        resp.on_hover_text(self.db_tooltip.clone());
    }

    // ----------------------------
    // HELP popup
    // ----------------------------

    fn open_help(&mut self, ctx: &egui::Context, topic: HelpTopic) {
        self.show_help = true;
        self.load_help(ctx, topic);
    }

    fn load_help(&mut self, ctx: &egui::Context, topic: HelpTopic) {
        self.help_topic = topic;
        self.help_loading = true;
        self.help_text.clear();

        // Render immediately (shows "Loading..." for one frame)
        ctx.request_repaint();

        let argv = topic.argv();
        let (out, err, _code) = match self.run_exe_capture(&argv) {
            Ok(t) => t,
            Err(e) => {
                self.help_loading = false;
                self.help_text = format!("❌ Failed to load help:\n{e}\n");
                self.help_last_loaded_at = Some(Instant::now());
                return;
            }
        };

        let mut text = String::new();
        if !out.trim().is_empty() {
            text.push_str(&out);
            if !out.ends_with('\n') {
                text.push('\n');
            }
        }
        if !err.trim().is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&err);
            if !err.ends_with('\n') {
                text.push('\n');
            }
        }

        if text.trim().is_empty() {
            text = "No help output captured.\n".to_string();
        }

        self.help_text = text;
        self.help_loading = false;
        self.help_last_loaded_at = Some(Instant::now());
    }

    fn show_help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }

        // Stage actions from inside the window, apply after `.show`.
        let mut requested_topic: Option<HelpTopic> = None;
        let mut requested_reload = false;
        let mut requested_close = false;

        // Use a dedicated OS-level viewport so the Help can be dragged outside the main window.
        let vp_id = egui::ViewportId::from_hash_of("navicomputer_help_viewport");
        let vp_builder = egui::ViewportBuilder::default()
            .with_title("Command Help")
            .with_inner_size([920.0, 560.0]);

        // If supported, allow the user to close the OS window (X button).
        // We'll mirror that into `self.show_help` via `requested_close`.
        ctx.show_viewport_immediate(vp_id, vp_builder, |ctx, _class| {
            // A local window inside the viewport (for standard egui look + padding).
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Topic:");

                    let mut selected = self.help_topic;
                    ui.selectable_value(&mut selected, HelpTopic::General, "General");
                    ui.selectable_value(&mut selected, HelpTopic::Route, "route");
                    ui.selectable_value(&mut selected, HelpTopic::Waypoint, "waypoint");
                    ui.selectable_value(&mut selected, HelpTopic::Search, "search");
                    ui.selectable_value(&mut selected, HelpTopic::Info, "info");
                    ui.selectable_value(&mut selected, HelpTopic::Near, "near");

                    if selected != self.help_topic {
                        requested_topic = Some(selected);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            requested_close = true;
                        }
                        if ui.button("Reload").clicked() {
                            requested_reload = true;
                        }
                    });
                });

                ui.add_space(6.0);

                ui.label(
                    egui::RichText::new(
                        "Tip: you can also run `--help`, `route --help`, etc. directly in the CMD box.",
                    )
                    .weak(),
                );

                ui.add_space(6.0);

                if self.help_loading {
                    ui.label(egui::RichText::new("Loading…").italics());
                    ui.add_space(6.0);
                }

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.help_text)
                                .font(egui::TextStyle::Monospace)
                                .interactive(false)
                                .desired_width(f32::INFINITY),
                        );
                    });

                if let Some(ts) = self.help_last_loaded_at {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("Loaded: {:?} ago", ts.elapsed()))
                            .weak()
                            .small(),
                    );
                }
            });

            // If the user closes the viewport window via OS controls, egui will request close.
            let wants_close = ctx.input(|i| i.viewport().close_requested());
            if wants_close {
                requested_close = true;
            }

            // Keep repainting while visible.
            ctx.request_repaint();
        });

        if requested_close {
            self.show_help = false;
            // Best-effort: ask egui to close the viewport window.
            ctx.send_viewport_cmd_to(vp_id, egui::ViewportCommand::Close);
        }

        if let Some(topic) = requested_topic {
            self.load_help(ctx, topic);
        } else if requested_reload {
            self.load_help(ctx, self.help_topic);
        }
    }

    fn with_clipboard_text<F: FnOnce(&mut Clipboard) -> anyhow::Result<String>>(
        &mut self,
        f: F,
    ) -> Option<String> {
        let mut cb = Clipboard::new().ok()?;
        f(&mut cb).ok()
    }

    fn clipboard_get_text(&mut self) -> Option<String> {
        self.with_clipboard_text(|cb| cb.get_text().map_err(|e| anyhow::anyhow!(e)))
    }

    fn send_copy_cut_paste_event(&mut self, ctx: &egui::Context, kind: &str) {
        match kind {
            "copy" => {
                ctx.input_mut(|i| i.events.push(egui::Event::Copy));
            }
            "cut" => {
                ctx.input_mut(|i| i.events.push(egui::Event::Cut));
            }
            "paste" => {
                if let Some(text) = self.clipboard_get_text()
                    && !text.is_empty()
                {
                    ctx.input_mut(|i| i.events.push(egui::Event::Text(text)));
                }
            }
            _ => {}
        }
    }

    fn save_selection(ctx: &egui::Context, id: egui::Id) -> Option<egui::text::CCursorRange> {
        egui::text_edit::TextEditState::load(ctx, id).and_then(|st| st.cursor.char_range())
    }

    fn restore_selection(ctx: &egui::Context, id: egui::Id, sel: &egui::text::CCursorRange) {
        if let Some(mut st) = egui::text_edit::TextEditState::load(ctx, id) {
            st.cursor.set_char_range(Some(*sel));
            st.store(ctx, id);
        }
    }
}

impl eframe::App for NavicomputerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let cmd_id = egui::Id::new("navicomputer_command");
        let out_id = egui::Id::new("navicomputer_output");

        // --- Ticks / housekeeping
        self.tick_bootstrap(ctx);
        self.tick_status_deadline();

        // Snapshot current selections so we can restore them when opening context menus
        self.cmd_saved_sel = Self::save_selection(ctx, cmd_id);
        self.out_saved_sel = Self::save_selection(ctx, out_id);

        // --- Window title
        let base = "SW Galaxy Map — Navicomputer";
        let title = if self.running {
            format!("{base} — Running…")
        } else {
            format!("{base} — Ready")
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // --- Global shortcuts
        let (enter, esc, ctrl_s, f1) = ctx.input(|i| {
            let enter = i.key_pressed(egui::Key::Enter);
            let esc = i.key_pressed(egui::Key::Escape);
            let ctrl_s = i.modifiers.ctrl && i.key_pressed(egui::Key::S);
            let f1 = i.key_pressed(egui::Key::F1);
            (enter, esc, ctrl_s, f1)
        });

        // History navigation only when command box is focused
        self.handle_history_keys(ctx, cmd_id);

        if enter {
            let focused = ctx.memory(|m| m.focused()) == Some(cmd_id);
            if focused && self.can_run() {
                self.run_command();
            }
        }
        if esc && self.can_clear() {
            self.clear_all();
        }
        if ctrl_s && self.last_json.is_some() && !self.running {
            self.export_json();
        }
        if f1 {
            self.open_help(ctx, HelpTopic::General);
        }

        // --- TOP PANEL: command input + buttons
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("CMD:");

                // Draw one consistent border via Frame; disable inner TextEdit frame
                let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
                let fill = ui.visuals().extreme_bg_color;

                egui::Frame::NONE
                    .fill(fill)
                    .stroke(stroke)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .corner_radius(egui::CornerRadius::same(3))
                    .show(ui, |ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.command)
                                .id(cmd_id)
                                .hint_text(r#"e.g. route compute "Corellia" "Tatooine""#)
                                .desired_width(800.0)
                                .interactive(true)
                                .frame(false),
                        );

                        // Context menu (right click): Copy / Cut / Paste
                        resp.context_menu(|ui| {
                            ui.set_min_width(140.0);

                            // Preserve selection: refocus + restore saved cursor range
                            ui.ctx().memory_mut(|m| m.request_focus(cmd_id));
                            if let Some(sel) = &self.cmd_saved_sel {
                                Self::restore_selection(ui.ctx(), cmd_id, sel);
                            }

                            if ui.button("Copy").clicked() {
                                self.send_copy_cut_paste_event(ui.ctx(), "copy");
                                ui.close();
                            }
                            if ui.button("Cut").clicked() {
                                self.send_copy_cut_paste_event(ui.ctx(), "cut");
                                ui.close();
                            }
                            if ui.button("Paste").clicked() {
                                self.send_copy_cut_paste_event(ui.ctx(), "paste");
                                ui.close();
                            }
                        });
                    });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let run = ui
                        .add_enabled(self.can_run(), egui::Button::new("Run"))
                        .on_hover_text("Run command (Enter)");
                    if run.clicked() {
                        self.run_command();
                    }

                    let clear = ui
                        .add_enabled(self.can_clear(), egui::Button::new("Clear"))
                        .on_hover_text("Clear console (Esc)");
                    if clear.clicked() {
                        self.clear_all();
                    }

                    let help = ui.button("Help").on_hover_text("Show help (F1)");
                    if help.clicked() {
                        self.open_help(ctx, HelpTopic::General);
                    }
                });
            });

            ui.add_space(4.0);
            if let Some(err) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }
        });

        // --- Shared frame with no gaps/borders
        let base_frame = egui::Frame::NONE
            .inner_margin(egui::Margin::same(0))
            .outer_margin(egui::Margin::same(0))
            .corner_radius(egui::CornerRadius::ZERO)
            .stroke(egui::Stroke::NONE);

        // --- BOTTOM PANEL: version + db dot + export
        const BOTTOM_BAR_HEIGHT: f32 = 44.0;
        let bottom_fill = ctx.style().visuals.panel_fill;
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .exact_height(BOTTOM_BAR_HEIGHT)
            .frame(base_frame.fill(bottom_fill))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            self.db_status_indicator(ui);
                            ui.add_space(6.0);

                            ui.label(
                                egui::RichText::new(format!("v{}", Self::app_version()))
                                    .monospace()
                                    .color(ui.visuals().weak_text_color()),
                            )
                            .on_hover_text(
                                "sw_galaxy_map — Star Wars galaxy navicomputer (GUI + CLI)",
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let export = ui
                                        .add_enabled(
                                            self.last_json.is_some() && !self.running,
                                            egui::Button::new("Export JSON"),
                                        )
                                        .on_hover_text("Export last JSON output (Ctrl+S)");
                                    if export.clicked() {
                                        self.export_json();
                                    }
                                },
                            );
                        });
                    });
            });

        // --- STATUS PANEL: very bottom
        const STATUS_BAR_HEIGHT: f32 = 26.0;
        let status_fill = egui::Color32::from_gray(210);
        egui::TopBottomPanel::bottom("status_panel")
            .resizable(false)
            .exact_height(STATUS_BAR_HEIGHT)
            .frame(base_frame.fill(status_fill))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.status).weak());
                    });
            });

        // --- CENTRAL: output (scrollable, selection-friendly)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label("Output:");

            // Altezza residua reale sotto la label (riempie fino ai pannelli bottom)
            let h = ui.available_height().max(80.0);
            let w = ui.available_width();

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .scroll_source(egui::scroll_area::ScrollSource {
                    drag: false,
                    ..Default::default()
                })
                .stick_to_bottom(true)
                // IMPORTANT: garantisce che l'area scrollata abbia almeno quell'altezza
                .min_scrolled_height(h)
                .show(ui, |ui| {
                    // Qui creiamo un'area che "occupa" h, e dentro ci mettiamo il TextEdit
                    ui.allocate_ui_with_layout(
                        egui::vec2(w, h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            let resp = ui.add_sized(
                                [w, h],
                                egui::TextEdit::multiline(&mut self.output)
                                    .id(out_id)
                                    .font(egui::TextStyle::Monospace)
                                    .interactive(false)
                                    .desired_width(f32::INFINITY)
                                    .frame(true), // opzionale: io lo lascerei true per definire l'area
                            );

                            resp.context_menu(|ui| {
                                ui.set_min_width(120.0);

                                ui.ctx().memory_mut(|m| m.request_focus(out_id));
                                if let Some(sel) = &self.out_saved_sel {
                                    Self::restore_selection(ui.ctx(), out_id, sel);
                                }

                                if ui.button("Copy").clicked() {
                                    ui.ctx().input_mut(|i| i.events.push(egui::Event::Copy));
                                    ui.close();
                                }
                            });
                        },
                    );
                });
        });

        // Help popup (renders on top of everything)
        self.show_help_window(ctx);
    }
}
