use eframe::{egui::{self, Color32, Context, Ui}, epaint::tessellator::Path};
use egui_extras::loaders::file_loader::FileLoader;
use tokio::sync::mpsc::Sender;
use wgpu::naga::diagnostic_filter::FilterableTriggeringRule;

use crate::{ExportType, Msg, identifiers};

pub trait Modal: Send + Sync {
    fn show(&self, ctx: &Context, tx: Sender<Msg>);
}
pub struct ExportModal {
    svg_id: egui::Id,
    export_type: ExportType,
}
impl ExportModal {
    pub fn new(svg_id: egui::Id, export_type: ExportType) -> Self {
        Self {
            svg_id,
            export_type,
        }
    }
}
impl Modal for ExportModal {
    fn show(&self, ctx: &Context, tx: Sender<Msg>) {
        egui::Modal::new(egui::Id::new("egui_modal"))
            //.backdrop_color(Color32::BLACK)
            .show(ctx, |ui| {
                ui.heading("Confirm action");
                ui.separator();
                if ui.button("Close").clicked() {
                    tx.try_send(Msg::PopModal);
                };
            });
    }
}
