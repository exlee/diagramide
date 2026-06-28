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

/// Named egui font families registered at startup in `lib.rs`. Regular uses
/// SpaceMono; bold uses SpaceMono-Bold so `**bold**` renders with true weight.
const REG_FAMILY: &str = "SpaceMono";
const BOLD_FAMILY: &str = "SpaceMonoBold";

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
    /// renderer to scroll heading `#i` into view; consumed on use.
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
        if self.topic.is_grammar() {
            render_grammar(ui, &mut self.scroll_target);
        } else {
            // Guide keeps the existing monospace look (egui default Monospace).
            ui.style_mut().override_font_id = Some(egui::TextStyle::Monospace.resolve(ui.style()));
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

// ── Grammar rendering (parsed once, TOC + markdown) ───────────────────────
//
// The grammar document is parsed with pulldown-cmark *once* into a cached
// `Vec<Block>` (see `grammar_blocks`). Rendering then iterates the cached
// blocks, emitting one widget per block via `egui::text::LayoutJob` (so inline
// `**bold**`/`*italic*`/`` `code` `` are formatted correctly). This keeps the
// per-frame cost to a few hundred widgets with no markdown parsing or string
// re-allocation, which is what makes the window scroll smoothly.

#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
    link_target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    Heading {
        level: u8,
        /// 0-based index among all headings; used as the TOC / scroll target.
        idx: usize,
        spans: Vec<Span>,
    },
    Para(Vec<Span>),
    ListItem(Vec<Span>),
    /// A fenced code block; text preserves embedded newlines.
    Code(String),
    /// Raw HTML converted to readable plain text. The bundled grammar doc has
    /// a few handwritten HTML tables; dropping them makes the help look broken.
    Html(String),
    TableRow(Vec<Vec<Span>>),
    Hr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GrammarDoc {
    blocks: Vec<Block>,
    anchors: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TocEntry {
    level: u8,
    idx: usize,
    text: String,
}

/// Span-collecting context on the parser stack. Each variant owns the spans
/// accumulated for its block.
enum Ctx {
    Para(Vec<Span>),
    Heading(u8, usize, Vec<Span>),
    ListItem(Vec<Span>),
    Cell(Vec<Span>),
}

impl Ctx {
    fn spans_mut(&mut self) -> &mut Vec<Span> {
        match self {
            Ctx::Para(s) | Ctx::Heading(_, _, s) | Ctx::ListItem(s) | Ctx::Cell(s) => s,
        }
    }
}

fn heading_level(l: pulldown_cmark::HeadingLevel) -> u8 {
    use pulldown_cmark::HeadingLevel::*;
    match l {
        H1 => 1,
        H2 => 2,
        H3 => 3,
        H4 => 4,
        H5 => 5,
        H6 => 6,
    }
}

/// Parse the document into renderable blocks. Headings carry a stable `idx`
/// (0-based, in document order) that the TOC, anchors, and scroll-to-heading
/// share.
fn parse_doc(src: &str) -> GrammarDoc {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let normalized = normalize_doc_tables(src);
    let mut blocks: Vec<Block> = Vec::new();
    let mut anchors = std::collections::HashMap::new();
    let mut pending_anchors: Vec<String> = Vec::new();
    let mut ctx_stack: Vec<Ctx> = Vec::new();
    let mut bold = 0u32;
    let mut italic = 0u32;
    let mut link_target: Option<String> = None;
    let mut heading_counter = 0usize;
    // `Some` while inside a fenced code block; Text/SoftBreak accumulate here.
    let mut code_buf: Option<String> = None;
    let mut row_cells: Vec<Vec<Span>> = Vec::new();

    for event in Parser::new_ext(&normalized, opts) {
        match event {
            Event::Start(Tag::Paragraph) => ctx_stack.push(Ctx::Para(Vec::new())),
            Event::End(TagEnd::Paragraph) => {
                if let Some(Ctx::Para(s)) = ctx_stack.pop()
                    && !s.is_empty()
                {
                    blocks.push(Block::Para(s));
                }
            },

            Event::Start(Tag::Heading { level, .. }) => {
                let idx = heading_counter;
                heading_counter += 1;
                for anchor in pending_anchors.drain(..) {
                    anchors.insert(anchor, idx);
                }
                ctx_stack.push(Ctx::Heading(heading_level(level), idx, Vec::new()));
            },
            Event::End(TagEnd::Heading(_)) => {
                if let Some(Ctx::Heading(level, idx, spans)) = ctx_stack.pop() {
                    if is_table_row_text(&plain_text(&spans)) {
                        blocks.push(Block::Para(spans));
                    } else {
                        blocks.push(Block::Heading { level, idx, spans });
                    }
                }
            },

            Event::Start(Tag::List(_)) => {},
            Event::End(TagEnd::List(_)) => {},
            Event::Start(Tag::Item) => ctx_stack.push(Ctx::ListItem(Vec::new())),
            Event::End(TagEnd::Item) => {
                if let Some(Ctx::ListItem(s)) = ctx_stack.pop() {
                    blocks.push(Block::ListItem(s));
                }
            },

            Event::Start(Tag::CodeBlock(_)) => {
                code_buf = Some(String::new());
            },
            Event::End(TagEnd::CodeBlock) => {
                if let Some(text) = code_buf.take() {
                    blocks.push(Block::Code(text));
                }
            },

            Event::Start(Tag::Table(_)) | Event::End(TagEnd::Table) => {},
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                row_cells.clear();
            },
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                if !row_cells.is_empty() {
                    blocks.push(Block::TableRow(std::mem::take(&mut row_cells)));
                }
            },
            Event::Start(Tag::TableCell) => ctx_stack.push(Ctx::Cell(Vec::new())),
            Event::End(TagEnd::TableCell) => {
                if let Some(Ctx::Cell(s)) = ctx_stack.pop() {
                    row_cells.push(s);
                }
            },

            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => link_target = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => link_target = None,

            Event::Text(t) => {
                if let Some(buf) = code_buf.as_mut() {
                    buf.push_str(t.as_ref());
                } else {
                    push_span(
                        &mut ctx_stack,
                        &t,
                        bold > 0,
                        italic > 0,
                        false,
                        link_target.clone(),
                    );
                }
            },
            Event::Code(t) => {
                push_span(
                    &mut ctx_stack,
                    &t,
                    bold > 0,
                    italic > 0,
                    true,
                    link_target.clone(),
                );
            },
            Event::Html(t) => {
                pending_anchors.extend(extract_anchor_ids(&t));
                let text = html_to_text(&t);
                if !text.trim().is_empty() {
                    blocks.push(Block::Html(text));
                }
            },
            Event::InlineHtml(t) => {
                pending_anchors.extend(extract_anchor_ids(&t));
                let text = html_to_text(&t);
                if !text.is_empty() {
                    push_span(
                        &mut ctx_stack,
                        &text,
                        bold > 0,
                        italic > 0,
                        false,
                        link_target.clone(),
                    );
                }
            },
            Event::SoftBreak | Event::HardBreak => {
                if let Some(buf) = code_buf.as_mut() {
                    buf.push('\n');
                } else if let Some(ctx) = ctx_stack.last_mut() {
                    let spans = ctx.spans_mut();
                    if let Some(last) = spans.last_mut() {
                        last.text.push('\n');
                    } else {
                        spans.push(Span {
                            text: "\n".into(),
                            bold: false,
                            italic: false,
                            code: false,
                            link_target: None,
                        });
                    }
                }
            },
            Event::Rule => blocks.push(Block::Hr),

            // Ignore everything else (footnotes, task-list markers, etc.).
            _ => {},
        }
    }

    GrammarDoc { blocks, anchors }
}

