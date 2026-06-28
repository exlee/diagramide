# Pikchr Grammar

This file describes the grammar of the input files to Pikchr.  Keywords
and operators are shown in **bold**.  Non-terminal symbols are shown
in *italic*.  Special token classes are shown in ALL-CAPS.  A grammar
symbol followed by "*" means zero-or-more.  A grammar symbol
followed by "?" means zero-or-one.  Parentheses are used for grouping.
Two grammar symbols within "(..|..)" means one or the other.
 Marks of the form
"&#9654;info" are links to more information and are
not part of the grammar.

The following special token classes are recognized:

  *  NEWLINE  &rarr;  A un-escaped newline character, U+000A.
     A backslash followed by zero or more whitespace characters
     and then a U+000A character is interpreted as ordinary whitespace,
     not as a NEWLINE.

  *  LABEL  &rarr;  An object or place label starting with an
     upper-case ASCII letter and continuing with zero or more
     ASCII letters, digits, and/or underscores.  A LABEL always starts
     with an upper-case letter.

  *  VARIABLE  &rarr;  A variable name consisting of a lower-case
     ASCII letter or "$" or "@" and followed by zero or more
     ASCII letters, digits, and/or underscores.  VARIABLEs may
     contain upper-case letters, but they never begin with an upper-case.
     In this way, VARIABLEs are distinct from LABELs.

  *  NUMBER  &rarr;  A numeric literal.  The value can be a decimal
     integer, a floating point value, or a hexadecimal literal
     starting with "0x".  Decimal and floating point values can
     optionally be followed by a two-character unit designator that is
     one of:  "in", "cm", "px", "pt", "pc", or "mm".  There can be
     no whitespace in between the numeric portion of the constant and
     the unit.

  *  ORDINAL  &rarr;  A non-zero integer literal followed by one of the
     suffixes "st", "nd", "rd", or "th".  Examples: "1st", "2nd",
    "3rd", "4th", "5th", and so forth.   As a special case, "first"
     is accepted as an alternative spelling of "1st".

  *  STRING  &rarr;  A string literal that begins and ends with
     double-quotes (U+0022).  Within the string literal, a double-quote
     character can be escaped using backslash (U+005c).  A backslash
     can also be used to escape a backslash.  No other escape sequences
     are recognized and standalone backslashes are elided from the output.
     Newlines are permitted in strings.

  *  COLORNAME &rarr;  One of the 140 official HTML color names, in
     any mixture of upper and lower cases.  The value of a COLORNAME is
     an integer which is the 24-bit RGB value of that color.  Two
     additional color names of "None" and "Off" are also recognized and
     have a value of -1.

  *  CODEBLOCK &rarr;   All tokens contained within nested {...}.  This
     is only used as the body of a "define" statement.

There are many non-terminals in the grammar, but a few are more important.
If you are new to the Pikchr language, begin by focusing on these
six:

  *  *statement* &rarr;  A Pikchr script is just a list of statements.

  *  *attribute* &rarr;  Each graphic object is configured with zero or
     more attributes.

  *  *object* &rarr;  A reference to a prior graphic object, or `this` to
     refer to the current object.

  *  *place* &rarr;  A specific point associated with an *object*.

  *  *position* &rarr;  Any (2-D) point in space.  An (x,y) pair.

  *  *expr* &rarr;  A scalar expression.


A complete input file to Pikchr consists of a single *statement-list*.

## *statement-list*: [&#9654;info](#reference-stmtlist.md)

  * *statement*?
  * *statement-list* NEWLINE *statement*?
  * *statement-list* **;** *statement*?

