//! CLIPS output: facts asserted against the diagram templates become Pikchr.
//!
//! The editor content is split into top-level forms. Constructs (`defrule`,
//! `deftemplate`, ...) are built first, then the environment is reset so
//! `deffacts` take effect, then the remaining forms run in order: a form whose
//! head names a deftemplate is asserted as a fact, anything else is evaluated
//! as a command. Finally the rules run and every fact whose relation is a
//! diagram template is written out as one Pikchr statement, in fact order.
//!
//! A fresh [`Environment`] is created for every evaluation. CLIPS environments
//! are thread-affine, so evaluation happens on one blocking thread.

use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    rc::Rc,
};

use clips_bindings::{Environment, Fact, Router, Value};
use tokio::task;

/// Rule firings allowed per evaluation. Rule bases that never settle are
/// reported instead of hanging the editor.
pub const DEFAULT_RULE_LIMIT: u64 = 10_000;

/// Templates for closed shapes: labels inside, sized by width/height/radius.
const BLOCK_SHAPES: &[&str] = &[
    "box", "circle", "ellipse", "oval", "cylinder", "file", "diamond", "dot", "text",
];
/// Templates for open shapes: paths with a start, an end, and arrowheads.
const LINE_SHAPES: &[&str] = &["arrow", "line", "spline", "arc"];

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
     (slot at) (slot with) (slot same) \
     (slot color) (slot fill) (slot thickness) (slot dashed) (slot dotted) \
     (slot invisible) (multislot style) (multislot attrs)";
const BLOCK_SLOTS: &str = "(slot width) (slot height) (slot radius) (slot diameter) (slot fit)";
const LINE_SLOTS: &str = "(slot from) (slot to) (slot dir) (slot length) (slot heads) \
     (slot chop) (slot radius) (multislot then)";

/// The deftemplates every CLIPS editor starts with.
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
    out
}

/// Names of every template the translator writes out or resolves.
fn diagram_templates() -> BTreeSet<&'static str> {
    BLOCK_SHAPES
        .iter()
        .chain(LINE_SHAPES)
        .chain(["move", "anchor", "direction", "pikchr"].iter())
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
pub async fn safe_eval_clips(source: String) -> Result<String, String> {
    task::spawn_blocking(move || {
        std::panic::catch_unwind(|| eval_clips(&source, DEFAULT_RULE_LIMIT))
            .map_err(|_| "CLIPS environment panicked".to_string())?
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Evaluate `source` and translate the resulting facts to Pikchr.
pub fn eval_clips(source: &str, rule_limit: u64) -> Result<String, String> {
    let forms = split_forms(source)?;
    let capture = SharedCapture::default();
    let mut env = Environment::new().map_err(|error| error.to_string())?;
    env.register_router("diagramide", 40, capture.clone())
        .map_err(|error| format!("{error:?}"))?;

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
        if env.build(form).is_err() {
            return Err(fail(&capture, form, "Construct rejected".to_string()));
        }
    }
    env.reset();

    for form in commands {
        let head = head(form);
        let is_template = env
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

    let fired = env.run(Some(rule_limit));
    if fired >= rule_limit {
        return Err(format!("Rule firing limit of {rule_limit} reached"));
    }
    let errors = capture.0.borrow().errors.trim().to_string();
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut pikchr = facts_to_pikchr(&env)?;
    let stdout = std::mem::take(&mut capture.0.borrow_mut().stdout);
    for line in stdout.lines() {
        pikchr.push_str("# ");
        pikchr.push_str(line);
        pikchr.push('\n');
    }
    Ok(pikchr)
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
                    return Err(format!("Unbalanced ')' at byte {index}"));
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

fn statement(row: &Row, names: &Names) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    match row.relation.as_str() {
        "anchor" => return None,
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
    if let Some(at) = row.single("at") {
        match row.single("with") {
            Some(with) => parts.push(format!(
                "with .{} at {}",
                edge_name(with.trim_start_matches('.')),
                names.substitute(at)
            )),
            None => parts.push(format!("at {}", names.substitute(at))),
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
    parts.extend(row.multi("style").iter().cloned());
    parts.extend(row.multi("attrs").iter().map(|a| names.substitute(a)));

    let mut line = parts.join(" ");
    if let Some(id) = row.single("id") {
        line = format!("{}: {line}", names.labels[id]);
    }
    Some(line)
}

/// Write every diagram fact as a Pikchr statement, ordered by `order` then by
/// assertion.
fn facts_to_pikchr(env: &Environment) -> Result<String, String> {
    let templates = diagram_templates();
    let mut rows = Vec::new();
    for fact in env.facts() {
        let row = snapshot(&fact)?;
        if templates.contains(row.relation.as_str()) {
            rows.push(row);
        }
    }
    rows.sort_by_key(|row| (row.order, row.index));
    let names = Names::new(&rows);
    let mut out = String::new();
    for row in &rows {
        if let Some(statement) = statement(row, &names) {
            out.push_str(&statement);
            out.push('\n');
        }
    }
    Ok(out)
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
            "B: box width 2cm fill lightgray dashed thick rad 0.1\n\
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
               (defrule hello (box (id ?x)) => (printout t "drew " ?x crlf))"#,
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
}
