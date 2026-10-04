# Svgbob Guide

Svgbob turns ASCII art into clean vector drawings. Characters that look like
lines, corners, and arrows become lines, corners, and arrows. Everything else
stays as text.

Every example below is live: click a drawing to see its source, click the
source to see the drawing again.

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

`+` makes a sharp corner. `.` on top and `'` at the bottom make rounded ones.

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

Arrows connect boxes. Route them with corners.

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

`*` makes a filled dot, `o` an open dot, `#` a filled square at the end of a line.

~~~ svgbob toggle
 *-------o     o-------*     #-------#

 *       o
 |       |
 |       |
 o       *
~~~

## Circles and shapes

Circles are recognised from a fixed set of patterns. The bottom-left corner
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

Any run of characters that is not a line or a shape is rendered as text in
the diagram font. A dash, slash, or pipe inside a word is still drawn as a
line, so write such labels with spaces or avoid the character.

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

  *  Use the Svgbob editor's block cursor and Replace mode to draw lines
     without shifting text. Insert mode is for typing labels.
  *  Keep one space between a box edge and its label so the edge is not read
     as part of the word.
  *  A `+` with only one neighbour is drawn as a plus sign, not a corner.
  *  Align columns with spaces, never tabs.
  *  Letters that double as arrowheads, such as `v`, must not sit on a line.
     Move the label or the line one column.

## Composition in DiagramIDE

`!!NAME!!` pastes the raw text of another editor into the canvas before
rendering.

`$$NAME$$` pastes the generated source of another Svgbob editor.

Overlays place one editor's drawing on top of another, column by column,
without adding lines. Declare the marker at the top of the canvas:

~~~
9 = Badge
AAA  9
AAA
AAA
~~~

Every `9` column is replaced by the rows of the editor named `Badge`, so
small repeated symbols can be kept in one place.
