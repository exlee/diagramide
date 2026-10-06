//! CLIPS output: facts asserted against the diagram templates become Pikchr.
//!
//! The editor content is split into top-level forms. Constructs (`defrule`,
//! `deftemplate`, ...) are built first, then the environment is reset so
//! `deffacts` take effect, then the remaining forms run in order: a form whose
//! head names a deftemplate is asserted as a fact, anything else is evaluated
//! as a command. Finally the rules run and every fact whose relation is a
//! diagram template is written out as one Pikchr statement, in fact order.
//!
//! After the rules settle, a layout pass renders the Pikchr once more with a
//! `print` line per shape and writes the centre each shape landed on into its
//! `x` and `y` slots (Pikchr units, inches). Rules may react to the new
//! values; when they change the diagram, the pass repeats, up to
//! [`MAX_LAYOUT_PASSES`] times.
//!
//! A fresh [`Environment`] is created for every evaluation. CLIPS environments
//! are thread-affine, so evaluation happens on one blocking thread.

use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use clips_bindings::{Environment, Fact, Router, Value};

use crate::OutputType;
use tokio::task;

/// Rule firings allowed per evaluation. Rule bases that never settle are
/// reported instead of hanging the editor.
pub const DEFAULT_RULE_LIMIT: u64 = 10_000;

/// Layout passes per evaluation. Each pass measures the diagram, writes the
/// positions into the facts, and runs the rules again.
pub const MAX_LAYOUT_PASSES: usize = 4;

/// Label given to a shape without an `id` while measuring.
const MEASURE_LABEL: &str = "Clips_fact_";

/// Templates for closed shapes: labels inside, sized by width/height/radius.
const BLOCK_SHAPES: &[&str] = &[
    "box", "circle", "ellipse", "oval", "cylinder", "file", "diamond", "dot", "text",
];
/// Templates for open shapes: paths with a start, an end, and arrowheads.
const LINE_SHAPES: &[&str] = &["arrow", "line", "spline", "arc"];

/// Ordered (positional) facts the translator understands. They have no
/// deftemplate, so a top-level form with one of these heads is asserted as
/// is: `(text-from editor-name)`, `(source-line name 1 "...")`.
const ORDERED_TEMPLATES: &[&str] = &[
    "pikchr-from",
    "text-from",
    "lines-from",
    "source-text",
    "source-line",
];

/// Another editor's content, looked up by editor name from `text-from`,
/// `lines-from`, and `pikchr-from` facts.
#[derive(Clone, Debug, Default)]
pub struct EditorSource {
    pub raw: Option<String>,
    pub generated: Option<String>,
    pub output_type: Option<OutputType>,
}

/// Editor name → content, as the handler sees it at evaluation time.
pub type Sources = HashMap<String, EditorSource>;

/// What the translator needs besides the facts.
struct Context {
    output_type: OutputType,
}

/// Constructs that must be built before facts are asserted.
const CONSTRUCTS: &[&str] = &[
    "deftemplate",
    "defrule",
    "deffacts",
    "deffunction",
    "defglobal",
    "defclass",
    "definstances",
    "defmessage-handler",
    "defgeneric",
    "defmethod",
    "defmodule",
];

const COMMON_SLOTS: &str = "(slot id) (slot order (default 0)) (multislot label) \
     (slot at) (multislot at-pos) (multislot at-rel) (slot with) (slot same) \
     (slot color) (slot fill) (slot thickness) (slot dashed) (slot dotted) \
     (slot invisible) (multislot style) (multislot attrs) (slot x) (slot y)";
const BLOCK_SLOTS: &str = "(slot width) (slot height) (slot radius) (slot diameter) (slot fit)";
const LINE_SLOTS: &str = "(slot from) (slot to) (slot dir) (slot length) (slot heads) \
     (slot chop) (slot radius) (multislot then)";

