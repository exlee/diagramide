# Hagoromo guide

Hagoromo is a diagram language in the style of Haskell Diagrams. A diagram is
a value. Operators combine small values into larger ones. The Render window
shows the final value of the script.

You write scripts in Gluon, a small typed language embedded in DiagramIDE.
Select a drawing to see its source. Select the source to see the drawing.

## Script shape

A script imports the Hagoromo prelude, binds what it needs from `prim`, and
ends with 1 expression of type Diagram.

~~~ hagoromo toggle source
let { prim, (<>), (|||), (===), (|>) } = import! hagoromo
let { circle, square, fc, color } = prim

circle 1.0 ||| (square 2.0 |> fc color.red)
~~~

The prelude exports 5 names:

| Name | Meaning |
|---|---|
| `prim` | Record with every primitive, listed under Reference |
| `a <> b` | Draw `b` on top of `a`, both centered on the same origin |
| `a \|\|\| b` | Place `b` to the right of `a`, touching |
| `a === b` | Place `b` below `a`, touching |
| `x \|> f` | Apply `f` to `x`, left to right like a method chain |

`|>` binds tighter than the 3 layout operators, so
`a <> b |> fc color.red` fills only `b`.

Gluon rules:

  *  Numbers used as sizes are floats. Write `1.0`, not `1`.
  *  Negative numbers need parentheses in argument position: `translate (-1.0) 0.0`.
  *  Comments start with `//`.
  *  `let name = value` binds a name. `let f x y = body` defines a function.
  *  Lists use square brackets: `[circle 1.0, square 1.0]`.

## Shapes

Shapes are centered on the origin, with a black stroke and no fill.

~~~ hagoromo toggle
let { prim, (|||) } = import! hagoromo
let { circle, square, rect, equilateral_triangle, reg_poly, hcat_sep } = prim

hcat_sep 0.5 [circle 1.0, square 2.0, rect 3.0 1.5, equilateral_triangle 2.0, reg_poly 6 1.0]
~~~

| Primitive | Arguments | Result |
|---|---|---|
| `circle r` | radius | circle |
| `square s` | side | square |
| `rect w h` | width, height | rectangle |
| `equilateral_triangle s` | side | triangle pointing up |
| `reg_poly n s` | sides, side length | regular polygon |
| `polygon points` | list of `point x y` | closed polygon |
| `polyline points` | list of `point x y` | open path |
| `text s size` | string, font size | text centered on origin |
| `strut_x w`, `strut_y h` | length | invisible spacer |
| `diagram_empty` | | empty diagram |

~~~ hagoromo toggle
let { prim } = import! hagoromo
let { polygon, polyline, point, hcat_sep } = prim

let arrow = polygon [point 0.0 0.0, point 2.0 0.0, point 2.0 (-0.5), point 3.0 0.5, point 2.0 1.5, point 2.0 1.0, point 0.0 1.0]
let zig = polyline [point 0.0 0.0, point 0.5 1.0, point 1.0 0.0, point 1.5 1.0, point 2.0 0.0]

hcat_sep 1.0 [arrow, zig]
~~~

## Styling

Style functions take the diagram last, so they chain with `|>`.

~~~ hagoromo toggle
let { prim, (|||), (|>) } = import! hagoromo
let { circle, fc, lc, lw, opacity, dashing, color, hcat_sep } = prim

hcat_sep 0.5 [
    circle 1.0 |> fc color.red,
    circle 1.0 |> lc color.blue |> lw 0.15,
    circle 1.0 |> fc color.green |> opacity 0.4,
    circle 1.0 |> dashing [0.3, 0.15] 0.0
]
~~~

| Function | Effect |
|---|---|
| `fc c` | fill color. Alias `fill_color` |
| `lc c` | stroke color. Alias `stroke_color` |
| `lw w` | stroke width in diagram units. Alias `stroke_width` |
| `opacity a` | opacity 0.0 to 1.0, applied to the whole subtree. Use it for translucency. Color alpha doesn't render |
| `dashing pattern offset` | dashed stroke. `pattern` is a list of on/off lengths |
| `bg c` | paint a background rectangle behind the diagram |
| `bold` | bold text inside the diagram |
| `font_family name` | font for text inside the diagram |

