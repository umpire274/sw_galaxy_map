// src/gui/app.rs
use chrono::Local;
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct NavicomputerApp {
    from: String,
    to: String,

    // Text output (human readable)
    output: String,

    // JSON payload ready for export (stringified)
    last_json: Option<String>,

    // System/status message shown in the gray status area (no borders)
    status: String,

    // UI state
    error: Option<String>,
    computing: bool,

    // Track focus changes between frames to auto-select text on TAB focus.
    prev_focus: Option<egui::Id>,

    // Bootstrap
    boot_lines: Vec<&'static str>,
    boot_step: usize,           // index of current displayed line
    boot_next: Option<Instant>, // when to advance to next line

    // DB connection status (GUI-level, not the actual pool)
    db_connected: bool,

    status_deadline: Option<Instant>,
    ready_status: &'static str,
}

impl NavicomputerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let boot_lines = vec![
            "Boot sequence initiated...",
            "Power grid: nominal. Core systems: online.",
            "Navigation stack: ready. Awaiting route parameters.",
        ];

        let status = boot_lines
            .first()
            .copied()
            .unwrap_or("Navicomputer ready.")
            .to_string();

        Self {
            from: String::new(),
            to: String::new(),
            output: String::new(),
            last_json: None,

            // IMPORTANT: set immediately so it's visible in the first frame
            status,

            error: None,
            computing: false,
            prev_focus: None,

            boot_lines,
            boot_step: 0,
            boot_next: Some(Instant::now() + Duration::from_millis(300)),

            db_connected: true,

            ready_status: "Navicomputer ready. All systems are online.",
            status_deadline: None,
        }
    }

    fn clear_fields(&mut self) {
        self.from.clear();
        self.to.clear();
        self.output.clear();
        self.last_json = None;
        self.error = None;
        self.computing = false;
    }

    fn can_compute(&self) -> bool {
        if self.computing {
            return false;
        }
        let (from, to) = self.normalized_from_to();
        !from.is_empty() && !to.is_empty() && !from.eq_ignore_ascii_case(&to)
    }

    fn can_export(&self) -> bool {
        self.last_json.is_some() && !self.computing
    }

    fn can_clear(&self) -> bool {
        !self.from.is_empty()
            || !self.to.is_empty()
            || !self.output.is_empty()
            || self.last_json.is_some()
            || self.error.is_some()
            || self.computing
    }

    fn sanitize_for_filename(s: &str) -> String {
        // Keep it filesystem friendly: alnum + '-' + '_', whitespace -> '-', others -> '_'
        let mut out = String::with_capacity(s.len());
        for ch in s.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                out.push(ch);
            } else if ch.is_whitespace() {
                out.push('-');
            } else {
                out.push('_');
            }
        }
        out
    }

    fn default_export_filename(&self) -> String {
        let from = Self::sanitize_for_filename(self.from.trim());
        let to = Self::sanitize_for_filename(self.to.trim());
        let ts = Local::now().format("%Y%m%d-%H%M%S");
        format!("route-{}-{}-{}.json", from, to, ts)
    }

    fn compute_route_stub(&mut self) {
        self.computing = true;
        self.error = None;
        self.set_status("Computing route...");

        let (from, to) = self.normalized_from_to();

        if from.is_empty() || to.is_empty() {
            self.set_status_ttl("FROM and TO must be provided.", Duration::from_secs(4));
            self.computing = false;
            return;
        }

        if from.eq_ignore_ascii_case(&to) {
            self.set_status_ttl("FROM and TO must be different.", Duration::from_secs(4));
            self.computing = false;
            return;
        }

        // opzionale: scrivi indietro i valori normalizzati così l’utente vede la forma “pulita”
        self.from = from.clone();
        self.to = to.clone();

        self.output = format!(
            "Route — {} → {}\n\n(placeholder)\n- compute not wired yet\n",
            from, to
        );

        let json_obj = serde_json::json!({
            "from": from,
            "to": to,
            "timestamp": Local::now().to_rfc3339(),
            "route": null,
            "note": "placeholder export from GUI; wiring will be added in 0.7.0"
        });

        self.last_json =
            Some(serde_json::to_string_pretty(&json_obj).unwrap_or_else(|_| "{}".to_string()));

        self.set_status_ttl("Route computation completed.", Duration::from_secs(5));
        self.computing = false;
    }

    fn export_json(&mut self) {
        let Some(json) = self.last_json.clone() else {
            return;
        };

        let suggested = self.default_export_filename();

        let path: Option<PathBuf> = rfd::FileDialog::new()
            .set_title("Export route explanation (JSON)")
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
                    format!("Export completed: {}", path.display()).to_string(),
                    Duration::from_secs(5),
                );
            }
        } else {
            self.set_status_ttl("Export cancelled.", Duration::from_secs(3));
        }
    }

    fn select_all_in_text_edit(ctx: &egui::Context, id: egui::Id) {
        if let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) {
            state.cursor.set_char_range(Some(egui::text::CCursorRange {
                primary: egui::text::CCursor::new(0),
                secondary: egui::text::CCursor::new(usize::MAX),
                h_pos: None,
            }));
            state.store(ctx, id);
        }
    }

    fn apply_focus_select_all(&mut self, ctx: &egui::Context, from_id: egui::Id, to_id: egui::Id) {
        let cur_focus = ctx.memory(|m| m.focused());
        if cur_focus != self.prev_focus {
            if cur_focus == Some(from_id) && self.prev_focus != Some(from_id) {
                Self::select_all_in_text_edit(ctx, from_id);
            } else if cur_focus == Some(to_id) && self.prev_focus != Some(to_id) {
                Self::select_all_in_text_edit(ctx, to_id);
            }
        }
    }

    fn tick_bootstrap(&mut self, ctx: &egui::Context) {
        let Some(next) = self.boot_next else {
            return;
        };

        let now = Instant::now();
        if now >= next {
            // advance to next line
            let next_step = self.boot_step + 1;

            if next_step < self.boot_lines.len() {
                self.boot_step = next_step;
                self.status = self.boot_lines[self.boot_step].to_string();
                self.boot_next = Some(now + Duration::from_millis(300));
            } else {
                self.boot_next = None;
                self.status = "Navicomputer ready. All systems are online.".to_string();
            }
        }

        // Forza repaint finché il bootstrap è attivo.
        if self.boot_next.is_some() {
            ctx.request_repaint();
        }
    }

    fn app_version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn update_window_title(&self, ctx: &egui::Context) {
        let base = "SW Galaxy Map — Navicomputer";
        let from = self.from.trim();
        let to = self.to.trim();

        let title = if self.computing {
            if !from.is_empty() && !to.is_empty() {
                format!("{base} — Computing: {from} → {to}")
            } else {
                format!("{base} — Computing…")
            }
        } else if !from.is_empty() && !to.is_empty() {
            format!("{base} — {from} → {to}")
        } else {
            format!("{base} — Ready")
        };

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    fn db_status_indicator(&self, ui: &mut egui::Ui) {
        // Base colors
        let base = if self.db_connected {
            egui::Color32::from_rgb(0, 170, 0)
        } else {
            egui::Color32::from_rgb(200, 40, 40)
        };

        let tooltip: String = if self.db_connected {
            "SQLite: connected".to_string()
        } else if let Some(err) = self.error.as_deref() {
            format!("SQLite: error\n{}", err)
        } else {
            "SQLite: error".to_string()
        };

        // Geometry
        let r = 6.0;
        let size = egui::vec2(r * 2.0 + 2.0, r * 2.0 + 2.0);
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
        let center = rect.center();

        let painter = ui.painter();

        // 1) Soft shadow (down-right)
        let shadow = egui::Color32::from_black_alpha(80);
        painter.circle_filled(center + egui::vec2(1.2, 1.2), r + 0.3, shadow);

        // 2) Main fill
        painter.circle_filled(center, r, base);

        // 3) Specular highlight (top-left) -> "3D" look
        let highlight = egui::Color32::from_white_alpha(110);
        painter.circle_filled(
            center + egui::vec2(-r * 0.35, -r * 0.35),
            r * 0.45,
            highlight,
        );

        // 4) Thin black border
        painter.circle_stroke(center, r, egui::Stroke::new(1.0, egui::Color32::BLACK));

        // Tooltip
        resp.on_hover_text(tooltip);
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

    fn normalize_input(s: &str) -> String {
        // trim + collassa whitespace interno
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn normalized_from_to(&self) -> (String, String) {
        (
            Self::normalize_input(&self.from),
            Self::normalize_input(&self.to),
        )
    }
}

impl eframe::App for NavicomputerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Stable IDs for focus/selection behavior.
        let from_id = egui::Id::new("navicomputer_from");
        let to_id = egui::Id::new("navicomputer_to");

        // Bootstrap updates (status changes every 300ms)
        self.tick_bootstrap(ctx);
        self.tick_status_deadline();

        // Dynamic window title (ready/computing + route context)
        self.update_window_title(ctx);

        // ----------------------------
        // Keyboard shortcuts (global)
        // ----------------------------
        let (enter, esc, ctrl_s) = ctx.input(|i| {
            let enter = i.key_pressed(egui::Key::Enter);
            let esc = i.key_pressed(egui::Key::Escape);
            let ctrl_s = i.modifiers.ctrl && i.key_pressed(egui::Key::S);
            (enter, esc, ctrl_s)
        });

        if enter && self.can_compute() {
            self.compute_route_stub();
        }
        if esc && self.can_clear() {
            self.clear_fields();
            // Optional: restore a ready status after clear (unless boot is still running)
            if self.boot_next.is_none() {
                self.status = "Navicomputer ready. All systems are online.".to_string();
            }
        }
        if ctrl_s && self.can_export() {
            self.export_json();
        }

        // ----------------------------
        // TOP PANEL (inputs + compute/clear)
        // ----------------------------
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("FROM:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.from)
                        .id(from_id)
                        .hint_text("Planet name or FID")
                        .desired_width(260.0),
                );

                ui.add_space(12.0);

                ui.label("TO:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.to)
                        .id(to_id)
                        .hint_text("Planet name or FID")
                        .desired_width(260.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let compute = ui
                        .add_enabled(self.can_compute(), egui::Button::new("Compute"))
                        .on_hover_text("Compute route (Enter)");
                    if compute.clicked() {
                        self.compute_route_stub();
                    }

                    let clear = ui
                        .add_enabled(self.can_clear(), egui::Button::new("Clear"))
                        .on_hover_text("Clear fields (Esc)");
                    if clear.clicked() {
                        self.clear_fields();
                        if self.boot_next.is_none() {
                            self.status = "Navicomputer ready. All systems are online.".to_string();
                        }
                    }
                });
            });

            ui.add_space(4.0);

            if let Some(err) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }
        });

        // Focus selection outside closures
        self.apply_focus_select_all(ctx, from_id, to_id);

        // Shared frame with no gaps/borders (explicit fill per panel below)
        let base_frame = egui::Frame::NONE
            .inner_margin(egui::Margin::same(0))
            .outer_margin(egui::Margin::same(0))
            .corner_radius(egui::CornerRadius::ZERO)
            .stroke(egui::Stroke::NONE);

        // ----------------------------
        // BOTTOM BAR (Export) - ABOVE status bar
        //   Left: version
        //   Right: Export JSON
        // ----------------------------
        const BOTTOM_BAR_HEIGHT: f32 = 44.0;
        let bottom_fill = ctx.style().visuals.panel_fill;

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .exact_height(BOTTOM_BAR_HEIGHT)
            .frame(base_frame.fill(bottom_fill))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                egui::Frame::NONE
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // DB status dot
                            self.db_status_indicator(ui);

                            ui.add_space(6.0);

                            // LEFT: version badge
                            ui.label(
                                egui::RichText::new(format!("v{}", Self::app_version()))
                                    .monospace()
                                    .color(ui.visuals().weak_text_color()),
                            )
                            .on_hover_text(
                                "sw_galaxy_map — Star Wars galaxy navicomputer (GUI + CLI)",
                            );

                            // spacer to push the button to the right
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let export = ui
                                        .add_enabled(
                                            self.can_export(),
                                            egui::Button::new("Export JSON"),
                                        )
                                        .on_hover_text("Export JSON (Ctrl+S)");
                                    if export.clicked() {
                                        self.export_json();
                                    }
                                },
                            );
                        });
                    });
            });

        // ----------------------------
        // STATUS BAR - LAST / VERY BOTTOM (messages only)
        // ----------------------------
        const STATUS_BAR_HEIGHT: f32 = 26.0;
        let status_fill = egui::Color32::from_gray(210);

        egui::TopBottomPanel::bottom("status_panel")
            .resizable(false)
            .exact_height(STATUS_BAR_HEIGHT)
            .frame(base_frame.fill(status_fill))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                egui::Frame::NONE
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        let msg = if self.status.trim().is_empty() {
                            " ".to_string()
                        } else {
                            self.status.clone()
                        };

                        ui.label(
                            egui::RichText::new(msg)
                                .monospace()
                                .color(egui::Color32::BLACK),
                        );
                    });
            });

        // ----------------------------
        // CENTRAL PANEL (output area) - BUILT LAST
        // ----------------------------
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label("Output:");

            let avail = ui.available_size();

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.set_min_size(avail);

                    ui.add_sized(
                        avail,
                        egui::TextEdit::multiline(&mut self.output)
                            .font(egui::TextStyle::Monospace)
                            .lock_focus(true)
                            .interactive(false),
                    );
                });
        });

        self.prev_focus = ctx.memory(|m| m.focused());
    }
}
