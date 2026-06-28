use std::sync::OnceLock;

use eframe::egui::{self, Context, Ui};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg, impl_id, impl_indexable, impl_visible,
    mini_window::{self, HasMenu, Id, MiniWindow, NormalWindow, RenderToggle, WindowView},
    state::DiagramBackground,
};

/// The full Pikchr grammar reference, assembled from the upstream per-topic
/// markdown pages into a single self-contained document. Bundled into the
/// binary so the in-app help works offline.
const PIKCHR_GRAMMAR_MD: &str = include_str!("../assets/docs/pikchr_grammar_full.md");

/// Which document a [`HelpWindow`] shows. `Overview` and the per-editor
/// variants render the User Guide (with a context section); `Grammar` renders
/// the full Pikchr grammar reference with a table of contents.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HelpTopic {
    #[default]
    Overview,
    Pikchr,
    Prolog,
    Tcl,
    Mruby,
    PlainText,
    Render,
    Grammar,
}

impl HelpTopic {
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => "DiagramIDE Help",
            Self::Pikchr => "Pikchr Help",
            Self::Prolog => "Prolog Help",
            Self::Tcl => "Tcl Help",
            Self::Mruby => "Ruby Help",
            Self::PlainText => "Plain Text Help",
            Self::Render => "Render Window Help",
            Self::Grammar => "Pikchr Grammar",
        }
    }

    /// Whether this topic renders the big grammar document (which needs a TOC
    /// and a wider window) rather than the guide body.
    fn is_grammar(self) -> bool {
        matches!(self, Self::Grammar)
    }
}

// ── Help as a first-class window ──────────────────────────────────────────

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
    /// markdown renderer to scroll heading `#i` into view; consumed on use.
    scroll_target: Option<usize>,
}

impl HelpWindow {
    pub fn new(id: egui::Id, topic: HelpTopic) -> Self {
        Self {
            id,
            visible: true,
            topic,
            index: 0,
            scroll_target: None,
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
        // The Guide topics are themselves help; the Grammar window documents
        // Pikchr, so its own Help button re-opens the Pikchr guide section.
        if self.topic.is_grammar() {
            HelpTopic::Pikchr
        } else {
            HelpTopic::Overview
        }
    }

    fn outer_window(&self, ctx: &Context) -> egui::Window<'static> {
        let default = if self.topic.is_grammar() {
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
        ui.style_mut().override_font_id = Some(egui::TextStyle::Monospace.resolve(ui.style()));
        if self.topic.is_grammar() {
            render_grammar(ui, &mut self.scroll_target);
        } else {
            render_guide(ui, self.topic, &tx);
        }
    }
}

impl_id!(HelpWindow, id);
impl_indexable!(HelpWindow);
impl_visible!(HelpWindow, visible);

// ── Guide rendering ───────────────────────────────────────────────────────

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(text)
            .monospace()
            .size(18.0)
            .color(ui.visuals().hyperlink_color),
    );
}

fn feature(ui: &mut egui::Ui, name: &str, description: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(name)
                .monospace()
                .color(ui.visuals().hyperlink_color),
        );
        ui.label(description);
    });
}

