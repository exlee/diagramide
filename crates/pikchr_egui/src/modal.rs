use std::{env, path::PathBuf, sync::Arc};

use eframe::{
    egui::{self, Context, Layout, Margin, Vec2},
};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{ExportType, Msg};

#[derive(serde::Serialize,serde::Deserialize,Clone, Debug)]
#[serde(tag = "type")]
pub enum ModalItem{
    ExportModal(ExportModal)
}

impl ModalItem {
    pub fn as_modal(&self) -> Arc<RwLock<dyn Modal>> {
        match self {
            ModalItem::ExportModal(modal) => Arc::new(RwLock::new(modal.clone()))
        }
    }
}

pub trait Modal: Send + Sync + std::fmt::Debug {
    fn show(&mut self, ctx: &Context, tx: Sender<Msg>);
    fn into_item(&self) -> ModalItem;
}

#[derive(serde::Serialize,serde::Deserialize, Clone, Debug)]
pub struct ExportModal {
    svg_id: egui::Id,
    export_type: ExportType,
    destination: String,
    #[allow(unused)]
    file_name: String,
}
impl ExportModal {
    pub fn new(svg_id: egui::Id, file_name: String, export_type: ExportType) -> Self {
        Self {
            svg_id,
            destination: Self::build_destination(&file_name, &export_type),
            export_type,
            file_name,
        }
    }
    fn build_destination(file: &str, export_type: &ExportType) -> String {
        let extension = match export_type {
            ExportType::Svg => "svg",
            ExportType::Png => "png",
        };
        let file_cleaned: String = file.chars()
            .filter(|&c| c.is_alphanumeric() || c == ' ')
            .collect();
        let file_cleaned = file_cleaned.split_whitespace().collect::<Vec<_>>().join("_");
        let joined_pb = env::current_dir().unwrap_or(PathBuf::from(".")).join(file_cleaned);
        let joined = joined_pb.to_string_lossy();
        format!("{}.{}", joined, extension)
    }

}
impl Modal for ExportModal {
    fn into_item(&self) -> ModalItem {
        ModalItem::ExportModal(self.clone())
    }
    fn show(&mut self, ctx: &Context, tx: Sender<Msg>) {
        egui::Modal::new(egui::Id::new("egui_modal"))
            //.backdrop_color(Color32::BLACK)
            .show(ctx, |ui| {
                let title = match self.export_type {
                    ExportType::Svg => "Export as SVG",
                    ExportType::Png => "Export as PNG",
                };
                ui.set_min_size(Vec2::from((400.0, 50.0)));
                ui.heading(title);
                ui.separator();

                ui.add_space(10.0);
                ui.add_sized(
                    (ui.available_width(), 30.0),
                    egui::TextEdit::singleline(&mut self.destination)
                        .margin(Margin::symmetric(4, 8)),
                );
                ui.add_space(10.0);

                ui.separator();
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui.button("Export").clicked() {
                        let _ = tx.try_send(Msg::Export(self.svg_id, self.destination.clone(), self.export_type));
                    };
                    ui.add_space(10.0);

                    if ui.button("Close").clicked() {
                        let _ = tx.try_send(Msg::PopModal);
                    };
                });
            });
    }
}
