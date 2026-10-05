//! Hagoromo editor: a Gluon script that evaluates to a diagram. The paired
//! Render window shows the SVG produced by [`crate::hagoromo`], so this editor
//! has no Output Type selector.

use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    editor::{self, GenericEditor, HandleEnter as _},
    hagoromo_completion, impl_generated_content, impl_id, impl_indexable, impl_target,
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
        let plain_tab = ui.input(|i| i.key_pressed(egui::Key::Tab) && i.modifiers.is_none());
        if plain_tab
            && ui.memory(|mem| mem.has_focus(editor_id))
            && let Some(suggestion) = self.suggestion(ctx, editor_id)
            && let Some(replacement) = suggestion.replacement()
        {
            ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
            self.apply_completion(ctx, editor_id, &suggestion, &replacement);
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
        let mut job = egui::text::LayoutJob::default();
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let strong = egui::TextFormat::simple(font.clone(), ui.visuals().strong_text_color());
        let weak = egui::TextFormat::simple(font, ui.visuals().weak_text_color());
        if let Some(suggestion) = suggestion {
            let mut candidates = suggestion.candidates.iter();
            if let Some(first) = candidates.next() {
                job.append(&first.name, 0.0, strong);
                job.append(&format!(" : {}", first.detail), 0.0, weak.clone());
            }
            for candidate in candidates {
                job.append(&format!("  {}", candidate.name), 0.0, weak.clone());
            }
        }
        job.wrap = egui::text::TextWrapping::truncate_at_width(ui.available_width());
        ui.label(job);
    }
}

impl HagoromoEditor {
    fn suggestion(
        &self,
        ctx: &Context,
        editor_id: egui::Id,
    ) -> Option<hagoromo_completion::Suggestion> {
        let range = egui::TextEdit::load_state(ctx, editor_id)?
            .cursor
            .char_range()?;
        if range.primary.index != range.secondary.index {
            return None;
        }
        hagoromo_completion::suggest(
            &self.content,
            range.primary.index,
            crate::hagoromo::completions(),
        )
    }

    fn apply_completion(
        &mut self,
        ctx: &Context,
        editor_id: egui::Id,
        suggestion: &hagoromo_completion::Suggestion,
        replacement: &str,
    ) {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) else {
            return;
        };
        let (content, cursor) = hagoromo_completion::apply(&self.content, suggestion, replacement);
        self.content = content;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(cursor),
            )));
        state.store(ctx, editor_id);
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