/// A hyperlink-styled, keyboard-focusable label that opens the Pikchr Grammar
/// reference in its own help window.
fn grammar_link(ui: &mut egui::Ui, tx: &Sender<Msg>) {
    let accent = ui.visuals().hyperlink_color;
    let resp = ui.add(
        egui::Label::new(
            egui::RichText::new("Open Pikchr Grammar reference \u{2192}")
                .monospace()
                .color(accent)
                .underline(),
        )
        .selectable(false)
        .sense(egui::Sense::click()),
    );
    if resp.clicked() {
        let _ = tx.try_send(Msg::ShowHelp(HelpTopic::Grammar));
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
}

fn common_editor_help(ui: &mut egui::Ui) {
    heading(ui, "Editing");
    feature(
        ui,
        "Live updates",
        "The render and dependent windows update automatically after you edit source.",
    );
    feature(
        ui,
        "Cmd/Ctrl+R",
        "Renames the focused editor. Names are used by cross-window references.",
    );
    feature(
        ui,
        "Enter",
        "Inserts a newline and automatically carries or adjusts indentation.",
    );
    feature(
        ui,
        "Close button",
        "Hides the window. Reopen it from Windows in the main menu.",
    );
    feature(
        ui,
        "Cmd/Ctrl + close button",
        "Permanently deletes the editor and its render window from the workspace.",
    );

    heading(ui, "Cross-window references");
    feature(
        ui,
        "!!NAME!!",
        "Includes the raw source text of another named editor. Plain text windows can be included this way.",
    );
    feature(
        ui,
        "$$NAME$$",
        "Includes the generated Pikchr output of another named diagram editor.",
    );
    ui.label("References can be nested up to three replacement passes.");
}

fn topic_help(ui: &mut egui::Ui, topic: HelpTopic, tx: &Sender<Msg>) {
    match topic {
        HelpTopic::Overview | HelpTopic::Grammar => {},
        HelpTopic::Pikchr => {
            common_editor_help(ui);
            heading(ui, "Pikchr");
            ui.label(
                "Write Pikchr directly. Valid source is rendered live in the paired Render window.",
            );
            ui.add_space(4.0);
            grammar_link(ui, tx);
        },
        HelpTopic::Prolog => {
            common_editor_help(ui);
            heading(ui, "Prolog");
            ui.label("Define a diagram//0 DCG. Its text output is interpreted as Pikchr.");
        },
        HelpTopic::Tcl => {
            common_editor_help(ui);
            heading(ui, "Tcl");
            ui.label("Return a string containing Pikchr source. The Tcl editor is available only when Tcl 8.6 can be loaded.");
        },
        HelpTopic::Mruby => {
            common_editor_help(ui);
            heading(ui, "Ruby");
            ui.label("Text written with print or puts becomes Pikchr source. The editor is available only when Ruby support is available.");
        },
        HelpTopic::PlainText => {
            common_editor_help(ui);
            heading(ui, "Plain text");
            ui.label("Stores reusable raw text. Include it from another editor with !!NAME!!; it has no generated Pikchr output or Render window.");
        },
        HelpTopic::Render => {
            heading(ui, "Render window");
            feature(
                ui,
                "Automatic preview",
                "Displays the paired editor's generated Pikchr output and redraws as the window is resized.",
            );
            feature(
                ui,
                "Export",
                "Exports SVG, PNG, transparent PNG, or copies generated Pikchr code to the clipboard.",
            );
            feature(
                ui,
                "Close button",
                "Hides the preview. Reopen it from Windows in the main menu.",
            );
            feature(
                ui,
                "Cmd/Ctrl + close button",
                "Permanently deletes only this Render window. It is recreated when its editor next renders.",
            );
        },
    }
}

/// The User Guide body: a context-specific section (if any) followed by the
/// full feature guide.
fn render_guide(ui: &mut egui::Ui, topic: HelpTopic, tx: &Sender<Msg>) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if topic != HelpTopic::Overview {
                topic_help(ui, topic, tx);
                ui.separator();
                heading(ui, "Full feature guide");
            } else {
                grammar_link(ui, tx);
                ui.add_space(4.0);
            }

            heading(ui, "Workspace");
            feature(
                ui,
                "Autosave",
                "The current workspace and window layout persist between app launches.",
            );
            feature(
                ui,
                "Save / Load Workspace",
                "Exports or imports the complete workspace as JSON.",
            );
            feature(
                ui,
                "Reset Workspace",
                "Deletes all workspace windows after confirmation.",
            );
            feature(
                ui,
                "Windows",
                "Shows or hides existing windows, plus the diagnostic Logger and Debug windows.",
            );
            feature(ui, "View", "Changes the scale of the complete interface.");

            common_editor_help(ui);

            heading(ui, "Editor types");
            feature(ui, "Pikchr", "Direct Pikchr source with live rendering.");
            feature(ui, "Prolog", "A diagram//0 DCG generates Pikchr source.");
            feature(
                ui,
                "Tcl",
                "A Tcl script returns Pikchr source when Tcl 8.6 is available.",
            );
            feature(
                ui,
                "Ruby",
                "print and puts output becomes Pikchr when Ruby support is available.",
            );
            feature(
                ui,
                "Plain text",
                "Reusable raw text for !!NAME!! references; no paired render.",
            );

            heading(ui, "Rendering and export");
            feature(
                ui,
                "Live Render window",
                "Diagram editors own a paired, resizable preview window.",
            );
            feature(
                ui,
                "Export",
                "Render windows export SVG, PNG, transparent PNG, and generated Pikchr source.",
            );
            feature(
                ui,
                "Errors",
                "Evaluation and rendering errors appear next to their editor and in the Logger.",
            );
        });
}

