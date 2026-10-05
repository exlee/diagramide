<p align="center">
  <img src="./icon.svg" width="140" alt="DiagramIDE logo" />
</p>
<h1 align="center">DiagramIDE</h1>
<p align="center"><em>Diagrams as source files.</em></p>

DiagramIDE is a desktop editor for writing [Pikchr] or [Svgbob] diagrams as
text. A live preview appears beside the source.

Write a diagram directly, assemble a diagram from named fragments, or generate a
diagram from a program in Prolog, Tcl, Ruby, CLIPS, or Gluon (Hagoromo editor).
Svgbob diagrams are plain-text ASCII art. Each diagram editor has
an Output Type of Pikchr (the default) or Svgbob. You can save SVG, PNG,
transparent PNG, or the generated source to a file, or copy the result to the
clipboard.

## Screenshots

![Direct Pikchr source and paired render window](./assets/images/pikchr_diagram.png)
*Pikchr source and its render window.*

![Ruby script that generates Pikchr](./assets/images/ruby_diagram.png)
*Ruby script that generates Pikchr.*

![Export menu](./assets/images/export.png)
*Export menu.*

## Features

**Source**
- Source editor with live preview. Choose Pikchr or Svgbob per editor.
- Svgbob canvas editor for ASCII art.
- **Hagoromo** editor: a [Gluon] script builds a diagram from combinators in the
  style of Haskell [Diagrams] (`circle 1.0 ||| square 2.0 |> fc color.red`).
  [hagoromo] renders the diagram in-process. The Gluon script produces the
  rendered diagram directly, so the Hagoromo editor has no Output Type.
- Plain-text editors hold fragments for reuse.

**Generation**
- **Prolog**: define the diagram as a DCG with root `diagram//0`. The Prolog
  editor runs [Trealla Prolog] embedded through WASM.
- **Tcl**: write a Tcl script that transforms text into diagram source. The Tcl
  editor requires Tcl 8.6 libraries.
- **Ruby**: write diagram source with `print` or `puts`. The Ruby editor
  requires a Ruby installation.
- **CLIPS**: assert facts such as `(box (id b) (label "Hello"))` and let rules
  add more facts. Each shape fact becomes 1 Pikchr statement. After rendering,
  DiagramIDE writes the rendered center of each shape into the `x` and `y`
  slots of the shape fact. The CLIPS editor runs the embedded [CLIPS] 6.4.2
  engine through [clips-bindings].

**Composition**
- `$$name$$` includes the generated source of another editor. The including
  editor and the included editor need the same Output Type.
- `!!name!!` includes another editor's raw source.

**Workspaces**
- Keep related editors, render windows, and fragments together. Workspaces
  save automatically.

**Export**
- SVG, opaque PNG, and transparent PNG.
- Copy generated Pikchr, Svgbob, or Hagoromo source.
- Renders use the Space Mono font.

## Installation

Build from source with `cargo install --path .`, download the
[1.1.0 release][release-1-1-0], or try the [nightly release][nightly].

[release-1-1-0]: https://github.com/exlee/diagramide/releases/tag/v1.1.0
[nightly]: https://github.com/exlee/diagramide/releases/tag/latest

## Satellite projects

The repository is a Cargo workspace. The root crate is DiagramIDE. The other
crates are:

| Project | Crate · path | Role | License |
|---|---|---|---|
| **pikchr.pro** | `pikchr_pro` · `crates/pikchr_pro` | Prolog to Pikchr to SVG library and CLI | GPL-3.0-only |
| **pikchr.pl** | `pikchr_pl` · `crates/pikchr_pl` | First GUI, built with iced. DiagramIDE replaces pikchr.pl. | GPL-3.0-only |
| `trealla-wasm` | `crates/trealla_wasm` | Trealla Prolog on a WASM runtime | MIT |

### pikchr.pro (CLI)

`pikchr_pro` reads a Prolog file with a `diagram//0` DCG on STDIN and writes
SVG to STDOUT:

```
cat my_diagram.pl | pikchr_pro > output.svg
```

### pikchr.pl

