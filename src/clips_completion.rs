//! Completion for the CLIPS editor. What is offered depends on where the
//! symbol before the cursor stands:
//!
//! - right after `(`: functions, deftemplates and constructs; inside a fact or
//!   pattern of a known deftemplate, that template's slots;
//! - as an argument: deffacts, deftemplate, defrule or deffunction names for
//!   the commands that take them, direction words for `dir`, shape ids for
//!   `from`, `to`, `at`, `with` and `same`, and a few constants elsewhere;
//! - `?name`: the variables of the enclosing top-level form and defglobals.
//!
//! Definitions come from the diagram prelude and the editor text, so a
//! deftemplate completes its slots as soon as it is typed.

use std::{collections::BTreeMap, sync::OnceLock};

use crate::completion::{Candidate, Suggestion};

/// Constructs, offered at the top level.
const CONSTRUCTS: &[&str] = &[
    "defclass",
    "deffacts",
    "deffunction",
    "defgeneric",
    "defglobal",
    "definstances",
    "defmessage-handler",
    "defmethod",
    "defmodule",
    "defrule",
    "deftemplate",
    "include",
];

/// Conditional elements, offered for the patterns of a defrule.
const CONDITIONAL_ELEMENTS: &[&str] = &[
    "and", "declare", "exists", "forall", "logical", "not", "or", "test",
];

/// Slot attributes, offered inside `(slot name ...)`.
const SLOT_ATTRIBUTES: &[&str] = &[
    "allowed-floats",
    "allowed-instance-names",
    "allowed-integers",
    "allowed-lexemes",
    "allowed-numbers",
    "allowed-strings",
    "allowed-symbols",
    "allowed-values",
    "cardinality",
    "default",
    "default-dynamic",
    "range",
    "type",
];

const CONSTANTS: &[&str] = &["FALSE", "TRUE", "crlf", "nil"];

const DIRECTIONS: &[&str] = &["down", "left", "right", "up"];

/// Slots whose value names another shape.
const SHAPE_SLOTS: &[&str] = &["at", "from", "same", "to", "with"];

/// Ordered facts the translator understands, with their arguments.
const ORDERED_FACTS: &[(&str, &str)] = &[
    ("lines-from", "editor"),
    ("pikchr-from", "editor"),
    ("source-line", "editor n line"),
    ("source-text", "editor text"),
    ("text-from", "editor"),
];

/// Commands whose arguments name a construct of one kind.
const NAME_ARGUMENTS: &[(&str, Kind)] = &[
    ("ppdeffacts", Kind::Deffacts),
    ("undeffacts", Kind::Deffacts),
    ("deftemplate-slot-names", Kind::Deftemplate),
    ("ppdeftemplate", Kind::Deftemplate),
    ("undeftemplate", Kind::Deftemplate),
    ("matches", Kind::Defrule),
    ("ppdefrule", Kind::Defrule),
    ("refresh", Kind::Defrule),
    ("undefrule", Kind::Defrule),
    ("ppdeffunction", Kind::Deffunction),
    ("undeffunction", Kind::Deffunction),
];

