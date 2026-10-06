//! Hagoromo editor: a Gluon script that evaluates to a diagram. The paired
//! Render window shows the SVG produced by [`crate::hagoromo`], so this editor
//! has no Output Type selector.

use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    editor::{self, GenericEditor, HandleEnter as _},
    completion, hagoromo_completion, impl_generated_content, impl_id, impl_indexable, impl_target,
    impl_visible,
    mini_window::{self, HasMenu, HasName as _, MiniWindow, RenderToggle},
    sender_ext::DebouncedTrySend as _,
    setter_getter_for_trait,
    text_highlighting::memoized_syntax_layouter,
};

/// Syntect has no Gluon grammar; OCaml is the closest ML-family syntax.
const GLUON_SYNTAX: &str = "OCaml";

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct HagoromoEditor {
    pub id: egui::Id,
    pub(crate) visible: bool,
    target_svg: egui::Id,
    content: String,
    /// The script after `!!name!!` and `$$name$$` substitution. A `!!name!!`
    /// naming another Hagoromo editor becomes `ref_name`, bound to that script.
    expanded_content: String,
    index: usize,
    name: String,
    error: Option<String>,
    #[serde(default = "default_render")]
    pub(crate) render: bool,
}

fn default_render() -> bool {
    true
}

impl HagoromoEditor {
    pub fn new(id: egui::Id, target_svg: egui::Id) -> Self {
        Self {
            visible: true,
            expanded_content: String::new(),
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
let { prim, (<>), (|||), (===), (|>) } = import! hagoromo
let { circle, square, text, fc, color } = prim

let node label =
    circle 1.0 |> fc color.white <> text label 0.6

node "a" ||| node "b"
"#
        .trim()
        .into()
    }
}

impl mini_window::EditorWindow for HagoromoEditor {
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

impl HasMenu for HagoromoEditor {}
impl MiniWindow for HagoromoEditor {
    fn get_title(&self) -> String {
        format!("Hagoromo - {}", self.get_name())
    }
    fn help_topic(&self) -> crate::help::HelpTopic {
        crate::help::HelpTopic::Hagoromo
    }
    fn can_save_to_library(&self) -> bool {
        true
    }
}
impl GenericEditor for HagoromoEditor {
    fn editor_spec(&mut self, editor_id: egui::Id, ui: &mut Ui) -> egui::text_edit::TextEditOutput {
        egui::TextEdit::multiline(&mut self.content)
            .code_editor()
            .desired_width(f32::INFINITY)
            .id(editor_id)
            .layouter(&mut |ui, textbuffer, wrap_width| {
                memoized_syntax_layouter(editor_id, ui, textbuffer, wrap_width, GLUON_SYNTAX)
            })
            .show(ui)
    }

    fn handle_enter(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) {
        self.handle_indent(ctx, ui, editor_id, |current_line| {
            let indent = editor::get_line_indent(current_line);
            if current_line.trim_end().ends_with('=') {
                format!("{indent}    ")
            } else {
                indent
            }
        });
    }

    fn editor_on_changed(&self, tx: Sender<Msg>, ctx: &Context) {
        let _ = tx.try_send_debounced(
            self.id,
            300,
            Msg::UpdateHagoromo(ctx.clone(), self.id, self.content.clone()),
        );
    }

    fn line_comment(&self) -> Option<&'static str> {
        Some("//")
    }

    fn initialize(&mut self, _tx: Sender<Msg>) {}

    fn handle_tab_binding(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) -> bool {
        if let Some((suggestion, replacement)) =
            completion::take_tab(ui, editor_id, || self.suggestion(ctx, editor_id))
        {
            completion::accept(ctx, editor_id, &mut self.content, &suggestion, &replacement);
            return true;
        }
        editor::HandleEnter::handle_tab(self, ctx, ui, editor_id)
    }

    fn has_footer(&self) -> bool {
        true
    }

    fn show_footer(&mut self, ctx: &Context, ui: &mut Ui, editor_id: egui::Id) {
        let suggestion = ui
            .memory(|mem| mem.has_focus(editor_id))
            .then(|| self.suggestion(ctx, editor_id))
            .flatten();
        completion::show_footer(ui, suggestion.as_ref());
    }
}

impl HagoromoEditor {
    fn suggestion(&self, ctx: &Context, editor_id: egui::Id) -> Option<completion::Suggestion> {
        let cursor = completion::cursor(ctx, editor_id)?;
        hagoromo_completion::suggest(&self.content, cursor, crate::hagoromo::completions())
    }
}

impl RenderToggle for HagoromoEditor {
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
    fn source_format(&self) -> crate::SourceFormat {
        crate::SourceFormat::Hagoromo
    }
}

impl crate::mini_window::EditorType for HagoromoEditor {
    fn get_editor_type(&self) -> crate::EditorType {
        crate::EditorType::Hagoromo
    }
}

impl editor::Editor for HagoromoEditor {}
impl_id!(HagoromoEditor, id);
impl_indexable!(HagoromoEditor);
impl_visible!(HagoromoEditor, visible);
impl_generated_content!(HagoromoEditor, expanded_content);
impl_target!(HagoromoEditor, target_svg);
setter_getter_for_trait! { (content => String | content.clone() => String) for HagoromoEditor as raw_content for mini_window::RawContent }
setter_getter_for_trait! { (error => Option<String> | error.clone() => Option<String>) for HagoromoEditor as error for mini_window::HasError }
setter_getter_for_trait! { (name => String | name.clone() => String) for HagoromoEditor as name for mini_window::HasName }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_renders() {
        let editor = HagoromoEditor::new(egui::Id::new("h"), egui::Id::new("h-svg"));
        let svg = crate::hagoromo::render_hagoromo(&editor.content).unwrap();
        assert!(svg.contains("<circle"));
        assert!(svg.contains("<text"));
    }

    #[test]
    fn has_no_output_selector_and_exports_hagoromo_source() {
        let editor = HagoromoEditor::new(egui::Id::new("h"), egui::Id::new("h-svg"));
        assert!(editor.has_renderer());
        assert!(!editor.has_output_selector());
        assert_eq!(editor.source_format(), crate::SourceFormat::Hagoromo);
    }
}
