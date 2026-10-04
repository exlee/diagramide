//! Completion for the Hagoromo editor: matches the identifier before the
//! cursor against the Hagoromo catalog, script-local `let` bindings and Gluon
//! keywords.

use crate::hagoromo::Completion;

const KEYWORDS: &[&str] = &[
    "else", "if", "in", "let", "match", "rec", "then", "type", "with",
];

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

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Candidates for the identifier ending at char index `cursor`, or `None`
/// when the cursor is not at the end of an identifier.
pub fn suggest(text: &str, cursor: usize, catalog: &[Completion]) -> Option<Suggestion> {
    let cursor_byte = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(i, _)| i);
    let (before, after) = text.split_at(cursor_byte);
    if after.starts_with(is_ident_char) {
        return None;
    }
    let line = &before[before.rfind('\n').map_or(0, |i| i + 1)..];
    if line.contains("//") || line.matches('"').count() % 2 == 1 {
        return None;
    }

    let prefix_start = before
        .rfind(|c: char| !is_ident_char(c) && c != '.')
        .map_or(0, |i| {
            i + before[i..].chars().next().map_or(1, char::len_utf8)
        });
    let prefix = &before[prefix_start..];
    if prefix.is_empty() || !prefix.starts_with(|c: char| c.is_alphabetic() || c == '_') {
        return None;
    }

    let depth = prefix.matches('.').count();
    let locals = local_names(text, prefix_start);
    let mut candidates: Vec<Candidate> = catalog
        .iter()
        .map(|c| Candidate {
            name: c.name.clone(),
            detail: c.signature.clone(),
        })
        .chain(locals.into_iter().map(|name| Candidate {
            name,
            detail: "local".to_string(),
        }))
        .chain(KEYWORDS.iter().map(|k| Candidate {
            name: k.to_string(),
            detail: "keyword".to_string(),
        }))
        .filter(|c| c.name.starts_with(prefix) && c.name.matches('.').count() == depth)
        .collect();
    candidates.sort_by(|a, b| (a.name != prefix, &a.name).cmp(&(b.name != prefix, &b.name)));
    candidates.dedup_by(|a, b| a.name == b.name);
    if candidates.is_empty() {
        return None;
    }

    Some(Suggestion {
        start: text[..prefix_start].chars().count(),
        prefix: prefix.to_string(),
        candidates,
    })
}

/// Names bound by `let` anywhere in `text`, skipping the identifier being
/// typed at `typing_at` so it does not complete to itself.
fn local_names(text: &str, typing_at: usize) -> Vec<String> {
    let mut names = Vec::new();
    let mut tokens = tokenize(text).peekable();
    while let Some((_, token)) = tokens.next() {
        if token != "let" {
            continue;
        }
        if tokens.peek().is_some_and(|(_, t)| *t == "rec") {
            tokens.next();
        }
        match tokens.next() {
            Some((_, "{")) => {
                for (offset, token) in tokens.by_ref() {
                    if token == "}" {
                        break;
                    }
                    if offset != typing_at
                        && token.starts_with(|c: char| c.is_alphabetic() || c == '_')
                    {
                        names.push(token.to_string());
                    }
                }
            },
            Some((offset, token))
                if offset != typing_at
                    && token.starts_with(|c: char| c.is_alphabetic() || c == '_') =>
            {
                names.push(token.to_string());
            },
            _ => {},
        }
    }
    names
}

