#![forbid(unsafe_code)]
mod app;
mod devices;
mod engine;
mod export;
mod layouts;
mod preferences;
mod xkb;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Typefinch")
            .with_inner_size([1060.0, 760.0])
            .with_min_inner_size([720.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "typefinch",
        options,
        Box::new(|cc| Ok(Box::new(app::Typefinch::new(cc)))),
    )
}
