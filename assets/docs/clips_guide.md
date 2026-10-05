# CLIPS guide

The CLIPS editor describes a diagram as facts. Each fact names a shape and
fills in its slots. Rules match facts and assert more. When no rule is left to
fire, every shape fact becomes 1 Pikchr statement, and the paired Render
window draws the result.

All of CLIPS 6.4.2 is available.

Select a drawing to see its source. Select the source to see the drawing.

## Program shape

A program is a list of forms. A fact whose head is a shape name is asserted.
Constructs (`defrule`, `deftemplate`, `deffacts`, `deffunction`,
`defglobal`) are built first, wherever they appear. Any other form is a
command, evaluated in place.

~~~ clips toggle source
(box (id b) (label "Hello"))
(circle (id c) (label "World"))
(arrow (from b) (to c))
~~~

Evaluation order:

  1.  Constructs are built.
  2.  The environment is reset, so `deffacts` and `initial-fact` exist.
  3.  Facts and commands run top to bottom.
  4.  Rules fire until the agenda is empty, up to 10000 firings.
  5.  Shape facts are written out, in assertion order.

Comments start with `;`. Strings use double quotes. Symbols need no quotes,
so `lightgray`, `2cm`, and `150%` are plain symbols.

## Shapes and ids

Every shape template takes an `id`. The id becomes the Pikchr label, with the
first letter capitalized and characters Pikchr rejects turned into `_`:
`(id b)` is `B`, `(id my-box)` is `My_box`. A shape without an id gets no label.

~~~ clips toggle
(box (id b) (label "B"))
(circle (id c) (label "C") (at "B.e + (1cm, 0)"))
(diamond (id d) (label "D") (at "C.e + (1cm, 0)"))
(arrow (from b) (to c))
(arrow (from c) (to d))
~~~

The block shapes are `box`, `circle`, `ellipse`, `oval`, `cylinder`, `file`,
`diamond`, `dot`, and `text`. The line shapes are `arrow`, `line`, `spline`,
and `arc`. `move` moves the drawing position without drawing.

~~~ clips toggle
(box (label "box"))
(circle (label "circle"))
(ellipse (label "ellipse"))
(oval (label "oval"))
(direction (dir down))
(cylinder (label "cylinder") (at "1st box.s - (0, 1.2)"))
(file (label "file") (at "1st circle.s - (0, 1.2)"))
(diamond (label "diamond") (at "1st ellipse.s - (0, 1.2)"))
(dot (at "1st oval.s - (0, 1.2)"))
~~~

## Slots shared by every shape

| Slot | Pikchr | Example |
|---|---|---|
| `id` | label | `(id b)` becomes `B:` |
| `label` | quoted text, multislot | `(label "line 1" "line 2")` |
| `at` | `at` place | `(at "B.s + (0, -1)")`, `(at b.ne)` |
| `at-pos` | `at X, Y` | `(at-pos 1 0.5)` |
| `at-rel` | `at PLACE + (X, Y)` | `(at-rel b.s 0 -0.5)` |
| `with` | `with .edge at` | `(with nw) (at b.se)` |
| `same` | `same as` | `(same b)` |
| `color` | `color` | `(color red)` |
| `fill` | `fill` | `(fill lightgray)` |
| `thickness` | `thickness` | `(thickness 0.05)` |
| `dashed` | `dashed`, optionally with a length | `(dashed yes)`, `(dashed 0.1)` |
| `dotted` | `dotted`, optionally with a length | `(dotted yes)`, `(dotted 0.05)` |
| `invisible` | `invisible` | `(invisible yes)` |
| `x`, `y` | written after layout, see Positions | `(x ?x&~nil)` |
| `style` | raw words appended | `(style thick)`, `(style bold italic)` |
| `attrs` | raw attributes appended | `(attrs "rad 0.1" "behind B")` |
| `order` | sort key before assertion order | `(order -1)` |

Flags accept `yes`, `TRUE`, or `true`. Block shapes add `width`, `height`,
`radius`, `diameter`, and `fit`:

~~~ clips toggle
(box (label "wide") (width 2cm) (height 0.5cm) (fill lightblue))
(circle (label "r") (radius 0.3) (color red) (dashed yes))
(oval (label "fit") (fit yes) (thickness 0.04))
(text (label "bold" "and italic") (style bold italic) (at "last oval.e + (1.2cm, 0)"))
~~~

