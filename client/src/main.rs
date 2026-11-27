// Launch as a GUI application on Windows: no console window when started from
// Explorer, the Start Menu, or a desktop shortcut.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod api;
mod audio;
mod avatar;
mod config;
mod events;
mod native;
mod net;
mod screen;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Wireline")
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Wireline",
        options,
        Box::new(|cc| Ok(Box::new(ui::SkyApp::new(cc)))),
    )
}