The original GUI, built with iced. DiagramIDE (built with egui) replaces
pikchr.pl, but pikchr.pl remains in the repository and ships in nightly
releases.

## Status: alpha

DiagramIDE creates and exports diagrams, but the user interface and behavior
are unfinished:

- An update can wipe your autosaved workspace. Keep backups.
- Some features are missing, such as indenting selected lines and shortcuts for
  many actions.
- Expect leftover code, verbose debug output, and undocumented behavior.

### Hidden features

- <kbd>Cmd/Ctrl</kbd>+<kbd>R</kbd> renames the focused editor.
- <kbd>Cmd/Ctrl</kbd>+select the × button to delete a window. Without the
  modifier, the × button hides the window.

## Wrapper languages

A language can be added to DiagramIDE if the language meets 2 requirements:

- The language returns text valid for the editor's Output Type.
- The language embeds in Rust.

Prolog (Trealla on WASM) was the first wrapper language. DCGs make diagrams
declarative, and atoms make diagrams composable. After writing a Prolog helper
library (embedded in `pikchr.pl`, not DiagramIDE), I often preferred raw Pikchr.

Tcl was the second wrapper language. Tcl suits the text transformation
DiagramIDE performs. Other candidates are M4 as a macro layer and Markdown for
diagrams with text. Each candidate, including Starlark, depends on whether the
language embeds in Rust.

## Why DiagramIDE

![Categorization](./assets/images/categorization.png)

- Diagrams communicate well, but drawing and updating diagrams is hard.
- Most diagramming tools reach a point where you must accept poor output or work
  around the tool.
- Graphics programs lack composition. You can't define a node once and edit
  every instance.
- Of the existing diagram languages, [Pikchr] suits DiagramIDE best, but Pikchr
  scripting has no conditionals and limited loops.
- Pikchr SVG output has no embedded font or background. The missing font and
  background make Pikchr SVG hard to use in code, and high-quality
  rasterization requires extra work.
- You want to see the diagram while you write the diagram source.

## License

**DiagramIDE** uses the **Business Source License 1.1** (BSL). The satellite
projects **pikchr.pl** and **pikchr.pro** use **GPL-3.0-only**.

- **DiagramIDE** (`diagramide`, root crate): Business Source License 1.1. Source-available; mandated or corporate use requires a commercial license. Converts to **GPL-3.0-or-later** on 2029-01-01. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
- **pikchr.pl and pikchr.pro** (`pikchr_pl`, `pikchr_pro`): GPL-3.0-only. See each crate's `LICENSE`.
- **trealla-wasm** (`trealla_wasm`): MIT. See its `LICENSE`.
- **Space Mono font**: SIL Open Font License 1.1. See [`assets/fonts/LICENSE.SpaceMono`](./assets/fonts/LICENSE.SpaceMono).
- **Tabler Icons**: MIT. See [`assets/icons/LICENSE.Tabler`](./assets/icons/LICENSE.Tabler).
- **Trealla Prolog**: MIT-style license. See [`crates/trealla_wasm/native/tpl/LICENSE`](./crates/trealla_wasm/native/tpl/LICENSE).
- **Pikchr**: the author disclaims copyright (zero-clause BSD). See the header of [`crates/pikchr_pro/native/pikchr/pikchr.c`](./crates/pikchr_pro/native/pikchr/pikchr.c).
- **hagoromo** and **Gluon**: MIT.
- **CLIPS** 6.4.2: MIT No Attribution. **clips-bindings**: MIT.
- **Svgbob**: Apache-2.0, pinned to the [`exlee/svgbob` optimization revision](https://github.com/exlee/svgbob/tree/axk-optimization-work).

[Pikchr]: https://pikchr.org
[Svgbob]: https://github.com/ivanceras/svgbob
[hagoromo]: https://crates.io/crates/hagoromo
[CLIPS]: https://clipsrules.net
[clips-bindings]: https://crates.io/crates/clips-bindings
[Gluon]: https://gluon-lang.org
[Diagrams]: https://diagrams.github.io
[Trealla Prolog]: https://github.com/trealla-prolog/trealla
