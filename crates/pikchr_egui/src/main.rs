use eframe::egui::{self, ViewportBuilder};
use parking_lot::RwLock;
use pikchr_egui::{Msg, PikchrEgui, SPACE_MONO_BYTES, state::AppState, text_highlighting};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    println!("Available backends: {:?}", wgpu::Backends::all());
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder::default().with_app_id("sh.axk.pikchrpl"),
        ..Default::default()
    };

    tokio::spawn(async { text_highlighting::get_config() });


    eframe::run_native(
        "Pikchr.pl",
        native_options,
        Box::new(|cc| {
            catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::FRAPPE);
            Ok(Box::new(PikchrEgui::new(cc)))
        }),
    )
}
