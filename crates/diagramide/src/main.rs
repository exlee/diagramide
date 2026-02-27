use eframe::egui::ViewportBuilder;
use diagramide::{ DiagramIDE, text_highlighting};

#[tokio::main]
async fn main() -> eframe::Result<()> {
    println!("Available backends: {:?}", wgpu::Backends::all());
    
    let native_options = eframe::NativeOptions {
        persist_window: true,
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder::default().with_app_id("sh.axk.pikchrpl"),
        ..Default::default()
    };

    tokio::spawn(async { text_highlighting::get_config() });


    eframe::run_native(
        "DiagramIDE",
        native_options,
        Box::new(|cc| {
            catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::FRAPPE);
            Ok(Box::new(DiagramIDE::new(cc)))
        }),
    )
}


