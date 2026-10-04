//! CLIPS editor: facts asserted against the diagram templates become Pikchr,
//! which the paired Render window draws. See [`crate::clips`].

use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    editor::{self, GenericEditor, HandleEnter as _},
    impl_generated_content, impl_id, impl_indexable, impl_target, impl_visible,
    mini_window::{self, HasMenu, HasName as _, MiniWindow, RenderToggle},
    sender_ext::DebouncedTrySend as _,
    setter_getter_for_trait,
    text_highlighting::memoized_syntax_layouter,
};

/// Syntect has no CLIPS grammar; CLIPS is Lisp-shaped.
const CLIPS_SYNTAX: &str = "Lisp";

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ClipsEditor {
    pub id: egui::Id,
    pub(crate) visible: bool,
    target_svg: egui::Id,
    content: String,
    pikchr_content: String,
    index: usize,
    name: String,
    error: Option<String>,
    #[serde(default = "default_render")]
    pub(crate) render: bool,
}

fn default_render() -> bool {
    true
}

impl ClipsEditor {
    pub fn new(id: egui::Id, target_svg: egui::Id) -> Self {
        Self {
            visible: true,
            pikchr_content: String::new(),
            content: Self::template_content(),
            name: id.short_debug_format(),
            id,
            target_svg,
            index: 1,
            error: None,
            render: true,
        }
    }

    fn template_content() -> String {
        r#"
; Facts become Pikchr statements, in order.
(box (id b) (label "Hello"))
(circle (id c) (label "World"))
(arrow (from b) (to c))

; Rules can add more.
(defrule caption
  (circle (id ?c))
  =>
  (assert (text (label "a rule drew this") (at (str-cat ?c ".s - (0, 0.4)")))))
"#
        .trim()
        .into()
    }
}

impl mini_window::EditorWindow for ClipsEditor {
    fn get_editor_window(&self) -> crate::mini_window::EditorWindowView<'_> {
        crate::mini_window::EditorWindowView {
            index: &self.index,
            id: &self.id,
            content: self as &dyn mini_window::GeneratedContent,
            editor_type: self as &dyn mini_window::EditorType,
            mini_window: self as &dyn MiniWindow,
            name: &self.name,
        }
    }
}

impl HasMenu for ClipsEditor {}
impl MiniWindow for ClipsEditor {
    fn get_title(&self) -> String {
        format!("CLIPS - {}", self.get_name())
    }
    fn help_topic(&self) -> crate::help::HelpTopic {
        crate::help::HelpTopic::Clips
    }
    fn can_save_to_library(&self) -> bool {
        true
    }
}

impl GenericEditor for ClipsEditor {
    fn editor_spec(&mut self, editor_id: egui::Id, ui: &mut Ui) -> egui::text_edit::TextEditOutput {
        egui::TextEdit::multiline(&mut self.content)
            .code_editor()
            .desired_width(f32::INFINITY)
            .id(editor_id)
            .layouter(&mut |ui, textbuffer, wrap_width| {
                memoized_syntax_layouter(editor_id, ui, textbuffer, wrap_width, CLIPS_SYNTAX)
            })
            .show(ui)
    }

    fn handle_enter(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) {
        self.handle_indent(ctx, ui, editor_id, |current_line| {
            let indent = editor::get_line_indent(current_line);
            let opened = current_line.matches('(').count() > current_line.matches(')').count();
            if opened {
                format!("{indent}  ")
            } else {
                indent
            }
        });
    }

    fn editor_on_changed(&self, tx: Sender<Msg>, ctx: &Context) {
        let _ = tx.try_send_debounced(
            self.id,
            400,
            Msg::UpdateClips(ctx.clone(), self.id, self.content.clone()),
        );
    }

    fn initialize(&mut self, _tx: Sender<Msg>) {}
}

impl crate::mini_window::EditorType for ClipsEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        crate::EditorType::Clips
    }
}

impl RenderToggle for ClipsEditor {
    fn has_renderer(&self) -> bool {
        true
    }
    fn render_enabled(&self) -> bool {
        self.render
    }
    fn set_render_enabled(&mut self, on: bool) {
        self.render = on;
    }
    fn has_output_selector(&self) -> bool {
        false
    }
    fn output_type(&self) -> crate::OutputType {
        crate::OutputType::Pikchr
    }
    fn set_output_type(&mut self, _output_type: crate::OutputType) {}
}

impl editor::Editor for ClipsEditor {}
impl_id!(ClipsEditor, id);
impl_indexable!(ClipsEditor);
impl_visible!(ClipsEditor, visible);
impl_generated_content!(ClipsEditor, pikchr_content);
impl_target!(ClipsEditor, target_svg);
setter_getter_for_trait! { (content => String | content.clone() => String) for ClipsEditor as raw_content for mini_window::RawContent }
setter_getter_for_trait! { (error => Option<String> | error.clone() => Option<String>) for ClipsEditor as error for mini_window::HasError }
setter_getter_for_trait! { (name => String | name.clone() => String) for ClipsEditor as name for mini_window::HasName }