Line shapes add `from`, `to`, `dir`, `length`, `heads`, `chop`, `radius`,
and `then`:

~~~ clips toggle
(box (id a) (label "A"))
(box (id b) (label "B") (at "A.e + (3cm, 0)"))
(arrow (from a) (to b) (heads <->) (label "both"))
(line (from a.s) (dir S) (length 1cm) (then "right 3cm") (to b.s) (dashed yes))
(spline (from a.n) (then "up 1cm" "right 1.5cm") (to b.n) (heads ->) (color blue))
~~~

`heads` is `->`, `<-`, or `<->`. An `arrow` has `->` by default. Use `line`
for no arrowhead.

## Places and anchors

In `at`, `from`, `to`, `same`, `then`, and `attrs`, a token that names a fact
is replaced by its label: `b` becomes `B` and `b.ne` becomes `B.ne`. Other
text passes through to Pikchr, so place arithmetic works as in Pikchr. Quote
a place that contains spaces.

An `anchor` fact gives an edge of a shape its own name. It draws nothing.

~~~ clips toggle source
(box (id b) (label "B"))
(anchor (id corner) (obj b) (dir NE))
(line (from corner) (dir N) (length 150%))
(dot (at corner) (color red))
~~~

The anchor `dir` is a Pikchr edge: `N`, `NE`, `E`, `SE`, `S`, `SW`, `W`,
`NW`, `C`, or for lines `start` and `end`. Case doesn't matter.

## Positions

After the rules settle, the diagram is laid out once and every shape fact
gets the centre Pikchr gave it in its `x` and `y` slots, in Pikchr units
(inches). Then the rules run again, so a rule can react to where a shape
landed. If the rules change the diagram, it's laid out and measured again,
up to 4 times.

~~~ clips toggle source
(box (id a) (label "A"))
(box (id b) (label "B"))
(defrule mark-centres
  (box (id ?i) (x ?x&~nil) (y ?y))
  =>
  (assert (dot (id (sym-cat c- ?i)) (at (str-cat ?x ", " ?y)) (color red))))
(defrule report (declare (salience -10))
  (box (id ?i) (x ?x&~nil) (y ?y))
  =>
  (printout t ?i " at " ?x ", " ?y crlf))
~~~

`x` and `y` are outputs. To place a shape from coordinates, set its `at`
slot, or `at-pos`: `(at-pos X Y)` places the centre at the point `X, Y`, and
`(at-rel PLACE X Y)` offsets a place. `(anchor ?id ne)` returns the place
`id.ne` for use in `at`, `at-rel`, `from`, or `to`. Three functions set `at-pos`: `(modify-at ?f ?x ?y)` writes both coordinates, `(modify-at-x ?f ?x)` and `(modify-at-y ?f ?y)` write one
and keep the measured other. The one-coordinate forms fail with an error
until the shape has been measured.

~~~ clips toggle source
(box (id a) (label "Hello"))
(circle (id c) (label "World"))
(defrule circle-beside-box
  (box (id a) (x ?x&~nil) (y ?y))
  ?c <- (circle (id c) (at-pos))
  =>
  (modify-at ?c (+ ?x 1.5) ?y))
~~~

Writing a position changes the fact, so rules that match the shape fire
again. `(at-pos)` above keeps the rule from firing a second time. Match `(x nil)` in a rule that must run only before layout, and
`(x ?x&~nil)` in a rule that must run only after. Values you set in `x` and
`y` are overwritten; they're outputs, not attributes.

## Directions

`dir` with an optional `length` moves the pen. Cardinal points and Pikchr
direction words map to Pikchr directions. Diagonals and numbers use `go`:

| `dir` | Pikchr |
|---|---|
| `N`, `north`, `up` | `up` |
| `S`, `south`, `down` | `down` |
| `E`, `east`, `right` | `right` |
| `W`, `west`, `left` | `left` |
| `NE`, `NW`, `SE`, `SW` | `go ne` |
| a number | `go heading 30` |

~~~ clips toggle
(box (id b) (label "B"))
(arrow (from b.e) (dir E) (length 1cm))
(arrow (from b.n) (dir NE) (length 1cm))
(arrow (from b.s) (dir 200) (length 1cm))
(arrow (from b.w) (dir left) (length 1cm) (color gray))
~~~

A `direction` fact changes the layout direction for the shapes that follow,
like a bare `down` in Pikchr:

