use std::sync::Arc;

use eframe::egui::{self, Context, Ui};
use parking_lot::RwLock;
use tokio::sync::{mpsc::Sender, watch};

use crate::{
    AppState, EditorType, Msg,
    editor::{self, Editor, HandleEnter as _},
    impl_id, impl_indexable, impl_initialize, impl_initialize_tx, impl_pikchr_content, impl_target,
    impl_visible,
    mini_window::{
        self, EditorWindow, Error as _, HasMenu, InitializeWatchTx as _, MiniWindow, RawContent,
    },
    setter_getter_for_trait,
    text_highlighting::memoized_syntax_layouter,
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
            content: self as &dyn mini_window::PikchrContent,
            editor_type: self as &dyn mini_window::EditorType,
            mini_window: self as &dyn MiniWindow,
        }
    }
}
impl HasMenu for PikchrEditor {
    fn has_menu(&self) -> bool {
        false
    }

    fn menu(&self, ui: &mut Ui, tx: Sender<Msg>) {
        ui.menu_button("View", |ui| {
            if ui.button("Font Size").clicked() {
                let _ = tx.try_send(Msg::FontSizeModal(self.id));
                ui.close();
            }
        });
    }
}
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
            let editor_id = ui.make_persistent_id(self.id);

            let indent_requested = self.handle_enter(ctx, ui, editor_id);
            if indent_requested {
                self.handle_indent(ctx, ui, editor_id, |current_line| {
                    editor::get_line_indent(current_line)
                });
            }

            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                let editor = egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.content)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .id(editor_id)
                                .frame(false)
                                .layouter(&mut |ui, textbuffer, wrap_width| {
                                    memoized_syntax_layouter(
                                        editor_id, ui, textbuffer, wrap_width, "Pikchr",
                                    )
                                }),
                        )
                    })
                    .inner;

                if editor.changed() {
                    let _ = self
                        .watch_tx
                        .as_ref()
                        .expect("Should be initialized")
                        .send((ctx.clone(), self.id, self.get_raw_content()));
                }
            });
        });
    }
}
impl PikchrEditor {}
impl Editor for PikchrEditor {}
impl crate::mini_window::EditorType for PikchrEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        EditorType::Pikchr
    }
}

//impl crate::mini_window::InitializeWatchTx for PikchrEditor { }
impl_id!(PikchrEditor, id);
impl_target!(PikchrEditor, target_svg);
impl_visible!(PikchrEditor, visible);
impl_initialize!(PikchrEditor, initialized);
impl_indexable!(PikchrEditor);
impl_pikchr_content!(PikchrEditor, content);
impl_initialize_tx!(
    PikchrEditor, watch_tx,
    on_change: |(ctx,id,_)| Msg::UpdatePikchr(ctx, id),
    data: (Context,egui::Id, String),
    empty: (Context::default(),egui::Id::new(""), String::new())
);

setter_getter_for_trait! { (content => String | content.clone() => String) for PikchrEditor as raw_content for mini_window::RawContent }
setter_getter_for_trait! { (error => Option<String> | error.clone() => Option<String>) for PikchrEditor as error for mini_window::Error }
