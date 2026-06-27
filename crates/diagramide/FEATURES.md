# DiagramIDE feature guide

## Window behavior

- The normal close button hides a window. Reopen it from the **Windows** menu.
- Holding **Cmd** on macOS or **Ctrl** elsewhere while clicking close permanently
  deletes that window from the workspace.
- Deleting a diagram editor also deletes its paired Render window.
- Deleting only a Render window is temporary in practice: the editor recreates it
  the next time it successfully renders.
- The workspace and window layout are automatically persisted between launches.
- Workspaces can also be saved to and loaded from JSON files.

## Editors and shortcuts

- **Cmd/Ctrl+R** renames the focused editor.
- **Enter** inserts a newline with language-aware automatic indentation.
- Editors evaluate and update their Render windows automatically while typing.
- Pikchr editors render Pikchr directly.
- Prolog editors evaluate a `diagram//0` DCG into Pikchr.
- Tcl editors return Pikchr text and are available when Tcl 8.6 can be loaded.
- Ruby editors use `print` and `puts` output as Pikchr and are available when Ruby
  support is available.
- Plain text editors hold reusable raw text and do not have Render windows.

## Cross-window references

- `!!NAME!!` includes the raw source of another named editor. This works with
  Plain text windows.
- `$$NAME$$` includes the generated Pikchr output of another named diagram editor.
- References update dependent windows automatically and can be nested up to three
  replacement passes.

## Rendering and export

- Every diagram editor has a paired, resizable live Render window.
- Render windows can export SVG, PNG, transparent PNG, or copy generated Pikchr
  source to the clipboard.
- Evaluation and render errors appear next to the editor and in the Logger window.

## Workspace tools

- The **Workspace** menu lists every workspace; select one to switch.
- The active workspace's name is shown in the central panel heading.
- Each workspace keeps its own editors and cross-window references; global
  settings (theme, diagram background, logger, view scale) are shared.
- New, Rename, Duplicate, and Delete (with confirmation) are available per
  workspace. The last workspace can never be deleted.
- **Reset Active Workspace…** clears only the currently active workspace.
- **Save Workspace** exports the active workspace as JSON; **Load Workspace**
  imports a file as a brand-new workspace and switches to it.
- All workspaces are persisted transparently between launches; older
  single-workspace save files are migrated automatically.

## Other tools

- The **Windows** menu shows or hides editor windows, the Logger, and egui's
  Debug window. Render windows are controlled from each editor's **R** button.
- The **View** menu scales the complete interface.
- The top-level **?** button opens the complete guide; each window's **?** button
  opens contextual help.