~~~ clips toggle
(box (label "1"))
(direction (dir down))
(box (label "2"))
(direction (dir right))
(box (label "3"))
~~~

## Rules

A rule has patterns left of `=>` and actions right of it. Patterns match
facts, including shape facts. Actions usually assert more facts. Facts a rule
asserts come after the facts asserted before the rules ran.

~~~ clips toggle source
(deftemplate step (slot n) (slot name))

(defrule draw-step
  (step (n ?n) (name ?name))
  =>
  (assert (box (id (sym-cat s ?n)) (label ?name) (order ?n))))

(defrule connect
  (declare (salience -10))
  (step (n ?a))
  (step (n ?b&:(= ?b (+ ?a 1))))
  =>
  (assert (arrow (from (sym-cat s ?a)) (to (sym-cat s ?b)) (order 10))))

(step (n 1) (name "fetch"))
(step (n 2) (name "parse"))
(step (n 3) (name "render"))
~~~

Rules fire from the most recent activation, so the boxes are asserted as 3,
2, 1. `(order ?n)` writes them in step order. `order` defaults to 0, so the
arrows use `(order 10)` to come after the boxes. Pikchr can refer only to a
label that's already defined. The negative salience doesn't change the
output. It keeps the agenda readable when you watch rules fire.

Useful functions in actions:

  *  `sym-cat` and `str-cat` join values into a symbol or a string.
  *  `gensym*` makes a new symbol for an id.
  *  `+` `-` `*` `/` do arithmetic.
  *  `printout t ... crlf` writes text as `#` comments at the end of the
     generated Pikchr.

### Salience and ordering

Rules fire in agenda order. `declare (salience N)` runs a rule before rules
with lower salience. The `order` slot sorts output regardless of when a fact
was asserted, so a rule that fires late can still draw first. Here the
backgrounds are asserted after the frames but written before them, so the
frames draw on top.

~~~ clips toggle
(deftemplate item (slot name) (slot x))

(defrule frame
  (declare (salience 10))
  (item (name ?n) (x ?x))
  =>
  (assert (box (id (sym-cat f ?n)) (label ?n) (at (str-cat "(" ?x ", 0)")))))

(defrule background
  (item (x ?x))
  =>
  (assert (box (at (str-cat "(" ?x ", 0)")) (width 1.2cm) (height 1cm)
               (fill lightyellow) (color lightyellow) (order -1))))

(item (name "a") (x 0))
(item (name "b") (x 2cm))
~~~

### Deffacts

`deffacts` lists facts that exist after every reset. The environment resets
after constructs are built, so these facts exist before the remaining forms
run. Constructs are built in source order, so put a `deffacts` after the
`deftemplate` it uses.

~~~ clips toggle
(deftemplate swatch (slot name))

(deffacts palette
  (swatch (name red))
  (swatch (name green))
  (swatch (name blue)))

(defrule draw-swatch
  (swatch (name ?c))
  =>
  (assert (box (label ?c) (fill ?c) (color ?c) (width 1cm) (height 0.6cm))))
~~~

## Raw Pikchr

When no slot fits, `attrs` appends raw attributes to 1 statement, and a
`pikchr` fact writes a whole raw line. Both substitute fact ids.

~~~ clips toggle source
(box (id b) (label "B") (attrs "rad 0.15" "thick"))
(pikchr (text "circle at" b.e "+ (1.5cm, 0) rad 0.3 fill orange"))
(pikchr (text "arrow from" b.e "to last circle.w chop"))
~~~

If Pikchr rejects an attribute, the error appears beside the editor.

## Editing

The editor runs Parinfer in smart mode. Indentation decides structure, and
closing parens follow it:

- Typing `(` adds its `)`.
- Indenting a line moves it into the form above. Dedenting moves it out.
- Moving an opening paren carries its indented lines along.

Text that arrives from a file or the library keeps its parens. Its
indentation is adjusted to match them. Inside an unclosed string, Parinfer
changes nothing until the string is closed.

## Other editors

A request fact reads another editor by editor name. Write the editor name
bare when the editor name is a single word or a number. Quote the editor
name otherwise.

| Request fact | Result |
|---|---|
| `(text-from NAME)` | asserts `(source-text NAME "text")` with the full text of editor NAME |
| `(lines-from NAME)` | asserts `(source-line NAME N "line")` for each line of editor NAME, with `N` counted from 1 |
| `(pikchr-from NAME)` | asserts `(group (id NAME) (text "pikchr"))` with the generated Pikchr of editor NAME |
| `(include NAME)` | reads the text of editor NAME as CLIPS source at the position of the request fact |

