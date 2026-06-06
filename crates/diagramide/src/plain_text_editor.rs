use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    editor::{self, GenericEditor, HandleEnter as _},
    impl_id, impl_indexable, impl_visible,
    mini_window::{self, HasMenu, HasName as _, MiniWindow},
    sender_ext::DebouncedTrySend as _,
    setter_getter_for_trait,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct PlainTextEditor {
    pub id: egui::Id,
    pub(crate) visible: bool,
    content: String,
    index: usize,
    name: String,
    error: Option<String>,
}

impl PlainTextEditor {
    pub fn new(id: egui::Id) -> Self {
        Self {
            id,
            visible: true,
            content: String::new(),
            index: 1,
            name: id.short_debug_format(),
            error: None,
        }
    }
}

impl HasMenu for PlainTextEditor {}

impl MiniWindow for PlainTextEditor {
    fn get_title(&self) -> String {
        format!("Plain text - {}", self.get_name())
    }
}

impl mini_window::NormalWindow for PlainTextEditor {
    fn get_window(&self) -> mini_window::WindowView<'_> {
        mini_window::WindowView {
            index: &self.index,
            id: &self.id,
            mini_window: self as &dyn MiniWindow,
        }
    }
}

impl GenericEditor for PlainTextEditor {
    fn editor_spec(&mut self, editor_id: egui::Id, ui: &mut Ui) -> egui::text_edit::TextEditOutput {
        egui::TextEdit::multiline(&mut self.content)
            .code_editor()
            .desired_width(f32::INFINITY)
            .id(editor_id)
            .show(ui)
    }

    fn handle_enter(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) {
        self.handle_indent(ctx, ui, editor_id, editor::get_line_indent);
    }

    fn editor_on_changed(&self, tx: Sender<Msg>, ctx: &Context) {
        let _ = tx.try_send_debounced(self.id, 100, Msg::UpdatePlainText(ctx.clone(), self.id));
    }

    fn initialize(&mut self, _tx: Sender<Msg>) {}
}

impl crate::mini_window::EditorType for PlainTextEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        crate::EditorType::PlainText
    }
}

impl editor::Editor for PlainTextEditor {}
impl_id!(PlainTextEditor, id);
impl_indexable!(PlainTextEditor);
impl_visible!(PlainTextEditor, visible);
setter_getter_for_trait! { (content => String | content.clone() => String) for PlainTextEditor as raw_content for mini_window::RawContent }
setter_getter_for_trait! { (error => Option<String> | error.clone() => Option<String>) for PlainTextEditor as error for mini_window::HasError }
setter_getter_for_trait! { (name => String | name.clone() => String) for PlainTextEditor as name for mini_window::HasName }
