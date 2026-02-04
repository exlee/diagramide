use eframe::egui::ViewportBuilder;
use pikchr_egui::{ Msg, PikchrEgui, text_highlighting};
#[cfg(debug_assertions)]
use tracing_subscriber::layer::SubscriberExt;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    println!("Available backends: {:?}", wgpu::Backends::all());
    #[cfg(debug_assertions)]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    ).unwrap();
    
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


