//! CLIPS editor: facts asserted against the diagram templates become Pikchr,
//! which the paired Render window draws. See [`crate::clips`].

use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    editor::{self, GenericEditor, HandleEnter},
    impl_generated_content, impl_id, impl_indexable, impl_target, impl_visible,
    mini_window::{self, HasMenu, HasName as _, MiniWindow, RenderToggle},
    parinfer,
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
    /// Last text parinfer accepted. Content that differs without an edit
    /// came from outside the editor.
    #[serde(skip)]
    parinfer_text: String,
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
            parinfer_text: String::new(),
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

    /// Runs paren mode once on text from outside the editor (a load, the
    /// template), so later edits start from indentation that matches the parens.
    fn adopt_external_content(&mut self) {
        if self.content != self.parinfer_text {
            if let Some(fixed) = parinfer::paren(&self.content) {
                self.content = fixed;
            }
            self.parinfer_text = self.content.clone();
        }
    }

    /// Lets parinfer rewrite the edit made since the last accepted text.
    fn infer_parens(&mut self, ctx: &Context, editor_id: egui::Id, prev_cursor: Option<usize>) {
        if self.content == self.parinfer_text {
            return;
        }
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id)
            && let Some(range) = state.cursor.char_range()
            && let Some(edit) = parinfer::smart(
                &self.parinfer_text,
                prev_cursor,
                &self.content,
                range.primary.index,
                range.secondary.index,
            )
        {
            self.content = edit.text;
            let cursor = egui::text::CCursorRange::one(egui::text::CCursor::new(edit.cursor));
            state.cursor.set_char_range(Some(cursor));
            state.store(ctx, editor_id);
        }
        self.parinfer_text = self.content.clone();
    }
}

fn cursor_index(ctx: &Context, editor_id: egui::Id) -> Option<usize> {
    let state = egui::TextEdit::load_state(ctx, editor_id)?;
    Some(state.cursor.char_range()?.primary.index)
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
        self.adopt_external_content();
        let ctx = ui.ctx().clone();
        let prev_cursor = cursor_index(&ctx, editor_id);
        let output = egui::TextEdit::multiline(&mut self.content)
            .code_editor()
            .desired_width(f32::INFINITY)
            .id(editor_id)
            .layouter(&mut |ui, textbuffer, wrap_width| {
                memoized_syntax_layouter(editor_id, ui, textbuffer, wrap_width, CLIPS_SYNTAX)
            })
            .show(ui);
        if output.response.changed() {
            self.infer_parens(&ctx, editor_id, prev_cursor);
        }
        output
    }

    fn handle_enter(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) {
        self.adopt_external_content();
        let prev_cursor = cursor_index(ctx, editor_id);
        self.handle_indent(ctx, ui, editor_id, |current_line| {
            let indent = editor::get_line_indent(current_line);
            let opened = current_line.matches('(').count() > current_line.matches(')').count();
            if opened {
                format!("{indent}  ")
            } else {
                indent
            }
        });
        self.infer_parens(ctx, editor_id, prev_cursor);
    }

    fn handle_tab_binding(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) -> bool {
        self.adopt_external_content();
        let prev_cursor = cursor_index(ctx, editor_id);
        let changed = HandleEnter::handle_tab(self, ctx, ui, editor_id);
        if changed {
            self.infer_parens(ctx, editor_id, prev_cursor);
        }
        changed
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

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;

    use super::*;

    const EDITOR_ID: &str = "clips_parinfer_editor";

    /// Drives the editor the way `InnerWindow` does: Enter, then the text area.
    fn harness(content: &str, cursor: usize) -> Harness<'static, ClipsEditor> {
        let editor_id = egui::Id::new(EDITOR_ID);
        let mut editor = ClipsEditor::new(egui::Id::new("clips"), egui::Id::new("render"));
        editor.content = content.to_owned();
        let mut harness = Harness::new_ui_state(
            move |ui, editor: &mut ClipsEditor| {
                let ctx = ui.ctx().clone();
                if HandleEnter::handle_enter(editor, &ctx, ui, editor_id) {
                    GenericEditor::handle_enter(editor, &ctx, ui, editor_id);
                }
                editor.editor_spec(editor_id, ui);
            },
            editor,
        );
        harness.run();
        let mut state = egui::TextEdit::load_state(&harness.ctx, editor_id).unwrap();
        let range = egui::text::CCursorRange::one(egui::text::CCursor::new(cursor));
        state.cursor.set_char_range(Some(range));
        state.store(&harness.ctx, editor_id);
        harness
            .ctx
            .memory_mut(|memory| memory.request_focus(editor_id));
        harness.run();
        harness
    }

    fn type_text(harness: &mut Harness<'_, ClipsEditor>, text: &str) {
        for c in text.chars() {
            harness.event(egui::Event::Text(c.to_string()));
            harness.run();
        }
    }

    #[test]
    fn typed_open_paren_is_closed() {
        let mut harness = harness("", 0);
        type_text(&mut harness, "(box");
        assert_eq!(harness.state().content, "(box)");
    }

    #[test]
    fn enter_then_typing_extends_the_form() {
        let mut harness = harness("(box (id b))", 11);
        harness.key_press(egui::Key::Enter);
        harness.run();
        type_text(&mut harness, "(label \"x\"");
        assert_eq!(harness.state().content, "(box (id b)\n  (label \"x\"))");
    }

    #[test]
    fn loaded_content_is_reindented_to_match_parens() {
        let harness = harness("(defrule r\n(a)\n=>\n(b))", 0);
        assert_eq!(harness.state().content, "(defrule r\n (a)\n =>\n (b))");
    }
}
