use std::sync::Arc;

use eframe::egui::{self, Context, Ui};
use egui_extras::syntax_highlighting::{self, CodeTheme};
use parking_lot::RwLock;
use tokio::sync::{mpsc::Sender, watch};

use crate::{
    AppState, EditorType, Msg, impl_content, impl_id, impl_indexable, impl_initialize, impl_initialize_tx, impl_target, impl_visible, mini_window::{HasMenu, Indexable, InitializeWatchTx as _, MiniWindow}
};

#[derive(Clone,Debug)]
pub struct PikchrEditor {
    pub id: egui::Id,
    target_svg: egui::Id,
    pub(crate) visible: bool,
    content: String,
    index: usize,
    initialized: bool,
    watch_tx: Option<watch::Sender<(egui::Id, String)>>,
}
impl PikchrEditor {
    pub fn new(id: egui::Id, target_svg: egui::Id) -> Self {
        Self {
            visible: true,
            content: String::new(),
            id,
            target_svg,
            index: 1,
            watch_tx: None,
            initialized: false,
        }
    }
}

impl HasMenu for PikchrEditor{}
impl MiniWindow for PikchrEditor {
    fn get_title(&self) -> String {
        format!("Pikchr Editor ({}) - {}", self.get_index(), self.id.short_debug_format())
    }

    fn inner_window(
        &mut self,
        _ctx: &Context,
        ui: &mut Ui,
        tx: Sender<Msg>,
        _app_state: Arc<RwLock<AppState>>,
    ) {

        self.initialize(tx);
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            let editor = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content).code_editor(),
            );

            if editor.changed() {
                let _ = self.watch_tx.as_ref().expect("Should be initialized").send((
                    self.id,
                    self.content.clone(),
                ));
            }
        });
    }
}
impl crate::mini_window::EditorType for PikchrEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        EditorType::Pikchr
    }
}
impl_id!(PikchrEditor, id);
impl_target!(PikchrEditor, target_svg);
impl_visible!(PikchrEditor, visible);
impl_initialize!(PikchrEditor, initialized);
impl_indexable!(PikchrEditor);
impl_content!(PikchrEditor, content);
impl_initialize_tx!(
    PikchrEditor, watch_tx,
    on_change: |(id,content)| Msg::Batch(vec![Msg::UpdateContent(id, content), Msg::UpdatePikchr(id)]),
    data: (egui::Id, String),
    empty: (egui::Id::new(""), String::new())
);
