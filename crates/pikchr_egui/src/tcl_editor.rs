use std::sync::Arc;

use eframe::egui::{self, Context, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{
    AppState, Msg,
    editor::{self, HandleEnter as _},
    impl_id, impl_indexable, impl_pikchr_content, impl_target, impl_visible,
    mini_window::{self, Error as _, HasMenu, MiniWindow},
    setter_getter_for_trait,
    text_highlighting::memoized_syntax_layouter,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TclEditor {
    pub id: egui::Id,
    pub(crate) visible: bool,
    target_svg: egui::Id,
    content: String,
    pikchr_content: String,
    index: usize,
    error: Option<String>,
}
impl TclEditor {
    pub fn new(id: egui::Id, target_svg: egui::Id) -> Self {
        Self {
            visible: true,
            pikchr_content: String::new(),
            content: String::new(),
            id,
            target_svg,
            index: 1,
            error: None,
        }
    }
}
impl mini_window::EditorWindow for TclEditor {
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

impl HasMenu for TclEditor {}
impl MiniWindow for TclEditor {
    fn get_title(&self) -> String {
        format!("Tcl Editor - {}", self.id.short_debug_format())
    }

    fn inner_window(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        tx: Sender<Msg>,
        _app_state: Arc<RwLock<AppState>>,
    ) {
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
                                    memoized_syntax_layouter(editor_id, ui, textbuffer, wrap_width, "Tcl")
                                }),
                        )
                    })
                    .inner;

                if editor.changed() {
                    let _ = tx.try_send(Msg::UpdateTcl(ctx.clone(), self.id, self.content.clone()));
                }
            });
        });
    }
}

impl crate::mini_window::EditorType for TclEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        crate::EditorType::Tcl
    }
}

impl editor::Editor for TclEditor {}
impl_id!(TclEditor, id);
impl_indexable!(TclEditor);
impl_visible!(TclEditor, visible);
impl_pikchr_content!(TclEditor, pikchr_content);
impl_target!(TclEditor, target_svg);
setter_getter_for_trait! { (content => String | content.clone() => String) for TclEditor as raw_content for mini_window::RawContent }
setter_getter_for_trait! { (error => Option<String> | error.clone() => Option<String>) for TclEditor as error for mini_window::Error }