/// Arguments of the built-in functions a diagram is most likely to use.
/// Every other built-in completes with a bare "function".
const SIGNATURES: &[(&str, &str)] = &[
    ("*", "<number> <number>+"),
    ("+", "<number> <number>+"),
    ("-", "<number> <number>+"),
    ("/", "<number> <number>+"),
    ("<", "<number> <number>+"),
    ("<=", "<number> <number>+"),
    ("<>", "<number> <number>+"),
    ("=", "<number> <number>+"),
    (">", "<number> <number>+"),
    (">=", "<number> <number>+"),
    ("abs", "<number>"),
    ("and", "<expression>+"),
    ("assert", "<fact>+"),
    ("bind", "<variable> <expression>*"),
    ("break", ""),
    ("create$", "<expression>*"),
    ("delete$", "<multifield> <begin> <end>"),
    ("div", "<number> <number>+"),
    ("do-for-all-facts", "(<fact-set>) <query> <action>*"),
    ("do-for-fact", "(<fact-set>) <query> <action>*"),
    ("duplicate", "<fact> <slot-override>*"),
    ("eq", "<expression> <expression>+"),
    ("eval", "<string>"),
    ("explode$", "<string>"),
    ("fact-slot-value", "<fact> <slot>"),
    ("facts", "[<module>] [<start> [<end> [<max>]]]"),
    ("find-all-facts", "(<fact-set>) <query>"),
    ("find-fact", "(<fact-set>) <query>"),
    ("first$", "<multifield>"),
    ("float", "<number>"),
    ("format", "<logical-name> <format> <expression>*"),
    ("gensym*", ""),
    ("if", "<condition> then <action>* [else <action>*]"),
    ("implode$", "<multifield>"),
    ("insert$", "<multifield> <index> <expression>+"),
    ("integer", "<number>"),
    ("length$", "<multifield>"),
    ("loop-for-count", "<range> [do] <action>*"),
    ("lowcase", "<lexeme>"),
    ("max", "<number>+"),
    ("member$", "<expression> <multifield>"),
    ("min", "<number>+"),
    ("mod", "<number> <number>"),
    ("modify", "<fact> <slot-override>*"),
    ("neq", "<expression> <expression>+"),
    ("not", "<expression>"),
    ("nth$", "<index> <multifield>"),
    ("or", "<expression>+"),
    ("printout", "<logical-name> <expression>*"),
    ("progn$", "(<variable> <multifield>) <action>*"),
    ("rest$", "<multifield>"),
    ("retract", "<fact>+"),
    ("return", "[<expression>]"),
    ("round", "<number>"),
    ("str-cat", "<expression>*"),
    ("str-index", "<lexeme> <lexeme>"),
    ("str-length", "<lexeme>"),
    ("sub-string", "<begin> <end> <string>"),
    ("subseq$", "<multifield> <begin> <end>"),
    ("switch", "<expression> (case <value> then <action>*)+ [(default <action>*)]"),
    ("sym-cat", "<expression>*"),
    ("upcase", "<lexeme>"),
    ("while", "<condition> [do] <action>*"),
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Deffacts,
    Deftemplate,
    Defrule,
    Deffunction,
}

/// Names of the built-in functions of a fresh CLIPS environment, built once.
fn builtin_functions() -> &'static [String] {
    static BUILTINS: OnceLock<Vec<String>> = OnceLock::new();
    BUILTINS.get_or_init(|| {
        let Ok(mut env) = clips_bindings::Environment::new() else {
            return Vec::new();
        };
        let Ok(list) = env.eval("(get-function-list)") else {
            return Vec::new();
        };
        let mut names: Vec<String> = list
            .as_multifield()
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| name.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    })
}

fn builtin_candidates() -> impl Iterator<Item = Candidate> {
    let signatures: BTreeMap<&str, &str> = SIGNATURES.iter().copied().collect();
    builtin_functions().iter().map(move |name| Candidate {
        name: name.clone(),
        detail: match signatures.get(name.as_str()) {
            Some(args) if !args.is_empty() => format!("function {args}"),
            _ => "function".to_string(),
        },
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Token<'a> {
    Open,
    Close,
    Atom(&'a str),
    Str,
}

/// Delimiters of a CLIPS symbol. `&`, `|` and `~` join pattern constraints
/// (`?y&~nil`), so they end a symbol too.
fn is_symbol_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '(' | ')' | '"' | ';' | '&' | '|' | '~')
}

/// Tokens with their byte offsets. The flag is true when `text` ends inside
/// a string or a comment.
fn tokenize(text: &str) -> (Vec<(usize, Token<'_>)>, bool) {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        match c {
            '(' => tokens.push((start, Token::Open)),
            ')' => tokens.push((start, Token::Close)),
            ';' => {
                if !chars.any(|(_, c)| c == '\n') {
                    return (tokens, true);
                }
            },
            '"' => {
                let mut escaped = false;
                let closed = chars.any(|(_, c)| {
                    let end = !escaped && c == '"';
                    escaped = !escaped && c == '\\';
                    end
                });
                if !closed {
                    return (tokens, true);
                }
                tokens.push((start, Token::Str));
            },
            c if is_symbol_char(c) => {
                let mut end = start + c.len_utf8();
                while let Some(&(i, c)) = chars.peek() {
                    if !is_symbol_char(c) {
                        break;
                    }
                    end = i + c.len_utf8();
                    chars.next();
                }
                tokens.push((start, Token::Atom(&text[start..end])));
            },
            _ => {},
        }
    }
    (tokens, false)
}

/// Everything the editor text and the prelude define.
#[derive(Debug, Default)]
struct Definitions {
    /// Deftemplate name → slot names, in declaration order.
    templates: BTreeMap<String, Vec<String>>,
    /// Deffunction name → parameters.
    functions: BTreeMap<String, Vec<String>>,
    deffacts: Vec<String>,
    rules: Vec<String>,
    globals: Vec<String>,
    /// Values of every `(id ...)` slot.
    ids: Vec<String>,
}