#[cfg(test)]
fn parse_blocks(src: &str) -> Vec<Block> {
    parse_doc(src).blocks
}

fn normalize_doc_tables(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut lines = src.lines().peekable();
    let mut in_fence = false;
    let mut fence = "";

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let mark = &trimmed[..3];
            if in_fence && mark == fence {
                in_fence = false;
                fence = "";
            } else if !in_fence {
                in_fence = true;
                fence = mark;
            }
            out.push_str(line);
            out.push('\n');
            continue;
        }

        out.push_str(line);
        out.push('\n');

        if !in_fence
            && is_table_row_text(line)
            && lines
                .peek()
                .is_some_and(|next| is_legacy_table_separator(next))
        {
            let _ = lines.next();
            out.push_str(&gfm_table_separator(line));
            out.push('\n');
        }
    }

    out
}

fn is_legacy_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|ch| ch == '-')
}

fn gfm_table_separator(header: &str) -> String {
    let columns = pipe_column_count(header).max(1);
    let mut out = String::new();
    out.push('|');
    for _ in 0..columns {
        out.push_str(" --- |");
    }
    out
}

fn pipe_column_count(row: &str) -> usize {
    let trimmed = row.trim();
    let count = trimmed.matches('|').count();
    if trimmed.starts_with('|') && trimmed.ends_with('|') {
        count.saturating_sub(1)
    } else {
        count + 1
    }
}