// ── Grammar rendering (TOC + markdown) ────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct TocEntry {
    level: u8,
    text: String,
}

/// Cached table of contents for the grammar document. Built once from the
/// (compile-time-fixed) markdown, shared by every Grammar help window.
fn grammar_toc() -> &'static [TocEntry] {
    static TOC: OnceLock<Vec<TocEntry>> = OnceLock::new();
    TOC.get_or_init(|| parse_toc(PIKCHR_GRAMMAR_MD))
}

/// Parse ATX headings outside of fenced code blocks. Code-fence `#` comment
/// lines (Pikchr source blocks) must be skipped so they don't pollute the TOC.
fn parse_toc(src: &str) -> Vec<TocEntry> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence: &str = "";
    for line in src.lines() {
        let trimmed = line.trim_start();
        if !in_fence {
            if trimmed.starts_with("```") {
                in_fence = true;
                fence = "```";
                continue;
            }
            if trimmed.starts_with("~~~") {
                in_fence = true;
                fence = "~~~";
                continue;
            }
        } else if trimmed.starts_with(fence) {
            in_fence = false;
            fence = "";
            continue;
        }
        if in_fence {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix('#') else {
            continue;
        };
        let mut level = 1u8;
        let mut text = rest;
        while let Some(r) = text.strip_prefix('#') {
            level += 1;
            text = r;
        }
        let text = text.trim();
        if !text.is_empty() {
            out.push(TocEntry {
                level,
                text: text.to_owned(),
            });
        }
    }
    out
}

/// Render the Grammar view: a resizable left sidebar TOC and the markdown body.
/// `scroll_target` is read/written by both panels so a TOC click scrolls the
/// body in the same frame.
fn render_grammar(ui: &mut egui::Ui, scroll_target: &mut Option<usize>) {
    let accent = ui.visuals().hyperlink_color;

    egui::SidePanel::left("grammar_toc")
        .resizable(true)
        .width_range(140.0..=340.0)
        .default_width(210.0)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Contents")
                    .monospace()
                    .size(14.0)
                    .color(accent),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, entry) in grammar_toc().iter().enumerate() {
                        // Show only the top structural levels to keep the TOC
                        // navigable; deeper sub-headings remain reachable by
                        // scrolling the body.
                        if entry.level > 3 {
                            continue;
                        }
                        let indent = (entry.level as f32 - 1.0) * 10.0;
                        ui.horizontal(|ui| {
                            ui.add_space(indent);
                            let rich = egui::RichText::new(&entry.text).monospace().size(11.5);
                            if ui.add(egui::Button::new(rich).frame(false)).clicked() {
                                *scroll_target = Some(i);
                            }
                        });
                    }
                });
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_markdown(ui, PIKCHR_GRAMMAR_MD, scroll_target);
            });
    });
}