impl Definitions {
    /// Scans `text`, skipping the token at byte `typing_at` so the symbol being
    /// typed does not complete to itself.
    fn scan(text: &str, typing_at: usize, into: &mut Definitions) {
        let (tokens, _) = tokenize(text);
        let atom = |i: usize| match tokens.get(i) {
            Some(&(offset, Token::Atom(name))) if offset != typing_at => Some(name),
            _ => None,
        };
        for i in 0..tokens.len() {
            if tokens[i].1 != Token::Open {
                continue;
            }
            match (atom(i + 1), atom(i + 2)) {
                (Some("deftemplate"), Some(name)) => {
                    let slots = into.templates.entry(name.to_string()).or_default();
                    for (j, _) in form_children(&tokens, i) {
                        if let (Some("slot" | "multislot" | "field" | "multifield"), Some(slot)) =
                            (atom(j + 1), atom(j + 2))
                        {
                            slots.push(slot.to_string());
                        }
                    }
                },
                (Some("deffunction"), Some(name)) => {
                    let params = form_children(&tokens, i)
                        .next()
                        .map(|(j, end)| (j + 1..end).filter_map(&atom).map(str::to_string).collect())
                        .unwrap_or_default();
                    into.functions.insert(name.to_string(), params);
                },
                (Some("deffacts"), Some(name)) => into.deffacts.push(name.to_string()),
                (Some("defrule"), Some(name)) => into.rules.push(name.to_string()),
                (Some("defglobal"), _) => into.globals.extend(
                    (i + 2..form_end(&tokens, i))
                        .filter_map(&atom)
                        .filter(|name| name.starts_with("?*"))
                        .map(str::to_string),
                ),
                (Some("id"), Some(id)) if !id.starts_with(['?', '$']) => {
                    into.ids.push(id.to_string());
                },
                _ => {},
            }
        }
    }

    fn names(&self, kind: Kind) -> Vec<String> {
        match kind {
            Kind::Deffacts => self.deffacts.clone(),
            Kind::Deftemplate => self.templates.keys().cloned().collect(),
            Kind::Defrule => self.rules.clone(),
            Kind::Deffunction => self.functions.keys().cloned().collect(),
        }
    }

    fn template_candidates(&self) -> impl Iterator<Item = Candidate> + '_ {
        self.templates
            .iter()
            .map(|(name, slots)| Candidate {
                name: name.clone(),
                detail: format!("deftemplate {}", slots.join(" ")),
            })
            .chain(ORDERED_FACTS.iter().map(|(name, args)| Candidate {
                name: name.to_string(),
                detail: format!("ordered fact {args}"),
            }))
    }

    fn function_candidates(&self) -> impl Iterator<Item = Candidate> + '_ {
        self.functions
            .iter()
            .map(|(name, params)| Candidate {
                name: name.clone(),
                detail: format!("deffunction {}", params.join(" ")),
            })
            .chain(builtin_candidates())
    }
}

/// Index one past the `Close` matching the `Open` at `open`, or the token count
/// when the form is unfinished.
fn form_end(tokens: &[(usize, Token<'_>)], open: usize) -> usize {
    let mut depth = 0usize;
    for (i, (_, token)) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::Open => depth += 1,
            Token::Close => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            },
            _ => {},
        }
    }
    tokens.len()
}

/// The forms directly inside the form opened at `open`, as `(open, end)`
/// token index pairs.
fn form_children<'t>(
    tokens: &'t [(usize, Token<'_>)],
    open: usize,
) -> impl Iterator<Item = (usize, usize)> + 't {
    let end = form_end(tokens, open);
    let mut i = open + 1;
    std::iter::from_fn(move || {
        while i < end {
            if tokens[i].1 == Token::Open {
                let child = (i, form_end(tokens, i));
                i = child.1;
                return Some(child);
            }
            i += 1;
        }
        None
    })
}

/// A form open at the cursor: where it starts, its head once typed, and
/// whether `=>` has passed at its level.
struct Frame<'a> {
    open: usize,
    head: Option<&'a str>,
    rhs: bool,
}

