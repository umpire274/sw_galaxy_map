mod app;

use anyhow::Result;

/// Run the native egui application.
pub fn run() -> Result<()> {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/sw_galaxy_map_256.png"
    )))
    .expect("Failed to load icon");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_title("SW Galaxy Map — Navicomputer")
            .with_inner_size([1100.0, 600.0]),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "SW Galaxy Map — Navicomputer",
        native_options,
        Box::new(|cc| Ok(Box::new(app::NavicomputerApp::new(cc)))),
    ) {
        eprintln!("Errore avviando la GUI: {err}");
    }

    Ok(())
}