/// Decode a span into the nearest enclosing context's span list with the
/// current inline flags, after expanding HTML entities.
fn push_span(
    ctx_stack: &mut [Ctx],
    text: &str,
    bold: bool,
    italic: bool,
    code: bool,
    link_target: Option<String>,
) {
    let Some(ctx) = ctx_stack.last_mut() else {
        return;
    };
    ctx.spans_mut().push(Span {
        text: decode_entities(text),
        bold,
        italic,
        code,
        link_target,
    });
}

/// Expand the HTML entities that appear in the Pikchr docs (`&rarr;`,
/// `&nbsp;`, `&#9654;`, …) to their Unicode characters. Anything that is not a
/// recognized entity is left verbatim.
fn decode_entities(input: &str) -> String {
    const NAMED: &[(&str, &str)] = &[
        ("amp", "&"),
        ("lt", "<"),
        ("gt", ">"),
        ("quot", "\""),
        ("apos", "'"),
        ("nbsp", "\u{00A0}"),
        ("rarr", "\u{2192}"),
        ("larr", "\u{2190}"),
        ("uarr", "\u{2191}"),
        ("darr", "\u{2193}"),
        ("harr", "\u{2194}"),
        ("mdash", "\u{2014}"),
        ("ndash", "\u{2013}"),
        ("hellip", "\u{2026}"),
        ("copy", "\u{00A9}"),
        ("reg", "\u{00AE}"),
        ("trade", "\u{2122}"),
        ("deg", "\u{00B0}"),
        ("times", "\u{00D7}"),
        ("divide", "\u{00F7}"),
        ("plusmn", "\u{00B1}"),
        ("le", "\u{2264}"),
        ("ge", "\u{2265}"),
        ("ne", "\u{2260}"),
        ("asymp", "\u{2248}"),
        ("infin", "\u{221E}"),
        ("alpha", "\u{03B1}"),
        ("beta", "\u{03B2}"),
        ("gamma", "\u{03B3}"),
        ("delta", "\u{03B4}"),
        ("pi", "\u{03C0}"),
        ("sigma", "\u{03C3}"),
        ("tau", "\u{03C4}"),
        ("omega", "\u{03C9}"),
        ("sum", "\u{2211}"),
        ("prod", "\u{220F}"),
    ];
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        if let Some(semi) = after.find(';')
            && semi <= 12
        {
            let body = &after[..semi];
            let matched = if let Some(num) = body.strip_prefix('#') {
                let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
                    u32::from_str_radix(hex, 16)
                } else {
                    num.parse::<u32>()
                };
                match code.ok().and_then(char::from_u32) {
                    Some(c) => {
                        out.push(c);
                        true
                    },
                    None => false,
                }
            } else if let Some((_, v)) = NAMED.iter().find(|(n, _)| *n == body) {
                out.push_str(v);
                true
            } else {
                false
            };
            if matched {
                rest = &after[semi + 1..];
                continue;
            }
        }
        // Not a recognized entity: emit the '&' literally and keep scanning.
        out.push('&');
        rest = after;
    }
    out.push_str(rest);
    out
}

