use diagramide::{DiagramIDE, text_highlighting};
use eframe::egui::ViewportBuilder;


#[tokio::main]
async fn main() -> eframe::Result<()> {
    setup_tracing();
    println!("Available backends: {:?}", wgpu::Backends::all());

    let root_logger = diagramide::logger::init_logger();
    let _guard = slog_scope::set_global_logger(root_logger);
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

fn setup_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, Registry, EnvFilter, Layer};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let layer = {
        #[cfg(feature = "profile")]
        {
            tracing_tracy::TracyLayer::default().boxed()
        }
        #[cfg(not(feature = "profile"))]
        {
            fmt::layer().boxed()
        }
    };

    Registry::default()
        .with(filter)
        .with(layer)
        .init();
}