fn frames<'a>(tokens: &[(usize, Token<'a>)]) -> Vec<Frame<'a>> {
    let mut stack: Vec<Frame<'a>> = Vec::new();
    for (i, (_, token)) in tokens.iter().enumerate() {
        match token {
            Token::Open => {
                if let Some(top) = stack.last_mut() {
                    top.head.get_or_insert("");
                }
                stack.push(Frame {
                    open: i,
                    head: None,
                    rhs: false,
                });
            },
            Token::Close => {
                stack.pop();
            },
            Token::Atom(name) => {
                if let Some(top) = stack.last_mut() {
                    top.rhs |= top.head.is_some() && *name == "=>";
                    top.head.get_or_insert(name);
                }
            },
            Token::Str => {
                if let Some(top) = stack.last_mut() {
                    top.head.get_or_insert("");
                }
            },
        }
    }
    stack
}

fn named(names: impl IntoIterator<Item = impl Into<String>>, detail: &str) -> Vec<Candidate> {
    names
        .into_iter()
        .map(|name| Candidate {
            name: name.into(),
            detail: detail.to_string(),
        })
        .collect()
}

/// Candidates for the symbol ending at char index `cursor`, or `None` when the
/// cursor is not at the end of a symbol or nothing matches.
pub fn suggest(text: &str, cursor: usize) -> Option<Suggestion> {
    let cursor_byte = text
        .char_indices()
        .nth(cursor)
        .map_or(text.len(), |(i, _)| i);
    let (before, after) = text.split_at(cursor_byte);
    if after.starts_with(is_symbol_char) {
        return None;
    }
    let prefix_start = before
        .rfind(|c: char| !is_symbol_char(c))
        .map_or(0, |i| i + before[i..].chars().next().map_or(1, char::len_utf8));
    let prefix = &before[prefix_start..];
    let numeric = prefix
        .trim_start_matches(['-', '+'])
        .starts_with(|c: char| c.is_ascii_digit() || c == '.');
    if prefix.is_empty() || numeric {
        return None;
    }
    let (tokens, unfinished) = tokenize(&text[..prefix_start]);
    if unfinished {
        return None;
    }
    let start = text[..prefix_start].chars().count();

    let mut definitions = Definitions::default();
    Definitions::scan(&crate::clips::prelude(), usize::MAX, &mut definitions);
    Definitions::scan(text, prefix_start, &mut definitions);

    let stack = frames(&tokens);
    if prefix.starts_with('?') || prefix.starts_with("$?") {
        let candidates = variables(text, &tokens, &stack, prefix_start, prefix.starts_with('$'))
            .chain(named(definitions.globals.clone(), "defglobal"));
        return Suggestion::new(start, prefix, candidates);
    }

    let top = stack.last()?;
    let parent_frame = stack.len().checked_sub(2).map(|i| &stack[i]);
    let parent = parent_frame.and_then(|frame| frame.head);
    let rhs = parent_frame.is_some_and(|frame| frame.rhs);
    let candidates: Vec<Candidate> = match (top.head, parent) {
        (None, None) => named(CONSTRUCTS.iter().copied(), "construct")
            .into_iter()
            .chain(definitions.template_candidates())
            .chain(definitions.function_candidates())
            .collect(),
        (None, Some("deftemplate")) => named(["multislot", "slot"], "slot kind"),
        (None, Some("slot" | "multislot")) => named(SLOT_ATTRIBUTES.iter().copied(), "slot attribute"),
        (None, Some("defrule")) if !rhs => named(CONDITIONAL_ELEMENTS.iter().copied(), "conditional element")
            .into_iter()
            .chain(definitions.template_candidates())
            .collect(),
        (None, Some("deffacts")) => definitions.template_candidates().collect(),
        (None, Some(parent)) => {
            let template = match parent {
                "modify" | "duplicate" => modified_template(&tokens, &stack),
                _ => Some(parent),
            };
            match template.and_then(|t| definitions.templates.get(t)) {
                Some(slots) => named(slots.iter().cloned(), "slot"),
                None => definitions
                    .function_candidates()
                    .chain(definitions.template_candidates())
                    .collect(),
            }
        },
        (Some(head), parent) => {
            let in_fact = parent.is_some_and(|p| definitions.templates.contains_key(p));
            if let Some((_, kind)) = NAME_ARGUMENTS.iter().find(|(name, _)| *name == head) {
                named(definitions.names(*kind), &format!("{kind:?}").to_lowercase())
            } else if in_fact && head == "dir" {
                named(DIRECTIONS.iter().copied(), "direction")
            } else if in_fact && SHAPE_SLOTS.contains(&head) {
                named(definitions.ids.clone(), "id")
            } else {
                named(definitions.ids.clone(), "id")
                    .into_iter()
                    .chain(named(CONSTANTS.iter().copied(), "constant"))
                    .collect()
            }
        },
    };
    Suggestion::new(start, prefix, candidates)
}

