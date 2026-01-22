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
        !self.from.trim().is_empty() && !self.to.trim().is_empty() && !self.computing
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
        self.status = "Computing route...".to_string();

        let from = self.from.trim().to_string();
        let to = self.to.trim().to_string();

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

        self.status = "Route computation completed.".to_string();
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
                self.status = "Export failed.".to_string();
            } else {
                self.error = None;
                self.status = format!("Export completed: {}", path.display());
            }
        } else {
            self.status = "Export cancelled.".to_string();
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
}

impl eframe::App for NavicomputerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Stable IDs for focus/selection behavior.
        let from_id = egui::Id::new("navicomputer_from");
        let to_id = egui::Id::new("navicomputer_to");

        // Bootstrap updates (status changes every 300ms)
        self.tick_bootstrap(ctx);

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
                    if ui
                        .add_enabled(self.can_compute(), egui::Button::new("Compute"))
                        .clicked()
                    {
                        self.compute_route_stub();
                    }

                    if ui
                        .add_enabled(self.can_clear(), egui::Button::new("Clear"))
                        .clicked()
                    {
                        self.clear_fields();
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

        // Shared frame with no gaps/borders.
        // IMPORTANT: we always set an explicit fill per panel to avoid "black seams".
        let base_frame = egui::Frame::NONE
            .inner_margin(egui::Margin::same(0))
            .outer_margin(egui::Margin::same(0))
            .corner_radius(egui::CornerRadius::ZERO)
            .stroke(egui::Stroke::NONE);

        // We want:
        //   - bottom_bar (Export) above
        //   - status_bar as the LAST panel at the very bottom (as you prefer)
        //
        // With multiple TopBottomPanel::bottom, the rule is: the LAST called becomes the lowest one.
        // So: call bottom_bar first, then status_bar.

        // ----------------------------
        // BOTTOM BAR (Export) - ABOVE status bar
        // ----------------------------
        const BOTTOM_BAR_HEIGHT: f32 = 44.0;
        let bottom_fill = ctx.style().visuals.panel_fill;

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .exact_height(BOTTOM_BAR_HEIGHT)
            .frame(base_frame.fill(bottom_fill))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), BOTTOM_BAR_HEIGHT),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.add_space(8.0);
                        if ui
                            .add_enabled(self.can_export(), egui::Button::new("Export JSON"))
                            .clicked()
                        {
                            self.export_json();
                        }
                        ui.add_space(8.0);
                    },
                );
            });

        // ----------------------------
        // STATUS BAR - LAST / VERY BOTTOM (gray, no borders)
        // ----------------------------
        const STATUS_BAR_HEIGHT: f32 = 24.0;

        // Use an explicit light-ish fill; do NOT rely on faint_bg_color, which can be dark on some themes.
        // This keeps it consistently "light gray" in both dark/light themes.
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

                        // Use normal text color for readability on light gray.
                        ui.label(
                            egui::RichText::new(msg)
                                .monospace()
                                .color(egui::Color32::BLACK),
                        );
                    });
            });

        // ----------------------------
        // CENTRAL PANEL (output area) - BUILT LAST so it uses remaining space
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
