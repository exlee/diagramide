use eframe::egui::{self, Vec2};
use resvg::tiny_skia;
use resvg::usvg::{self, fontdb};
use std::sync::Arc;

const RENDER_WIDTH: f32 = 512.0;

use crate::sub_window::{Indexable, InitializeWatchTx, MiniWindow};
use crate::{Msg, SPACE_MONO_BYTES, impl_id, impl_indexable, impl_initialize, impl_initialize_tx, impl_visible};

pub fn render_svg_to_texture(
    ctx: &egui::Context,
    svg_content: &str,
    name: &str,
    scale: f32,
) -> Option<egui::TextureHandle> {
    let mut db = fontdb::Database::new();
    db.load_font_data(SPACE_MONO_BYTES.to_vec());

    // 2. Parse the SVG
    let xml_opt = usvg::Options {
        fontdb: Arc::new(db),
        ..Default::default()
    };
		dbg!(scale);
    let scale = scale;
    let tree: usvg::Tree = usvg::Tree::from_str(svg_content, &xml_opt).ok()?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());

    // 1. Determine base multiplier to target RENDER_WIDTH
    let base_mult = RENDER_WIDTH / w;

    // 2. Calculate final dimensions with scale, clamped to hardware limit
    let final_width_f = (w * base_mult * scale).min(16384.0);
    let final_height_f = (h * base_mult * scale).min(16384.0);

    // 3. Recalculate effective scale to ensure transform matches clamped pixel dimensions
    let effective_scale_x = final_width_f / w;
    let effective_scale_y = final_height_f / h;

    let width = final_width_f.ceil() as u32;
    let height = final_height_f.ceil() as u32;

    if width == 0 || height == 0 {
        return None;
    }

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    // Use the effective scale to ensure the SVG content fills the clamped pixmap exactly
    let transform = tiny_skia::Transform::from_scale(effective_scale_x, effective_scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], pixmap.data());

    // 6. Load into GPU memory
    Some(ctx.load_texture(name, image, egui::TextureOptions::LINEAR))
}

pub struct SvgWindow {
    id: egui::Id,
    pub diagram_texture: Option<egui::TextureHandle>,
    pub svg_string: Option<String>,
    pub initial_size: Vec2,
    pub prev_size: Option<Vec2>,
    pub scale: f32,
    watch_tx: Option<tokio::sync::watch::Sender<egui::Id>>,
    visible: bool,
    index: usize,
    initialized: bool,
}

impl SvgWindow {
    pub fn new(id: egui::Id) -> Self {
        Self {
            id,
            index: 1,
            diagram_texture: None,
            svg_string: None,
            initial_size: Vec2::from((200.0,200.0)),
            prev_size: None,
            scale: 1.5,
            visible: true,
            initialized: false,
            watch_tx: None,
        }
    }
}
impl_indexable!(SvgWindow);
impl_initialize!(SvgWindow, initialized);
impl_initialize_tx!(
    SvgWindow, watch_tx,
    on_change: |id| Msg::RequestRedraw(id),
    data: egui::Id,
    empty: egui::Id::new("")
);
impl MiniWindow for SvgWindow {
    fn outer_window(&self, ctx: &egui::Context) -> egui::Window<'static> {
        egui::Window::new(self.get_title())
            .id(self.id)
            .resizable(true)
            .default_size(self.initial_size)
            .frame(egui::Frame::window(&ctx.style()).inner_margin(0.0))
    }
    fn inner_window(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        tx: tokio::sync::mpsc::Sender<crate::Msg>,
        _app_state: Arc<parking_lot::RwLock<crate::AppState>>,
    ) {
        self.initialize(tx);
        if self.diagram_texture.is_none() {
            return;
        }
        let texture = self.diagram_texture.as_ref().expect("Just checked");
        egui::Frame::new()
            .fill(egui::Color32::WHITE)
            .inner_margin(40.0)
            .show(ui, |ui| {
                let available = ui.available_size();
                if self.prev_size.is_some() && self.prev_size != Some(available.ceil()) {
                    self.scale = (available.ceil() / self.initial_size.ceil()).max_elem();
                    let _ = self.watch_tx.as_ref().expect("Should be initialized").send(self.id);
                }
                self.prev_size = Some(available.ceil());

                ui.set_min_size(available);

                let logical_size = texture.size_vec2() / self.scale;
                let aspect = logical_size.x / logical_size.y;
                let mut new_size = available;

                if available.x / available.y > aspect {
                    new_size.x = available.y * aspect;
                } else {
                    new_size.y = available.x / aspect;
                }

                let img = egui::Image::new(texture).fit_to_exact_size(new_size).uv(
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                );

                ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);

                ui.centered_and_justified(|ui| {
                    ui.add(img);
                });
            });
    }

    fn should_show(&self) -> bool {
        self.diagram_texture.is_some() && self.visible
    }

    fn should_be_listed(&self) -> bool {
        self.diagram_texture.is_some()
    }

    fn get_title(&self) -> String {
        format!("SVG ({})", self.get_index())
    }
}

impl_id!(SvgWindow, id);
impl_visible!(SvgWindow, visible);
