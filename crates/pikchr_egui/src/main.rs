use parking_lot::RwLock;
use pikchr_egui::{Msg, PikchrEgui, state::AppState};
use std::sync::Arc;
use tokio::sync::mpsc;



#[tokio::main]
async fn main() -> eframe::Result<()> {
    println!("Available backends: {:?}", wgpu::Backends::all());
    let (tx, rx) = mpsc::channel::<Msg>(100);
    let state = Arc::new(RwLock::new(AppState::new()));
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    let ui_state = state.clone();
    eframe::run_native(
        "Pikchr.pl",
        native_options,
        Box::new(|cc| {
            catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::FRAPPE);
            Ok(Box::new(PikchrEgui::new(cc, rx, tx, ui_state)))
        }),
    )
}
