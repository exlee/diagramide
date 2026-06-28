# Changelog

## [Unreleased]

## [1.0.0] - 2026-06-28

Initial release of the DiagramIDE workspace.

### DiagramIDE

#### Added
- Add the root DiagramIDE desktop application for authoring Pikchr diagrams as text with live previews.
- Add multi-editor workspaces with persisted state, named snippets, render windows, and workspace management.
- Add SVG, PNG, transparent PNG, and Pikchr-source export flows.
- Add generation editors for Prolog, Tcl, Ruby, and plain-text composition.
- Add built-in help windows with bundled Pikchr grammar/reference material and syntax highlighting.
- Add theming, diagram background controls, icons, bundled fonts, and macOS application bundle metadata.
- Add CI and generated nightly/release artifact workflows for Linux, macOS, and Windows targets.

#### Fixed
- Fix rendering, export, and background handling across SVG and PNG output.
- Fix workspace persistence, window sizing, rename handling, and editor focus behavior.
- Fix Tcl compatibility, Ruby naming, and generated-source inclusion edge cases.
- Fix CI build coverage, macOS artifact packaging, and clippy/test issues found during release preparation.

### pikchr.pro

#### Added
- Add the `pikchr_pro` library and CLI for transforming Prolog DCGs into Pikchr-rendered SVG output.
- Bundle Pikchr C sources and Trealla-backed Prolog execution for the Prolog-to-diagram pipeline.
- Add sync/async feature split and reusable Prolog engine abstractions.

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
- Add the `trealla-wasm` crate wrapping Trealla Prolog over WASM for text-to-text Prolog execution.
- Bundle the Trealla WASM runtime artifact, attribution, license, and build integration.

#### Fixed
- Fix all-architecture Wasmtime build configuration used by CI and release builds.
