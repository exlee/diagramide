//! Parinfer for Lisp-shaped editors: closing parens follow indentation.
//!
//! Wraps `parinfer_rust`. Parinfer addresses text by line and display column;
//! egui addresses it by char index. The conversions here bridge the two.

use parinfer_rust::{
    parinfer,
    types::{Options, Request},
};
use unicode_width::UnicodeWidthChar as _;

/// Text and cursor (char index) after parinfer rewrote an edit.
#[derive(Debug, PartialEq, Eq)]
pub struct Edit {
    pub text: String,
    pub cursor: usize,
}

/// Runs smart mode on an edit that turned `prev` into `text`.
///
/// `cursor` and `prev_cursor` are char indices. `anchor` is the other end of
/// a selection; a non-empty selection turns off smart dragging, as in other
/// parinfer editors. Returns `None` when parinfer changes nothing or refuses
/// (for example, inside an unclosed string).
pub fn smart(
    prev: &str,
    prev_cursor: Option<usize>,
    text: &str,
    cursor: usize,
    anchor: usize,
) -> Option<Edit> {
    let (cursor_line, cursor_x) = line_and_column(text, cursor);
    let prev_cursor = prev_cursor.map(|index| line_and_column(prev, index));
    let selection_start_line =
        (anchor != cursor).then(|| line_and_column(text, anchor.min(cursor)).0);
    let request = Request {
        mode: "smart".into(),
        text: text.into(),
        options: Options {
            cursor_x: Some(cursor_x),
            cursor_line: Some(cursor_line),
            prev_cursor_x: prev_cursor.map(|(_, x)| x),
            prev_cursor_line: prev_cursor.map(|(line, _)| line),
            prev_text: Some(prev.into()),
            selection_start_line,
            ..options()
        },
    };
    let answer = parinfer::process(&request);
    if !answer.success || answer.text == text {
        return None;
    }
    let cursor = match (answer.cursor_line, answer.cursor_x) {
        (Some(line), Some(x)) => char_index(&answer.text, line, x),
        _ => cursor.min(answer.text.chars().count()),
    };
    Some(Edit {
        text: answer.text.into_owned(),
        cursor,
    })
}

/// Runs paren mode, which fixes indentation to match the parens.
///
/// Used on text that arrives from outside the editor, so smart mode later
/// infers the structure the parens already state. Returns `None` when the
/// text is already consistent or cannot be balanced.
pub fn paren(text: &str) -> Option<String> {
    let answer = parinfer::paren_mode(text, &options());
    (answer.success && answer.text != text).then(|| answer.text.into_owned())
}

/// CLIPS syntax: `;` comments, `"` strings, and `|` as the constraint
/// connective `or`, so `|` never quotes a symbol.
fn options() -> Options {
    Options {
        cursor_x: None,
        cursor_line: None,
        prev_cursor_x: None,
        prev_cursor_line: None,
        prev_text: None,
        selection_start_line: None,
        changes: vec![],
        comment_char: ';',
        string_delimiters: vec!["\"".into()],
        lisp_vline_symbols: false,
        lisp_block_comments: false,
        guile_block_comments: false,
        scheme_sexp_comments: false,
        janet_long_strings: false,
        hy_bracket_strings: false,
    }
}

fn line_and_column(text: &str, index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut x = 0;
    for c in text.chars().take(index) {
        if c == '\n' {
            line += 1;
            x = 0;
        } else {
            x += c.width().unwrap_or(0);
        }
    }
    (line, x)
}

fn char_index(text: &str, line: usize, x: usize) -> usize {
    let mut index = 0;
    let mut chars = text.chars().peekable();
    for _ in 0..line {
        for c in chars.by_ref() {
            index += 1;
            if c == '\n' {
                break;
            }
        }
    }
    let mut column = 0;
    while column < x
        && let Some(&c) = chars.peek()
        && c != '\n'
    {
        column += c.width().unwrap_or(0);
        index += 1;
        chars.next();
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inserts `typed` at `at` in `prev` and runs smart mode with the cursor
    /// after the insertion.
    fn type_at(prev: &str, at: usize, typed: &str) -> Option<Edit> {
        let mut text: Vec<char> = prev.chars().collect();
        text.splice(at..at, typed.chars());
        let text: String = text.into_iter().collect();
        let cursor = at + typed.chars().count();
        smart(prev, Some(at), &text, cursor, cursor)
    }

    #[test]
    fn open_paren_gets_closed() {
        let edit = type_at("", 0, "(").unwrap();
        assert_eq!(edit.text, "()");
        assert_eq!(edit.cursor, 1);
    }

    #[test]
    fn indented_line_joins_the_form_above() {
        let prev = "(box (id b))\n";
        let edit = type_at(prev, prev.len(), "  (label \"x\")").unwrap();
        assert_eq!(edit.text, "(box (id b)\n  (label \"x\"))");
    }

    #[test]
    fn dedented_line_leaves_the_form() {
        let prev = "(box (id b)\n  (label \"x\"))";
        let text = "(box (id b)\n(label \"x\"))";
        let edit = smart(prev, Some(14), text, 12, 12).unwrap();
        assert_eq!(edit.text, "(box (id b))\n(label \"x\")");
    }

    #[test]
    fn untouched_balanced_text_is_left_alone() {
        let prev = "(box (id b))";
        let text = "(box (id c))";
        assert_eq!(smart(prev, Some(9), text, 10, 10), None);
    }

    #[test]
    fn unclosed_string_is_left_alone() {
        assert_eq!(type_at("(box (label x))", 12, "\""), None);
    }

    #[test]
    fn comments_do_not_count_parens() {
        assert_eq!(type_at("; note\n(box)", 6, " ("), None);
    }

    #[test]
    fn vertical_bar_is_a_constraint_not_a_symbol_quote() {
        let edit = type_at("(a)\n", 4, "  red|blue").unwrap();
        assert_eq!(edit.text, "(a\n  red|blue)");
    }

    #[test]
    fn paren_mode_indents_children_inside_their_form() {
        let fixed = paren("(defrule r\n(a)\n=>\n(b))").unwrap();
        assert_eq!(fixed, "(defrule r\n (a)\n =>\n (b))");
    }

    #[test]
    fn paren_mode_keeps_consistent_text() {
        assert_eq!(paren("(defrule r\n  (a)\n  =>\n  (b))"), None);
    }

    #[test]
    fn columns_count_display_width() {
        let text = "(a \"日本\")\n(b)";
        for index in 0..text.chars().count() {
            let (line, x) = line_and_column(text, index);
            assert_eq!(char_index(text, line, x), index, "index {index}");
        }
        assert_eq!(line_and_column(text, 6), (0, 8));
    }
}
