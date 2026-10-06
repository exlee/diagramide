# Changelog

## [Unreleased]

### Added

- The CLIPS editor completes names. After `(` it offers built-in functions, deffunctions, deftemplates, and constructs. Inside a fact or pattern it offers the slots of that template, also for `modify` of a bound fact. Commands such as `undeffacts` complete deffacts names, `from` and `to` complete shape ids, and `?` completes the variables of the current form. Press Tab to complete as far as all matches agree.

## [1.2.0] - 2026-10-05

### Added

- Add a CLIPS editor that turns facts asserted against shape templates, plus rules, into Pikchr. CLIPS 6.4.2 is vendored and built through clips-bindings.
- Add a CLIPS Guide to Help, with a live preview for each example.
- Add a layout pass to the CLIPS editor. After the rules run, each shape fact receives the rendered center of the shape in the `x` and `y` slots, and the rules run again. Add the `at-pos` and `at-rel` slots and the `modify-at`, `modify-at-x`, `modify-at-y`, and `anchor` functions to place shapes from coordinates. Report rule errors only from the last layout pass.
- Add request facts to the CLIPS editor. `text-from`, `lines-from`, and `pikchr-from` assert `source-text`, `source-line`, and `group` facts with the content of another editor. `include` reads another editor as CLIPS source. Add the `raw-pikchr`, `raw-text`, and `group` output facts. A Render window shares the name of its editor and no longer hides the editor from request facts.
- The CLIPS editor runs Parinfer (parinfer-rust, smart mode): closing parens follow indentation, and typing `(` adds its `)`. Loaded text has its indentation fixed to match its parens first.
- Add a Hagoromo editor. You write a Gluon script that combines shapes in the style of Haskell Diagrams, and hagoromo renders the result to SVG in-process. The editor has no Output Type because the script defines the diagram.
- Add a Hagoromo Guide to Help. Each example shows its drawing, and you can select the drawing to see its source.
- The Hagoromo editor completes names. The bottom line lists the names that match the word before the cursor and shows the type of the first match. Press Tab to complete as far as all matches agree.
- In a Hagoromo editor, `!!NAME!!` that refers to another Hagoromo editor becomes `ref_NAME`, a binding to that editor's diagram. Referenced scripts are defined before the scripts that use them, including nested references.
- Add an Svgbob Guide to Help.
- Text editors draw a block cursor.
- Ctrl-D deletes the character after the cursor. Ctrl-/ (Cmd-/ on macOS) comments or uncomments the current line or every selected line.

### Changed

- Rewrite user-facing copy to follow the Microsoft Writing Style Guide.
- Hide gluon_vm INFO logs by default.
- Add a cargo-deny configuration.

### Fixed

- Error messages no longer cover the editor. An error appears below the editor window, or above it when the error does not fit below. A long error scrolls instead of extending past the screen edge.

## [1.1.0] - 2026-08-04

### Added

- Add per-editor Pikchr and Svgbob output selection with renderer switching.
- Add a dedicated Svgbob canvas editor with Insert and Replace modes, rectangle selection, keyboard navigation, generated-reference overlays, and directional Replace-mode continuation.
- Add file and clipboard export actions for rendered images and generated source.
- Add per-window zoom controls.
- Add interaction performance workloads and benchmark baselines.

### Changed

- Stop wrapping Svgbob canvases and keep the cursor in view.
- Switch to Tabler icons and add Output Type and edit-mode indicators.
- Make the help guide easier to read and document editor and export behavior.
- Optimize measured interaction paths and reorganize serialization, message handling, grammar help, mini-window, and Svgbob components.
- Update Wasmtime and WASI to 46.0.2 and refresh vulnerable transitive dependencies.

### Fixed

- Restore RON compatibility and recover from incompatible saved workspaces without data loss.
- Fix Svgbob undo and redo history.
- Prevent dropped render updates and sync render-window visibility with editor toggles.
- Fix column paste truncating canvas rows.
- Improve Svgbob keyboard, rectangle-selection, viewport, and canvas behavior.

### Security

- Update dependencies to fix RustSec vulnerabilities reported before 1.1.0.

## [1.0.0] - 2026-06-28

First release of the DiagramIDE workspace.

### DiagramIDE

#### Added
- Add the DiagramIDE desktop app for writing Pikchr diagrams as text with live previews.
- Add multi-editor workspaces with persisted state, named snippets, render windows, and workspace management.
- Add SVG, PNG, transparent PNG, and Pikchr source export.
- Add generation editors for Prolog, Tcl, Ruby, and plain-text composition.
- Add help windows with bundled Pikchr grammar and reference, with syntax highlighting.
- Add theming, diagram background controls, icons, bundled fonts, and macOS application bundle metadata.
- Add CI and nightly and release artifact workflows for Linux, macOS, and Windows.

#### Fixed
- Fix rendering, export, and background handling across SVG and PNG output.
- Fix workspace persistence, window sizing, rename handling, and editor focus behavior.
- Fix Tcl compatibility, Ruby naming, and generated-source inclusion edge cases.
- Fix CI build coverage, macOS artifact packaging, and clippy and test issues found before release.

### pikchr.pro

#### Added
- Add the `pikchr_pro` library and CLI, which turn Prolog DCGs into SVG through Pikchr.
- Bundle Pikchr C sources and Trealla-backed Prolog execution for the Prolog-to-diagram pipeline.
- Split sync and async features and add reusable Prolog engine abstractions.

#### Fixed
- Fix Prolog module loading, error trimming, render triggering, and cross-platform build behavior.

### pikchr.pl

#### Added
- Add the original iced-based Pikchr GUI and CLI artifact shipped alongside DiagramIDE.
- Add Prolog helper modules, file watching, heredoc parsing, editor state persistence, keybindings, and undo support.
- Add bundled font and native Prolog support files used by the older GUI.

#### Fixed
- Fix indentation, command shortcuts, dirty indicators, dependency updates, and generated-source inclusion behavior.

### trealla-wasm

#### Added
- Add the `trealla-wasm` crate, which runs Trealla Prolog on WASM for text-to-text Prolog execution.
- Bundle the Trealla WASM runtime artifact, attribution, license, and build integration.

#### Fixed
- Fix all-architecture Wasmtime build configuration used by CI and release builds.