/// Functions that place a shape. `anchor` builds the place `id.pos` for
/// `at`, `at-rel`, `from`, and `to`. `modify-at` writes both coordinates,
/// `modify-at-x` and `modify-at-y` keep the other coordinate the layout pass
/// measured, and fail when the shape hasn't been measured yet. `coords` and
/// `between` build Pikchr places for `at`, `from`, and `to`: `(coords ?x ?y)` is
/// the point `(x,y)`, `(between ?ratio ?a ?b)` is `ratio between A and B`.
const PLACEMENT_FUNCTIONS: &str = "\
    (deffunction anchor (?id ?pos) (sym-cat ?id . ?pos))
    (deffunction modify-at (?f ?x ?y) (modify ?f (at-pos ?x ?y)))
    (deffunction modify-at-x (?f ?x) (bind ?y (fact-slot-value ?f y)) (if (eq ?y nil) then (printout werror \"modify-at-x: \" (fact-slot-value ?f id) \" has no measured y yet; match (y ?y&~nil)\" crlf) (return FALSE)) (modify ?f (at-pos ?x ?y)))
    (deffunction modify-at-y (?f ?y) (bind ?x (fact-slot-value ?f x)) (if (eq ?x nil) then (printout werror \"modify-at-y: \" (fact-slot-value ?f id) \" has no measured x yet; match (x ?x&~nil)\" crlf) (return FALSE)) (modify ?f (at-pos ?x ?y)))
    (deffunction coords (?x ?y) (str-cat \"(\" ?x \",\" ?y \")\"))
    (deffunction between (?ratio ?from ?to) (str-cat ?ratio \" between \" ?from \" and \" ?to))";

/// The deftemplates and functions every CLIPS editor starts with.
pub fn prelude() -> String {
    let mut out = String::new();
    for shape in BLOCK_SHAPES {
        out.push_str(&format!(
            "(deftemplate {shape} {COMMON_SLOTS} {BLOCK_SLOTS})\n"
        ));
    }
    for shape in LINE_SHAPES {
        out.push_str(&format!(
            "(deftemplate {shape} {COMMON_SLOTS} {LINE_SLOTS})\n"
        ));
    }
    out.push_str(
        "(deftemplate move (slot id) (slot order (default 0)) (slot dir) (slot length) \
         (slot at) (slot from) (slot to) (multislot then) (multislot attrs))\n",
    );
    out.push_str("(deftemplate anchor (slot id) (slot obj) (slot dir (default c)))\n");
    out.push_str("(deftemplate direction (slot order (default 0)) (slot dir))\n");
    out.push_str("(deftemplate pikchr (slot order (default 0)) (multislot text))\n");
    out.push_str("(deftemplate group (slot id) (slot order (default 0)) (multislot text) (slot at) (multislot at-pos) (multislot at-rel) (slot with) (slot x) (slot y))\n");
    out.push_str("(deftemplate raw-pikchr (slot order (default 0)) (multislot text))\n");
    out.push_str("(deftemplate raw-text (slot order (default 0)) (multislot text))\n");
    for line in PLACEMENT_FUNCTIONS.lines() {
        out.push_str(line.trim_start());
        out.push('\n');
    }
    out
}

/// Names of every template the translator writes out or resolves.
fn diagram_templates() -> BTreeSet<&'static str> {
    BLOCK_SHAPES
        .iter()
        .chain(LINE_SHAPES)
        .chain(["move", "anchor", "direction", "pikchr"].iter())
        .chain(["raw-pikchr", "raw-text", "group"].iter())
        .copied()
        .collect()
}

/// Collects what CLIPS prints: `stdout` for the script's own output, the
/// diagnostic names for errors and warnings.
#[derive(Default)]
struct Capture {
    stdout: String,
    errors: String,
}

#[derive(Clone, Default)]
struct SharedCapture(Rc<RefCell<Capture>>);

impl Router for SharedCapture {
    fn query(&mut self, logical_name: &str) -> bool {
        matches!(logical_name, "stdout" | "werror" | "wwarning")
    }

    fn write(&mut self, logical_name: &str, text: &str) {
        let mut capture = self.0.borrow_mut();
        match logical_name {
            "stdout" => capture.stdout.push_str(text),
            _ => capture.errors.push_str(text),
        }
    }
}

/// Evaluate a CLIPS program on a blocking thread and translate its facts to
/// Pikchr.
pub async fn safe_eval_clips(
    source: String,
    output_type: OutputType,
    sources: Sources,
) -> Result<String, String> {
    task::spawn_blocking(move || {
        std::panic::catch_unwind(|| {
            eval_clips_with(&source, DEFAULT_RULE_LIMIT, output_type, &sources)
        })
            .map_err(|_| "CLIPS environment panicked".to_string())?
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Evaluate `source` and translate the resulting facts to Pikchr, with no
/// other editors to draw from.
pub fn eval_clips(source: &str, rule_limit: u64) -> Result<String, String> {
    eval_clips_with(source, rule_limit, OutputType::Pikchr, &Sources::new())
}

/// Evaluate `source` and translate the resulting facts to `output_type`.
/// `sources` resolves `text-from`, `lines-from`, and `pikchr-from`.
pub fn eval_clips_with(
    source: &str,
    rule_limit: u64,
    output_type: OutputType,
    sources: &Sources,
) -> Result<String, String> {
    let context = Context { output_type };
    let forms = expand_includes(split_forms(source)?, sources, &mut Vec::new())?;
    let capture = SharedCapture::default();
    let mut env = Environment::new().map_err(|error| error.to_string())?;
    env.register_router("diagramide", 40, capture.clone())
        .map_err(|error| format!("{error:?}"))?;
    // Some parse failures, such as a malformed deffacts, reach the parser
    // hook without anything being written to `werror`.
    let diagnostics = capture.clone();
    env.set_parser_error_hook(move |diagnostic| {
        let mut capture = diagnostics.0.borrow_mut();
        for text in [&diagnostic.error, &diagnostic.warning] {
            if !text.is_empty() {
                capture.errors.push_str(text);
                capture.errors.push('\n');
            }
        }
    });

    for construct in prelude().lines() {
        env.build(construct)
            .map_err(|error| format!("Prelude failed: {error}"))?;
    }

    let fail = |capture: &SharedCapture, form: &str, fallback: String| -> String {
        let errors = std::mem::take(&mut capture.0.borrow_mut().errors);
        let errors = errors.trim();
        if errors.is_empty() {
            format!("{fallback}\n{form}")
        } else {
            errors.to_string()
        }
    };

    let (constructs, commands): (Vec<&str>, Vec<&str>) = forms
        .iter()
        .map(String::as_str)
        .partition(|form| CONSTRUCTS.contains(&head(form)));

    for form in constructs {
        // `Build` rejects a deffacts that needs an implied deftemplate and says
        // nothing about it; `LoadFromString` handles every construct.
        if env.load_str(form).is_err() {
            return Err(fail(&capture, form, "Construct rejected".to_string()));
        }
    }
    env.reset();

    for form in commands {
        let head = head(form);
        let is_template = ORDERED_TEMPLATES.contains(&head)
            || env
                .find_deftemplate(head)
                .map_err(|error| error.to_string())?
                .is_some();
        let outcome = if is_template {
            env.assert_string(form).map(drop)
        } else {
            env.eval(form).map(drop)
        };
        if outcome.is_err() || !capture.0.borrow().errors.trim().is_empty() {
            return Err(fail(&capture, form, "Form rejected".to_string()));
        }
    }

    // Errors from a rule pass are held back: a later pass that runs clean
    // supersedes them, so a rule that fails before layout and succeeds after
    // is not an error.
    let mut resolved = BTreeSet::new();
    let mut origins = HashMap::new();
    let mut pass_errors = settle(&mut env, &capture, rule_limit, sources, &mut resolved, &mut origins)?;

    let mut rows = collect_rows(&env, &origins)?;
    let mut pikchr = rows_to_pikchr(&rows, &context);
    for _ in 0..MAX_LAYOUT_PASSES {
        // A Pikchr error here shows up in the Render window; the facts keep
        // whatever positions they had.
        let Some(positions) = measure(&rows, &context) else {
            break;
        };
        if !apply_positions(&env, &rows, &positions, &mut origins)? {
            break;
        }
        pass_errors = settle(&mut env, &capture, rule_limit, sources, &mut resolved, &mut origins)?;
        rows = collect_rows(&env, &origins)?;
        let next = rows_to_pikchr(&rows, &context);
        if next == pikchr {
            break;
        }
        pikchr = next;
    }
    if !pass_errors.is_empty() {
        return Err(pass_errors);
    }
    let stdout = std::mem::take(&mut capture.0.borrow_mut().stdout);
    for line in stdout.lines() {
        pikchr.push_str("# ");
        pikchr.push_str(line);
        pikchr.push('\n');
    }
    Ok(pikchr)
}

/// Run the rules once. The firing limit is fatal; CLIPS errors from the pass
/// are returned as text for the caller to keep or discard.
fn run_pass(env: &mut Environment, capture: &SharedCapture, rule_limit: u64) -> Result<String, String> {
    let fired = env.run(Some(rule_limit));
    if fired >= rule_limit {
        return Err(format!(
            "Rules fired {rule_limit} times, the limit. Check for a rule that never stops firing."
        ));
    }
    Ok(std::mem::take(&mut capture.0.borrow_mut().errors).trim().to_string())
}

/// Run the rules, then satisfy any new `text-from`, `lines-from`, and
/// `pikchr-from` requests; repeat while requests keep appearing.
fn settle(
    env: &mut Environment,
    capture: &SharedCapture,
    rule_limit: u64,
    sources: &Sources,
    resolved: &mut BTreeSet<(String, String)>,
    origins: &mut HashMap<i64, i64>,
) -> Result<String, String> {
    resolve_sources(env, sources, resolved, origins)?;
    let mut errors = run_pass(env, capture, rule_limit)?;
    for _ in 0..MAX_LAYOUT_PASSES {
        if !resolve_sources(env, sources, resolved, origins)? {
            break;
        }
        errors = run_pass(env, capture, rule_limit)?;
    }
    Ok(errors)
}

/// Assert `source-text` and `source-line` facts for every request fact not
/// yet served, and check `pikchr-from` requests. Returns whether anything
/// was asserted.
fn resolve_sources(
    env: &mut Environment,
    sources: &Sources,
    resolved: &mut BTreeSet<(String, String)>,
    origins: &mut HashMap<i64, i64>,
) -> Result<bool, String> {
    let mut requests = Vec::new();
    for fact in env.facts() {
        let relation = fact.relation();
        if !matches!(relation.as_str(), "text-from" | "lines-from" | "pikchr-from") {
            continue;
        }
        let row = snapshot(&fact)?;
        let Some(name) = row.multi("implied").first() else {
            return Err(format!("({relation}) needs an editor name"));
        };
        let key = (relation.clone(), name.clone());
        if resolved.insert(key) {
            requests.push((relation, name.clone(), row.index));
        }
    }
    let mut asserted = false;
    for (relation, name, request_index) in requests {
        let source = sources
            .get(&name)
            .ok_or_else(|| format!("({relation} {name}): no editor named {name}"))?;
        match relation.as_str() {
            "pikchr-from" => {
                if source.output_type != Some(OutputType::Pikchr) {
                    return Err(format!(
                        "(pikchr-from {name}): {name} doesn't produce Pikchr output"
                    ));
                }
                let text = source.generated.clone().unwrap_or_default();
                let group = env
                    .assert_string(&format!(
                        "(group (id {}) (text {}))",
                        atom(&name),
                        quote(text.trim_end())
                    ))
                    .map_err(|error| format!("(pikchr-from {name}): {error}"))?;
                // The group takes the request's place in the output order.
                origins.insert(group.index(), request_index);
                asserted = true;
            },
            "text-from" => {
                let text = source.raw.clone().unwrap_or_default();
                env.assert_string(&format!("(source-text {} {})", atom(&name), quote(&text)))
                    .map_err(|error| format!("(text-from {name}): {error}"))?;
                asserted = true;
            },
            _ => {
                let text = source.raw.clone().unwrap_or_default();
                for (index, line) in text.lines().enumerate() {
                    env.assert_string(&format!(
                        "(source-line {} {} {})",
                        atom(&name),
                        index + 1,
                        quote(line)
                    ))
                    .map_err(|error| format!("(lines-from {name}): {error}"))?;
                }
                asserted = true;
            },
        }
    }
    Ok(asserted)
}

/// Whether CLIPS source has a `(head NAME)` form for any of `heads`, with
/// `NAME` bare or quoted. Used for editor dependency tracking.
pub fn references_editor(content: &str, name: &str, heads: &[&str]) -> bool {
    heads.iter().any(|head| {
        content.match_indices(&format!("({head}")).any(|(at, marker)| {
            let rest = content[at + marker.len()..].trim_start();
            if rest.len() == content[at + marker.len()..].len() {
                return false;
            }
            let token = if let Some(quoted) = rest.strip_prefix('"') {
                quoted.split('"').next().unwrap_or("")
            } else {
                rest.split(|c: char| c.is_whitespace() || c == ')')
                    .next()
                    .unwrap_or("")
            };
            token == name
        })
    })
}

/// Includes nested more deeply than this are an error, which also stops a
/// cycle of editors including each other.
const MAX_INCLUDE_DEPTH: usize = 8;

/// Replace every top-level `(include NAME)` form with the forms of editor
/// `NAME`'s text, recursively. An editor with Text output contributes its
/// output instead of its source. `stack` holds the names being included.
fn expand_includes(
    forms: Vec<String>,
    sources: &Sources,
    stack: &mut Vec<String>,
) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(forms.len());
    for form in forms {
        if head(&form) != "include" {
            out.push(form);
            continue;
        }
        let name = include_name(&form)
            .ok_or_else(|| format!("(include) needs an editor name\n{form}"))?;
        if stack.len() >= MAX_INCLUDE_DEPTH || stack.contains(&name) {
            return Err(format!(
                "(include {name}): includes nest too deep ({})",
                stack.join(" > ")
            ));
        }
        let source = sources
            .get(&name)
            .ok_or_else(|| format!("(include {name}): no editor named {name}"))?;
        let text = if source.output_type == Some(OutputType::Text) {
            source.generated.clone()
        } else {
            source.raw.clone()
        };
        let text = text.ok_or_else(|| format!("(include {name}): {name} has no text"))?;
        let nested = split_forms(&text).map_err(|error| format!("(include {name}): {error}"))?;
        stack.push(name);
        out.extend(expand_includes(nested, sources, stack)?);
        stack.pop();
    }
    Ok(out)
}

/// The editor name in `(include NAME)` or `(include "NAME")`.
fn include_name(form: &str) -> Option<String> {
    let rest = form.trim_start().strip_prefix('(')?.trim_start();
    let rest = rest.strip_prefix("include")?.trim_start();
    let name = if let Some(quoted) = rest.strip_prefix('"') {
        quoted.split('"').next()?
    } else {
        rest.split(|c: char| c.is_whitespace() || c == ')').next()?
    };
    (!name.is_empty()).then(|| name.to_string())
}

/// First symbol of a parenthesised form, or the empty string.
fn head(form: &str) -> &str {
    let inner = form.trim_start().trim_start_matches('(');
    inner
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .next()
        .unwrap_or("")
}

/// Split CLIPS source into top-level forms, honouring strings and `;`
/// comments. Text outside any form is ignored.
pub fn split_forms(source: &str) -> Result<Vec<String>, String> {
    let mut forms = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;
    for (index, c) in source.char_indices() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            ';' => in_comment = true,
            '"' => in_string = true,
            '(' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            },
            ')' => {
                if depth == 0 {
                    return Err(format!("Extra ')' at byte {index}"));
                }
                depth -= 1;
                if depth == 0 {
                    let begin = start.take().unwrap_or(index);
                    forms.push(source[begin..=index].to_string());
                }
            },
            _ => {},
        }
    }
    if in_string {
        return Err("Unterminated string".to_string());
    }
    if depth > 0 {
        return Err(format!(
            "Missing {depth} closing parenthes{}",
            if depth == 1 { "is" } else { "es" }
        ));
    }
    Ok(forms)
}

/// The Pikchr label an `id` slot value maps to: capitalised, with every
/// character Pikchr rejects replaced by `_`.
pub fn pikchr_label(id: &str) -> String {
    let mut label: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if label.is_empty()
        || !label
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
    {
        label.insert(0, 'L');
    }
    let first = label.remove(0).to_ascii_uppercase();
    label.insert(0, first);
    label
}

/// Pikchr edge name for a compass or anchor symbol (`NE` → `ne`, `center` →
/// `center`).
fn edge_name(dir: &str) -> String {
    dir.to_ascii_lowercase()
}

/// A snapshot of one fact: relation, slot values rendered as text.
struct Row {
    relation: String,
    index: i64,
    order: i64,
    slots: HashMap<String, Slot>,
}

enum Slot {
    Single(String),
    Multi(Vec<String>),
}

fn value_text(value: &Value<'_>) -> String {
    if let Some(i) = value.as_i64() {
        i.to_string()
    } else if let Some(f) = value.as_f64() {
        f.to_string()
    } else if let Some(s) = value.as_str() {
        s.to_string()
    } else if let Some(fact) = value.as_fact() {
        fact.slot("id")
            .ok()
            .and_then(|id| id.as_str().map(str::to_string))
            .unwrap_or_default()
    } else {
        String::new()
    }
}

fn snapshot(fact: &Fact<'_>) -> Result<Row, String> {
    let relation = fact.relation();
    let mut slots = HashMap::new();
    for name in fact.slot_names().iter() {
        let Some(name) = name.as_str().map(str::to_string) else {
            continue;
        };
        let value = fact.slot(&name).map_err(|error| error.to_string())?;
        let slot = if let Some(multi) = value.as_multifield() {
            Slot::Multi(multi.iter().map(|item| value_text(&item)).collect())
        } else {
            let text = value_text(&value);
            if text == "nil" {
                continue;
            }
            Slot::Single(text)
        };
        slots.insert(name, slot);
    }
    let order = match slots.get("order") {
        Some(Slot::Single(order)) => order.parse().unwrap_or(0),
        _ => 0,
    };
    Ok(Row {
        relation,
        index: fact.index(),
        order,
        slots,
    })
}

impl Row {
    fn single(&self, slot: &str) -> Option<&str> {
        match self.slots.get(slot) {
            Some(Slot::Single(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    fn multi(&self, slot: &str) -> &[String] {
        match self.slots.get(slot) {
            Some(Slot::Multi(values)) => values,
            _ => &[],
        }
    }

    fn flag(&self, slot: &str) -> bool {
        self.single(slot)
            .is_some_and(|value| matches!(value, "TRUE" | "yes" | "true"))
    }
}

/// Resolves fact ids and anchors to Pikchr places.
struct Names {
    labels: HashMap<String, String>,
    anchors: HashMap<String, (String, String)>,
}

impl Names {
    fn new(rows: &[Row]) -> Self {
        let mut labels = HashMap::new();
        let mut anchors = HashMap::new();
        for row in rows {
            let Some(id) = row.single("id") else {
                continue;
            };
            if row.relation == "anchor" {
                let obj = row.single("obj").unwrap_or_default().to_string();
                let dir = row.single("dir").unwrap_or("c").to_string();
                anchors.insert(id.to_string(), (obj, dir));
            } else {
                labels.insert(id.to_string(), pikchr_label(id));
            }
        }
        Self { labels, anchors }
    }

    /// `b` → `B`, `b.ne` → `B.ne`, anchor `a` → `B.ne`, anything else as is.
    fn place(&self, reference: &str) -> String {
        let (base, suffix) = match reference.split_once('.') {
            Some((base, suffix)) => (base, Some(suffix)),
            None => (reference, None),
        };
        let resolved = if let Some(label) = self.labels.get(base) {
            label.clone()
        } else if let Some((obj, dir)) = self.anchors.get(base) {
            format!("{}.{}", self.place(obj), edge_name(dir))
        } else {
            return reference.to_string();
        };
        match suffix {
            Some(suffix) => format!("{resolved}.{suffix}"),
            None => resolved,
        }
    }

    /// Replace whole tokens of raw Pikchr text that name a fact or anchor.
    fn substitute(&self, raw: &str) -> String {
        raw.split(' ')
            .map(|token| self.place(token))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// An editor name as a CLIPS value: a bare symbol when the name is one, so
/// that `(source-text notes ?t)` matches, otherwise a string.
fn atom(name: &str) -> String {
    let symbol = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    let integer = !name.is_empty() && name.chars().all(|c| c.is_ascii_digit());
    if symbol || integer {
        name.to_string()
    } else {
        quote(name)
    }
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `dir`/`length` → `up 150%`, `go 150% ne`, `go 1cm heading 30`.
fn heading(dir: &str, length: Option<&str>) -> String {
    let length = length.map(|l| format!(" {l}")).unwrap_or_default();
    let lowered = dir.to_ascii_lowercase();
    match lowered.as_str() {
        "n" | "north" | "up" => format!("up{length}"),
        "s" | "south" | "down" => format!("down{length}"),
        "e" | "east" | "right" => format!("right{length}"),
        "w" | "west" | "left" => format!("left{length}"),
        "ne" | "nw" | "se" | "sw" => format!("go{length} {lowered}"),
        _ if lowered.parse::<f64>().is_ok() => format!("go{length} heading {lowered}"),
        _ => format!("go{length} {dir}"),
    }
}

fn push_valued(out: &mut Vec<String>, row: &Row, names: &Names, slot: &str, keyword: &str) {
    if let Some(value) = row.single(slot) {
        out.push(format!("{keyword} {}", names.substitute(value)));
    }
}

/// `dashed`, `dotted`: a flag alone or a flag with a length.
fn push_dash(out: &mut Vec<String>, row: &Row, slot: &str) {
    match row.single(slot) {
        Some("TRUE" | "yes" | "true") => out.push(slot.to_string()),
        Some("FALSE" | "no" | "false") => {},
        Some(value) => out.push(format!("{slot} {value}")),
        None => {},
    }
}

/// The place a shape goes: `at` is a Pikchr place, `(at-pos X Y)` is the
/// point `X, Y`, `(at-rel PLACE X Y)` is `PLACE + (X, Y)`. The first set
/// slot wins. `None` when none is set.
fn at_text(row: &Row, names: &Names) -> Option<String> {
    if let Some(place) = row.single("at") {
        return Some(names.substitute(place));
    }
    if let [x, y] = row.multi("at-pos") {
        return Some(format!("{x}, {y}"));
    }
    if let [place, x, y] = row.multi("at-rel") {
        return Some(format!("{} + ({x}, {y})", names.substitute(place)));
    }
    None
}

fn statement(row: &Row, names: &Names, context: &Context) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    match row.relation.as_str() {
        "anchor" | "text-from" | "lines-from" | "pikchr-from" | "source-text" | "source-line" => {
            return None;
        },
        "raw-text" => return Some(row.multi("text").join(" ")),
        "raw-pikchr" => {
            if context.output_type != OutputType::Pikchr {
                return None;
            }
            return Some(names.substitute(&row.multi("text").join(" ")));
        },
        "group" => {
            // A Pikchr sub-diagram: `Id: [ ... ] at ...`. The body keeps its
            // own labels, so nothing inside is substituted.
            let mut out = String::new();
            if let Some(id) = row.single("id") {
                out.push_str(&format!("{}: ", names.labels[id]));
            }
            out.push_str("[\n");
            for text in row.multi("text") {
                out.push_str(text.trim_end());
                out.push('\n');
            }
            out.push(']');
            if let Some(at) = at_text(row, names) {
                match row.single("with") {
                    Some(with) => out.push_str(&format!(
                        " with .{} at {at}",
                        edge_name(with.trim_start_matches('.'))
                    )),
                    None => out.push_str(&format!(" at {at}")),
                }
            }
            return Some(out);
        },
        "direction" => return row.single("dir").map(|dir| heading(dir, None)),
        "pikchr" => {
            return Some(names.substitute(&row.multi("text").join(" ")));
        },
        "move" => {
            parts.push("move".to_string());
            if let Some(dir) = row.single("dir") {
                parts.push(heading(dir, row.single("length")));
            } else if let Some(length) = row.single("length") {
                parts.push(length.to_string());
            }
            push_valued(&mut parts, row, names, "from", "from");
            push_valued(&mut parts, row, names, "to", "to");
            push_valued(&mut parts, row, names, "at", "at");
            for segment in row.multi("then") {
                parts.push(format!("then {}", names.substitute(segment)));
            }
            parts.extend(row.multi("attrs").iter().map(|a| names.substitute(a)));
            return Some(parts.join(" "));
        },
        _ => {},
    }

    parts.push(row.relation.clone());
    parts.extend(row.multi("label").iter().map(|label| quote(label)));
    // Text properties such as `bold` bind to the string before them.
    parts.extend(row.multi("style").iter().cloned());

    if LINE_SHAPES.contains(&row.relation.as_str()) {
        if let Some(heads) = row.single("heads") {
            match heads {
                "->" | "<-" | "<->" => parts.push(heads.to_string()),
                "none" | "FALSE" | "no" => {},
                other => parts.push(other.to_string()),
            }
        }
        push_valued(&mut parts, row, names, "from", "from");
        if let Some(dir) = row.single("dir") {
            parts.push(heading(dir, row.single("length")));
        }
        for segment in row.multi("then") {
            parts.push(format!("then {}", names.substitute(segment)));
        }
        push_valued(&mut parts, row, names, "to", "to");
        push_valued(&mut parts, row, names, "radius", "radius");
        if row.flag("chop") {
            parts.push("chop".to_string());
        }
    } else {
        push_valued(&mut parts, row, names, "width", "width");
        push_valued(&mut parts, row, names, "height", "height");
        push_valued(&mut parts, row, names, "radius", "radius");
        push_valued(&mut parts, row, names, "diameter", "diameter");
        if row.flag("fit") {
            parts.push("fit".to_string());
        }
    }

    if let Some(same) = row.single("same") {
        parts.push(format!("same as {}", names.place(same)));
    }
    if let Some(at) = at_text(row, names) {
        match row.single("with") {
            Some(with) => parts.push(format!(
                "with .{} at {at}",
                edge_name(with.trim_start_matches('.'))
            )),
            None => parts.push(format!("at {at}")),
        }
    }
    push_valued(&mut parts, row, names, "color", "color");
    push_valued(&mut parts, row, names, "fill", "fill");
    push_valued(&mut parts, row, names, "thickness", "thickness");
    push_dash(&mut parts, row, "dashed");
    push_dash(&mut parts, row, "dotted");
    if row.flag("invisible") {
        parts.push("invisible".to_string());
    }
    parts.extend(row.multi("attrs").iter().map(|a| names.substitute(a)));

    let mut line = parts.join(" ");
    if let Some(id) = row.single("id") {
        line = format!("{}: {line}", names.labels[id]);
    }
    Some(line)
}

/// Snapshot every diagram fact, ordered by `order` then by assertion. A fact
/// re-asserted by a layout pass keeps the place of the fact it replaced.
fn collect_rows(env: &Environment, origins: &HashMap<i64, i64>) -> Result<Vec<Row>, String> {
    let templates = diagram_templates();
    let mut rows = Vec::new();
    for fact in env.facts() {
        let row = snapshot(&fact)?;
        if templates.contains(row.relation.as_str()) {
            rows.push(row);
        }
    }
    rows.sort_by_key(|row| (row.order, *origins.get(&row.index).unwrap_or(&row.index)));
    Ok(rows)
}

/// Write every row as one Pikchr statement.
fn rows_to_pikchr(rows: &[Row], context: &Context) -> String {
    let names = Names::new(rows);
    let mut out = String::new();
    for row in rows {
        if let Some(statement) = statement(row, &names, context) {
            out.push_str(&statement);
            out.push('\n');
        }
    }
    out
}

/// The centre of every shape row, keyed by fact index, as Pikchr lays the
/// diagram out. `None` when Pikchr rejects the diagram.
fn measure(rows: &[Row], context: &Context) -> Option<HashMap<i64, (f64, f64)>> {
    let names = Names::new(rows);
    let mut text = String::new();
    let mut prints = String::new();
    for row in rows {
        let Some(statement) = statement(row, &names, context) else {
            continue;
        };
        let is_shape = BLOCK_SHAPES.contains(&row.relation.as_str())
            || LINE_SHAPES.contains(&row.relation.as_str())
            || row.relation == "group";
        if !is_shape {
            text.push_str(&statement);
            text.push('\n');
            continue;
        }
        let label = match row.single("id") {
            Some(id) => {
                text.push_str(&statement);
                names.labels[id].clone()
            },
            None => {
                let label = format!("{MEASURE_LABEL}{}", row.index);
                text.push_str(&format!("{label}: {statement}"));
                label
            },
        };
        text.push('\n');
        prints.push_str(&format!("print \"{}\", {label}.x, {label}.y\n", row.index));
    }
    if prints.is_empty() {
        return Some(HashMap::new());
    }
    text.push_str(&prints);
    let rendered = pikchr_pro::pikchr::render(&text, None, 1).ok()?;
    let output = rendered.into_string();
    if output.contains("ERROR:") {
        return None;
    }
    Some(parse_positions(&output))
}

/// Pikchr writes `print` output before the SVG, one `<br>` line per
/// statement: `12 0.75 0.5<br>`.
fn parse_positions(output: &str) -> HashMap<i64, (f64, f64)> {
    let head = output.split("<svg").next().unwrap_or("");
    head.lines()
        .filter_map(|line| {
            let mut words = line.trim_end_matches("<br>").split_whitespace();
            let index = words.next()?.parse().ok()?;
            let x = words.next()?.parse().ok()?;
            let y = words.next()?.parse().ok()?;
            Some((index, (x, y)))
        })
        .collect()
}

/// Write measured positions into the `x`/`y` slots of facts whose values
/// differ. Returns whether any fact changed.
fn apply_positions(
    env: &Environment,
    rows: &[Row],
    positions: &HashMap<i64, (f64, f64)>,
    origins: &mut HashMap<i64, i64>,
) -> Result<bool, String> {
    let mut changed = false;
    for row in rows {
        let Some(&(x, y)) = positions.get(&row.index) else {
            continue;
        };
        let current = |slot: &str| row.single(slot).and_then(|v| v.parse::<f64>().ok());
        if current("x") == Some(x) && current("y") == Some(y) {
            continue;
        }
        let Some(fact) = env.find_fact(row.index) else {
            continue;
        };
        let replacement = fact
            .modifier()
            .and_then(|mut m| {
                m.float("x", x)?;
                m.float("y", y)?;
                m.modify()
            })
            .map_err(|error| format!("Position update failed: {error}"))?;
        let origin = *origins.get(&row.index).unwrap_or(&row.index);
        origins.insert(replacement.index(), origin);
        changed = true;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pikchr(source: &str) -> String {
        eval_clips(source, DEFAULT_RULE_LIMIT).unwrap_or_else(|error| panic!("{error}"))
    }

    fn renders(source: &str) -> String {
        let code = pikchr(source);
        let svg = crate::render::render(crate::OutputType::Pikchr, &code)
            .unwrap_or_else(|error| panic!("{error}\n{code}"));
        assert!(svg.starts_with("<svg"), "{svg}");
        code
    }

    #[test]
    fn labelled_shapes_and_a_connector() {
        let code = renders(
            r#"(box (id b) (label "Hello"))
               (circle (id c) (label "World"))
               (line (from b) (to c))"#,
        );
        assert_eq!(
            code,
            "B: box \"Hello\"\nC: circle \"World\"\nline from B to C\n"
        );
    }

    #[test]
    fn anchors_resolve_to_edges_and_directions_to_pikchr_moves() {
        let code = renders(
            "(box (id b))\n(anchor (id a) (dir NE) (obj b))\n(line (from a) (dir N) (length 150%))",
        );
        assert_eq!(code, "B: box\nline from B.ne up 150%\n");
    }

    #[test]
    fn diagonal_directions_use_go() {
        assert_eq!(heading("NE", Some("1cm")), "go 1cm ne");
        assert_eq!(heading("30", None), "go heading 30");
        assert_eq!(heading("W", None), "left");
    }

    #[test]
    fn rules_generate_shapes_in_firing_order() {
        let code = renders(
            r#"(deftemplate step (slot n))
               (defrule draw (step (n ?n)) => (assert (box (id (sym-cat s ?n)) (label (str-cat "Step " ?n)))))
               (step (n 1))"#,
        );
        assert_eq!(code, "S1: box \"Step 1\"\n");
    }

    #[test]
    fn order_slot_overrides_assertion_order() {
        let code = pikchr("(box (id second) (order 2))\n(box (id first) (order 1))");
        assert_eq!(code, "First: box\nSecond: box\n");
    }

    #[test]
    fn attributes_and_raw_passthrough() {
        let code = renders(
            r#"(box (id b) (width 2cm) (fill lightgray) (dashed yes) (style thick) (attrs "rad 0.1"))
               (direction (dir down))
               (circle (radius 0.3) (at "B.s + (0, -1)") (color red) (dotted 0.05) (invisible TRUE))
               (arrow (heads <->) (from b.e) (dir E) (length 1cm) (then "down 1cm") (chop TRUE))
               (pikchr (text "line from" b.n "up 1cm"))
               (move (dir right) (length 2cm))"#,
        );
        assert_eq!(
            code,
            "B: box thick width 2cm fill lightgray dashed rad 0.1\n\
             down\n\
             circle radius 0.3 at B.s + (0, -1) color red dotted 0.05 invisible\n\
             arrow <-> from B.e right 1cm then down 1cm chop\n\
             line from B.n up 1cm\n\
             move right 2cm\n"
        );
    }

    #[test]
    fn deffacts_and_printout_are_supported() {
        let code = pikchr(
            r#"(deffacts start (box (id a)))
               (defrule hello (box (id ?x) (x nil)) => (printout t "drew " ?x crlf))"#,
        );
        assert_eq!(code, "A: box\n# drew a\n");
    }

    #[test]
    fn commands_run_between_facts() {
        let code = pikchr("(box (id a))\n(assert (box (id b)))\n(+ 1 2)");
        assert!(code.starts_with("A: box\nB: box\n"), "{code}");
    }

    #[test]
    fn construct_errors_are_reported_with_clips_text() {
        let error = eval_clips("(defrule broken (box (nosuch 1)) => )", DEFAULT_RULE_LIMIT)
            .expect_err("unknown slot must fail");
        assert!(error.contains("nosuch"), "{error}");
    }

    #[test]
    fn deffacts_before_its_template_reports_the_clips_error() {
        let error = eval_clips(
            "(deffacts p (swatch (name red)))\n(deftemplate swatch (slot name))",
            DEFAULT_RULE_LIMIT,
        )
        .expect_err("template conflict must fail");
        assert!(
            error.contains("swatch") && !error.contains("Construct rejected"),
            "{error}"
        );
    }

    #[test]
    fn unknown_function_is_an_error() {
        let error = eval_clips("(frobnicate 1)", DEFAULT_RULE_LIMIT).expect_err("must fail");
        assert!(error.to_lowercase().contains("frobnicate"), "{error}");
    }

    #[test]
    fn runaway_rules_hit_the_limit() {
        let error = eval_clips(
            "(defrule forever (box (id ?x)) => (assert (box (id (gensym*)))))\n(box (id a))",
            50,
        )
        .expect_err("non-terminating rule base must stop");
        assert!(error.contains("limit"), "{error}");
    }

    #[test]
    fn labels_are_capitalised_and_sanitised() {
        assert_eq!(pikchr_label("b"), "B");
        assert_eq!(pikchr_label("my-box"), "My_box");
        assert_eq!(pikchr_label("Box2"), "Box2");
        assert_eq!(pikchr_label("1st"), "L1st");
    }

    #[test]
    fn split_forms_skips_comments_and_strings() {
        let forms = split_forms("; (not a form)\n(a \"x ) y\") (b ; c)\n)").unwrap();
        assert_eq!(forms, vec!["(a \"x ) y\")", "(b ; c)\n)"]);
        assert!(split_forms("(a").is_err());
        assert!(split_forms(")").is_err());
    }

    /// `# id x y` comment lines printed by a reporting rule, keyed by id.
    fn reported(code: &str) -> HashMap<String, (f64, f64)> {
        code.lines()
            .filter_map(|line| {
                let mut words = line.strip_prefix("# ")?.split_whitespace();
                let id = words.next()?.to_string();
                let x = words.next()?.parse().ok()?;
                let y = words.next()?.parse().ok()?;
                Some((id, (x, y)))
            })
            .collect()
    }

    const REPORT: &str = r#"(defrule report (declare (salience -10))
        (box (id ?i) (x ?x&~nil) (y ?y)) => (printout t ?i " " ?x " " ?y crlf))"#;

    #[test]
    fn shapes_receive_their_rendered_centre() {
        let code = pikchr(&format!("{REPORT}\n(box (id a))\n(box (id b))"));
        let at = reported(&code);
        let (a, b) = (at["a"], at["b"]);
        assert!(b.0 > a.0, "{code}");
        assert_eq!(a.1, b.1, "{code}");
    }

    #[test]
    fn rules_react_to_positions_and_the_diagram_is_measured_again() {
        let code = renders(&format!(
            r#"{REPORT}
               (defrule mark (box (id ?i) (x ?x&~nil) (y ?y))
                 => (assert (dot (id (sym-cat d- ?i)) (at (str-cat ?x ", " ?y)))))
               (box (id a))"#
        ));
        assert!(code.contains("D_a: dot at "), "{code}");
        assert_eq!(reported(&code).len(), 1, "{code}");
    }

    #[test]
    fn positions_are_outputs_not_attributes() {
        assert_eq!(pikchr("(box (id a) (x 5) (y 5))"), "A: box\n");
    }

    #[test]
    fn shapes_without_an_id_are_measured_too() {
        let code = pikchr(
            r#"(defrule report (declare (salience -10)) (circle (x ?x&~nil)) => (printout t "seen" crlf))
               (circle)"#,
        );
        assert_eq!(code, "circle\n# seen\n");
    }

    #[test]
    fn order_survives_the_layout_pass() {
        let code = pikchr(
            "(box (id second) (order 2))\n(box (id first) (order 1))\n(box (id b))\n(box (id a))",
        );
        assert_eq!(code, "B: box\nA: box\nFirst: box\nSecond: box\n");
    }

    #[test]
    fn pikchr_errors_leave_positions_unset() {
        let code = pikchr(&format!("{REPORT}\n(box (id a) (attrs \"nonsense\"))"));
        assert_eq!(code, "A: box nonsense\n");
    }

    #[test]
    fn print_output_parses_before_the_svg() {
        let at = parse_positions("3 0.75 -0.5<br>\n7 1 2<br>\n<svg>3 9 9<br>");
        assert_eq!(at.len(), 2);
        assert_eq!(at[&3], (0.75, -0.5));
        assert_eq!(at[&7], (1.0, 2.0));
    }

    #[test]
    fn modify_at_places_a_shape_from_coordinates() {
        let code = renders(
            r#"(box (id a))
               (circle (id c))
               (defrule beside (box (id a) (x ?x&~nil) (y ?y)) ?c <- (circle (id c) (at-pos))
                 => (modify-at ?c (+ ?x 0.2) ?y))"#,
        );
        let at = code
            .lines()
            .find_map(|l| l.strip_prefix("C: circle at "))
            .unwrap_or_else(|| panic!("{code}"));
        let (x, y) = at.split_once(", ").unwrap();
        assert!(x.parse::<f64>().unwrap() > 0.0, "{code}");
        assert!(y.parse::<f64>().is_ok(), "{code}");
    }

    #[test]
    fn modify_at_x_keeps_the_measured_y() {
        let code = renders(
            r#"(box (id a))
               (defrule shift ?a <- (box (id a) (y ?y&~nil) (at-pos)) => (modify-at-x ?a 3))"#,
        );
        assert_eq!(code, "A: box at 3, 0\n", "{code}");
    }

    #[test]
    fn modify_at_y_keeps_the_measured_x() {
        let code = renders(
            r#"(box (id a))
               (defrule shift ?a <- (box (id a) (x ?x&~nil) (at-pos)) => (modify-at-y ?a 2))"#,
        );
        assert_eq!(code, "A: box at 0, 2\n", "{code}");
    }

    #[test]
    fn modify_at_x_before_layout_fails_quietly_when_the_rule_stops_matching() {
        let code = pikchr("(box (id a))\n(defrule early ?a <- (box (id a) (y nil)) => (modify-at-x ?a 3))");
        assert_eq!(code, "A: box\n");
    }

    #[test]
    fn at_pos_and_at_rel_build_places() {
        let code = renders(
            r#"(box (id b))
               (circle (id c) (at-pos 1 2.5))
               (dot (at b.ne))
               (text (label "t") (with sw) (at-rel b.s 0 -0.5))
               (dot (at c) (at-pos 9 9))"#,
        );
        assert_eq!(
            code,
            "B: box\nC: circle at 1, 2.5\ndot at B.ne\ntext \"t\" with .sw at B.s + (0, -0.5)\ndot at C\n"
        );
    }

    #[test]
    fn anchor_function_builds_a_place_the_translator_resolves() {
        let code = renders(
            r#"(box (id b))
               (defrule tag (box (id ?i) (x nil)) => (assert (dot (at-rel (anchor ?i ne) 0.1 0.1)))
                 (assert (text (label "n") (at (anchor ?i n)))))"#,
        );
        assert_eq!(code, "B: box\ndot at B.ne + (0.1, 0.1)\ntext \"n\" at B.n\n");
    }

    #[test]
    fn coords_and_between_build_places_the_translator_resolves() {
        let code = renders(
            r#"(box (id a))
               (box (id b) (at (coords 3 1)))
               (defrule mid (box (id a) (x nil)) (box (id b) (x nil))
                 => (assert (dot (at (between 0.5 a b)))))"#,
        );
        assert_eq!(code, "A: box\nB: box at (3,1)\ndot at 0.5 between A and B\n");
    }
    
    #[test]
    fn errors_before_layout_are_forgiven_when_the_next_pass_is_clean() {
        let code = renders(
            r#"(box (id a))
               (circle (id c))
               (defrule move-circle (box (id a) (x ?bx) (y ?y)) ?c <- (circle (id c) (at-pos))
                 => (modify-at-x ?c (+ ?bx 0.2)))"#,
        );
        assert!(code.contains("C: circle at 0.2, 0"), "{code}");
    }

    #[test]
    fn errors_in_the_last_pass_are_reported() {
        let error = eval_clips(
            "(box (id a))\n(defrule bad (box (id a)) => (+ a 1))",
            DEFAULT_RULE_LIMIT,
        )
        .expect_err("persistent rule error must surface");
        assert!(error.contains("ARGACCES2"), "{error}");
    }

    fn sources() -> Sources {
        let mut sources = Sources::new();
        sources.insert(
            "notes".to_string(),
            EditorSource {
                raw: Some("first \"quoted\"\nsecond".to_string()),
                generated: None,
                output_type: None,
            },
        );
        sources.insert(
            "other".to_string(),
            EditorSource {
                raw: Some("box \"O\"".to_string()),
                generated: Some("O: box \"O\"\n".to_string()),
                output_type: Some(OutputType::Pikchr),
            },
        );
        sources.insert(
            "ascii".to_string(),
            EditorSource {
                raw: Some("+--+".to_string()),
                generated: Some("<svg/>".to_string()),
                output_type: Some(OutputType::Svgbob),
            },
        );
        sources
    }

    fn with_sources(source: &str, output_type: OutputType) -> Result<String, String> {
        eval_clips_with(source, DEFAULT_RULE_LIMIT, output_type, &sources())
    }

    #[test]
    fn raw_facts_pass_text_through() {
        let code = with_sources(
            "(box (id b))\n(raw-text (text \"# a comment\") (order 1))\n(raw-pikchr (text \"line from\" b.e \"right\") (order -1))",
            OutputType::Pikchr,
        )
        .unwrap();
        assert_eq!(code, "line from B.e right\nB: box\n# a comment\n");
        let svgbob = with_sources("(raw-text (text \"+--+\"))\n(raw-pikchr (text \"box\"))", OutputType::Svgbob).unwrap();
        assert_eq!(svgbob, "+--+\n");
    }

    #[test]
    fn text_from_and_lines_from_become_source_facts() {
        let code = with_sources(
            r#"(text-from notes)
               (lines-from "notes")
               (defrule whole (source-text notes ?t) => (printout t "text=" ?t crlf))
               (defrule each (source-line notes ?n ?l) => (assert (box (id (sym-cat l ?n)) (label ?l))))"#,
            OutputType::Pikchr,
        )
        .unwrap();
        assert!(code.contains("L1: box \"first \\\"quoted\\\"\""), "{code}");
        assert!(code.contains("L2: box \"second\""), "{code}");
        assert!(code.contains("# text=first \"quoted\""), "{code}");
    }

    #[test]
    fn rules_may_request_sources_late() {
        let code = with_sources(
            r#"(deftemplate want (slot editor))
               (defrule ask (want (editor ?e)) => (assert (text-from ?e)))
               (defrule show (source-text ?e ?t) => (assert (box (id ?e) (label ?t))))
               (want (editor notes))"#,
            OutputType::Pikchr,
        )
        .unwrap();
        assert!(code.starts_with("Notes: box \"first"), "{code}");
    }

    #[test]
    fn pikchr_from_becomes_a_measured_group() {
        let code = with_sources(
            r#"(pikchr-from other)
               (box (id b) (at-rel other.e 1 0))
               (defrule measured (group (id other) (x ?x&~nil)) => (printout t "measured" crlf))"#,
            OutputType::Pikchr,
        )
        .unwrap();
        assert_eq!(code, "Other: [\nO: box \"O\"\n]\nB: box at Other.e + (1, 0)\n# measured\n");
        let svg = crate::render::render(OutputType::Pikchr, &code).unwrap();
        assert!(svg.starts_with("<svg"), "{svg}");
    }

    #[test]
    fn groups_can_be_asserted_by_hand() {
        let code = renders("(group (id g) (text \"box\" \"circle\") (with nw) (at-pos 0 0))");
        assert_eq!(code, "G: [\nbox\ncircle\n] with .nw at 0, 0\n");
    }

    #[test]
    fn missing_or_mismatched_editors_are_errors() {
        let error = with_sources("(text-from nope)", OutputType::Pikchr).unwrap_err();
        assert!(error.contains("nope"), "{error}");
        let error = with_sources("(pikchr-from ascii)", OutputType::Pikchr).unwrap_err();
        assert!(error.contains("ascii"), "{error}");
    }

    #[test]
    fn references_are_found_bare_quoted_and_nested() {
        let heads = &["text-from", "lines-from"];
        assert!(references_editor("(text-from notes)", "notes", heads));
        assert!(references_editor("(lines-from \"my notes\")", "my notes", heads));
        assert!(references_editor("(defrule r => (assert (text-from\n  notes)))", "notes", heads));
        assert!(!references_editor("(text-from notes2)", "notes", heads));
        assert!(!references_editor("(text-fromnotes)", "notes", heads));
        assert!(!references_editor("(pikchr-from notes)", "notes", heads));
    }

    #[test]
    fn include_splices_another_editors_forms() {
        let mut sources = sources();
        sources.insert(
            "lib".to_string(),
            EditorSource {
                raw: Some("(deftemplate step (slot n))\n(defrule draw (step (n ?n)) => (assert (box (id (sym-cat s ?n)))))\n(include \"helpers\")".to_string()),
                generated: None,
                output_type: None,
            },
        );
        sources.insert(
            "helpers".to_string(),
            EditorSource { raw: Some("(deffacts seed (step (n 1)))".to_string()), generated: None, output_type: None },
        );
        let code = eval_clips_with("(include lib)\n(step (n 2))", DEFAULT_RULE_LIMIT, OutputType::Pikchr, &sources).unwrap();
        assert_eq!(code, "S2: box\nS1: box\n");
        let error = eval_clips_with("(include nope)", DEFAULT_RULE_LIMIT, OutputType::Pikchr, &sources).unwrap_err();
        assert!(error.contains("nope"), "{error}");
    }

    #[test]
    fn include_takes_the_output_of_a_text_output_editor() {
        let mut sources = Sources::new();
        sources.insert(
            "facts".to_string(),
            EditorSource {
                raw: Some("puts '(box (id a))'".to_string()),
                generated: Some("(box (id a))\n(box (id b))\n".to_string()),
                output_type: Some(OutputType::Text),
            },
        );
        let code = eval_clips_with("(include facts)", DEFAULT_RULE_LIMIT, OutputType::Pikchr, &sources).unwrap();
        assert_eq!(code.matches("box").count(), 2, "{code}");
    }

    #[test]
    fn include_cycles_are_errors() {
        let mut sources = Sources::new();
        sources.insert("a".to_string(), EditorSource { raw: Some("(include b)".to_string()), generated: None, output_type: None });
        sources.insert("b".to_string(), EditorSource { raw: Some("(include a)".to_string()), generated: None, output_type: None });
        let error = eval_clips_with("(include a)", DEFAULT_RULE_LIMIT, OutputType::Pikchr, &sources).unwrap_err();
        assert!(error.contains("a > b"), "{error}");
    }

    #[test]
    fn numeric_editor_names_assert_as_numbers() {
        let mut sources = Sources::new();
        sources.insert("1179".to_string(), EditorSource { raw: Some("hi".to_string()), generated: Some("box \"hi\"".to_string()), output_type: Some(OutputType::Pikchr) });
        let code = eval_clips_with(
            "(text-from 1179)\n(pikchr-from 1179)\n(defrule r (source-text 1179 ?t) (group (id 1179) (x nil)) => (assert (box (label ?t))))",
            DEFAULT_RULE_LIMIT,
            OutputType::Pikchr,
            &sources,
        )
        .unwrap();
        assert_eq!(code, "L1179: [\nbox \"hi\"\n]\nbox \"hi\"\n");
    }
}
