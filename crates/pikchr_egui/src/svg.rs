use eframe::egui::{self, Context, Layout, Vec2};
use std::fmt;
use std::sync::Arc;


use crate::mini_window::{self, HasMenu, InitializeWatchTx, MiniWindow};
use crate::{
    Msg, impl_id, impl_indexable, impl_initialize, impl_initialize_tx,
    impl_visible,
};


#[derive(serde::Serialize,serde::Deserialize, Clone)]
pub struct SvgWindow {
    pub id: egui::Id,
    pub owner_id: egui::Id,
    #[serde(skip)]
    pub diagram_texture: Option<egui::TextureHandle>,
    pub svg_string: Option<String>,
    pub initial_size: Vec2,
    pub prev_size: Option<Vec2>,
    pub scale: f32,
    #[serde(skip)]
    image: Option<egui::ColorImage>,
    #[serde(skip)]
    watch_tx: Option<tokio::sync::watch::Sender<(egui::Context, egui::Id)>>,
    pub(crate) visible: bool,
    index: usize,
    #[serde(skip_serializing,default)]
    initialized: bool,
}
impl fmt::Debug for SvgWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SvgWindow")
            .field("id", &self.id)
            // Use a placeholder string for the non-Debug field
            .field("diagram_texture", &self.diagram_texture.as_ref().map(|_| "TextureHandle(...)"))
            .field("image", &self.image.as_ref().map(|_| "Image(...)"))
            .field("svg_string", &self.svg_string)
            .field("initial_size", &self.initial_size)
            .field("prev_size", &self.prev_size)
            .field("scale", &self.scale)
            // Skip complex channels or internal types entirely if irrelevant
            .field("watch_tx", &"Option<Sender>") 
            .field("visible", &self.visible)
            .finish_non_exhaustive() // Indicates other fields exist (index, initialized)
    }
}

impl SvgWindow {
    pub fn new(id: egui::Id, owner_id: egui::Id) -> Self {
        Self {
            id,
            owner_id,
            index: 1,
            diagram_texture: None,
            svg_string: None,
            initial_size: Vec2::from((300.0, 300.0)),
            prev_size: None,
            scale: 1.5,
            image: None,
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
    on_change: |(ctx,id)| Msg::RequestRedraw(ctx,id),
    data: (Context, egui::Id),
    empty: (egui::Context::default(), egui::Id::new(""))
);
impl HasMenu for SvgWindow {
    fn has_menu(&self) -> bool {
        true
    }
    fn menu(&self, ui: &mut egui::Ui, tx: tokio::sync::mpsc::Sender<Msg>) {
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("PNG").clicked() {
                    let _ = tx.try_send(Msg::ExportModal(self.id, self.get_title(), crate::ExportType::Png));
                };
                if ui.button("SVG").clicked() {
                    let _ = tx.try_send(Msg::ExportModal(self.id, self.get_title(), crate::ExportType::Svg));
                };
                ui.label("Export");
            });
    }
}
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
        self.initialize(tx.clone());
        if self.diagram_texture.is_none() {
            return;
        }
        let texture = self.diagram_texture.as_ref().expect("Just checked");
        egui::Frame::new().inner_margin(10.0).show(ui, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::WHITE)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        let available = ui.available_size();
                        if self.prev_size.is_some() && self.prev_size != Some(available.ceil()) {
                            self.scale = (available.ceil() / self.initial_size.ceil()).max_elem();
                            let _ = self
                                .watch_tx
                                .as_ref()
                                .expect("Should be initialized")
                                .send((ui.ctx().clone(), self.id));
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
        format!("Render - {:?}", self.owner_id)
    }
}

pub struct SvgWindowView<'a> {
    pub diagram_texture: &'a mut Option<egui::TextureHandle>,
    pub svg_string: &'a mut Option<String>,
    pub scale: &'a mut f32,
    pub image: &'a mut Option<egui::ColorImage>,
    pub id: &'a egui::Id,
}
impl mini_window::SvgWindow for SvgWindow {
    fn get_svg_window(&mut self) -> self::SvgWindowView<'_> {
        SvgWindowView {
            id: &mut self.id,
            diagram_texture: &mut self.diagram_texture,
            svg_string: &mut self.svg_string,
            scale: &mut self.scale,
            image: &mut self.image,
        }

    }
}

impl mini_window::NormalWindow for SvgWindow {
    fn get_window(&self) -> mini_window::WindowView<'_> {
        mini_window::WindowView {
            index: &self.index,
            id: &self.id,
            mini_window: self as &dyn MiniWindow,
        }
    }
}
impl_id!(SvgWindow, id);
impl_visible!(SvgWindow, visible);