`include` is applied before constructs are built. Templates, rules, and
`deffacts` from editor NAME behave as if they were written in the current
editor. Included editors can include other editors, up to 8 levels.

`source-text` and `source-line` are ordered facts. A rule matches an ordered
fact by position: `(source-line notes ?n ?line)`. A rule can assert a
request fact. The source facts are asserted before the next rule pass. The
current editor is re-evaluated when editor NAME changes, as with `!!NAME!!`
and `$$NAME$$`.

~~~
(lines-from notes)
(defrule one-box-per-line
  (source-line notes ?n ?line)
  =>
  (assert (box (id (sym-cat l ?n)) (label ?line) (order ?n))))
~~~

`pikchr-from` requires an editor with Pikchr output. The `group` fact is a
Pikchr sub-diagram, written as `Name: [ ... ]`. A `group` fact accepts `at`,
`at-pos`, `at-rel`, and `with`, and receives `x` and `y` from the layout
pass. Assert a `group` fact to wrap Pikchr text:

~~~ clips toggle source
(group (id legend) (text "box \"Legend\"" "circle \"dot\""))
(box (id main) (label "Main") (at-rel legend.e 1 0))
~~~

Two output facts write text without a shape. Both have an `order` slot.

  *  `(raw-pikchr (text ...))` writes Pikchr. Fact ids in the text are
     replaced by labels. The fact is skipped when the output type is not
     Pikchr. `pikchr` is an alias of `raw-pikchr`.
  *  `(raw-text (text ...))` writes the text unchanged, for every output
     type.

## Errors and limits

  *  CLIPS parse and runtime errors appear beside the editor with the CLIPS
     message, such as `[TMPLTDEF1] Invalid slot 'nosuch'`.
  *  A form whose head is neither a template nor a function is an error.
  *  Rule firing stops with an error after 10000 firings, so a rule that
     asserts a fact matching its own pattern doesn't hang the editor.
  *  Loops inside 1 rule action have no limit. Avoid `loop-for-count` with
     large bounds.
  *  `(printout t ...)` output becomes `#` comment lines after the diagram.
  *  Errors raised while rules run are reported only from the last layout
     pass. A rule that fails before positions exist and succeeds after them
     is fine; a rule that fails on every pass is an error.

## Template reference

Every CLIPS editor has these templates. Unset slots default to `nil` and
don't appear in the output.

~~~
(deftemplate box|circle|ellipse|oval|cylinder|file|diamond|dot|text
  (slot id) (slot order (default 0)) (multislot label)
  (slot at) (multislot at-pos) (multislot at-rel) (slot with) (slot same)
  (slot color) (slot fill) (slot thickness) (slot dashed) (slot dotted)
  (slot invisible) (multislot style) (multislot attrs) (slot x) (slot y)
  (slot width) (slot height) (slot radius) (slot diameter) (slot fit))

(deftemplate arrow|line|spline|arc
  (slot id) (slot order (default 0)) (multislot label)
  (slot at) (multislot at-pos) (multislot at-rel) (slot with) (slot same)
  (slot color) (slot fill) (slot thickness) (slot dashed) (slot dotted)
  (slot invisible) (multislot style) (multislot attrs) (slot x) (slot y)
  (slot from) (slot to) (slot dir) (slot length) (slot heads)
  (slot chop) (slot radius) (multislot then))

(deftemplate move
  (slot id) (slot order (default 0)) (slot dir) (slot length)
  (slot at) (slot from) (slot to) (multislot then) (multislot attrs))

(deftemplate anchor (slot id) (slot obj) (slot dir (default c)))
(deftemplate direction (slot order (default 0)) (slot dir))
(deftemplate pikchr (slot order (default 0)) (multislot text))
(deftemplate raw-pikchr (slot order (default 0)) (multislot text))
(deftemplate raw-text (slot order (default 0)) (multislot text))
(deftemplate group (slot id) (slot order (default 0)) (multislot text)
  (slot at) (multislot at-pos) (multislot at-rel) (slot with) (slot x) (slot y))
~~~

Your own templates, rules, functions, and globals are standard CLIPS. For the
language, see the CLIPS Basic Programming Guide.
