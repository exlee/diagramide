# DiagramIDE feature guide

## Windows

- The close button hides a window. Reopen it from the **Windows** menu.
- Hold **Cmd** (macOS) or **Ctrl** (other platforms) and select close to delete
  the window from the workspace.
- Deleting a diagram editor also deletes its Render window.
- A deleted Render window comes back the next time its editor renders.
- The workspace and window layout persist between launches.
- You can save a workspace to JSON and load it back.

## Editors and shortcuts

- **Cmd/Ctrl+R** renames the focused editor.
- **Enter** inserts a new line with indentation for the editor's language.
- Editors re-render as you type.
- Pikchr editors render Pikchr directly.
- Prolog editors evaluate a `diagram//0` DCG into Pikchr.
- Tcl editors return Pikchr text. They need Tcl 8.6.
- Ruby editors use `print` and `puts` output as Pikchr. They need Ruby.
- CLIPS editors assert facts against shape templates (`box`, `circle`, `arrow`,
  and others) and run rules. Each shape fact becomes 1 Pikchr statement, in
  assertion order. After layout, each shape fact gets its centre in `x` and
  `y`, and the rules run again. Parinfer keeps closing parens in step with
  indentation while you type.
- Plain text editors hold reusable text and have no Render window.

## Cross-window references

- `!!NAME!!` includes the raw source of another named editor, including Plain
  text editors.
- `$$NAME$$` includes the generated Pikchr output of another diagram editor.
- Dependent windows update when a referenced editor changes. References nest up
  to 3 replacement passes.

## Rendering and export

- Each diagram editor has a resizable Render window.
- From a Render window, export SVG, PNG, or transparent PNG, or copy the
  generated Pikchr source.
- Evaluation and render errors appear next to the editor and in the Logger
  window.

## Workspaces

- The **Workspace** menu lists every workspace. Select one to switch.
- The central panel heading shows the active workspace's name.
- Each workspace has its own editors and references. Theme, diagram background,
  logger, and view scale are shared.
- Each workspace has New, Rename, Duplicate, and Delete actions. Delete asks for
  confirmation. You can't delete the last workspace.
- **Reset Workspace...** deletes all editors and windows in the active workspace.
- **Save Workspace** exports the active workspace as JSON. **Load Workspace**
  imports a file as a new workspace and switches to it.
- All workspaces persist between launches. Older single-workspace save files
  migrate automatically.

## Other tools

- The **Windows** menu shows or hides editors, the Logger, and the egui Debug
  window. Each editor's **R** button shows or hides its Render window.
- The **View** menu scales the interface.
- The top-level **?** button opens the full guide. Each window's **?** button
  opens help for that window.