/// The template of the fact `(modify ?f ...)` changes, read from the
/// `?f <- (template ...)` pattern of the enclosing rule.
fn modified_template<'a>(tokens: &[(usize, Token<'a>)], stack: &[Frame<'a>]) -> Option<&'a str> {
    let modify = &stack[stack.len() - 2];
    let Some((_, Token::Atom(var))) = tokens.get(modify.open + 2) else {
        return None;
    };
    let rule = &tokens[stack[0].open..];
    rule.windows(4).find_map(|window| match window {
        [(_, Token::Atom(v)), (_, Token::Atom("<-")), (_, Token::Open), (_, Token::Atom(t))]
            if v == var =>
        {
            Some(*t)
        },
        _ => None,
    })
}

/// Variables of the top-level form around the cursor, without `$`, or with it
/// when `multifield` is set. Skips the variable at byte `typing_at`.
fn variables<'t>(
    text: &'t str,
    tokens: &[(usize, Token<'t>)],
    stack: &[Frame<'_>],
    typing_at: usize,
    multifield: bool,
) -> impl Iterator<Item = Candidate> + 't {
    let form_start = stack.first().map_or(typing_at, |frame| tokens[frame.open].0);
    let (form_tokens, _) = tokenize(&text[form_start..]);
    let form_end = form_end(&form_tokens, 0);
    let mut names: Vec<String> = form_tokens[..form_end]
        .iter()
        .filter(|(offset, _)| form_start + offset != typing_at)
        .filter_map(|(_, token)| match token {
            Token::Atom(name) => name.trim_start_matches('$').strip_prefix('?'),
            _ => None,
        })
        .filter(|name| !name.is_empty() && !name.starts_with('*'))
        .map(|name| format!("{}?{name}", if multifield { "$" } else { "" }))
        .collect();
    names.sort();
    names.dedup();
    named(names, "variable").into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::apply;

    fn at_end(text: &str) -> Option<Suggestion> {
        suggest(text, text.chars().count())
    }

    fn names(text: &str) -> Vec<String> {
        at_end(text)
            .map(|s| s.candidates.into_iter().map(|c| c.name).collect())
            .unwrap_or_default()
    }

    fn detail(text: &str, name: &str) -> String {
        at_end(text)
            .and_then(|s| s.candidates.into_iter().find(|c| c.name == name))
            .map(|c| c.detail)
            .unwrap_or_default()
    }

    #[test]
    fn builtin_functions_come_from_clips() {
        let functions = builtin_functions();
        assert!(functions.iter().any(|f| f == "str-cat"));
        assert!(functions.iter().any(|f| f == "printout"));
    }

    #[test]
    fn head_position_offers_functions_with_signatures() {
        assert_eq!(names("(defrule r => (printo"), ["printout"]);
        assert_eq!(
            detail("(defrule r => (printo", "printout"),
            "function <logical-name> <expression>*"
        );
        assert!(names("(defrule r => (str-").contains(&"str-cat".to_string()));
    }

    #[test]
    fn top_level_offers_constructs_and_templates() {
        let deff = names("(deff");
        assert!(deff.contains(&"deffacts".to_string()) && deff.contains(&"deffunction".to_string()));
        assert_eq!(detail("(deff", "deffacts"), "construct");
        assert_eq!(names("(circ"), ["circle"]);
        assert!(detail("(circ", "circle").starts_with("deftemplate id order label"));
        assert_eq!(names("(text-f"), ["text-from"]);
    }

    #[test]
    fn constructs_are_only_offered_at_the_top_level() {
        assert!(!names("(defrule r => (assert (def").contains(&"defrule".to_string()));
    }

    #[test]
    fn user_deftemplates_complete_with_their_slots() {
        let text = "(deftemplate step (slot n) (multislot tags))\n";
        assert_eq!(names(&format!("{text}(ste")), ["step"]);
        assert_eq!(detail(&format!("{text}(ste"), "step"), "deftemplate n tags");
        assert_eq!(names(&format!("{text}(deffacts seed (step (t")), ["tags"]);
        assert_eq!(names(&format!("{text}(defrule r (step (")), Vec::<String>::new());
        assert_eq!(names(&format!("{text}(defrule r (ste")), ["step"]);
    }

    #[test]
    fn slots_of_prelude_templates() {
        assert_eq!(names("(box (wi"), ["width", "with"]);
        assert_eq!(names("(arrow (ch"), ["chop"]);
        assert_eq!(names("(defrule r (box (id ?i)) => (assert (circle (la"), ["label"]);
    }

    #[test]
    fn modify_offers_slots_of_the_bound_pattern() {
        let text = "(defrule r ?c <- (circle (id c)) => (modify ?c (fi";
        assert_eq!(names(text), ["fill", "fit"]);
    }

    #[test]
    fn defrule_patterns_offer_conditional_elements() {
        assert_eq!(names("(defrule r (ex"), ["exists"]);
        assert_eq!(names("(defrule r (te"), ["test", "text", "text-from"]);
        assert!(
            names("(defrule r (box) => (te").iter().all(|name| name != "test"),
            "the actions after => are calls, not patterns"
        );
    }

    #[test]
    fn deftemplate_body_offers_slot_kinds_and_attributes() {
        assert_eq!(names("(deftemplate t (mu"), ["multislot"]);
        assert_eq!(names("(deftemplate t (slot s (def"), ["default", "default-dynamic"]);
    }

    #[test]
    fn deffacts_names_complete_where_commands_take_them() {
        let text = "(deffacts start (box (id a)))\n(deffacts stop)\n";
        assert_eq!(names(&format!("{text}(undeffacts st")), ["start", "stop"]);
        assert_eq!(detail(&format!("{text}(undeffacts st"), "start"), "deffacts");
        assert_eq!(names(&format!("{text}(ppdeffacts sta")), ["start"]);
    }

    #[test]
    fn rule_and_function_names_complete_where_commands_take_them() {
        let text = "(defrule draw =>)\n(deffunction double (?x) (* 2 ?x))\n";
        assert_eq!(names(&format!("{text}(ppdefrule d")), ["draw"]);
        assert_eq!(names(&format!("{text}(undeffunction d")), ["double"]);
        assert_eq!(names(&format!("{text}(defrule r => (dou")), ["double"]);
        assert_eq!(detail(&format!("{text}(defrule r => (dou"), "double"), "deffunction ?x");
        assert_eq!(detail("(modify-at-", "modify-at-x"), "deffunction ?f ?x");
    }

    #[test]
    fn slot_values_offer_directions_and_shape_ids() {
        assert_eq!(names("(arrow (dir ri"), ["right"]);
        let text = "(box (id start))\n(circle (id stop))\n(arrow (from start) (to st";
        assert_eq!(names(text), ["start", "stop"]);
        assert_eq!(names("(defrule r => (printout t cr"), ["crlf"]);
    }

    #[test]
    fn variables_come_from_the_enclosing_form() {
        let text = "(defrule a (box (id ?id) (x ?x)) => )\n(defrule b (box (id ?i) (label $?words)) => (printout t ?";
        assert_eq!(names(text), ["?i", "?words"]);
        assert_eq!(names(&format!("{text}w")), ["?words"]);
        assert_eq!(names("(defrule b (box (label $?words)) => (bind $?r $?w"), ["$?words"]);
    }

    #[test]
    fn variable_being_typed_is_not_its_own_candidate() {
        assert!(at_end("(defrule r (box (id ?ne").is_none());
    }

    #[test]
    fn defglobals_complete_as_variables() {
        let text = "(defglobal ?*size* = 1)\n(defrule r => (printout t ?*s";
        assert_eq!(names(text), ["?*size*"]);
    }

    #[test]
    fn no_suggestion_mid_word_in_numbers_strings_or_comments() {
        assert!(suggest("(printout t)", 3).is_none());
        assert!(at_end("(box (x 1.5").is_none());
        assert!(at_end("(box (x -1").is_none());
        assert!(at_end("(box (label \"cir").is_none());
        assert!(at_end("; (circ").is_none());
        assert!(at_end("circ").is_none(), "outside any form");
        assert!(at_end("").is_none());
    }

    #[test]
    fn replacement_completes_the_shared_prefix() {
        let s = at_end("(deftemplate t (slot s (allowed-i").unwrap();
        assert_eq!(s.replacement().as_deref(), Some("allowed-in"));
        let s = at_end("(box (diam").unwrap();
        assert_eq!(s.replacement().as_deref(), Some("diameter"));
        let (text, cursor) = apply("(box (diam))", &s, "diameter");
        assert_eq!(text, "(box (diameter))");
        assert_eq!(cursor, 14);
    }
}
