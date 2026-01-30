use std::sync::Arc;

use eframe::egui::{self,Context, Ui};
use egui_extras::syntax_highlighting::{self, CodeTheme};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{AppState, Msg, impl_content, impl_id, impl_indexable, impl_target, impl_visible, mini_window::{self, HasMenu, Indexable, MiniWindow}};

#[derive(Debug)]
pub struct PrologEditor {
    pub id: egui::Id,
    pub(crate) visible: bool,
    target_svg: egui::Id,
    content: String,
    pikchr_content: String,
    index: usize,
}
impl PrologEditor {
    pub fn new(id: egui::Id, target_svg: egui::Id) -> Self {
        Self { visible: true, pikchr_content: String::new(), content: String::new(), id, target_svg, index: 1 }
    }
}
impl mini_window::EditorWindow for PrologEditor {
    fn get_editor_window(&self) -> crate::mini_window::EditorWindowView<'_> {
        crate::mini_window::EditorWindowView {
            index: &self.index,
            id: &self.id,
            content: &self.content,
            editor_type: Box::new(self as &dyn mini_window::EditorType),
            mini_window: Box::new(self as &dyn MiniWindow),
        }
    }
}

impl HasMenu for PrologEditor {}
impl MiniWindow for PrologEditor {
    fn get_title(&self) -> String {
        format!("Prolog Editor - {}", self.get_index())
    }

    fn inner_window(&mut self, ctx: &Context, ui: &mut Ui, tx: Sender<Msg>, _app_state: Arc<RwLock<AppState>>) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            let editor = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content).code_editor(),
            );

            if editor.changed() {
                let _ =  tx
                    .try_send(Msg::UpdateProlog(self.id, self.target_svg,self.content.clone()));
            }
        });
    }
}

impl crate::mini_window::EditorType for PrologEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        crate::EditorType::Prolog
    }
}

impl_id!(PrologEditor, id);
impl_indexable!(PrologEditor);
impl_visible!(PrologEditor, visible);
impl_content!(PrologEditor, pikchr_content);
impl_target!(PrologEditor, target_svg);
