# CLIPS Guide

The CLIPS editor describes a diagram as facts. Each fact names a shape and
fills in its slots; rules can look at the facts and assert more. When the
rules settle, every shape fact becomes one Pikchr statement, and the paired
Render window draws the result.

CLIPS is a rule engine: facts are data, rules fire when facts match their
patterns. Nothing beyond that is needed to draw, but everything in CLIPS 6.4.2
is available.

Every example below is live: click a drawing to see its source, click the
source to see the drawing again.

## Program shape

A program is a list of forms. Facts whose head is a shape name are asserted.
Constructs (`defrule`, `deftemplate`, `deffacts`, `deffunction`,
`defglobal`) are built first, whatever their position. Any other form is a
command and is evaluated in place.

~~~ clips toggle source
(box (id b) (label "Hello"))
(circle (id c) (label "World"))
(arrow (from b) (to c))
~~~

The order of evaluation is fixed:

  1.  Constructs are built.
  2.  The environment is reset, so `deffacts` and `initial-fact` exist.
  3.  Facts and commands run top to bottom.
  4.  Rules fire until the agenda is empty, up to 10000 firings.
  5.  Shape facts are written out, in assertion order.

Comments start with `;`. Strings use double quotes. Symbols need no quotes,
so `lightgray`, `2cm`, and `150%` are plain symbols.

## Shapes and ids

Every shape template takes an `id`. The id becomes the Pikchr label, with the
first letter capitalised and other characters Pikchr rejects turned into `_`:
`(id b)` is `B`, `(id my-box)` is `My_box`. Shapes without an id get no label.

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
| `id` | label | `(id b)` → `B:` |
| `label` | quoted text, multislot | `(label "line 1" "line 2")` |
| `at` | `at` place | `(at "B.s + (0, -1)")` |
| `with` | `with .edge at` | `(with nw) (at b.se)` |
| `same` | `same as` | `(same b)` |
| `color` | `color` | `(color red)` |
| `fill` | `fill` | `(fill lightgray)` |
| `thickness` | `thickness` | `(thickness 0.05)` |
| `dashed` | `dashed`, optionally with a length | `(dashed yes)`, `(dashed 0.1)` |
| `dotted` | `dotted`, optionally with a length | `(dotted yes)`, `(dotted 0.05)` |
| `invisible` | `invisible` | `(invisible yes)` |
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

`heads` is `->`, `<-`, or `<->`. An `arrow` already has `->`; use `line` for
no arrowhead.

## Places and anchors

In `at`, `from`, `to`, `same`, `then`, and `attrs`, a token that names a fact
is replaced by its label, so `b` becomes `B` and `b.ne` becomes `B.ne`. Any
other text passes through to Pikchr, so arithmetic on places works as it does
in Pikchr. Quote a place when it contains spaces.

An `anchor` fact gives an edge of a shape its own name. It draws nothing.

~~~ clips toggle source
(box (id b) (label "B"))
(anchor (id corner) (obj b) (dir NE))
(line (from corner) (dir N) (length 150%))
(dot (at corner) (color red))
~~~

The anchor `dir` is a Pikchr edge: `N`, `NE`, `E`, `SE`, `S`, `SW`, `W`,
`NW`, `C`, or for lines `start` and `end`. Case does not matter.

## Directions

`dir` with an optional `length` moves the pen. Cardinal points and Pikchr's
own words become Pikchr directions; diagonals and numbers use `go`:

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

A rule has patterns on the left of `=>` and actions on the right. Patterns
match facts, including shape facts, and actions usually assert more. Facts a
rule asserts take their place after the facts asserted before the rules ran.

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

Rules fire from the most recent activation, so the three boxes would be
asserted as 3, 2, 1. `(order ?n)` writes them in step order instead. Every
fact has `order` 0 unless set, so the arrows take `(order 10)` to come after
the boxes: Pikchr can only refer to a label that is already defined. The
negative salience is not needed for the output, but it keeps the agenda
readable when watching rules fire.

Some functions worth knowing in actions: `sym-cat` and `str-cat` join
values into a symbol or a string, `gensym*` makes a fresh symbol for an id,
`+` `-` `*` `/` do arithmetic, and `printout t ... crlf` writes text that
appears as `#` comments at the end of the generated Pikchr.

### Salience and ordering

Rules fire in agenda order. `declare (salience N)` runs a rule earlier than
rules with lower salience. The `order` slot sorts the output independently of
when a fact was asserted, so a rule that fires late can still draw first.
Here the backgrounds are asserted after the frames but written before them,
so the frames are drawn on top.

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

`deffacts` lists facts that exist after every reset. Since the environment is
reset after constructs are built, these facts are present before the forms
below them run. Constructs are built in source order, so a `deffacts` must
come after the `deftemplate` it uses.

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

When a slot is missing, `attrs` appends raw attributes to one statement and a
`pikchr` fact writes a whole raw line. Both substitute fact ids.

~~~ clips toggle source
(box (id b) (label "B") (attrs "rad 0.15" "thick"))
(pikchr (text "circle at" b.e "+ (1.5cm, 0) rad 0.3 fill orange"))
(pikchr (text "arrow from" b.e "to last circle.w chop"))
~~~

An attribute Pikchr rejects shows its error beside the editor, which is the
quickest way to check what Pikchr accepts.

## Errors and limits

  *  CLIPS parse and runtime errors appear beside the editor with the CLIPS
     message, such as `[TMPLTDEF1] Invalid slot 'nosuch'`.
  *  A form whose head is neither a template nor a function is an error.
  *  Rule firing stops after 10000 firings with an error, so a rule that
     asserts a fact matching its own pattern does not hang the editor.
  *  Loops inside a single rule action are not limited. Avoid
     `loop-for-count` with large bounds.
  *  `(printout t ...)` output becomes `#` comment lines after the diagram.

## Template reference

These templates exist in every CLIPS editor. Unset slots default to `nil` and
are omitted from the output.

~~~
(deftemplate box|circle|ellipse|oval|cylinder|file|diamond|dot|text
  (slot id) (slot order (default 0)) (multislot label)
  (slot at) (slot with) (slot same)
  (slot color) (slot fill) (slot thickness) (slot dashed) (slot dotted)
  (slot invisible) (multislot style) (multislot attrs)
  (slot width) (slot height) (slot radius) (slot diameter) (slot fit))

(deftemplate arrow|line|spline|arc
  (slot id) (slot order (default 0)) (multislot label)
  (slot at) (slot with) (slot same)
  (slot color) (slot fill) (slot thickness) (slot dashed) (slot dotted)
  (slot invisible) (multislot style) (multislot attrs)
  (slot from) (slot to) (slot dir) (slot length) (slot heads)
  (slot chop) (slot radius) (multislot then))

(deftemplate move
  (slot id) (slot order (default 0)) (slot dir) (slot length)
  (slot at) (slot from) (slot to) (multislot then) (multislot attrs))

(deftemplate anchor (slot id) (slot obj) (slot dir (default c)))
(deftemplate direction (slot order (default 0)) (slot dir))
(deftemplate pikchr (slot order (default 0)) (multislot text))
~~~

User templates, rules, functions, and globals are ordinary CLIPS. See the
CLIPS Basic Programming Guide for the language itself.