fn html_to_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '<' {
            out.push(ch);
            continue;
        }

        let mut tag = String::new();
        for tag_ch in chars.by_ref() {
            if tag_ch == '>' {
                break;
            }
            tag.push(tag_ch);
        }
        let tag_name = tag
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("");
        match tag_name {
            "a" => {},
            "blockquote" | "table" => {
                if !out.ends_with('\n') && !out.trim().is_empty() {
                    out.push('\n');
                }
            },
            "tr" => {
                if !out.ends_with('\n') && !out.trim().is_empty() {
                    out.push('\n');
                }
            },
            "td" | "th" => {
                let trimmed = out.trim_end();
                if !trimmed.is_empty() && !trimmed.ends_with('|') && !trimmed.ends_with('\n') {
                    out.push_str(" | ");
                }
            },
            _ => {},
        }
    }
    decode_entities(&out)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_anchor_ids(input: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = input;
    while let Some(pos) = rest.find("id=\"") {
        let after = &rest[pos + 4..];
        let Some(end) = after.find('"') else {
            break;
        };
        ids.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    ids
}

fn grammar_doc() -> &'static GrammarDoc {
    static DOC: OnceLock<GrammarDoc> = OnceLock::new();
    DOC.get_or_init(|| parse_doc(PIKCHR_GRAMMAR_MD))
}

fn grammar_blocks() -> &'static [Block] {
    &grammar_doc().blocks
}

fn grammar_link_target(target: &str) -> Option<usize> {
    let anchor = target.strip_prefix('#')?;
    grammar_doc().anchors.get(anchor).copied()
}

fn grammar_toc() -> &'static [TocEntry] {
    static TOC: OnceLock<Vec<TocEntry>> = OnceLock::new();
    TOC.get_or_init(|| {
        grammar_blocks()
            .iter()
            .filter_map(|b| match b {
                Block::Heading { level, idx, spans } => Some(TocEntry {
                    level: *level,
                    idx: *idx,
                    text: toc_text(spans),
                }),
                _ => None,
            })
            .collect()
    })
}

fn toc_text(spans: &[Span]) -> String {
    plain_text(spans)
        .replace("\u{25B6}info", "")
        .trim()
        .to_owned()
}

fn plain_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

fn is_table_row_text(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('|') && trimmed.matches('|').count() >= 2
}

/// Build a wrapped `LayoutJob` from parsed spans: bold uses the SpaceMono-Bold
/// family (true weight), italic tilts glyphs, inline code gets a faint
/// background, links take the accent color + underline.
fn build_job(
    spans: &[Span],
    size: f32,
    base_color: egui::Color32,
    accent: egui::Color32,
    code_bg: egui::Color32,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;
    for s in spans {
        let family = if s.bold { BOLD_FAMILY } else { REG_FAMILY };
        let color = if s.link_target.is_some() {
            accent
        } else {
            base_color
        };
        let format = egui::text::TextFormat {
            font_id: egui::FontId::new(size, egui::FontFamily::Name(family.into())),
            color,
            italics: s.italic,
            background: if s.code {
                code_bg
            } else {
                egui::Color32::TRANSPARENT
            },
            underline: if s.link_target.is_some() {
                egui::Stroke::new(1.0, accent)
            } else {
                egui::Stroke::NONE
            },
            ..Default::default()
        };
        job.append(&s.text, 0.0, format);
    }
    job
}

fn has_links(spans: &[Span]) -> bool {
    spans.iter().any(|s| s.link_target.is_some())
}

