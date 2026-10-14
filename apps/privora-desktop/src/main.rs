#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod state;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 900.0])
            .with_min_inner_size([1040.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Privora",
        options,
        Box::new(|cc| Ok(Box::new(app::PrivoraApp::new(cc)))),
    )
}