### Colors

`color` is a record of named colors and constructors.

| Name | Value |
|---|---|
| `color.black`, `color.white`, `color.red`, `color.green`, `color.blue`, `color.silver` | fixed colors |
| `color.transparent` | no paint |
| `color.rgb r g b` | components 0.0 to 1.0 |
| `color.rgb_bytes r g b` | components 0 to 255 |
| `color.from_hex "#rrggbb"` | CSS hex string |

~~~ hagoromo toggle
let { prim, (|>) } = import! hagoromo
let { square, fc, lw, color, hcat } = prim

let swatch c = square 1.0 |> fc c |> lw 0.0

hcat [
    swatch (color.rgb 0.9 0.3 0.2),
    swatch (color.rgb_bytes 40 120 200),
    swatch (color.from_hex "#f5c518"),
    swatch color.silver
]
~~~

## Combining diagrams

Diagrams compose by bounding box. Placing one beside another moves it until
the boxes touch.

~~~ hagoromo toggle
let { prim, (<>), (|||), (===), (|>) } = import! hagoromo
let { circle, square, fc, color } = prim

let a = circle 1.0 |> fc color.silver
let b = square 2.0 |> fc color.red

(a ||| b) === (b ||| a) === (a <> b)
~~~

| Function | Effect |
|---|---|
| `atop a b` | same as `a <> b` |
| `beside_right a b` | same as `a \|\|\| b` |
| `beside_down a b` | same as `a === b` |
| `beside dir a b` | place `b` next to `a` in direction `dir` |
| `hcat ds`, `vcat ds` | row or column of touching diagrams |
| `hcat_sep gap ds`, `vcat_sep gap ds` | row or column with a gap |

Directions for `beside` live in `direction`: `direction.right`,
`direction.left`, `direction.up`, `direction.down`.

~~~ hagoromo toggle
let { prim, (|>) } = import! hagoromo
let { circle, square, beside, direction, fc, color } = prim

let dot = circle 0.4 |> fc color.blue
let base = square 2.0

beside direction.up base dot
~~~

## Transforms

Transforms take the diagram last.

| Function | Effect |
|---|---|
| `translate dx dy` | move |
| `translate_x dx`, `translate_y dy` | move along one axis |
| `scale f` | uniform scale |
| `scale_xy fx fy` | scale per axis |
| `rotate radians` | rotate about the origin |
| `rotate_by turns` | rotate by a fraction of a full turn |
| `reflect_x`, `reflect_y` | mirror |

~~~ hagoromo toggle
let { prim, (<>), (|>) } = import! hagoromo
let { square, rotate_by, fc, lw, opacity, color } = prim

let tile = square 2.0 |> lw 0.03

tile <> (tile |> rotate_by 0.125) <> (tile |> rotate_by 0.0625 |> fc color.blue |> opacity 0.3)
~~~

`<>` centers both operands, so a rotated copy sits on top of the original.
Translate before stacking to add an offset.

~~~ hagoromo toggle
let { prim, (<>), (|>) } = import! hagoromo
let { circle, translate, fc, color } = prim

let ring = circle 1.0 |> fc color.silver

ring <> (ring |> translate 1.0 0.0) <> (ring |> translate 0.5 0.9)
~~~

## Alignment

`hcat` and `vcat` line up the centers of their items. Align functions move a
diagram so a bounding box edge sits on the origin, which changes where it
lands in a row or column.

~~~ hagoromo toggle
let { prim, (|>) } = import! hagoromo
let { circle, square, hcat, align_bottom, fc, color } = prim

hcat [
    circle 0.5 |> fc color.red |> align_bottom,
    square 2.0 |> align_bottom,
    circle 1.0 |> fc color.blue |> align_bottom
]
~~~