## *statement*:  [&#9654;info](#reference-stmt.md)
  * *object-definition*
  * LABEL **:** *object-definition*
  * LABEL **:** *place*
  * *direction*
  * VARIABLE *assignment-op* *expr*
  * **define** VARIABLE CODEBLOCK     [&#9654;info](#reference-macro.md)
  * **print** *print-argument* (**,** *print-argument*)\*
  * **assert (** *expr* **==** *expr* **)**
  * **assert (** *position* **==** *position* **)**


## *direction*:
  * **right**
  * **down**
  * **left**
  * **up**

## *assignment-op*:
  * **=**
  * **+=**
  * **-=**
  * **\*=**
  * **/=**

## *print-argument*:
  * *expr*
  * STRING

## *object-definition*:
  * *object-class* *attribute*\*
  * STRING *text-attribute*\* *attribute*\*
  * **[** *statement-list* **]** *attribute*\*

## *object-class*:
  * **arc**
  * **arrow**
  * **box**          [&#9654;info](#reference-boxobj.md)
  * **circle**       [&#9654;info](#reference-circleobj.md)
  * **cylinder**     [&#9654;info](#reference-cylinderobj.md)
  * **diamond**      [&#9654;info](#reference-diamondobj.md)
  * **dot**
  * **ellipse**      [&#9654;info](#reference-ellipseobj.md)
  * **file**         [&#9654;info](#reference-fileobj.md)
  * **line**
  * **move**
  * **oval**         [&#9654;info](#reference-ovalobj.md)
  * **spline**
  * **text**

## *attribute*:
  * *path-attribute*              [&#9654;info](#reference-pathattr.md)
  * *location-attribute*          [&#9654;info](#reference-locattr.md)
  * STRING *text-attribute*\*     [&#9654;info](#reference-annotate.md)
  * **same**
  * **same as** *object*
  * *numeric-property* *new-property-value*
  * **dashed** *expr*?
  * **dotted** *expr*?
  * **color** *color-expr*
  * **fill** *color-expr*
  * **behind** *object*      [&#9654;info](#reference-behind.md)
  * **cw**
  * **ccw**
  * **&lt;-**                [&#9654;info](#reference-arrowdir.md)
  * **-&gt;**                [&#9654;info](#reference-arrowdir.md)
  * **&lt;-&gt;**            [&#9654;info](#reference-arrowdir.md)
  * **invis**|**invisible**  [&#9654;info](#reference-invis.md)
  * **thick**                [&#9654;info](#reference-thickthin.md)
  * **thin**                 [&#9654;info](#reference-thickthin.md)
  * **solid**                [&#9654;info](#reference-thickthin.md)
  * **chop**                 [&#9654;info](#reference-chop.md)
  * **fit**                  [&#9654;info](#reference-fit.md)

## *color-expr*: [&#9654;info](#reference-colorexpr.md)
  * *expr*

## *new-property-value*:  [&#9654;info](#reference-newpropval.md)
  * *expr*
  * *expr* **%**

## *numeric-property*:  [&#9654;info](#reference-numprop.md)
  * **diameter**
  * **ht**
  * **height**
  * **rad**
  * **radius**
  * **thickness**
  * **width**
  * **wid**

## *text-attribute*:  [&#9654;info](#reference-textattr.md)
  * **above**
  * **aligned**
  * **below**
  * **big**
  * **bold**
  * **mono**
  * **monospace**
  * **center**
  * **italic**
  * **ljust**
  * **rjust**
  * **small**

## *path-attribute*:   [&#9654;info](#reference-pathattr.md)
  * **from** *position*
  * **then**? **to** *position*
  * **then**? **go**? *direction* *line-length*?
  * **then**? **go**? *direction* **until**? **even with** *position*
  * (**then**|**go**) *line-length*? **heading** *compass-angle*
  * (**then**|**go**) *line-length*? *compass-direction*
  * **close**

## *line-length*:  [&#9654;info](#reference-linelen.md)

  * *expr*
  * *expr* **%**

## *compass-angle*:   [&#9654;info](#reference-compassangle.md)

  * *expr*

## *compass-direction*:
  * **n**
  * **north**
  * **ne**
  * **e**
  * **east**
  * **se**
  * **s**
  * **south**
  * **sw**
  * **w**
  * **west**
  * **nw**

## *location-attribute*: [&#9654;info](#reference-locattr.md)
  * **at** *position*
  * **with** *edgename* **at** *position*
  * **with** *dot-edgename* **at** *position*

## *position*:  [&#9654;info](#reference-position.md)

  *  *expr* **,** *expr*
  *  *place*
  *  *place* **+** *expr* **,** *expr*
  *  *place* **-** *expr* **,** *expr*
  *  *place* **+ (** *expr* **,** *expr* **)**
  *  *place* **- (** *expr* **,** *expr* **)**
  *  **(** *position* **,** *position* **)**
  *  **(** *position* **)**
  *  *fraction* **of the way between** *position* **and** *position*
  *  *fraction* **way between** *position* **and** *position*
  *  *fraction* **between** *position* **and** *position*
  *  *fraction* **<** *position* **,** *position* **>**
  *  *distance* *which-way-from* *position*

## *fraction*:
  *  *expr*

## *distance*
  *  *expr*

## *which-way-from*:

  *  **above**
  *  **below**
  *  **right of**
  *  **left of**
  *  **n of**
  *  **north of**
  *  **ne of**
  *  **e of**
  *  **east of**
  *  **se of**
  *  **s of**
  *  **south of**
  *  **sw of**
  *  **w of**
  *  **west of**
  *  **nw of**
  *  **heading** *compass-angle* **from**

## *place*:      [&#9654;info](#reference-place.md)

  *  *object*
  *  *object* *dot-edgename*
  *  *edgename* **of** *object*
  *  ORDINAL **vertex of** *object*

## *object*:

  *  LABEL
  *  *object* **.** LABEL
  *  *nth-object* **of**|**in** *object*

## *nth-object*:

  *  ORDINAL *object-class*
  *  ORDINAL **last** *object-class*
  *  ORDINAL **previous** *object-class*
  *  **last** *object-class*
  *  **previous** *object-class*
  *  **last**
  *  **previous**
  *  ORDINAL **[]**
  *  ORDINAL **last []**
  *  ORDINAL **previous []**
  *  **last []**
  *  **previous []**

## *dot-edgename*:
  * **.n**
  * **.north**
  * **.t**
  * **.top**
  * **.ne**
  * **.e**
  * **.east**
  * **.right**
  * **.se**
  * **.s**
  * **.south**
  * **.bot**
  * **.bottom**
  * **.sw**
  * **.w**
  * **.west**
  * **.left**
  * **.nw**
  * **.c**
  * **.center**
  * **.start**
  * **.end**

## *edgename*:
  * **n**
  * **north**
  * **ne**
  * **e**
  * **east**
  * **se**
  * **s**
  * **south**
  * **sw**
  * **w**
  * **west**
  * **nw**
  * **t**
  * **top**
  * **bot**
  * **bottom**
  * **left**
  * **right**
  * **c**
  * **center**
  * **start**
  * **end**


## *expr*:

  *  NUMBER
  *  VARIABLE
  *  COLORNAME
  *  *place* **.x**
  *  *place* **.y**
  *  *object* *dot-property*
  *  **(** *expr* **)**
  *  *expr* **+** *expr*
  *  *expr* **-** *expr*
  *  *expr* **\*** *expr*
  *  *expr* **/** *expr*
  *  **-** *expr*
  *  **+** *expr*
  *  **abs (** *expr* **)**
  *  **cos (** *expr* **)**
  *  **dist (** *position* **,** *position* **)**
  *  **int (** *expr* **)**
  *  **max (** *expr* **,** *expr* **)**
  *  **min (** *expr* **,** *expr* **)**
  *  **sin (** *expr* **)**
  *  **sqrt (** *expr* **)**

## *dot-property*:

  * **.color**
  * **.dashed**
  * **.diameter**
  * **.dotted**
  * **.fill**
  * **.ht**
  * **.height**
  * **.rad**
  * **.radius**
  * **.thickness**
  * **.wid**
  * **.width**


---

## Linked reference articles

<a id="reference-stmtlist.md"></a>

### statement-list


A complete Pikchr source document consists of a list of zero or more
statements. Individual statements within the list are separated from
each other by semicolons ("`;`") and/or newlines.  Surplus semicolons
and newlines are ignored.  A zero-length string, or a string consisting
of only semicolons and newlines, is a valid Pikchr document.

The *statement-list* is also a subpart of the syntax for 
the `[]`-collection object.

#### Rules

  * *statement* 
  * *statement-list* NEWLINE *statement*
  * *statement-list* **;** *statement*

#### Bubble Chart

~~~~~ pikchr indent
$r = 0.2in
linerad = 0.75*$r
linewid = 0.25

# Start and end blocks
#
box "statement-list" bold fit
line down 75% from last box.sw
dot rad 250% color black
X0: last.e + (0.3,0)
arrow from last dot to X0
move right 2in
box wid 5% ht 25% fill black
X9: last.w - (0.3,0)
arrow from X9 to last box.w


# The main rule that goes straight through from start to finish
#
box "statement" italic fit at 0.5<X0,X9>
arrow to X9
arrow from X0 to last box.w

# The by-pass line
#
arrow right $r from X0 then up $r \
  then right until even with 1/2 way between X0 and X9
line right until even with X9 - ($r,0) \
  then down until even with X9 then right $r

# The Loop-back rule
#
oval "\"&#92;n\"" fit at $r*1.2 below 1/2 way between X0 and X9
line right $r from X9-($r/2,0) then down until even with last oval \
   then to last oval.e ->
line from last oval.w left until even with X0-($r,0) \
   then up until even with X0 then right $r
oval "\";\"" fit at $r*1.2 below last oval
line from 2*$r right of 2nd last oval.e left $r \
   then down until even with last oval \
   then to last oval.e ->
line from last oval.w left $r then up until even with 2nd last oval \
   then left 2*$r ->
~~~~~

#### Whitespace

Whitespace other than a newline is ignored.  If a backslash is followed
by one or more whitespace characters ending in a newline, then the
backslash and all of the spaces that follow, including the newline,
are considered whitespace.  Thus, a backslash at the end of a line
causes a statement to continue onto the next line.

#### Comments

Three comment formats are supported:

   *  The "`#`" character and all characters that follow up to but not
      including the next newline character.  (Bourne-shell style comments.)

   *  Two forward slashes ("`//`") and all characters that follow up to
      but not including the next newline character.  (C++ style comments.)

   *  The sequence "`/*`" and all characters that follow up to and including
      the next "`*/`".  (C style comments.)

The first form (#-comments) is the only form supported by legacy-PIC.
The C++ and C style commenting is new to Pikchr.

For #-comments and //-comments, the newline that follows is not part of the
comment.  Hence that newline will terminate the current statement.  There
is no way to escape the newline at the end of a #- or //-comment.  If you
need a comment at the end of a line but want to continue the statement on
the next line, you must use `/*..*/` style comments.


<a id="reference-stmt.md"></a>

### statement


#### Rules

  * *object-definition*
  * LABEL **:** *object-definition*
  * LABEL **:** *place*
  * *direction*
  * VARIABLE *assignment-op* *expr*
  * **define** VARIABLE CODEBLOCK
  * **print** *print-argument* (**,** *print-argument*)\*
  * **assert (** *expr* **==** *expr* **)**
  * **assert (** *position* **==** *position* **)**


#### Labels

A label can be attached to either an *object* or a *place* so that
the object or place can be more easily referenced by subsequent statements.
Labels always begin with an upper-case ASCII character.

Labels do not have to be unique.  When there are two or more
labels with the same name, the later one takes precedence.
This allows a label to be effectively redefined.  New labels do not
come into existence until after the object or place to which they are
attached has been completely parsed and analyzed.  This allows labels
to be redefined in terms of themselves.  Consider an example:

~~~~~
/* 01 */        down
/* 02 */  Root: dot "First \"Root\"" above color red
/* 03 */        circle wid 50% at Root + (1.5cm, -1.5cm)
/* 04 */        arrow dashed from previous to Root chop
/* 05 */  Root: 3cm right of Root   // Move the location of Root 3cm right
/* 06 */        arrow from last circle to Root chop
/* 07 */        dot "Second \"Root\"" above color blue at Root
~~~~~

Line 05 redefines Root in terms of itself.
In the rendering below, you can see that the dashed arrow drawn before
Root was redefined goes to the original Root, but the solid arrow drawn
afterwards goes to the revised location for Root.

~~~~~ pikchr center toggle
/* 01 */        down
/* 02 */  Root: dot "First \"Root\"" above color red
/* 03 */        circle wid 50% at Root + (1.5cm, -1.5cm)
/* 04 */        arrow dashed from previous to Root chop
/* 05 */  Root: 3cm right of Root   // Move the location of Root 3cm right
/* 06 */        arrow from last circle to Root chop
/* 07 */        dot "Second \"Root\"" above color blue at Root
~~~~~

#### Variables

Variable names begin with a lower-case ASCII letter or with "`$`"
or with "`@`".  The $- and @- variable names are a Pikchr extension
designed to help prevent collisions between variable names and the
(numerous) keywords in the Pikchr language.

Pikchr has built-in variables as follows:

>
| Variable Name | &nbsp;&nbsp; Initial Value &nbsp;&nbsp; |: Purpose         |
------------------------------------------------------------------------------
| arcrad        |: 0.250 :| Default arc radius                               |
| arrowhead     |: 2.000 :| *Not used by Pikchr*                             |
| arrowht       |: 0.080 :| Length of arrowheads                             |
| arrowwid      |: 0.060 :| Width of arrowheads                              |
| boxht         |: 0.500 :| Default height of "box" objects                  |
| boxrad        |: 0.000 :| Default corner radius for "box" objects          |
| boxwid        |: 0.750 :| Default width for "box" objects                  |
| charht        |: 0.140 :| Average height of a character                    |
| charwid       |: 0.080 :| Average width of a character                     |
| circlerad     |: 0.250 :| Default radius for "circle" objects              |
| color         |: 0.000 :| Default foreground color                         |
| cylht         |: 0.500 :| Default height for "cylinder" objects            |
| cylrad        |: 0.075 :| Default minor axis for ellipses in a cylinder    |
| cylwid        |: 0.750 :| Default width of a "cylinder" object             |
| dashwid       |: 0.050 :| Default width of dashes in dashed lines          |
| dotrad        |: 0.015 :| Default radius for a "dot" object                |
| ellipseht     |: 0.500 :| Default height for "ellipse" objects             |
| ellipsewid    |: 0.750 :| Default width for "ellipse" objects              |
| fileht        |: 0.750 :| Default height for "file" objects                |
| filerad       |: 0.150 :| Default corner fold length for "file" objects    |
| filewid       |: 0.500 :| Default width for "file" objects                 |
| fill          |: -1.00 :| Default fill color.  Negative means "none"       |
| lineht        |: 0.500 :| Default length for lines drawn up or down        |
| linewid       |: 0.500 :| Default length for lines drawn left or right     |
| movewid       |: 0.500 :| Default distance traversed by a "move"           |
| ovalht        |: 0.500 :| Default height of an "oval" object               |
| ovalwid       |: 1.000 :| Default width of an "oval" object                |
| scale         |: 1.000 :| Scale factor for drawing.  Larger is bigger.     |
| textht        |: 0.500 :| *Not used by Pikchr*                             |
| textwid       |: 0.750 :| *Not used by Pikchr*                             |
| thickness     |: 0.015 :| Default line thickness for all objects           |

In addition to the above, Pikchr recognizes the following variables
which are not initially defined, but if they are defined by the script
have special properties:

>
| Variable Name&nbsp;&nbsp;&nbsp;&nbsp; |: Purpose                           |
------------------------------------------------------------------------------
| bottommargin  | Extra border space added to the bottom of the diagram      |
| fgcolor       | Use this foreground color in place of black                |
| fontscale     | Scale factor applied to the font size of text              |
| layer         | The default layer for all subsequent objects               |
| leftmargin    | Extra border space added to the left of the diagram        |
| margin        | Extra border space added to all four sides of the diagram  |
| rightmargin   | Extra border space added to the right side of the diagram  |
| topmargin     | Extra border space added to the top side of the diagram    |


The "VARIABLE *assignment-op* *expr*" syntax is able to modify the value
of built-in variables, or create new variables.  In legacy-PIC, the only
*assignment-op* was "`=`".  Pikchr adds "`+=`", "`-=`", "`*=`", and
"`/=`" to make it easier to scale existing variables up or down.

##### Conflicts between variable names and keywords

Some of the built-in variables have names that conflict with keywords:

  *  color
  *  fill
  *  thickness

To access such variables as part of an expression, simply put them inside
of parentheses.  For example, to set the thickness of a box to be twice
the default thickness:

~~~ pikchr center toggle source
   box "Normal"
   move
   box "Double" "Thick" thickness 2*(thickness)
~~~

#### Define

The "`define`" statement creates a [macro](#reference-macro.md)
that can then be called in subsequent text.

#### Print

The "`print`" statement prints the strings and the values of the expressions
in its argument into the generated output in front of the 
"`<svg>`" element for the diagram.  This facility is intended for testing
and debugging purposes.  There is no known practical use for "`print`" in
a production Pikchr script.

The following Pikchr script demonstrates the effect of "print".
Click to toggle between the script and its rendering.

~~~ pikchr toggle source indent
   oval "Hello, World!" fit
   print "Oval at: ",previous.x, ",", previous.y
   line
   oval "2nd oval" fit
   print "2nd oval at: ",previous.x, ",", previous.y
~~~

#### Assert

The "`assert`" statement is intended for testing and debugging of Pikchr
scripts.  An assert() is a no-op if the equality comparison in its
argument is true.  But it raises an error if the condition is false.

Consider this script:

~~~
   oval "Hello, World!" fit
   assert( last oval.w == last oval.e ); # <-- should fail
~~~

And its rendering:

~~~ pikchr
   oval "Hello, World!" fit
   assert( last oval.w == last oval.e ); # <-- should fail
~~~


<a id="reference-macro.md"></a>

### Macros


A macro is created using a "`define`" statement:

~~~ pikchr toggle
$r = 0.2in
linerad = 0.75*$r
linewid = 0.25

# Start and end blocks
#
box "define-statement" bold fit
line down 50% from last box.sw
START: dot rad 250% color black
X0: last.e
move right 3.2in
END: box wid 5% ht 25% fill black
X9: last.w

# The main rule
#
arrow from X0 right 2*linerad+arrowht
oval "\"define\"" fit
arrow
oval "MACRONAME" fit
arrow
oval "{...}" fit
line right to X9
~~~

A define statement consists of the keyword "`define`" followed by
an identifier that is the name of the macro and then the body of
the macro contained within (possibly nested) curly braces.

After a macro is defined, the body of the macro is substituted in
place of any subsequent occurrence of the identifier that is the
macro name.  The macro name can occur anywhere.  The substitution
is performed by the lexical analyzer, before tokens are identified
and sent into the parser.  Note this distinction:  The "`define`"
statement used to create a new macro is recognized by the parser,
but the expansion of the macro is subsequent text happens in the
lexical analyzer.

#### Parameters

The invocation of a macro can be followed immediately by a
parenthesized list of parameters.  The open-parenthesis must immediately
follow the macro name with no intervening whitespace.  Parameters are
comma-separated.  There can be at most 9 parameters.

When parameters are present, they are substituted in the macro body
in place of "`$1`", "`$2`", ..., "`$9`" in the macro body.  If
"$N" (for N between 1 and 9) occurs in the macro body but there are
fewer than N parameters, then the "$N" is omitted.

#### Nested Macros

Macros can be nested up to a maximum depth that is determined at
compile-time.  (The current limit is 10.)

Arguments to nested macros can be arbitrary text, or a single "$N"
parameter, but not both.

#### Macros cannot be undefined or redefined

Once created, a macro cannot be redefined.  If you attempt to redefine
a macro by providing a second "`define`" statement with the same macro
name, the macro name will be replaced by the previous macro body definition
during lexical analysis, likely resulting in a syntax error.


<a id="reference-boxobj.md"></a>

### Box objects


A box is a rectangle with a specified width and height.  The default
width and height are the values of the "`boxwid`" and "`boxht`" variables.

~~~~ pikchr indent toggle
A: box thick
line thin color gray left 70% from 2mm left of A.nw
line same from 2mm left of A.sw
text "height" at (7/8<previous.start,previous.end>,1/2<1st line,2ndline>)
line thin color gray from previous text.n up until even with 1st line ->
line thin color gray from previous text.s down until even with 2nd line ->
X1: line thin color gray down 50% from 2mm below A.sw
X2: line thin color gray down 50% from 2mm below A.se
text "width" at (1/2<X1,X2>,6/8<X1.start,X1.end>)
line thin color gray from previous text.w left until even with X1 ->
line thin color gray from previous text.e right until even with X2 ->
~~~~

If a "`radius`" is specified, then the corners of the box are rounded using
arcs of the given radius.  The default radius for each new box is the value
of the "`boxrad`" variable which is initially 0.0.

~~~~ pikchr indent toggle
A: box thick rad 0.3*boxht
line thin color gray left 70% from 2mm left of (A.w,A.n)
line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<1st line,2ndline>)
line thin color gray from previous text.n up until even with 1st line ->
line thin color gray from previous text.s down until even with 2nd line ->
X1: line thin color gray down 50% from 2mm below (A.w,A.s)
X2: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X1,X2>,6/8<X1.start,X1.end>)
line thin color gray from previous text.w left until even with X1 ->
line thin color gray from previous text.e right until even with X2 ->
X3: line thin color gray right 70% from 2mm right of (A.e,A.s)
X4: line thin color gray right 70% from A.rad above start of X3
text "radius" at (6/8<X4.start,X4.end>,1/2<X3,X4>)
line thin color gray from (previous,X3) down 30% <-
line thin color gray from (previous text,X4) up 30% <-
~~~~

#### Boundary points:

~~~~ pikchr indent toggle
A: box thin
dot ".c" above at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw

A: box thin at 2.0*boxwid right of previous box rad 15px
dot ".c" above at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw
~~~~


<a id="reference-circleobj.md"></a>

### Circle objects


A circle is defined by one of:

   *  `radius`
   *  `diameter`
   *  `width`
   *  `height`

Only one of these values can be set for any particular circle; the others 
are determined automatically by the first.
The default radius is value of the "`circlerad`" variable.


~~~~ pikchr indent toggle
A: circle thick rad 120%
line thin color gray left 70% from 2mm left of (A.w,A.n)
line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<1st line,2ndline>)
line thin color gray from previous text.n up until even with 1st line ->
line thin color gray from previous text.s down until even with 2nd line ->
X1: line thin color gray down 50% from 2mm below (A.w,A.s)
X2: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X1,X2>,6/8<X1.start,X1.end>)
line thin color gray from previous text.w left until even with X1 ->
line thin color gray from previous text.e right until even with X2 ->
X3: line thin color gray right 70% from 2mm right of (A.e,A.s)
X4: line thin color gray right 70% from A.rad above start of X3
text "radius" at (6/8<X4.start,X4.end>,1/2<X3,X4>)
line thin color gray from (previous,X3) down 30% <-
line thin color gray from (previous text,X4) up 30% <-
line thin color gray <-> from A.sw to A.ne
line thin color gray from A.ne go 0.5*A.rad ne then 0.25*A.rad east
text " diameter" ljust at end of previous line
~~~~

#### Boundary points:

~~~~ pikchr indent toggle
A: circle thin
dot ".c" above at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw
~~~~


<a id="reference-cylinderobj.md"></a>

### Cylinder objects


A cylinder is a stylized projection of a cylinder into the 2-D space of
the diagram.  Cylinders are commonly used to represent bulk data storage in
software architecture diagrams, as legacy disk packs were cylindrical in shape.

The shape of a cylinder is defined by the width, height, and radius.
The radius is the minor axis of the ellipse that forms the top of the
cylinder, and the semiellipse the forms the bottom.

~~~~ pikchr indent toggle
A: cylinder thick rad 150%
line thin color gray left 70% from 2mm left of (A.w,A.n)
line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<1st line,2ndline>)
line thin color gray from previous text.n up until even with 1st line ->
line thin color gray from previous text.s down until even with 2nd line ->
X1: line thin color gray down 50% from 2mm below (A.w,A.s)
X2: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X1,X2>,6/8<X1.start,X1.end>)
line thin color gray from previous text.w left until even with X1 ->
line thin color gray from previous text.e right until even with X2 ->
X3: line thin color gray right 70% from 2mm right of (A.e,A.ne)
X4: line thin color gray right 70% from A.rad below start of X3
text "radius" at (6/8<X4.start,X4.end>,1/2<X3,X4>)
line thin color gray from (previous,X4) down 30% <-
line thin color gray from (previous text,X3) up 30% <-
~~~~

#### Boundary points:

~~~~ pikchr indent toggle
A: cylinder thin rad 80%
dot ".c" below at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw
~~~~


<a id="reference-diamondobj.md"></a>

### Diamond objects


A diamond acts much like a [box](#reference-boxobj.md) except that its corners
are rotated around the center point such that they become the shape’s
four primary cardinal points:

~~~~ pikchr indent toggle
D: diamond "Cardinal" "Points"
   dot ".n" above at D.n
   dot " .e" ljust at D.e
   dot ".s" below at D.s
   dot ".w " rjust at D.w
~~~~

Indeed, before Pikchr [acquired this primitive](/info/bc3bd914a3), the
workaround was to draw an invisible box to hold the text, then draw
lines between its cardinal points:

~~~~ pikchr indent toggle
box width 150% invis "“Diamond”" "Label"
line from last.w to last.n to last.e to last.s close
~~~~

This does work, and it has the advantage of being compatible with the
original PIC and with GNU `dpic`, but it also has a number of
weaknesses, one of which is evident in comparing the examples above: the
labels aren’t as well-centered when manually drawing the diamond.

Another is the need for that 150% fudge factor to the invisible box’s
width, without which the labels would be truncated by the dimensions
Pikchr calculates for the invisible bounding box:

~~~~ pikchr indent toggle
box invis "“Diamond”" "Label"
line from last.w to last.n to last.e to last.s close
~~~~

A third advantage falls out of this fact: the “`fit`” attribute works as
expected for Pikchr diamonds. It cannot with the manual PIC-compatible
workaround due to the lack of a properly-calculated bounding box, one taking
into account the rotated cardinal points:

~~~~ pikchr indent toggle
text "Unfitted:"
diamond "D"
text "Properly fitted:"
diamond "D" fit
text "Badly fitted:"
box invis "D" fit
line from last.w to last.n to last.e to last.s close
~~~~

There’s a fourth, more subtle advantage to having this primitive built
into the language: the location of the ordinal points is now
well-defined:

~~~~ pikchr indent toggle
D: diamond "Ordinal" "Points"
   dot " .ne" ljust above at D.ne
   dot " .se" ljust below at D.se
   dot ".sw " rjust below at D.sw
   dot ".nw " rjust above at D.nw
~~~~

To replicate that with the PIC-compatible hack above, you’d have to do
the geometry to work out where those points land along the lines. It’s
better to leave that bit of tedious math to the Pikchr renderer.

Unlike a box, you cannot currently round the corners on a diamond. You
can, however, programmatically override the default height and width
by redefining the `diamondht` and `diamondwid` variables. Here we show
two different ways of making the diamond 25% larger:

~~~~ pikchr indent toggle
D:  diamond thick "Diamond" "Dimensions" width 125% height 125%

X1: line thin color gray left 70% from 4mm left of (D.w,D.n)
X2: line same from 4mm left of (D.w,D.s)
    text "height" small at 1/2 way between X1 and X2
    line thin color gray from previous text.n up   until even with X1 ->
    line thin color gray from previous text.s down until even with X2 ->
X3: line thin color gray down 50% from 2mm below (D.w,D.s)
X4: line same from 2mm below (D.e,D.s)
    text "width" small at 1/2 way between X3 and X4
    line thin color gray from previous text.w left  until even with X3 ->
    line thin color gray from previous text.e right until even with X4 ->

    diamondht  *= 1.25
    diamondwid *= 1.25
    diamond thick "Diamond" "Dimensions" at 1.5in right of D
~~~~


<a id="reference-ellipseobj.md"></a>

### Ellipse Objects


The shape of an ellipse is determined solely by its height and width.

~~~~ pikchr indent toggle
A: ellipse thick
line thin color gray left 70% from 2mm left of (A.w,A.n)
line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<1st line,2ndline>)
line thin color gray from previous text.n up until even with 1st line ->
line thin color gray from previous text.s down until even with 2nd line ->
X1: line thin color gray down 50% from 2mm below (A.w,A.s)
X2: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X1,X2>,6/8<X1.start,X1.end>)
line thin color gray from previous text.w left until even with X1 ->
line thin color gray from previous text.e right until even with X2 ->
~~~~

Unlike a circle, ellipses have no radius, but if the
width and height are equal, it is visually identical to a circle.


#### Boundary Points

~~~~ pikchr indent toggle
A: ellipse thin
dot ".c" below at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw
~~~~


<a id="reference-fileobj.md"></a>

### File objects


A "file" is a stylized image of a piece of paper with the upper right
corner folded over.  Similar images are commonly used to represent "files".
The shape of a file object is defined by its width, height, and radius.
The radius is the height and width of the folded corner.  The default values
for height, radius, and width are controlled by variables
"`fileht`", "`filerad`", and "`filewid`".

~~~~ pikchr indent toggle
A: file thick rad 100%
line thin color gray left 70% from 2mm left of (A.w,A.n)
line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<1st line,2ndline>)
line thin color gray from previous text.n up until even with 1st line ->
line thin color gray from previous text.s down until even with 2nd line ->
X1: line thin color gray down 50% from 2mm below (A.w,A.s)
X2: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X1,X2>,6/8<X1.start,X1.end>)
line thin color gray from previous text.w left until even with X1 ->
line thin color gray from previous text.e right until even with X2 ->
X3: line thin color gray right 70% from 2mm right of (A.e,A.n)
X4: line thin color gray right 70% from A.rad below start of X3
text "radius" at (6/8<X4.start,X4.end>,1/2<X3,X4>)
line thin color gray from (previous,X4) down 30% <-
line thin color gray from (previous text,X3) up 30% <-
~~~~

#### Boundary points:

~~~~ pikchr indent toggle
A: file thin rad 80%
dot ".c" below at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw
~~~~


<a id="reference-ovalobj.md"></a>

### Oval objects


An oval is a box in which the narrow ends are formed by semicircles.
If the height is less than the width (the default) then the semicircles
are on the left and right.  If the width is less than the height, then the
semicircles are on the top and bottom:

~~~~ pikchr indent toggle
A: oval thick
X0: line thin color gray left 70% from 2mm left of (A.w,A.n)
X1: line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<X0,X1>)
line thin color gray from previous text.n up until even with X0 ->
line thin color gray from previous text.s down until even with X1 ->
X2: line thin color gray down 50% from 2mm below (A.w,A.s)
X3: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X2,X3>,6/8<X2.start,X2.end>)
line thin color gray from previous text.w left until even with X2 ->
line thin color gray from previous text.e right until even with X3 ->

A: oval thick wid A.ht ht A.wid at 2.0*A.wid right of A
X0: line thin color gray left 70% from 2mm left of (A.w,A.n)
X1: line same from 2mm left of (A.w,A.s)
text "height" at (7/8<previous.start,previous.end>,1/2<X0,X1>)
line thin color gray from previous text.n up until even with X0 ->
line thin color gray from previous text.s down until even with X1 ->
X2: line thin color gray down 50% from 2mm below (A.w,A.s)
X3: line thin color gray down 50% from 2mm below (A.e,A.s)
text "width" at (1/2<X2,X3>,6/8<X2.start,X2.end>)
line thin color gray from previous text.w left until even with X2 ->
line thin color gray from previous text.e right until even with X3 ->
~~~~

An oval works like a [box](#reference-boxobj.md) in which the radius is
set to half the minimum of the height and width.  An oval where the
width and height are the same is a [circle](#reference-circleobj.md)


#### Boundary points:

~~~~ pikchr indent toggle
A: oval thin
dot ".c" below at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw

A: oval thin  wid A.ht ht A.wid at 2.0*A.wid right of A
dot ".c" below at A
dot ".n" above at A.n
dot " .ne" ljust above at A.ne
dot " .e" ljust at A.e
dot " .se" ljust below at A.se
dot ".s" below at A.s
dot ".sw " rjust below at A.sw
dot ".w " rjust at A.w
dot ".nw " rjust above at A.nw
~~~~


<a id="reference-pathattr.md"></a>

### path-attribute


A *path-attribute* is used to provide the origin and direction of a line
object, that being an arc, arrow, line, move, or spline.  It is an error
to use a *path-attribute* on a block object, that being a box, circle,
cylinder, diamond, dot, ellipse, file, oval, or text.

There are seven forms:

  *  **from** *position*
  *  **then**? **to** *position*
  *  **then**? **go**? *direction* *distance*?
  *  **then**? **go**? *direction* **until**? **even with** *place*
  *  (**then**|**go**) *distance*? **heading** *compass-angle*
  *  (**then**|**go**) *distance*? *compass-direction*
  *  **close**

The "`from`" attribute is used to assign the starting location
of the line object (its ".start" value).  The other six forms
(collectively called "to" forms) assign
intermediate vertexes or the end point (.end).   If the "`from`"
is omitted, then "`from previous.end`" is assumed, or if there
is no previous object, "`from (0,0)`".   If no "to" forms are
provided then a single movement in the current layout direction
by either the "linewid" or "lineht" (depending on layout direction)
is used.

The "from" can occur
either before or after the various "to" subclauses.  That does not
matter.  But the order is important for the various "to" subclauses.

If there are two consecutive *direction* clauses (*direction* is
always one of "`up`", "`down`", "`left`", or "`right`") then
the two will be combined to specify a single line segment.
Hence, the following are equivalent:


  *  ... **right 4cm up 3cm** ...
  *  ... **go 5cm heading 53.13010235** ...

~~~ pikchr
leftmargin = 1cm
A1: arrow thick right 4cm up 3cm
dot at A1.start
X1: line thin color gray from (0,-3mm) down 0.4cm
X2: line same from (4cm,-3mm) down 0.4cm
arrow thin color gray from X1 to X2 "4cm" above
X3: line same from (4cm+3mm,0) right 0.4cm
X4: line same from (4cm+3mm,3cm) right .4cm
arrow thin color gray from X3 to X4 "3cm" aligned above
X5: line same from A1.start go 4mm heading 90+53.13010235
X6: line same from A1.end go 4mm heading 90+53.13010235
arrow thin color gray from X5 to X6 "5cm" below aligned
line same from (0,1cm) up 1cm
spline -> from 1.5cm heading 0 from A1.start \
   to 1.5cm heading 10 from A1.start \
   to 1.5cm heading 20 from A1.start \
   to 1.5cm heading 30 from A1.start \
   to 1.5cm heading 40 from A1.start \
   to 1.5cm heading 53.13 from A1.start \
   thin color gray "53.13&deg;" aligned center small
~~~

If two separate movements are desired, one 4cm right and another 3cm up,
then the "right" and "up" subphrases must be separated by the "`then`" keyword:

  *  ... **right 4cm then up 3cm** ...

~~~ pikchr
leftmargin = 1cm
A1: arrow thick right 4cm then up 3cm
dot at A1.start
X1: line thin color gray from (0,-3mm) down 0.4cm
X2: line same from (4cm,-3mm) down 0.4cm
arrow thin color gray from X1 to X2 "4cm" above
X3: line same from (4cm+3mm,0) right 0.4cm
X4: line same from (4cm+3mm,3cm) right .4cm
arrow thin color gray from X3 to X4 "3cm" aligned above
~~~

#### The "`until even with`" subclause

The "until even with" clause is a Pikchr extension (it does not exist
in PIC) that makes it easier to specify paths that follow a
"Manhattan geometry" (lines are axis-aligned) or that negotiate around
obstacles.  The phrase:

>  go *direction* until even with *position*

Means to continue the line in the specified *direction* until the
coordinate being changed matches the corresponding coordinate in
*position*. If the line is going up or down, then it continues until
the Y coordinate matches the Y coordinate of *position*.  If the line
is going left or right, then it continues until
the X coordinate matches the X coordinate of *position*.

For example, suppose in the diagram below that we want to draw an arrow 
that begins on Origin.s and ends on Destination.s but goes around
the Obstacle oval, clearing it by at least one centimeter.

~~~ pikchr toggle
box "Origin"
Obstacle: oval ht 300% wid 30% with .n at linewid right of Origin.ne;
box "Destination" with .nw at linewid right of Obstacle.n
line invis from 1st oval.s to 1st oval.n "Obstacle" aligned
~~~

The arrow might look like this:

~~~
   arrow from Origin.s \
      down until even with 1cm below Obstacle.s \
      then right until even with Destination.s \
      then to Destination.s
~~~

And we have (annotations added):

~~~ pikchr toggle
box "Origin"
Obstacle: oval ht 300% wid 30% with .n at linewid right of Origin.ne;
box "Destination" with .nw at linewid right of Obstacle.n
line invis from 1st oval.s to 1st oval.n "Obstacle" aligned
X: \
   arrow from Origin.s \
      down until even with 1cm below Obstacle.s \
      then right until even with Destination.s \
      then to Destination.s

line invis color gray from X.start to 2nd vertex of X \
    "down until even with" aligned small \
    "1cm below Obstacle.s" aligned small
line invis color gray from 2nd vertex of X to 3rd vertex of X \
    "right until even with Destination.s" aligned small above
line invis color gray from 3rd vertex of X to 4th vertex of X \
    "to Destination.s" aligned small above

# Evidence that the alternative arrow is equivalent:
assert( 2nd vertex of X == (Origin.s, 1cm below Obstacle.s) )
assert( 3rd vertex of X == (Destination.s, 1cm below Obstacle.s) )
~~~

The "**(** *position* **,** *position* **)**" syntax can be used
in a similar way.  The "**(** *position* **,** *position* **)**"
syntax means a point whose X coordinate is taken from the first
position and whose Y coordinate is taken from the second position.
So the line around the obstacle could have been written like this:

~~~ 
   arrow from Origin.s \
     to (Origin.s, 1cm below Obstacle.s) \
     then to (Destination.s, 1cm below Obstacle.s) \
     then to Destination.s
~~~

However, we believe the "`until even with`" notation is easier.

#### The "`close`" subclause

The "`close`" attribute closes a multi-segment path so that it
forms a polygon.  When "`close`" is used, the "`.end`" point of the
object is no longer the last vertex in the path but is instead
one of "`.e`", "`.s`", "`.w`", or "`.n`" according to the current
layout direction, as it would be for a block object.

The following diagram illustrates this behavior.  The "`.end`" of
each line is tagged with a red dot.  The line that uses "`close`"
has its end at the "`.e`" point of the bounding box since the
layout direction is "right".  The line without "`close`" has its
"`.end`" at the last vertex of the line.

~~~ pikchr toggle
line right 2cm then down .5cm then up 1cm right 1cm \
   then up 1cm left 1cm then down .5cm then left 2cm \
   close "with 'close'"
dot color red at last line.end

move to 2.5cm south of last line.start
line right 2cm then down .5cm then up 1cm right 1cm \
   then up 1cm left 1cm then down .5cm then left 2cm \
   then down 1cm "without 'close'"
dot color red at last line.end
~~~


<a id="reference-locattr.md"></a>

### location-attribute


A *location-attribute* is an attribute used to assign a location to
a block object (box, circle, cylinder, diamond, dot, ellipse, file, oval, or text).
If a *location-attribute* appears on a line object (arc, arrow, line, move,
or spline) an error is issued and processing stops.

There are three forms:

  *  **at** *position*
  *  **with** *edgename* **at** *position*
  *  **with** *dot-edgename* **at** *position*

The second and third forms are equivalent and only differ in
the "." that comes before the edge name.  PIC does not recognize
the second form, only the first and third.

If the "`with`" clause is omitted, then "`with center`" or
(equivalently) "`with .c`" is assumed.

This attribute causes the block object to be positioned so that
its *edgename* corner is at *position*.

If a block object omits this attribute, then a default location-attribute
is used as follows:

  *  **with .begin at previous.end**
  *  **with .c at (0,0)**

The first default form is what is normally used.  The second default
form is only used if there is no "previous" object.


<a id="reference-annotate.md"></a>

### Text annotations


Objects can have up to 5 separate text annotations.  Each annotation
can have multiple *[text-attributes](#reference-textattr.md)*.

The annotations normally appear stacked above and below the center of the
object.  However, this can be controlled through the use of
various *[text-attributes](#reference-textattr.md)*.

Text annotations are drawn even if the object is marked 
"[`invis`](#reference-invis.md)"


<a id="reference-behind.md"></a>

### The "behind" attribute


The "**behind** *object*" attribute causes the object currently under
construction to be drawn before the referenced *object*.  

Pikchr normally draws objects in the order that they appear in the
input script.  However, the "`behind`" attribute can be used to alter
the drawing order so that boxes used to implement background colors
or borders can be drawn before the objects they enclose, even though
the background-boxes are specified after the objects they enclose.

Consider this example:

~~~ pikchr toggle
    lineht *= 0.4
    $margin = lineht*2.5
    scale = 0.75
    fontscale = 1.1
    charht *= 1.15
    down
IN: box "Interface" wid 150% ht 75% fill white
    arrow
CP: box same "SQL Command" "Processor"
    arrow
VM: box same "Virtual Machine"
    arrow down 1.25*$margin
BT: box same "B-Tree"
    arrow
    box same "Pager"
    arrow
OS: box same "OS Interface"
    box same with .w at 1.25*$margin east of 1st box.e "Tokenizer"
    arrow
    box same "Parser"
    arrow
CG: box same ht 200% "Code" "Generator"
UT: box same as 1st box at (Tokenizer,Pager) "Utilities"
    move lineht
TC: box same "Test Code"
    arrow from CP to 1/4<Tokenizer.sw,Tokenizer.nw> chop
    arrow from 1/3<CG.nw,CG.sw> to CP chop

    box ht (IN.n.y-VM.s.y)+$margin wid IN.wid+$margin \
       at CP fill 0xd8ecd0 behind IN
#                          ^^^^^^^^^
####################################
    line invis from 0.25*$margin east of last.sw up last.ht \
        "Core" italic aligned

    box ht (BT.n.y-OS.s.y)+$margin wid IN.wid+$margin \
       at Pager fill 0xd0ece8 behind IN
#                             ^^^^^^^^^
#######################################
    line invis from 0.25*$margin east of last.sw up last.ht \
       "Backend" italic aligned

    box ht (Tokenizer.n.y-CG.s.y)+$margin wid IN.wid+$margin \
       at 1/2<Tokenizer.n,CG.s> fill 0xe8d8d0 behind IN
#                                             ^^^^^^^^^
#######################################################
    line invis from 0.25*$margin west of last.se up last.ht \
       "SQL Compiler" italic aligned

    box ht (UT.n.y-TC.s.y)+$margin wid IN.wid+$margin \
       at 1/2<UT,TC> fill 0xe0ecc8 behind IN
#                                  ^^^^^^^^^
############################################
    line invis from 0.25*$margin west of last.se up last.ht \
      "Accessories" italic aligned
~~~

In the diagram above, the white
component boxes are drawn first.  Then the larger boxes that
implement the various background colors are drawn relative to
the component boxes.  The "`behind`" attribute must be used to
cause the background boxes to appear to be behind the component
boxes.  Click on the diagram to see the source text.  Comments
have been inserted into the source text to help identify the
"`behind`" attributes amid all the others.


<a id="reference-arrowdir.md"></a>

### Arrowheads


Line objects ("line", "arrow", "spline", and "arc") can have one
of the following attributes to specify which ends of the line contain
arrowheads:

  *  **-&gt;**
  *  **&lt;-**
  *  **&lt;-&gt;**

The first form (**-&gt;**) means that there is an arrowhead at the end.
This is the default for "arrow".  The second form (**&lt;-**) means that
there is an arrowhead at the beginning only.  The third form means that
there are arrowheads at both ends.

Note that "`arrow`" and "`line ->`" look identical to one another.

If there are multiple occurrences of these attributes on a single object,
then the last one is the one that matters.

#### Enhancement 2021-06-11

To make it easier to embed pikchr scripts inside of larger HTML documents,
the arrow direction tokens now have alternative spellings.

| Legacy ASCII | HTML Entity           | Unicode Character |
------------------------------------------------------------
| &lt;-        | &amp;larr;            | &larr;            |
| &lt;-        | &amp;leftarrow;       | &leftarrow;       |
| -&gt;        | &amp;rarr;            | &rarr;            |
| -&gt;        | &amp;rightarrow;      | &rightarrow;      |
| &lt;-&gt;    | &amp;leftrightarrow;  | &leftrightarrow;  |

All the tokens in any row of the table above mean the same thing
to Pikchr and can be freely interchanged.  So, in other words,
each of the following Pikchr statements means the same thing:

  *  `line ->`
  *  `line &rarr;`
  *  `line &rightarrow;`
  *  `line →`


<a id="reference-invis.md"></a>

### The invis or invisible attribute


The "`invis`" or "`invisible`" attribute has the effect of setting
the "`thickness`" to zero.  This makes the object disappear.  However,
all text annotations associated with the object are still visible.

Draw rotated text by making the text an annotation on an "`invis`"
line and using the "`aligned`" *[text-attribute](#reference-textattr.md)*.


<a id="reference-thickthin.md"></a>

### The "thick" and "thin" attributes


The "`thick`" and "`thin`" attributes increase or decrease the stroke-width
for the lines used to draw an object.  There can be multiple "`thick`" or
"`thin`" attributes - the effects are cumulative.

### The `solid` attribute

The "`solid`" attribute changes the stroke-width back to its default,
and it disables "`dashed`" and "`dotted`".


<a id="reference-chop.md"></a>

### The "chop" Attribute


Line objects may have a single "`chop`" attribute.  When the chop
attribute is present, and if the line starts or ends at the center
of a block object, then that start or end is automatically moved to
the edge of the object.  For example:

~~~~ pikchr toggle
file "A"
cylinder "B" at 5cm heading 125 from A
arrow <-> from A to B "from A to B" aligned above color red
arrow <-> from A to B chop "from A to B chop" aligned below color blue
~~~~

In the example, both of the arrows use "`from A to B`"  The difference
is that the blue line adds the "`chop`" keyword whereas the red line
does not.

The chop feature only works if one or both ends of the line land on
the center of a block object.  If neither end of a line is on the
center of a block object, then the "`chop`" attribute is a no-op

#### Different From Legacy PIC

The chop attribute in Pikchr differs from the chop attribute in legacy PIC.
In PIC, the "`chop`" keyword can be followed by a distance and can appear
twice.  The chop keyword causes the line to be shortened by the amount
specified, or by `circlerad` if no distance is given.  The legacy "chop"
works okay if you are drawing lines between circles, but it mostly pointless
for lines between all other kinds of objects.  The enhanced "chop" in
Pikchr is intended to make the feature helpful on a wider variety of
diagrams.


<a id="reference-fit.md"></a>

### The "fit" attribute


The "`fit`" attribute causes an object to automatically adjust its
"`width`", "`height`", and/or "`radius`" so that it will enclose its
text annotations with a reasonable margin.

~~~ pikchr toggle
box "with" "\"fit\"" fit
move
box "without" "\"fit\""
~~~

The "`fit`" attribute only works with text annotations that occur
earlier in the object definition.  In other words, the "`fit`" keyword
should come after all text annotations have been defined.

#### Pikchr guesses at the size of text

Pikchr does not have access to the SVG rendering engine.  Therefore,
it cannot know the precise dimensions of text annotations.  It has to
guess.  Usually Pikchr does a reasonable job, but sometimes it can be
a little off, especially with unusual characters.  If "`fit`" causes the
object to be too narrow, you can try adding spaces at the beginning and
end of the longest text annotation.  You can also adjust the width
and height by a percentage after running "`fit`":

   *  `width 110%`
   *  `height 90%`
   *  `radius 120%`

And so forth.  Substitute percentage increases and decreases, as
appropriate, to make the text fit like you want.

#### Auto-fit

If at the end of an objection definition the requested width or height of the
object is less then or equal to zero, then that dimension is adjusted
upwards to enclose the text annotations.  Thus, by setting variables
like:

~~~
    boxwid = 0
    boxht = 0
~~~

You can cause all boxes to scale to enclose their text annotations.
(Caution:  boxes without any text annotations go to zero height and width
and thus disappear when auto-fit is enabled.)


<a id="reference-colorexpr.md"></a>

### color-expr


Pikchr tracks colors as 24-bit RGB values.  Black is 0.
White is 16777215. Other color values are in between these
two extremes.

Pikchr understands C-style hexadecimal numeric literals.  So it is
often convenient to express colors using 6-digit hex constants
like 0x000000 or 0xffffff (for black and white respectively) rather
than as base-10 literals.

Pikchr knows the names of the 140 standard HTML color names.  If you
use one of those color names in an expression, Pikchr will substitute
the corresponding RGB value.  For example, if you write:

~~~~~
    circle "Hi" fill Bisque
~~~~~

That is the equivalent of writing:

~~~~~
    circle "Hi" fill 0xffe4c4
~~~~~

Because 0xffe4c4 is the 24-bit RGB value for "Bisque".

To put it another way, Pikchr treats the keyword "Bisque" as an
alternative spelling for the numeric literal 0xffe4c4.


<a id="reference-newpropval.md"></a>

### new-property-value


When setting the value of certain numeric properties (like
"`width`" and "`radius`") you can specify either an absolute
amount, or a percentage relative to the current setting.

So, for example, you can say:

~~~~~
    box width 2.3cm
~~~~~

To create a box with a width of 2.3 centimeters - an absolute amount.
Or, if the current "`boxwid`" variable value is 2.0cm, then you could
do the same by saying:

~~~~~
    box width 115%
~~~~~


<a id="reference-numprop.md"></a>

### numeric-property


There are really only four numeric properties:

  * width
  * height
  * radius
  * thickness

The width and height are the size of most objects.  The radius is used
to set the size of circles.  The thickness value is the width of lines used to
draw each object.  The other property names are just aliases for these
four:

  * wid &rarr; an abbreviation for "width"
  * ht &rarr; an abbreviation for "height"
  * rad &rarr; an abbreviation for "radius"
  * diameter &rarr;  twice the radius

#### Radius Of A "box" Object

By default, boxes have a radius of 0.  But if you assign a positive
radius to a box, it causes the box to have rounded corners:

~~~~~ pikchr center
box "radius 0"
move
box "radius 5px" rad 5px
move
box "radius 20px" rad 20px
~~~~~

#### Dimensions Of A "circle" Object

If you change any of the "width", "height", "radius", or "diameter" of
a circle, the other three values are set automatically.

#### Radius Of A "cylinder" Object

The "radius" of a "cylinder" object is the semiminor axis of the ellipse
that forms the top of the "cylinder".

~~~~~ pikchr center
C: cylinder
line thin left from C.nw - (2mm,0)
line thin left from C.nw - (2mm,C.radius)
arrow <- from 3/4<first line.start,first line.end> up 30%
arrow <- from 3/4<2nd line.start,2nd line.end> down 30%
text "radius" above at end of 1st arrow
~~~~~

Some examples:

~~~~~ pikchr center
cylinder "radius 50%" rad 50%
move
cylinder "radius 100%" rad 100%
move
cylinder "radius 200%" "height 200%" rad 200% ht 200%
~~~~~


#### Radius Of A "file"

For a "file" object, the radius is the amount by which the upper right
corner is folded over.

~~~~~ pikchr center
F: file
line thin from 2mm right of (F.e,F.n) right 75%
line thin from F.rad below start of previous right 75%
arrow <- from 3/4<first line.start,first line.end> up 30%
arrow <- from 3/4<2nd line.start,2nd line.end> down 30%
text "radius" above at end of 1st arrow
~~~~~

#### Radius Of A "line"

Setting a radius on a line causes the corners to be rounded by that
amount.

~~~~~ pikchr center
line go 2cm heading 40 then 4cm heading 165 then 1cm heading 280\
   "radius" "0"
move to 3cm right of previous.start
line same "radius" "15px" rad 15px
move to 3cm right of previous.start
line same  "radius" "30px" rad 30px
~~~~~


<a id="reference-textattr.md"></a>

### text-attribute


Any string literal that is intended to be displayed on the
diagram can be followed by zero or more of the following
keywords, in any order:

  * **above**
  * **aligned**
  * **below**
  * **big**
  * **bold**
  * **mono**
  * **monospace**
  * **center**
  * **italic**
  * **ljust**
  * **rjust**
  * **small**

#### Attributes "above" and "below"

The "`above`" and "`below`" keywords control the location of the
text above or below the center point of the object with which
the text is associated.  If there is just one text on the object
and the "`above`" and "`below`" keywords are omitted, the text is
placed directly over the center of the object.  This causes
the text to appear in the middle of lines:

~~~~ pikchr indent
  line "on the line" wid 150%
~~~~

So if there is just a single text label on a line, you probably
want to include either the "`above`" or "`below`" keyword.

~~~~ pikchr indent
  line "above" above; move; line "below" below
~~~~

If there are two texts on the object, they straddle the center point
above and below, even without the use of the "`above`" and "`below`"
keywords:

~~~~ pikchr indent
  line wid 300% "text without \"above\"" "text without \"below\""
~~~~

The "`above`" and "`below`" attributes do not stack or accumulate.
Each "`above`" or "`below`" overrides any previous "`above`" or "`below`"
for the same text.

If there are multiple texts and all are marked "`above`" or "`below`", then
all are placed above or below the center point, in order of appearance.

~~~~ pikchr indent
  line width 200% "first above" above "second above" above
  move
  line same "first below" below "second below" below
~~~~

#### Attributes "ljust" and "rjust"

As the "`above`" and "`below`" keywords control up and down positioning of
the text, so the "`ljust`" and "`rjust`" keywords control left and right
positioning.

For a line, the "`ljust`" means that the left side of the text is flush
against the center point of the line.  And "`rjust`" means that the right
side of the text is flush against the center point of the line.
(In the following diagram, the red dot is at the center of the line.)

~~~~ pikchr indent
   line wid 200% "ljust" ljust above "rjust" rjust below
   dot color red at previous.c
~~~~

For a block object, "`ljust`" shifts the text to be left justified
against the left edge of the block (with a small margin) and
"`rjust`" puts the text against the right side of the object (with
the same margin).

~~~~ pikchr indent
   box "ljust" ljust "longer line" ljust "even longer line" ljust fit
   move
   box "rjust" rjust "longer line" rjust "even longer line" rjust fit
~~~~

The behavior of "`ljust`" and "`rjust`" for block objects in Pikchr differs
from legacy PIC.
In PIC, text is always justified around the center point, as in lines.
But this means there is no easy way to left justify multiple lines of
text within a "box" or "file", and so the behavior was changed for
Pikchr.

Pikchr allows two texts to fill the same vertical slot if one is
"`ljust`" and the other is "`rjust`".

~~~~ pikchr indent
  box wid 300% \
     "above-ljust" above ljust \
     "above-rjust" above rjust \
     "centered" center \
     "below-ljust" below ljust \
     "below-rjust" below rjust
~~~~

#### Attribute "center"

The "`center`" attribute cancels all prior "`above`", "`below`",
"`ljust`", and "`rjust`" attributes for the current text.

#### Attributes "bold" and "italic"

The "`bold`" and "`italic`" attributes cause the text to use a bold or
an italic font.  Fonts can be both bold and italic at the same time.

~~~~ pikchr indent
  box "bold" bold "italic" italic "bold-italic" bold italic fit
~~~~

##### Monospace Font Family <a id="font-family"></a>

The "`mono`" or "`monospace`" attributes cause the text object to use a
monospace font.

~~~~ pikchr indent toggle
  box "monospace" monospace fit
~~~~

#### Attribute "aligned"

The "`aligned`" attribute causes text associated with a straight line
to be rotated to align with that line.

~~~~ pikchr indent
  arrow go 150% heading 30 "aligned" aligned above
  move to 1cm east of previous.end
  arrow go 150% heading 170 "aligned" aligned above
  move to 1cm east of previous.end
  arrow go 150% north "aligned" aligned above
~~~~

To display rotated text not associated with a line attach the
text to a line that is marked "`invisible`"

~~~~ pikchr indent
  box ht 200% wid 50%
  line invis from previous.s to previous.n "rotated text" aligned
~~~~

#### Attributes "big" and "small"

The "`big`" and "`small`" attributes cause the text to be a little larger
or a little smaller, respectively.  Two "`big`" attributes cause the
text to be larger still, as do two "`small`" attributes.  But the text
size does not increase or decrease beyond two "`big`" or "`small`" keywords.

~~~~ pikchr indent
  box "small small" small small "small" small \
    "(normal)" italic \
    "big" big "big big" big big ht 200%
~~~~

A "`big`" keyword cancels any prior "`small`" keywords on the same text,
and a "`small`" keyword cancels any prior "`big`" keywords.


<a id="reference-linelen.md"></a>

### line-length


A *line-length* is an expression that specifies how long to draw a
line segment.  The value can be either absolute (ex: "`1.2cm`", 
"`.5in`", "`0.5*circlerad`", and so forth) or it can be a percentage value
(ex: "`85%`").

  * *expr*
  * *expr* **%**

If the percentage value is used, the basis is usually the
value stored in the "`linewid`" variable.  However, for a case of
either:

  * **up** *expr* **%**
  * **down** *expr* **%**

…then the percentage refers to the current "`lineht`" value instead.  The
"`linewid`" value is always used for headings even if the heading
is "`0`" or "`180`" or "`north`" or "`south`".

In most cases it does not matter whether "`linewid`" or "`lineht`"
gets used for the percentage basis since both variables have the
same initial default of 0.5in.


<a id="reference-compassangle.md"></a>

### compass-angle


Because of the extensive historical use of compass heading names
like "north" and "se" (short for "south-east") in PIC and Pikchr,
it makes sense that angles should be specified according to compass
degrees.   North is 0&deg; and the angle increases clockwise so that
east is 90&deg;, south is 180&deg;, west is 270&deg; and 360&deg; is
back to north again.

~~~ pikchr
C: dot
arrow up from C; text " 0&deg;"
arrow right from C; text "  90&deg;" rjust
arrow down from C; text "180&deg;" below
arrow left from C; text "270&deg;  " ljust
~~~

Even though heading angles are specified in degrees, the arguments
to the built-in "sin()" and "cos()" functions are in radians.


<a id="reference-position.md"></a>

### position


A *position* is a point on the SVG canvas.  A *[place](#reference-place.md)* is
a specific position associated with an object.  Every *place* is a *position*,
but not every *position* is a *place*.  This page is about *position*.

  *  *expr* **,** *expr*
  *  *place*
  *  *place* **+** *expr* **,** *expr*
  *  *place* **-** *expr* **,** *expr*
  *  *place* **+ (** *expr* **,** *expr* **)**
  *  *place* **- (** *expr* **,** *expr* **)**
  *  **(** *position* **,** *position* **)**
  *  **(** *position* **)**
  *  *fraction* **of the way between** *position* **and** *position*
  *  *fraction* **way between** *position* **and** *position*
  *  *fraction* **between** *position* **and** *position*
  *  *fraction* **<** *position* **,** *position* **>**
  *  *distance* *which-way-from* *position*

#### Absolute versus Place-relative Positions

One form of a position is an (X,Y) coordinate pair.  This works, but
its use is discouraged.  It is better to use positions that are 
either a *[place](#reference-place.md)* or are derived from one or more places.

#### The "**(** *position* **,** *position* **)**" Form

A place of the form "(pos1,pos2)" where pos1 and pos2 are other positions
means use the X coordinate from pos1 and the Y coordinate from pos2.

~~~ pikchr
leftmargin = 1cm;
P1: dot; text "P1" with .s at 2mm above P1
P2: dot at P1+(2cm,-2cm); text "P2" with .s at 2mm above P2
dot at (P1,P2); text "(P1,P2)" with .s at 2mm above last dot
dot at (P2,P1); text "(P2,P1)" with .s at 2mm above last dot
~~~

#### "*fraction* **of the way between**" Forms

All of these syntactic forms of position are the same:

  *  *fraction* **of the way between** *position* **and** *position*
  *  *fraction* **way between** *position* **and** *position*
  *  *fraction* **between** *position* **and** *position*
  *  *fraction* **<** *position* **,** *position* **>**

The last form is the most cryptic, but it is also the most compact
and hence ends up being the most often used.

In all cases *fraction* is an expression that usually evaluates to between 0.0
and 1.0.  The resulting position is that fraction along a line that
connects the first *position* to the second *position*.

The *fraction* can be less than 0.0 or greater than 1.0, in which case
the point is on the extended line that connects the two positions.

~~~ pikchr
P1: dot; text "P1" with .s at 2mm above P1
P2: dot at P1+(4cm,1.5cm); text "P2" with .s at 2mm above P2
line thin color gray dotted from -.5<P1,P2> to 1.5<P1,P2>
dot at 3/4<P1,P2>; text "3/4<P1,P2>" at (last dot,P1)
   arrow thin color gray from last text.n to 1mm south of last dot
dot at -0.25 of the way between P1 and P2
   text "-0.25 of the way between P1 and P2" at (last dot,P2)
   arrow thin color gray from last text.s to 1mm north of last dot
~~~

#### "*position* *which-way-from* *position*" Forms

It is very common to specify a position as an offset from some other
position using this format.  Some examples:

  *  1cm below Obstacle.s
  *  0.5*linewid left of C0.w
  *  dist(C2,C3) heading 30 from C2


<a id="reference-place.md"></a>

### place


A *place* is a specific point on an object.
A *[position](#reference-position.md)* is a more general concept that means
any X,Y coordinate on the drawing.  This page is about *place*.

  *  *object*
  *  *object* *dot-edgename*
  *  *edgename* **of** *object*
  *  ORDINAL **vertex of** *object*

Every object has at least 9 places.  Line objects have additional
places for each internal vertex.   Most places are on the boundary
of the object, though ".center" or ".c" is in the middle.  The
".start" and ".end" places might be in the interior of the object
for the case of lines.
Some places may overlap.
Places usually have multiple names.
There are 22 different place names to refer to the 9 potentially
distinct places.

For a block object, when the layout direction is "right", we have:

~~~ pikchr
B: box thick thick color blue

circle ".n" fit at 1.5cm heading 0 from B.n
    arrow thin from previous to B.n chop
circle ".north" fit at 3cm heading 15 from B.north
    arrow thin from previous to B.north chop
circle ".t" fit at 1.5cm heading 30 from B.t
    arrow thin from previous to B.t chop
circle ".top" fit at 3cm heading -15 from B.top
    arrow thin from previous to B.top chop
circle ".ne" fit at 1cm ne of B.ne; arrow thin from previous to B.ne chop
circle ".e" fit at 2cm heading 50 from B.e; arrow thin from previous to B.e chop
circle ".right" fit at 3cm heading 75 from B.right
    arrow thin from previous to B.right chop
circle ".end&sup1;" fit at 3cm heading 100 from B.end
    arrow thin from previous to B.end chop
circle ".se" fit at 1cm heading 110 from B.se
    arrow thin from previous to B.se chop
circle ".s" fit at 1.5cm heading 180 from B.s
    arrow thin from previous to B.s chop
circle ".south" fit at 3cm heading 195 from B.south
    arrow thin from previous to B.south chop
circle ".bot" fit at 1.8cm heading 215 from B.bot
    arrow thin from previous to B.bot chop
circle ".bottom" fit at 2.7cm heading 160 from B.bottom
    arrow thin from previous to B.bottom chop
circle ".sw" fit at 1cm sw of B.sw; arrow thin from previous to B.sw chop
circle ".w" fit at 2cm heading 270 from B.w
    arrow thin from previous to B.w chop
circle ".left" fit at 3cm heading 180+75 from B.left
    arrow thin from previous to B.left chop
circle ".start&sup1;" fit at 2.5cm heading 295 from B.start
    arrow thin from previous to B.start chop
circle ".nw" fit at 1cm nw of B.nw; arrow thin from previous to B.nw chop
circle ".c" fit at 2.5cm heading -25 from B.c
    line thin from previous to 0.5<previous,B.c> chop
    arrow thin from previous.end to B.c
circle ".center" fit at 3.6cm heading 180-44 from B.center
    line thin from previous to 0.5<previous,B.center> chop
    arrow thin from previous.end to B.center
circle "&lambda;" fit at 2.5cm heading 250 from B
    line from previous to 0.5<previous,B> chop
    arrow thin from previous.end to B
~~~

The diagram above is for a box with square corners.  The non-center
places for other block objects are always on the boundary of the
object.  Thus for an ellipse:

~~~ pikchr
B: ellipse thick thick color blue

circle ".n" fit at 1.5cm heading 0 from B.n
    arrow thin from previous to B.n chop
circle ".north" fit at 3cm heading 15 from B.north
    arrow thin from previous to B.north chop
circle ".t" fit at 1.5cm heading 30 from B.t
    arrow thin from previous to B.t chop
circle ".top" fit at 3cm heading -15 from B.top
    arrow thin from previous to B.top chop
circle ".ne" fit at 1cm ne of B.ne; arrow thin from previous to B.ne chop
circle ".e" fit at 2cm heading 50 from B.e; arrow thin from previous to B.e chop
circle ".right" fit at 3cm heading 75 from B.right
    arrow thin from previous to B.right chop
circle ".end&sup1;" fit at 3cm heading 100 from B.end
    arrow thin from previous to B.end chop
circle ".se" fit at 1cm heading 110 from B.se
    arrow thin from previous to B.se chop
circle ".s" fit at 1.5cm heading 180 from B.s
    arrow thin from previous to B.s chop
circle ".south" fit at 3cm heading 195 from B.south
    arrow thin from previous to B.south chop
circle ".bot" fit at 1.8cm heading 215 from B.bot
    arrow thin from previous to B.bot chop
circle ".bottom" fit at 2.7cm heading 160 from B.bottom
    arrow thin from previous to B.bottom chop
circle ".sw" fit at 1cm sw of B.sw; arrow thin from previous to B.sw chop
circle ".w" fit at 2cm heading 270 from B.w
    arrow thin from previous to B.w chop
circle ".left" fit at 3cm heading 180+75 from B.left
    arrow thin from previous to B.left chop
circle ".start&sup1;" fit at 2.5cm heading 295 from B.start
    arrow thin from previous to B.start chop
circle ".nw" fit at 1cm nw of B.nw; arrow thin from previous to B.nw chop
circle ".c" fit at 2.5cm heading -25 from B.c
    line thin from previous to 0.5<previous,B.c> chop
    arrow thin from previous.end to B.c
circle ".center" fit at 3.6cm heading 180-44 from B.center
    line thin from previous to 0.5<previous,B.center> chop
    arrow thin from previous.end to B.center
circle "&lambda;" fit at 2.5cm heading 250 from B
    line from previous to 0.5<previous,B> chop
    arrow thin from previous.end to B
~~~

The "&lambda;" case refers to when a bare object name is used.
A bare object name is the same as referring to the center of
the object.

In the previous two diagrams, the ".start" and ".end" objects
are marked with "&sup1;" because
the location of ".start" and ".end" varies 
according to the layout direction.  The previous diagrams assumed
a layout direction of "right".  For other layout directions, we have:

<blockquote>
<table border="1" cellpadding="10px" cellspacing="0">
<tr><th>Layout Direction<th>.start<th>.end
<tr><td>right<td>.w<td>.e
<tr><td>down<td>.n<td>.s
<tr><td>left<td>.e<td>.w
<tr><td>up<td>.s<td>.n
</table></blockquote>

For a line, the place names refer to a bounding box that
encloses the line:

~~~ pikchr
B: line thick thick color blue go 0.8 heading 350 then go 0.4 heading 120 \
    then go 0.5 heading 35 \
    then go 1.2 heading 190  then go 0.4 heading 340 "+"

   line thin dashed color gray from B.nw to B.ne to B.se to B.sw close

circle ".n" fit at 1.5cm heading 0 from B.n
    arrow thin from previous to B.n chop
circle ".north" fit at 3cm heading 15 from B.north
    arrow thin from previous to B.north chop
circle ".t" fit at 1.5cm heading 30 from B.t
    arrow thin from previous to B.t chop
circle ".top" fit at 3cm heading -15 from B.top
    arrow thin from previous to B.top chop
circle ".ne" fit at 1cm ne of B.ne; arrow thin from previous to B.ne chop
circle ".e" fit at 2cm heading 50 from B.e; arrow thin from previous to B.e chop
circle ".right" fit at 3cm heading 75 from B.right
    arrow thin from previous to B.right chop
circle ".end" fit at 2cm heading 120 from B.end
    arrow thin from previous to B.end chop
circle ".se" fit at 1cm heading 170 from B.se
    arrow thin from previous to B.se chop
circle ".s" fit at 1.5cm heading 180 from B.s
    arrow thin from previous to B.s chop
circle ".south" fit at 3cm heading 195 from B.south
    arrow thin from previous to B.south chop
circle ".bot" fit at 1.8cm heading 215 from B.bot
    arrow thin from previous to B.bot chop
circle ".bottom" fit at 2.7cm heading 160 from B.bottom
    arrow thin from previous to B.bottom chop
circle ".sw" fit at 1cm sw of B.sw; arrow thin from previous to B.sw chop
circle ".w" fit at 2cm heading 300 from B.w
    arrow thin from previous to B.w chop
circle ".left" fit at 3cm heading 280 from B.left
    arrow thin from previous to B.left chop
circle ".start" fit at 2.5cm heading 265 from B.start
    arrow thin from previous to B.start chop
circle ".nw" fit at 1cm nw of B.nw; arrow thin from previous to B.nw chop
circle ".c" fit at 2.5cm heading -15 from B.c
    line thin from previous to 0.5<previous,B.c> chop
    arrow thin from previous.end to B.c
circle ".center" fit at 3.3cm heading 110 from B.center
    line thin from previous to 0.5<previous,B.center> chop
    arrow thin from previous.end to B.center
circle "&lambda;" fit at 1.7cm heading 250 from B
    line from previous to 0.5<previous,B> chop
    arrow thin from previous.end to B
~~~

The ".start" of a line always refers to its first vertex.
The ".end" is usually the last vertex, except when the "`close`"
keyword is used, in which case the ".end" is the same as
".e", ".s", ".w", or ".n" depending on layout direction,
just like a block object.

The vertexes of a line object are also places:

~~~ pikchr
B: line -> thick color blue go 0.8 heading 350 then go 0.4 heading 120 \
    then go 0.5 heading 35 \
    then go 1.2 heading 190  then go 0.4 heading 340

oval "1st vertex" fit at 2cm heading 250 from 1st vertex of B
    arrow thin from previous to 1st vertex of B chop
oval "2nd vertex" fit at 2cm west of 2nd vertex of B
    arrow thin from previous to 2nd vertex of B chop
oval "3rd vertex" fit at 2cm north of 3rd vertex of B
    arrow thin from previous to 3rd vertex of B chop
oval "4th vertex" fit at 2cm east of 4th vertex of B
    arrow thin from previous to 4th vertex of B chop
oval "5th vertex" fit at 2cm east of 5th vertex of B
    arrow thin from previous to 5th vertex of B chop
oval "6th vertex" fit at 2cm heading 200 from 6th vertex of B
    arrow thin from previous to 6th vertex of B chop
~~~