fn rich_span(
    span: &Span,
    size: f32,
    base_color: egui::Color32,
    accent: egui::Color32,
) -> egui::RichText {
    let family = if span.bold { BOLD_FAMILY } else { REG_FAMILY };
    let color = if span.link_target.is_some() {
        accent
    } else {
        base_color
    };
    let mut text = egui::RichText::new(&span.text)
        .font(egui::FontId::new(size, egui::FontFamily::Name(family.into())))
        .color(color);
    if span.italic {
        text = text.italics();
    }
    text
}

fn render_linked_spans(
    ui: &mut egui::Ui,
    spans: &[Span],
    size: f32,
    base_color: egui::Color32,
    accent: egui::Color32,
    scroll_target: &mut Option<usize>,
) -> egui::Response {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in spans {
            let rich = rich_span(span, size, base_color, accent);
            if let Some(target) = span.link_target.as_deref() {
                let response = ui.add(egui::Button::new(rich).frame(false));
                if response.clicked()
                    && let Some(idx) = grammar_link_target(target)
                {
                    *scroll_target = Some(idx);
                }
            } else {
                ui.label(rich);
            }
        }
    })
    .response
}

/// Render the Grammar view: a resizable left sidebar TOC and the markdown body.
/// `scroll_target` is read/written by both panels so a TOC click scrolls the
/// body in the same frame.
fn render_grammar(ui: &mut egui::Ui, scroll_target: &mut Option<usize>) {
    fn reg_family() -> egui::FontFamily {
        egui::FontFamily::Name(REG_FAMILY.into())
    }
    let accent = ui.visuals().hyperlink_color;
    let body_color = ui.visuals().text_color();
    let code_bg = ui.visuals().faint_bg_color;
    let dim = ui.visuals().weak_text_color();

    egui::SidePanel::left("grammar_toc")
        .resizable(true)
        .width_range(140.0..=340.0)
        .default_width(210.0)
        .show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Contents")
                    .font(egui::FontId::new(14.0, reg_family()))
                    .color(accent),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in grammar_toc() {
                        // The bundled document appends linked articles after
                        // the grammar. Show the grammar productions and article
                        // titles, but keep article-local subsections in the body.
                        if entry.level > 3 {
                            continue;
                        }
                        let indent = (entry.level as f32 - 1.0) * 8.0;
                        ui.horizontal(|ui| {
                            ui.add_space(indent);
                            let rich = egui::RichText::new(&entry.text)
                                .font(egui::FontId::new(11.0, reg_family()));
                            if ui.add(egui::Button::new(rich).frame(false)).clicked() {
                                *scroll_target = Some(entry.idx);
                            }
                        });
                    }
                });
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let wrap = ui.available_width();
                let blocks = grammar_blocks();
                let mut i = 0;
                while i < blocks.len() {
                    let block = &blocks[i];
                    match block {
                        Block::Heading { level, idx, spans } => {
                            let size = match *level {
                                1 => 18.0,
                                2 => 16.0,
                                3 => 14.0,
                                _ => 12.5,
                            };
                            let resp = if has_links(spans) {
                                render_linked_spans(ui, spans, size, accent, accent, scroll_target)
                            } else {
                                let job = build_job(spans, size, accent, accent, code_bg, wrap);
                                ui.add(egui::Label::new(job).selectable(false))
                            };
                            if *scroll_target == Some(*idx) {
                                resp.scroll_to_me(Some(egui::Align::TOP));
                                *scroll_target = None;
                            }
                        },
                        Block::Para(spans) => {
                            if has_links(spans) {
                                render_linked_spans(
                                    ui,
                                    spans,
                                    12.0,
                                    body_color,
                                    accent,
                                    scroll_target,
                                );
                            } else {
                                let job =
                                    build_job(spans, 12.0, body_color, accent, code_bg, wrap);
                                ui.add(egui::Label::new(job).selectable(false));
                            }
                        },
                        Block::ListItem(spans) => {
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.label(
                                    egui::RichText::new("\u{2022}")
                                        .font(egui::FontId::new(12.0, reg_family()))
                                        .color(body_color),
                                );
                                if has_links(spans) {
                                    render_linked_spans(
                                        ui,
                                        spans,
                                        12.0,
                                        body_color,
                                        accent,
                                        scroll_target,
                                    );
                                } else {
                                    let job = build_job(
                                        spans,
                                        12.0,
                                        body_color,
                                        accent,
                                        code_bg,
                                        ui.available_width(),
                                    );
                                    ui.add(egui::Label::new(job).selectable(false));
                                }
                            });
                        },
                        Block::Code(text) => {
                            ui.label(
                                egui::RichText::new(text.as_str())
                                    .font(egui::FontId::new(11.0, reg_family()))
                                    .color(dim),
                            );
                        },
                        Block::Html(text) => {
                            ui.label(
                                egui::RichText::new(text.as_str())
                                    .font(egui::FontId::new(11.5, reg_family()))
                                    .color(body_color),
                            );
                        },
                        Block::TableRow(_) => {
                            let start = i;
                            let mut end = i + 1;
                            while matches!(blocks.get(end), Some(Block::TableRow(_))) {
                                end += 1;
                            }
                            render_table(
                                ui,
                                &blocks[start..end],
                                body_color,
                                accent,
                                code_bg,
                                reg_family(),
                            );
                            i = end - 1;
                        },
                        Block::Hr => {
                            ui.separator();
                        },
                    }
                    ui.add_space(2.0);
                    i += 1;
                }
            });
    });
}

