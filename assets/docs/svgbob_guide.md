# Svgbob guide

Svgbob turns ASCII art into vector drawings. Characters that look like lines,
corners, and arrows are drawn as lines, corners, and arrows. Everything else
stays text.

Select a drawing to see its source. Select the source to see the drawing.

## Lines

Dashes and pipes are lines. Slashes are diagonals. Lines join where they meet.

~~~ svgbob toggle
 -----------      |      /      \
                  |     /        \
                  |    /          \
~~~

Dashed lines use spaced dashes or colons.

~~~ svgbob toggle
 - - - - - -      :
                  :
                  :
~~~

## Corners and boxes

`+` makes a sharp corner. `.` at the top and `'` at the bottom make rounded ones.

~~~ svgbob toggle
 +-------+     .-------.     +-------.
 |       |     |       |     |       |
 |       |     |       |     |       |
 +-------+     '-------'     '-------+
~~~

Boxes can touch and share edges. Text inside a box stays text.

~~~ svgbob toggle
 +---------+---------+
 |  left   |  right  |
 +---------+---------+
 |      bottom       |
 +-------------------+
~~~

## Arrows

`<`, `>`, `^`, and `v` at the end of a line make arrowheads.

~~~ svgbob toggle
 ---->    <----    <---->

   ^        |
   |        |
   |        v
~~~

Route arrows between boxes with corners.

~~~ svgbob toggle
 +-------+        +-------+
 | start |------->| step  |
 +-------+        +---+---+
                      |
                      v
                  +-------+
                  |  end  |
                  +-------+
~~~

## Line endings

At the end of a line, `*` makes a filled dot, `o` an open dot, and `#` a filled square.

~~~ svgbob toggle
 *-------o     o-------*     #-------#

 *       o
 |       |
 |       |
 o       *
~~~

## Circles and shapes

Svgbob recognizes circles from a fixed set of patterns. The bottom-left corner
is a backtick and the bottom-right is a quote. Parentheses form the sides.

~~~ svgbob toggle
   ()       (_)       ,-.        .--.
                     (   )      (    )
                      `-'        `--'
~~~

Larger circles add a `.' '.` top row and slashes for the sides.

~~~ svgbob toggle
      _            ___              ____
    .' '.        ,'   `.          .'    `.
   (     )      /       \        /        \
    `._.'       \       /       (          )
                 `.___.'         \        /
                                  `.____.'
~~~

Diamonds come from slashes.

~~~ svgbob toggle
     .-.            /\
    /   \          /  \
   (     )        /    \
    \   /         \    /
     '-'           \  /
                    \/
~~~

## Text

Characters that aren't a line or a shape render as text. A dash, slash, or
pipe inside a word still draws as a line, so add spaces or avoid the character.

~~~ svgbob toggle
 +--------------------+
 |  name: value       |
 |  key: 1  size 10%  |
 +--------------------+
~~~

## Branching and trees

~~~ svgbob toggle
            +------+
            | root |
            +--+---+
               |
       +-------+-------+
       |               |
   +---+---+       +---+---+
   | left  |       | right |
   +-------+       +-------+
~~~

## Sequence style

Vertical lifelines with horizontal messages.

~~~ svgbob toggle
  client          backend
    |                |
    |---- request -->|
    |                |
    |<-- response ---|
    |                |
~~~

## Tips

  *  Draw lines in Replace mode so text doesn't shift. Type labels in Insert mode.
  *  Keep 1 space between a box edge and its label so the edge isn't read as
     part of the word.
  *  A `+` with 1 neighbor draws as a plus sign, not a corner.
  *  Align columns with spaces, not tabs.
  *  Keep letters that double as arrowheads, such as `v`, off lines. Move the
     label or the line 1 column.

## Composition in DiagramIDE

`!!NAME!!` inserts the raw text of another editor before rendering.

`$$NAME$$` inserts the generated source of another Svgbob editor.

An overlay draws one editor on top of another, column by column, without
adding lines. Declare the marker at the top of the canvas:

~~~
9 = Badge
AAA  9
AAA
AAA
~~~

The rows of the editor named `Badge` replace every `9` column. Keep small
repeated symbols in one editor this way.