Align functions: `align_left`, `align_right`, `align_top`, `align_bottom`, `center_x`, `center_y`.

## Trails

A trail is a path with no position of its own. `stroke_trail` turns it into a
diagram. Use trails to draw lines.

~~~ hagoromo toggle
let { prim, (<>), (|>) } = import! hagoromo
let { stroke_trail, hrule, vrule, trail_concat, lc, lw, color } = prim

let step = trail_concat (hrule 1.0) (vrule 1.0)
let stairs = trail_concat step (trail_concat step step)

stroke_trail stairs |> lc color.blue |> lw 0.1
~~~

| Function | Effect |
|---|---|
| `hrule len`, `vrule len` | horizontal or vertical segment |
| `trail_concat a b` | `b` continues where `a` ends |
| `trail_reflect_x t`, `trail_reflect_y t` | mirror |
| `trail_rotate_by turns t` | rotate |
| `stroke_trail t` | draw the trail as a diagram |

## Text

`text s size` draws `s` centered on the origin. The bounding box is estimated
from the character count, so leave room around labels.

~~~ hagoromo toggle
let { prim, (<>), (|||), (|>) } = import! hagoromo
let { circle, rect, text, fc, color, hcat_sep } = prim

let node label = circle 1.0 |> fc color.white <> text label 0.6
let box label = rect 3.0 1.4 |> fc (color.rgb 0.95 0.95 0.85) <> text label 0.5

hcat_sep 0.8 [node "a", box "process", node "b"]
~~~

## Reusing shapes

Define a shape once as a function and call it with different arguments.

~~~ hagoromo toggle
let { prim, (<>), (|>) } = import! hagoromo
let { circle, text, fc, color, hcat_sep, vcat_sep } = prim

let node label = circle 0.7 |> fc color.silver <> text label 0.5
let row labels = hcat_sep 0.4 labels

vcat_sep 0.6 [
    row [node "a", node "b", node "c"],
    row [node "d", node "e"]
]
~~~

## Composition in DiagramIDE

`!!NAME!!` inserts the raw text of another editor before the script runs.
Keep shared helper functions in a plain-text window and include them at the
top of each Hagoromo script.

`$$NAME$$` doesn't work in Hagoromo editors. A diagram value can't be
inserted into text.

## Reference

Everything in `prim`, grouped.

| Group | Names |
|---|---|
| Shapes | `circle`, `square`, `rect`, `equilateral_triangle`, `reg_poly`, `polygon`, `polyline`, `text`, `strut_x`, `strut_y`, `diagram_empty` |
| Style | `fc`, `fill_color`, `lc`, `stroke_color`, `lw`, `stroke_width`, `opacity`, `dashing`, `bg`, `bold`, `font_family`, `fill_gradient` |
| Layout | `atop`, `beside`, `beside_right`, `beside_down`, `hcat`, `vcat`, `hcat_sep`, `vcat_sep`, `position`, `appends` |
| Transform | `translate`, `translate_x`, `translate_y`, `scale`, `scale_xy`, `rotate`, `rotate_by`, `reflect_x`, `reflect_y` |
| Align | `align_left`, `align_right`, `align_top`, `align_bottom`, `center_x`, `center_y` |
| Trails | `hrule`, `vrule`, `trail_empty`, `trail_concat`, `trail_reflect_x`, `trail_reflect_y`, `trail_rotate_by`, `trail_len`, `trail_is_empty`, `stroke_trail` |
| Geometry | `point`, `point_x`, `point_y`, `vec2`, `vec2_x`, `vec2_y`, `vec2_length`, `rect_new`, `rect_width`, `rect_height` |
| Bounding box | `bbox`, `bbox_rect`, `bbox_union`, `bbox_translate`, `bbox_scale`, `bbox_from_rect`, `bbox_from_points`, `bbox_empty` |
| Records | `color`, `direction` |