/// Identifiers and single punctuation characters with their byte offsets.
fn tokenize(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut rest = text.char_indices().peekable();
    std::iter::from_fn(move || {
        while let Some(&(_, c)) = rest.peek() {
            if !c.is_whitespace() {
                break;
            }
            rest.next();
        }
        let (start, c) = rest.next()?;
        let mut end = start + c.len_utf8();
        if is_ident_char(c) {
            while let Some(&(i, c)) = rest.peek() {
                if !is_ident_char(c) {
                    break;
                }
                end = i + c.len_utf8();
                rest.next();
            }
        }
        Some((start, &text[start..end]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Vec<Completion> {
        [
            "circle",
            "color",
            "color.red",
            "color.rgb",
            "rect",
            "rect_new",
            "reflect_x",
            "prim.circle",
        ]
        .into_iter()
        .map(|name| Completion {
            name: name.to_string(),
            signature: format!("sig of {name}"),
        })
        .collect()
    }

    fn at_end(text: &str) -> Option<Suggestion> {
        suggest(text, text.chars().count(), &catalog())
    }

    fn names(suggestion: &Suggestion) -> Vec<&str> {
        suggestion
            .candidates
            .iter()
            .map(|c| c.name.as_str())
            .collect()
    }

    #[test]
    fn matches_prefix_at_the_same_depth() {
        let s = at_end("node (ci").unwrap();
        assert_eq!(s.prefix, "ci");
        assert_eq!(names(&s), ["circle"]);
        assert_eq!(s.start, 6);

        let s = at_end("co").unwrap();
        assert_eq!(
            names(&s),
            ["color"],
            "dotted members stay hidden until a dot is typed"
        );
    }

    #[test]
    fn dotted_prefix_matches_record_members() {
        let s = at_end("fc color.r").unwrap();
        assert_eq!(s.prefix, "color.r");
        assert_eq!(names(&s), ["color.red", "color.rgb"]);
        assert_eq!(s.start, 3);
        assert_eq!(names(&at_end("prim.c").unwrap()), ["prim.circle"]);
    }

    #[test]
    fn replacement_extends_to_the_shared_prefix() {
        assert_eq!(
            at_end("ci").unwrap().replacement().as_deref(),
            Some("circle")
        );
        assert_eq!(
            at_end("color.r").unwrap().replacement(),
            None,
            "red and rgb share only the typed prefix"
        );
        assert_eq!(
            at_end("rec").unwrap().replacement().as_deref(),
            None,
            "rec is a keyword and a prefix of rect"
        );
        assert_eq!(
            at_end("rect_").unwrap().replacement().as_deref(),
            Some("rect_new")
        );
    }

    #[test]
    fn exact_match_comes_first() {
        let s = at_end("rect").unwrap();
        assert_eq!(names(&s), ["rect", "rect_new"]);
        assert_eq!(s.replacement(), None);
    }

    #[test]
    fn includes_let_bindings_and_keywords() {
        let text = "let { circle } = prim\nlet node label = circle 1.0\nlet rec loop x = x\nno";
        let s = at_end(text).unwrap();
        assert_eq!(names(&s), ["node"]);
        assert_eq!(s.candidates[0].detail, "local");
        assert_eq!(names(&at_end(&format!("{text}\nlo")).unwrap()), ["loop"]);
        assert_eq!(names(&at_end("th").unwrap()), ["then"]);
    }

    #[test]
    fn identifier_being_typed_is_not_its_own_candidate() {
        assert!(at_end("let nod").is_none());
    }

    #[test]
    fn no_suggestion_mid_word_in_numbers_strings_or_comments() {
        assert!(suggest("circle", 2, &catalog()).is_none());
        assert!(at_end("1.0").is_none());
        assert!(at_end("text \"ci").is_none());
        assert!(at_end("// ci").is_none());
        assert!(at_end("").is_none());
        assert!(at_end("zzz").is_none());
    }

    #[test]
    fn apply_replaces_prefix_and_keeps_following_text() {
        let text = "żółw |> fc color.r\nnext";
        let cursor = "żółw |> fc color.r".chars().count();
        let s = suggest(text, cursor, &catalog()).unwrap();
        let (result, cursor) = apply(text, &s, "color.rgb");
        assert_eq!(result, "żółw |> fc color.rgb\nnext");
        assert_eq!(result.chars().nth(cursor), Some('\n'));
    }

    #[test]
    fn cursor_is_measured_in_chars() {
        let s = at_end("text \"żółw\" ci").unwrap();
        assert_eq!(s.start, 12);
        assert_eq!(names(&s), ["circle"]);
    }
}
