//! Editor completion shared by the Hagoromo and CLIPS editors: the suggestion
//! a language module computes, how Tab applies it, and the footer that lists
//! the candidates.

use eframe::egui::{self, Context, Ui};

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    /// Char index where the typed prefix starts.
    pub start: usize,
    pub prefix: String,
    /// Exact match first, then alphabetical.
    pub candidates: Vec<Candidate>,
}

impl Suggestion {
    /// Keeps the candidates starting with `prefix`, exact match first, then
    /// alphabetical. Of two candidates with one name, the earlier one stays.
    /// `None` when nothing matches.
    pub fn new(start: usize, prefix: &str, candidates: impl IntoIterator<Item = Candidate>) -> Option<Self> {
        let mut candidates: Vec<Candidate> = candidates
            .into_iter()
            .filter(|c| c.name.starts_with(prefix))
            .collect();
        candidates.sort_by(|a, b| (a.name != prefix, &a.name).cmp(&(b.name != prefix, &b.name)));
        candidates.dedup_by(|a, b| a.name == b.name);
        (!candidates.is_empty()).then(|| Suggestion {
            start,
            prefix: prefix.to_string(),
            candidates,
        })
    }

    /// Text that replaces the prefix when completion is accepted: the longest
    /// prefix shared by every candidate. `None` when that adds nothing.
    pub fn replacement(&self) -> Option<String> {
        let mut names = self.candidates.iter().map(|c| c.name.as_str());
        let first = names.next()?;
        let common = names.fold(first, |common, name| {
            let shared = common
                .char_indices()
                .zip(name.chars())
                .take_while(|((_, a), b)| a == b)
                .last()
                .map_or(0, |((i, a), _)| i + a.len_utf8());
            &common[..shared]
        });
        (common.len() > self.prefix.len()).then(|| common.to_string())
    }
}

/// Replaces the suggestion's prefix in `text` with `replacement`. Returns the
/// new text and the char index just after the replacement.
pub fn apply(text: &str, suggestion: &Suggestion, replacement: &str) -> (String, usize) {
    let start = text
        .char_indices()
        .nth(suggestion.start)
        .map_or(text.len(), |(i, _)| i);
    let end = start + suggestion.prefix.len();
    let mut result = String::with_capacity(text.len() + replacement.len());
    result.push_str(&text[..start]);
    result.push_str(replacement);
    result.push_str(&text[end..]);
    (result, suggestion.start + replacement.chars().count())
}

/// Char index of the editor's cursor, or `None` when text is selected.
pub fn cursor(ctx: &Context, editor_id: egui::Id) -> Option<usize> {
    let range = egui::TextEdit::load_state(ctx, editor_id)?
        .cursor
        .char_range()?;
    (range.primary.index == range.secondary.index).then_some(range.primary.index)
}

/// Writes the completion into `content` and moves the cursor after it.
pub fn accept(
    ctx: &Context,
    editor_id: egui::Id,
    content: &mut String,
    suggestion: &Suggestion,
    replacement: &str,
) {
    let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) else {
        return;
    };
    let (text, cursor) = apply(content, suggestion, replacement);
    *content = text;
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(cursor))));
    state.store(ctx, editor_id);
}

/// The suggestion to accept and its replacement, when plain Tab was pressed
/// in the focused editor and the suggestion extends the prefix. Consumes the
/// key press in that case.
pub fn take_tab(
    ui: &mut Ui,
    editor_id: egui::Id,
    suggest: impl FnOnce() -> Option<Suggestion>,
) -> Option<(Suggestion, String)> {
    let plain_tab = ui.input(|i| i.key_pressed(egui::Key::Tab) && i.modifiers.is_none());
    if !plain_tab || !ui.memory(|mem| mem.has_focus(editor_id)) {
        return None;
    }
    let suggestion = suggest()?;
    let replacement = suggestion.replacement()?;
    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));
    Some((suggestion, replacement))
}

/// One line: the first candidate with its detail, then the other names.
pub fn show_footer(ui: &mut Ui, suggestion: Option<&Suggestion>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str) -> Candidate {
        Candidate {
            name: name.to_string(),
            detail: format!("detail of {name}"),
        }
    }

    #[test]
    fn new_filters_sorts_and_keeps_the_first_duplicate() {
        let first = Candidate {
            name: "rect".to_string(),
            detail: "first".to_string(),
        };
        let s = Suggestion::new(
            0,
            "rect",
            [candidate("rect_new"), first.clone(), candidate("rect"), candidate("circle")],
        )
        .unwrap();
        assert_eq!(s.candidates, [first, candidate("rect_new")]);
        assert!(Suggestion::new(0, "zz", [candidate("rect")]).is_none());
    }
}
