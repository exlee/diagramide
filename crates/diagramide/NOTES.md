Features:
- Consider adding M4 editor
- Consider adding Markdown editor
- Add possibility to save/load combined/non-combined diagrams source codes
- Bundle saves (i.e. save with bunch of code exports instead of Workspace saving)
- (?) Theming support
- Per-editor font-size
- Export PNGs with transparent background
- Change font used in diagrams
- Change font used in editor

QoL changes:
- Improve window folding mechanism 
- Improve window layout mechanism
- Replace file-picker dialog with something more user-friendly
- Editors should have menubar with common actions

Architectural:
- Reconsider event-based architecture
- Implement testing to speed up feedback loop (note: difficult due to event-processing loop)

Underdeveloped:
- TCL library detection
- Documentation is non-existent, features are impossible to find (Help windows?)

Performance:
- (!) Lack of debounce starts to be visible introducing UI jank 
- Resizing window still can introduce jankiness

Known issues:
- Changing fields in State structs discards saved state (and probably prevents workspace loading as well)
- When using "!!TAG!!" for source inclusion dependencies aren't updated automatically
- Error in editor overlays code - consider moving to a side window?

