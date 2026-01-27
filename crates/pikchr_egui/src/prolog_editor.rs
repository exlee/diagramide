use std::sync::Arc;

use eframe::egui::{self,Context, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{AppState, Msg, impl_content, impl_id, impl_indexable, impl_target, impl_visible, sub_window::{Content, Indexable, IndexableMiniWindow, MiniWindow}};

pub struct PrologEditor {
    id: egui::Id,
    visible: bool,
    target_svg: egui::Id,
    content: String,
    index: usize,
}
impl PrologEditor {
    pub fn new(id: egui::Id, target_svg: egui::Id) -> Self {
        Self { visible: true, content: String::new(), id, target_svg, index: 1 }
    }
}
impl MiniWindow for PrologEditor {
    fn get_title(&self) -> String {
        format!("Prolog Editor - {}", self.get_index())
    }

    fn inner_window(&mut self, _ctx: &Context, ui: &mut Ui, tx: Sender<Msg>, _app_state: Arc<RwLock<AppState>>) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            let editor = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content).code_editor(),
            );

            if editor.changed() {
                let _ =  tx
                    .try_send(Msg::UpdateProlog(self.id.clone(), self.target_svg.clone(),self.content.clone()));
            }
        });
    }
}

impl crate::sub_window::EditorType for PrologEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        crate::EditorType::Prolog
    }
}

impl_id!(PrologEditor, id);
impl_indexable!(PrologEditor);
impl_visible!(PrologEditor, visible);
impl_content!(PrologEditor, content);
impl_target!(PrologEditor, target_svg);