fn render_table(
    ui: &mut egui::Ui,
    rows: &[Block],
    body_color: egui::Color32,
    accent: egui::Color32,
    code_bg: egui::Color32,
    family: egui::FontFamily,
) {
    let max_cols = rows
        .iter()
        .filter_map(|row| match row {
            Block::TableRow(cells) => Some(cells.len()),
            _ => None,
        })
        .max()
        .unwrap_or(1)
        .max(1);
    let cell_width = ((ui.available_width() - 18.0 * (max_cols.saturating_sub(1) as f32))
        / max_cols as f32)
        .clamp(80.0, 420.0);

    egui::Grid::new(("grammar_table", rows.as_ptr()))
        .striped(true)
        .num_columns(max_cols)
        .spacing([18.0, 4.0])
        .show(ui, |ui| {
            for (row_idx, row) in rows.iter().enumerate() {
                let Block::TableRow(cells) = row else {
                    continue;
                };
                for col in 0..max_cols {
                    if let Some(cell) = cells.get(col) {
                        let color = if row_idx == 0 { accent } else { body_color };
                        let size = if row_idx == 0 { 11.5 } else { 11.0 };
                        let job = build_job(cell, size, color, accent, code_bg, cell_width);
                        ui.add_sized(
                            [cell_width, 0.0],
                            egui::Label::new(job).selectable(false),
                        );
                    } else {
                        ui.label(egui::RichText::new("").font(egui::FontId::new(
                            11.0,
                            family.clone(),
                        )));
                    }
                }
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{
        Block, HelpTopic, PIKCHR_GRAMMAR_MD, Span, decode_entities, grammar_blocks, grammar_toc,
        gfm_table_separator, grammar_link_target, is_table_row_text, parse_blocks, toc_text,
    };

    #[test]
    fn help_topic_defaults_to_overview() {
        assert_eq!(HelpTopic::default(), HelpTopic::Overview);
    }

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
    }

    #[test]
    fn toc_excludes_fenced_comment_lines() {
        let toc = grammar_toc();
        assert!(toc.iter().any(|e| e.text == "Pikchr Grammar"), "missing H1");
        assert!(
            !toc.iter().any(|e| e.text == "Start and end blocks"),
            "fenced pikchr comment leaked into TOC"
        );
    }

    #[test]
    fn toc_keeps_grammar_productions_and_reference_titles_visible() {
        let visible: Vec<_> = grammar_toc()
            .iter()
            .filter(|e| e.level <= 3)
            .map(|e| e.text.as_str())
            .collect();
        assert!(
            visible.iter().any(|e| e.starts_with("statement-list")),
            "main grammar production is missing from visible TOC"
        );
        assert!(
            visible.iter().any(|e| e.starts_with("dot-property")),
            "late grammar production is missing from visible TOC"
        );
        assert!(
            visible.contains(&"Linked reference articles"),
            "linked-doc appendix is missing from visible TOC"
        );
        assert!(
            visible.contains(&"statement-list"),
            "linked article title is missing from visible TOC"
        );
        assert!(
            !visible.contains(&"Rules"),
            "article-local subsection leaked into visible TOC"
        );
        assert!(
            !visible.iter().any(|e| e.starts_with('|')),
            "table row leaked into visible TOC: {visible:?}"
        );
    }

    #[test]
    fn toc_text_drops_info_link_markers() {
        let spans = [Span {
            text: "*statement-list*: \u{25B6}info".into(),
            bold: false,
            italic: true,
            code: false,
            link_target: Some("#reference-stmtlist.md".into()),
        }];
        assert_eq!(toc_text(&spans), "*statement-list*:");
    }

    #[test]
    fn info_links_resolve_to_reference_headings() {
        let target = grammar_link_target("#reference-stmtlist.md").expect("stmtlist anchor");
        let heading = grammar_toc()
            .iter()
            .find(|entry| entry.idx == target)
            .expect("target heading");
        assert_eq!(heading.text, "statement-list");
    }

    #[test]
    fn pipe_table_rows_are_not_headings() {
        let blocks = parse_blocks("| Variable Name |: Purpose |\n------------------------------\n");
        assert!(
            blocks
                .iter()
                .all(|block| !matches!(block, Block::Heading { .. })),
            "setext-style table row was parsed as a heading: {blocks:?}"
        );
        assert!(
            blocks.iter().any(|block| matches!(block, Block::TableRow(_))),
            "legacy table was not normalized into table rows: {blocks:?}"
        );
        assert!(is_table_row_text("| Legacy ASCII | HTML Entity | Unicode |"));
        assert_eq!(
            gfm_table_separator("| Legacy ASCII | HTML Entity | Unicode |"),
            "| --- | --- | --- |"
        );
    }

    #[test]
    fn cached_blocks_match_fresh_parse() {
        assert_eq!(grammar_blocks(), parse_blocks(PIKCHR_GRAMMAR_MD));
    }

    /// Inline `**bold**`/`*italic*`/`` `code` `` must be parsed into spans, so
    /// the literal markers never reach the screen.
    #[test]
    fn inline_markup_is_parsed_into_spans() {
        let blocks = parse_blocks("a **b** c `d` e");
        let para = blocks
            .iter()
            .find_map(|b| match b {
                Block::Para(s) => Some(s),
                _ => None,
            })
            .expect("a paragraph");
        assert!(
            para.iter().any(|s| s.bold && s.text == "b"),
            "missing bold 'b'"
        );
        assert!(
            para.iter().any(|s| s.code && s.text == "d"),
            "missing code 'd'"
        );
        assert!(
            para.iter()
                .all(|s| !s.text.contains("**") && !s.text.contains('`')),
            "literal markup leaked into spans: {para:?}"
        );
    }

    #[test]
    fn html_entities_are_decoded() {
        assert_eq!(decode_entities("a &rarr; b"), "a \u{2192} b");
        assert_eq!(decode_entities("&#9654;"), "\u{25B6}");
        assert_eq!(decode_entities("&#x2192;"), "\u{2192}");
        assert_eq!(decode_entities("a & b"), "a & b");
        assert_eq!(decode_entities("&unknown;"), "&unknown;");
    }
}