/// Minimal, dependency-free Markdown renderer for the bundled grammar
/// reference. It walks the source line-by-line and distinguishes:
///   * fenced code blocks (``` or ~~~) -> monospace, slightly smaller;
///   * ATX headings (`#`..`######`) -> accent-coloured, graduated size, and
///     addressable by TOC index for `scroll_to_me` navigation;
///   * everything else (bullets, tables, prose) -> monospace body text.
///
/// Inline markup (`**bold**`, links, HTML entities) is rendered literally,
/// which is acceptable for a reference document.
///
/// Heading enumeration matches [`parse_toc`]: both skip fenced `#` lines, so a
/// TOC index maps 1:1 to a rendered heading.
fn render_markdown(ui: &mut egui::Ui, src: &str, scroll_target: &mut Option<usize>) {
    let accent = ui.visuals().hyperlink_color;
    let mut in_fence = false;
    let mut fence: &str = "";
    let mut heading_index = 0usize;

    for raw in src.lines() {
        let trimmed = raw.trim_start();

        if !in_fence {
            if trimmed.starts_with("```") {
                in_fence = true;
                fence = "```";
                continue;
            }
            if trimmed.starts_with("~~~") {
                in_fence = true;
                fence = "~~~";
                continue;
            }
        } else if trimmed.starts_with(fence) {
            in_fence = false;
            fence = "";
            continue;
        }

        if in_fence {
            ui.label(
                egui::RichText::new(if raw.is_empty() { " " } else { raw })
                    .monospace()
                    .size(11.0),
            );
            continue;
        }

        if trimmed.is_empty() {
            ui.add_space(4.0);
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('#') {
            let mut level = 1usize;
            let mut text = rest;
            while let Some(r) = text.strip_prefix('#') {
                level += 1;
                text = r;
            }
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            let size = match level {
                1 => 18.0,
                2 => 16.0,
                3 => 14.0,
                _ => 12.5,
            };
            ui.add_space(6.0);
            let resp = ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .monospace()
                        .size(size)
                        .color(accent),
                )
                .selectable(false),
            );
            if *scroll_target == Some(heading_index) {
                resp.scroll_to_me(Some(egui::Align::TOP));
                *scroll_target = None;
            }
            heading_index += 1;
            continue;
        }

        ui.label(egui::RichText::new(raw).monospace().size(12.0));
    }
}

#[cfg(test)]
mod tests {
    use super::{HelpTopic, PIKCHR_GRAMMAR_MD, grammar_toc, parse_toc};

    #[test]
    fn help_topic_defaults_to_overview() {
        assert_eq!(HelpTopic::default(), HelpTopic::Overview);
    }

    /// The bundled grammar reference must resolve at compile time and contain
    /// the expected top-level document.
    #[test]
    fn bundled_grammar_doc_is_present_and_well_formed() {
        assert!(!PIKCHR_GRAMMAR_MD.is_empty(), "grammar doc is empty");
        assert!(
            PIKCHR_GRAMMAR_MD.starts_with("# Pikchr Grammar"),
            "grammar doc is missing its H1 title"
        );
        for needle in [
            "## *statement-list*",
            "## *statement*",
            "## *attribute*",
            "## *position*",
            "## *expr*",
        ] {
            assert!(
                PIKCHR_GRAMMAR_MD.contains(needle),
                "grammar doc is missing section header {needle:?}"
            );
        }
        assert!(
            !PIKCHR_GRAMMAR_MD.contains("](./"),
            "grammar doc still contains dead ./X.md links"
        );
    }

    /// The TOC must skip Pikchr `#` comment lines inside fenced code blocks
    /// (e.g. `# Start and end blocks`), which the raw line census would
    /// otherwise mistake for headings.
    #[test]
    fn toc_excludes_fenced_comment_lines() {
        let toc = parse_toc(PIKCHR_GRAMMAR_MD);
        assert!(toc.iter().any(|e| e.text == "Pikchr Grammar"), "missing H1");
        assert!(
            !toc.iter().any(|e| e.text == "Start and end blocks"),
            "fenced pikchr comment leaked into TOC"
        );
    }

    /// The cached TOC and a fresh parse must agree.
    #[test]
    fn cached_toc_matches_fresh_parse() {
        assert_eq!(grammar_toc(), parse_toc(PIKCHR_GRAMMAR_MD).as_slice());
    }
}
