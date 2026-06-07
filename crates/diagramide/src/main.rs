use diagramide::{DiagramIDE, text_highlighting};
use eframe::egui::ViewportBuilder;


#[tokio::main]
async fn main() -> eframe::Result<()> {
    setup_tracing();

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../icon.png")).map_err(|e| eframe::Error::AppCreation(Box::new(e)))?;
    let root_logger = diagramide::logger::init_logger();
    let _guard = slog_scope::set_global_logger(root_logger);
    let native_options = eframe::NativeOptions {
        persist_window: true,
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder::default()
            .with_icon(icon)
            .with_app_id("sh.axk.diagramide"),
        ..Default::default()
    };
    tokio::spawn(async { text_highlighting::get_config() });

    eframe::run_native(
        "DiagramIDE",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(DiagramIDE::new(cc)))
        }),
    )
}

fn setup_tracing() {
    use tracing_subscriber::prelude::*;
    #[cfg(not(feature = "profile"))]
    use tracing_subscriber::fmt;
    use tracing_subscriber::{EnvFilter, Layer, Registry};

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
