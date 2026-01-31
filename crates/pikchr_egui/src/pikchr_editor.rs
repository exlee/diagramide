
use std::sync::Arc;

use eframe::egui::{self, Context, TextBuffer, Ui};
use parking_lot::RwLock;
use tokio::sync::{mpsc::Sender, watch};

use crate::{
    AppState, EditorType, Msg, impl_content, impl_id, impl_indexable, impl_initialize,
    impl_initialize_tx, impl_target, impl_visible,
    mini_window::{
        self, EditorWindow, Error as _, HasMenu, Indexable, InitializeWatchTx as _, MiniWindow,
    },
    setter_getter_for_trait, text_highlighting::{self, memoized_syntax_layouter},
};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PikchrEditor {
    pub id: egui::Id,
    target_svg: egui::Id,
    pub(crate) visible: bool,
    pub(crate) content: String,
    pub(crate) index: usize,
    #[serde(skip_serializing, default)]
    initialized: bool,
    #[serde(skip)]
    watch_tx: Option<watch::Sender<(egui::Context, egui::Id, String)>>,
    error: Option<String>,
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
            error: None,
        }
    }
}

impl EditorWindow for PikchrEditor {
    fn get_editor_window(&self) -> crate::mini_window::EditorWindowView<'_> {
        crate::mini_window::EditorWindowView {
            index: &self.index,
            id: &self.id,
            content: self as &dyn mini_window::Content,
            editor_type: self as &dyn mini_window::EditorType,
            mini_window: self as &dyn MiniWindow,
        }
    }
}
impl HasMenu for PikchrEditor {}
impl MiniWindow for PikchrEditor {
    fn get_title(&self) -> String {
        format!("Pikchr Editor - {}", self.id.short_debug_format())
    }

    fn inner_window(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        tx: Sender<Msg>,
        _app_state: Arc<RwLock<AppState>>,
    ) {
        self.initialize(tx);
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if self.error.is_some() {
                let t = egui::RichText::new(self.get_error().unwrap()).monospace();
                ui.label(t);
            } else {
                ui.label("");
            }
            let editor = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.content)
                    .code_editor()
                    .layouter(&mut |ui, textbuffer, wrap_width| {
                        memoized_syntax_layouter(ui, textbuffer, wrap_width, "Pikchr")
                    }),
            );

            if editor.changed() {
                let _ = self
                    .watch_tx
                    .as_ref()
                    .expect("Should be initialized")
                    .send((ctx.clone(), self.id, self.content.clone()));
            }
        });
    }
}
impl PikchrEditor {
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
    on_change: |(ctx,id,content)| Msg::Batch(vec![Msg::UpdateContent(id, content), Msg::UpdatePikchr(ctx, id)]),
    data: (Context,egui::Id, String),
    empty: (Context::default(),egui::Id::new(""), String::new())
);

setter_getter_for_trait! { (error => Option<String> | error.clone() => Option<String>) for PikchrEditor as error for mini_window::Error }
