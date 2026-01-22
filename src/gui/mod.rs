// src/gui/mod.rs
mod app;

use anyhow::Result;

pub fn run() -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SW Galaxy Map — Navicomputer")
            .with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    // `eframe::run_native` restituisce un `Result<_, eframe::Error>` che non è convertibile
    // in `anyhow::Error` (non è `Send + Sync`), quindi non usiamo `?` qui.
    // Inoltre la factory deve restituire direttamente `Box<dyn eframe::App>`.
    if let Err(err) = eframe::run_native(
        "SW Galaxy Map — Navicomputer",
        native_options,
        Box::new(|cc| Ok(Box::new(app::NavicomputerApp::new(cc)))),
    ) {
        eprintln!("Errore avviando la GUI: {err}");
    }

    Ok(())
}
