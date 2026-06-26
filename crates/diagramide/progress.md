# Scout Progress: mini_window.rs & editor.rs Analysis

Status: COMPLETE

Findings written to: /tmp/scout-miniwindow.md

Key findings:
1. 18 traits in mini_window.rs, ~10 are trivial getters that could be consolidated into a shared struct
2. InitializeWatchTx spawns unbounded tokio tasks polling every 100ms — only used by PikchrEditor and SvgWindow, while other editors already use the simpler DebouncedTrySend pattern
3. get_editor_window() copy-pasted across 4 editors; editor_spec() nearly identical across 4 editors
4. Minor per-frame String allocations in get_title() and error display path
