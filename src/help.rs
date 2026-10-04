use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

pub(crate) mod grammar;
mod guide;

use grammar::{GrammarViewState, render_document};
use guide::render_guide;

use crate::{
    Msg, impl_id, impl_indexable, impl_visible,
    mini_window::{self, HasMenu, Id, MiniWindow, NormalWindow, RenderToggle, WindowView},
    state::DiagramBackground,
};

/// A bundled markdown reference rendered with a table of contents and live
/// diagram previews. See [`grammar`] for the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HelpDoc {
    PikchrGrammar,
    HagoromoGuide,
    SvgbobGuide,
    ClipsGuide,
}

impl HelpDoc {
    pub const ALL: [Self; 4] = [
        Self::PikchrGrammar,
        Self::HagoromoGuide,
        Self::SvgbobGuide,
        Self::ClipsGuide,
    ];

    pub fn markdown(self) -> &'static str {
        match self {
            Self::PikchrGrammar => grammar::PIKCHR_GRAMMAR_MD,
            Self::HagoromoGuide => grammar::HAGOROMO_GUIDE_MD,
            Self::SvgbobGuide => grammar::SVGBOB_GUIDE_MD,
            Self::ClipsGuide => grammar::CLIPS_GUIDE_MD,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::PikchrGrammar => "Pikchr Grammar",
            Self::HagoromoGuide => "Hagoromo Guide",
            Self::SvgbobGuide => "Svgbob Guide",
            Self::ClipsGuide => "CLIPS Guide",
        }
    }

    /// The editor guide section the document's own Help button opens.
    pub fn editor_topic(self) -> HelpTopic {
        match self {
            Self::PikchrGrammar => HelpTopic::Pikchr,
            Self::HagoromoGuide => HelpTopic::Hagoromo,
            Self::SvgbobGuide => HelpTopic::Svgbob,
            Self::ClipsGuide => HelpTopic::Clips,
        }
    }

    pub fn topic(self) -> HelpTopic {
        match self {
            Self::PikchrGrammar => HelpTopic::Grammar,
            Self::HagoromoGuide => HelpTopic::HagoromoGuide,
            Self::SvgbobGuide => HelpTopic::SvgbobGuide,
            Self::ClipsGuide => HelpTopic::ClipsGuide,
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            Self::PikchrGrammar => 0,
            Self::HagoromoGuide => 1,
            Self::SvgbobGuide => 2,
            Self::ClipsGuide => 3,
        }
    }
}

/// Which document a [`HelpWindow`] shows. `Overview` and the per-editor
/// variants render the User Guide (with a context section); the document
/// variants render a bundled reference with a table of contents.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HelpTopic {
    #[default]
    Overview,
    Pikchr,
    Svgbob,
    Prolog,
    Tcl,
    Mruby,
    Clips,
    Hagoromo,
    PlainText,
    Render,
    Grammar,
    HagoromoGuide,
    SvgbobGuide,
    ClipsGuide,
}

impl HelpTopic {
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "DiagramIDE help",
            Self::Pikchr => "Pikchr help",
            Self::Svgbob => "Svgbob help",
            Self::Prolog => "Prolog help",
            Self::Tcl => "Tcl help",
            Self::Clips => "CLIPS help",
            Self::Mruby => "Ruby help",
            Self::Hagoromo => "Hagoromo help",
            Self::PlainText => "Plain-text help",
            Self::Render => "Render window help",
            Self::Grammar => HelpDoc::PikchrGrammar.title(),
            Self::HagoromoGuide => HelpDoc::HagoromoGuide.title(),
            Self::SvgbobGuide => HelpDoc::SvgbobGuide.title(),
            Self::ClipsGuide => HelpDoc::ClipsGuide.title(),
        }
    }

    /// The bundled document this topic renders. `None` for a guide section.
    pub fn document(self) -> Option<HelpDoc> {
        match self {
            Self::Grammar => Some(HelpDoc::PikchrGrammar),
            Self::HagoromoGuide => Some(HelpDoc::HagoromoGuide),
            Self::SvgbobGuide => Some(HelpDoc::SvgbobGuide),
            Self::ClipsGuide => Some(HelpDoc::ClipsGuide),
            _ => None,
        }
    }
}

// ── Help window ──────────────────────────────────────────

/// A help/documentation window. One window type that renders different content
/// depending on its [`HelpTopic`]: the User Guide, or the Pikchr Grammar
/// reference (with a sidebar table of contents).
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct HelpWindow {
    pub id: egui::Id,
    pub(crate) visible: bool,
    pub topic: HelpTopic,
    index: usize,
    /// Pending TOC navigation target for the Grammar view. `Some(i)` asks the
    /// renderer to scroll heading `#i` into view; consumed on use.
    scroll_target: Option<usize>,
    #[serde(skip, default)]
    grammar_view: GrammarViewState,
}

impl HelpWindow {
    pub fn new(id: egui::Id, topic: HelpTopic) -> Self {
        Self {
            id,
            visible: true,
            topic,
            index: 0,
            scroll_target: None,
            grammar_view: GrammarViewState::default(),
        }
    }
}

impl HasMenu for HelpWindow {}
impl RenderToggle for HelpWindow {}

impl MiniWindow for HelpWindow {
    fn get_title(&self) -> String {
        self.topic.title().to_owned()
    }

    fn help_topic(&self) -> HelpTopic {
        // The Guide topics are themselves help; a document window's own Help
        // button re-opens the guide section of the editor it documents.
        self.topic
            .document()
            .map_or(HelpTopic::Overview, HelpDoc::editor_topic)
    }

    fn outer_window(&self, ctx: &Context) -> egui::Window<'static> {
        let default = if self.topic.document().is_some() {
            (900.0, 650.0)
        } else {
            (520.0, 560.0)
        };
        egui::Window::new(self.get_title())
            .resizable(true)
            .default_size(default)
            .min_width(360.0)
            .id(self.get_id())
            .frame(egui::Frame::window(&ctx.style()).inner_margin(0.0))
    }
}

impl NormalWindow for HelpWindow {
    fn get_window(&self) -> WindowView<'_> {
        WindowView {
            index: &self.index,
            id: &self.id,
            mini_window: self as &dyn MiniWindow,
        }
    }
}

impl mini_window::InnerWindow for HelpWindow {
    fn inner_window(
        &mut self,
        _ctx: &Context,
        ui: &mut Ui,
        tx: Sender<Msg>,
        _background: DiagramBackground,
    ) {
        if let Some(doc) = self.topic.document() {
            render_document(ui, doc, &mut self.scroll_target, &mut self.grammar_view);
        } else {
            render_guide(ui, self.topic, &tx);
        }
    }
}

impl_id!(HelpWindow, id);
impl_indexable!(HelpWindow);
impl_visible!(HelpWindow, visible);
#[cfg(test)]
mod tests {
    use super::HelpTopic;

    #[test]
    fn help_topic_defaults_to_overview() {
        assert_eq!(HelpTopic::default(), HelpTopic::Overview);
    }

    #[test]
    fn every_document_round_trips_through_its_topic() {
        for doc in super::HelpDoc::ALL {
            assert_eq!(doc.topic().document(), Some(doc));
            assert_eq!(doc.topic().title(), doc.title());
            assert!(doc.editor_topic().document().is_none());
        }
    }
}
