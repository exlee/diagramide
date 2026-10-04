Features:
- Consider an M4 editor
- Consider a Markdown editor
- Save and load diagram source, combined or per editor
- Bundle saves: save code exports together instead of the whole workspace
- (?) Theming support
- Per-editor font size
- Change the diagram font
- Change the editor font
- Sync-Export: export automatically on change

Quality of life:
- Improve window folding
- Improve window layout
- Replace the file picker with an easier one
- Add a menu bar with common actions to editors

Architecture:
- Reconsider the event-based architecture
- Add tests to shorten the feedback loop (hard because of the event-processing loop)

Underdeveloped:
- Tcl library detection
- Tcl usage: the tcl-sys fork (needed to compile on CI) made everything harder
- Documentation is missing and features are hard to find (Help windows?)
- Editor lacks many quality-of-life features, such as region indenting and auto-formatting
- Error and success reporting: most actions give no notification

Performance:
- Resizing a window can still cause jank

Known issues:
- Changing fields in State structs discards saved state (and probably breaks workspace loading)

DONE:
- Editor errors overlay the code; consider a side window?
- Missing debounce causes visible UI jank
- Export PNGs with transparent background
- State persists window size
- Remove tracy and tracing
- /Does state need an extra Arc<RwLock> on windows? Seems superfluous./
- "!!TAG!!" source inclusion doesn't update dependencies automatically
