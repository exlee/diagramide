use std::sync::{Arc, OnceLock};
use std::{hash::Hash, hash::Hasher};

use eframe::egui;
use syntect::highlighting::{ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSet};

pub struct SyntectConfig {
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

static CONFIG: OnceLock<SyntectConfig> = OnceLock::new();

macro_rules! load_syntax {
    ($builder:ident, $file:literal) => {
        match SyntaxDefinition::load_from_str(include_str!($file), true, None) {
            Ok(syntax_definition) => $builder.add(syntax_definition),
            Err(err) => eprintln!("Error: {:?}", err),
        }
    };
}

pub fn get_config() -> &'static SyntectConfig {
    CONFIG.get_or_init(|| {
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        // Load a .sublime-syntax file for Prolog
        //builder.add_from_folder("assets/syntaxes", true).unwrap();
        load_syntax!(builder, "../assets/syntaxes/pikchr.sublime-syntax");
        load_syntax!(builder, "../assets/syntaxes/prolog.sublime-syntax");
        let syntax_set = builder.build();
        let theme_set = ThemeSet::load_defaults();
        SyntectConfig {
            syntax_set,
            theme_set,
        }
    })
}
pub fn syntax_layouter(
    ui: &egui::Ui,
    text: &dyn egui::TextBuffer,
    wrap_width: f32,
    syntax: &str,
) -> Arc<egui::Galley> {
    let config = get_config();
    let syntax_set = &config.syntax_set;
    let theme = &config.theme_set.themes.get("base16-eighties.dark").unwrap();
    let mut job = egui::text::LayoutJob::default();
    let syntax = syntax_set
        .find_syntax_by_name(syntax)
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

    let mut h = syntect::easy::HighlightLines::new(syntax, theme);

    for line in syntect::util::LinesWithEndings::from(text.as_str()) {
        let ranges: Vec<(syntect::highlighting::Style, &str)> =
            h.highlight_line(line, syntax_set).unwrap();
        for (style, text) in ranges {
            let color =
                egui::Color32::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            job.append(
                text,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(14.0),
                    color,
                    ..Default::default()
                },
            );
        }
    }
    job.wrap.max_width = wrap_width;

    ui.fonts_mut(|f| f.layout_job(job))
}
pub fn memoized_syntax_layouter(
    _editor_id: egui::Id,
    ui: &egui::Ui,
    textbuffer: &dyn egui::TextBuffer,
    wrap_width: f32,
    syntax: &str,
) -> Arc<egui::Galley> {
    syntax_layouter(ui, textbuffer, wrap_width, syntax)
}
pub fn memoized_syntax_layouter_old(
    editor_id: egui::Id,
    ui: &egui::Ui,
    textbuffer: &dyn egui::TextBuffer,
    wrap_width: f32,
    syntax: &str,
) -> Arc<egui::Galley> {
    let mut hash = None;
    let hashing_fn = || {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        textbuffer.as_str().hash(&mut hasher);
        wrap_width.to_bits().hash(&mut hasher);
        hasher.finish()
    };

    let textbuffer_len = textbuffer.as_str().len();
    let entry_id = editor_id.with("syntax_highlighter_cache");
    if let Some(cache) = ui.memory(|mem| {
        mem.data
            .get_temp::<(u64, usize, Arc<egui::Galley>)>(entry_id)
    }) && cache.1 == textbuffer_len
        && cache.0 == *hash.get_or_insert_with(hashing_fn)
    {
        return cache.2;
    }

    let galley = syntax_layouter(ui, textbuffer, wrap_width, syntax);

    let hash = hash.get_or_insert_with(hashing_fn);
    ui.ctx().memory_mut(|mem| {
        mem.data.insert_temp::<(u64, usize, Arc<egui::Galley>)>(
            entry_id,
            (*hash, textbuffer_len, galley.clone()),
        );
    });

    galley
}
