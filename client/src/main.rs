mod app;
mod api;
mod audio;
mod config;
mod events;
mod native;
mod net;
mod screen;
mod theme;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Skypeline")
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Skypeline",
        options,
        Box::new(|cc| Ok(Box::new(app::SkyApp::new(cc)))),
    )
}
