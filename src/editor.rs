use eframe::egui::{self, Context, Id, Ui, text_edit::TextEditOutput};
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    mini_window::{self, HasError, Id as IdTrait, InnerWindow},
    state::DiagramBackground,
};

pub fn get_line_indent(line: &str) -> String {
    let mut new_string = String::new();
    for c in line.chars() {
        if c.is_whitespace() {
            new_string.push(c);
        } else {
            break;
        }
    }
    new_string
}

pub fn get_last_line(line: &str) -> String {
    let mut new_string = String::new();
    if let Some(newline_pos) = line.rfind("\n") {
        new_string.push_str(&line[newline_pos + 1..]);
    } else {
        new_string.push_str(line);
    }
    new_string
}

/// Adjust a character index after removals were applied to the text.
/// `removals` is a slice of `(position, count)` pairs in ascending position
/// order, representing contiguous spans that were deleted.
fn adjust_after_removals(index: usize, removals: &[(usize, usize)]) -> usize {
    let mut subtract = 0;
    for &(pos, count) in removals {
        if pos + count <= index {
            subtract += count;
        } else if pos < index {
            subtract += index - pos;
        }
        // else: removal is entirely after index, no effect
    }
    index - subtract
}

fn adjust_after_insertions(index: usize, insertions: &[usize], insertion_len: usize) -> usize {
    index + insertions.iter().filter(|&&pos| pos <= index).count() * insertion_len
}

/// Collect the byte-offset of the first character of every line spanned by
/// the range `[start, end)` in `content`.
///
/// A line whose start equals `end` is excluded (the selection ends at its
/// very beginning, so it is not really part of the selected lines).
fn collect_line_starts(content: &str, start: usize, end: usize) -> Vec<usize> {
    let safe_start = start.min(content.len());
    let safe_end = end.min(content.len());

    let first_line_start = content[..safe_start]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(0);

    let mut line_starts: Vec<usize> = vec![first_line_start];

    let region = &content[first_line_start..safe_end];
    let mut base = first_line_start;
    for c in region.chars() {
        if c == '\n' {
            let next = base + 1;
            if next < end {
                line_starts.push(next);
            }
        }
        base += c.len_utf8();
    }

    line_starts
}

pub trait HandleEnter: mini_window::RawContent {
    fn handle_enter(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
    ) -> bool {
        let is_focused = ui.memory(|mem| mem.has_focus(editor_id));
        if is_focused {
            ui.input_mut(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
                    true
                } else {
                    false
                }
            })
        } else {
            false
        }
    }

    /// Handle Tab (indent) and Shift-Tab (dedent) key presses.
    ///
    /// When the editor is focused and Tab is pressed without Ctrl/Cmd:
    /// - **Tab** indents by 2 spaces (all selected lines, or the current line)
    /// - **Shift-Tab** dedents by 2 spaces (all selected lines, or current
    ///   line)
    fn handle_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, editor_id: egui::Id) -> bool {
        let is_focused = ui.memory(|mem| mem.has_focus(editor_id));
        if !is_focused {
            return false;
        }

        let action = ui.input(|i| {
            if i.key_pressed(egui::Key::Tab) {
                Some(i.modifiers.shift)
            } else {
                None
            }
        });

        let Some(shift) = action else {
            return false;
        };

        // Consume the Tab event so the TextEdit widget does not insert a '\t'.
        // Modifiers::NONE matches logically, so it covers both Tab and Shift-Tab
        // (but not Ctrl/Cmd+Tab, which we leave alone).
        ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
        });

        if shift {
            self.do_dedent(ctx, editor_id)
        } else {
            self.do_indent(ctx, editor_id)
        }
    }

    /// Indent: insert 2 spaces at the start of the current line or every
    /// selected line.
    fn do_indent(&mut self, ctx: &egui::Context, editor_id: egui::Id) -> bool {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) else {
            return false;
        };
        let Some(range) = state.cursor.char_range() else {
            return false;
        };

        let mut content = self.get_raw_content();
        let primary = range.primary.index;
        let secondary = range.secondary.index;
        let start = primary.min(secondary);
        let end = primary.max(secondary);

        let line_starts = collect_line_starts(&content, start, end);

        // Insert "  " right-to-left so earlier positions stay valid.
        for &pos in line_starts.iter().rev() {
            content.insert_str(pos, "  ");
        }

        let new_start = adjust_after_insertions(start, &line_starts, 2);
        let new_end = adjust_after_insertions(end, &line_starts, 2);
        let (new_primary, new_secondary) = if primary <= secondary {
            (new_start, new_end)
        } else {
            (new_end, new_start)
        };

        if new_primary == new_secondary {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(new_primary),
                )));
        } else {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(new_primary),
                    egui::text::CCursor::new(new_secondary),
                )));
        }

        state.store(ctx, editor_id);
        self.set_raw_content(content);
        true
    }

    /// Dedent: remove up to 2 leading spaces from the current line or every
    /// selected line.
    fn do_dedent(&mut self, ctx: &egui::Context, editor_id: egui::Id) -> bool {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) else {
            return false;
        };
        let Some(range) = state.cursor.char_range() else {
            return false;
        };

        let mut content = self.get_raw_content();
        let primary = range.primary.index;
        let secondary = range.secondary.index;
        let start = primary.min(secondary);
        let end = primary.max(secondary);

        let line_starts = collect_line_starts(&content, start, end);

        // Determine how many leading spaces (up to 2) each line has.
        let bytes = content.as_bytes();
        let mut removals: Vec<(usize, usize)> = Vec::new();
        for &ls in &line_starts {
            let mut count = 0;
            while count < 2 && ls + count < bytes.len() && bytes[ls + count] == b' ' {
                count += 1;
            }
            if count > 0 {
                removals.push((ls, count));
            }
        }

        if removals.is_empty() {
            return false;
        }

        // Apply removals right-to-left.
        for &(pos, count) in removals.iter().rev() {
            content.drain(pos..pos + count);
        }

        let new_start = adjust_after_removals(start, &removals);
        let new_end = adjust_after_removals(end, &removals);

        let (new_primary, new_secondary) = if primary <= secondary {
            (new_start, new_end)
        } else {
            (new_end, new_start)
        };

        if new_primary == new_secondary {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(new_primary),
                )));
        } else {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(new_primary),
                    egui::text::CCursor::new(new_secondary),
                )));
        }

        state.store(ctx, editor_id);
        self.set_raw_content(content);
        true
    }
    fn handle_indent<F>(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
        get_indent: F,
    ) where
        F: Fn(&str) -> String,
    {
        if let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id)
            && let Some(range) = state.cursor.char_range()
        {
            let mut content = self.get_raw_content();
            let mut cursor = range.primary.index;

            let cursor_line = get_last_line(&content[..range.primary.index]);

            let indent = get_indent(&cursor_line);
            let insertion = format!("\n{}", indent.as_str());
            content.insert_str(cursor, &insertion);
            cursor += insertion.len();

            let ch_range = egui::text::CCursorRange::one(egui::text::CCursor::new(cursor));
            state.cursor.set_char_range(Some(ch_range));

            state.store(ui.ctx(), editor_id);
            self.set_raw_content(content);
        }
    }
}

impl<T> HandleEnter for T where T: Editor + mini_window::RawContent {}
pub trait Editor {}

pub trait GenericEditor: HandleEnter + IdTrait {
    fn editor_spec(&mut self, editor_id: Id, ui: &mut Ui) -> TextEditOutput;
    fn handle_enter(&mut self, ctx: &Context, ui: &mut Ui, editor_id: Id);
    fn editor_on_changed(&self, tx: Sender<Msg>, ctx: &Context);
    fn initialize(&mut self, tx: Sender<Msg>);

    /// Override for editor-specific Tab bindings. Diagram source editors keep
    /// the shared indentation behavior; ASCII-art editors can opt out.
    fn handle_tab_binding(&mut self, ctx: &Context, ui: &mut Ui, editor_id: Id) -> bool {
        HandleEnter::handle_tab(self, ctx, ui, editor_id)
    }

    /// Override for editor-specific cursor movement. Source editors keep
    /// TextEdit's normal text-based arrow behavior; canvas-like editors can
    /// move through an explicit grid instead.
    fn handle_navigation_binding(&mut self, _ctx: &Context, _ui: &mut Ui, _editor_id: Id) -> bool {
        false
    }

    /// Override to reserve a one-line footer under the text area.
    fn has_footer(&self) -> bool {
        false
    }

    /// Draws the footer reserved by [`GenericEditor::has_footer`].
    fn show_footer(&mut self, _ctx: &Context, _ui: &mut Ui, _editor_id: Id) {}

    /// Override for editor-specific command bindings.
    fn handle_command_bindings(&mut self, ctx: &Context, ui: &mut Ui, tx: &Sender<Msg>) {
        let editor_id = self.get_id();
        if ui.memory(|mem| mem.has_focus(editor_id)) {
            ui.input_mut(|i| {
                if i.key_pressed(egui::Key::R) && i.modifiers.command {
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::R);
                    let _ = tx.try_send(Msg::RequestRename(ctx.clone(), editor_id));
                }
            });
        }
    }
}

/// Screen rectangle of the block cursor: one monospace cell at the primary
/// cursor. None while unfocused or while a selection is active.
fn block_cursor_rect(
    output: &TextEditOutput,
    focused: bool,
    cell_width: f32,
) -> Option<egui::Rect> {
    let cursor_range = output
        .cursor_range
        .filter(|range| focused && range.is_empty())?;
    let cursor_rect = output
        .galley
        .pos_from_cursor(cursor_range.primary)
        .translate(output.galley_pos.to_vec2());
    Some(egui::Rect::from_min_size(
        cursor_rect.min,
        egui::vec2(cell_width, cursor_rect.height()),
    ))
}

fn should_notify_editor_change(
    editor_changed: bool,
    enter_changed: bool,
    tab_changed: bool,
    navigation_changed: bool,
) -> bool {
    editor_changed || enter_changed || tab_changed || navigation_changed
}

impl<T> InnerWindow for T
where
    T: GenericEditor + HasError,
{
    fn inner_window(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        tx: Sender<Msg>,
        _background: DiagramBackground,
    ) {
        self.initialize(tx.clone());
        let response = ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            let editor_background = ui.visuals().window_fill();
            ui.visuals_mut().text_edit_bg_color = Some(editor_background);
            let editor_id = ui.make_persistent_id(self.get_id());

            let indent_requested = HandleEnter::handle_enter(self, ctx, ui, editor_id);

            if indent_requested {
                GenericEditor::handle_enter(self, ctx, ui, editor_id);
            }

            let tab_changed = GenericEditor::handle_tab_binding(self, ctx, ui, editor_id);
            let navigation_changed =
                GenericEditor::handle_navigation_binding(self, ctx, ui, editor_id);
            GenericEditor::handle_command_bindings(self, ctx, ui, &tx);

            let footer_height = if self.has_footer() {
                ui.text_style_height(&egui::TextStyle::Monospace) + ui.spacing().item_spacing.y
            } else {
                0.0
            };
            let editor = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .max_height((ui.available_height() - footer_height).max(0.0))
                .show(ui, |ui| {
                    ui.add_sized(ui.available_size(), |ui: &mut egui::Ui| {
                        // TextEdit only has a bar cursor. Hide it and paint a
                        // full monospace cell after the widget instead.
                        let bar_cursor = ui.visuals().text_cursor.clone();
                        ui.visuals_mut().text_cursor.stroke.color = egui::Color32::TRANSPARENT;
                        ui.visuals_mut().text_cursor.blink = false;
                        let output = self.editor_spec(editor_id, ui);
                        ui.visuals_mut().text_cursor = bar_cursor;

                        let focused = ui.memory(|mem| mem.has_focus(editor_id));
                        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
                        let cell_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font_id, ' '));
                        if let Some(cell) = block_cursor_rect(&output, focused, cell_width) {
                            ui.painter_at(output.text_clip_rect).rect_filled(
                                cell,
                                0.0,
                                ui.visuals().selection.bg_fill,
                            );
                        }
                        output.response
                    })
                })
                .inner;
            if self.has_footer() {
                self.show_footer(ctx, ui, editor_id);
            }

            if should_notify_editor_change(
                editor.changed(),
                indent_requested,
                tab_changed,
                navigation_changed,
            ) {
                self.editor_on_changed(tx.clone(), ctx);
            }
        });
        if let (resp, Some(err)) = (response, self.get_error()) {
            let window_rect = resp.response.rect;
            let screen_rect = ctx.content_rect();
            let storage_id = ui.id().with("err_h");

            // Height the error wanted in the previous frame
            let last_h = ctx.memory(|mem| mem.data.get_temp::<f32>(storage_id).unwrap_or(0.0));
            let placement = error_placement(window_rect, screen_rect, last_h);
            let show_on_top = placement.on_top;

            egui::Area::new(ui.id().with("floating_error"))
                .fixed_pos(placement.pos)
                .order(egui::Order::Tooltip)
                .constrain(false)
                .show(ctx, |ui| {
                    ui.set_width(window_rect.width());

                    // Needed because RichText trims whitespaces
                    let err = err
                        .lines()
                        .map(|line| format!("\u{200B}{line}")) // zero-width space
                        .collect::<Vec<_>>()
                        .join("\n");

                    let mut frame = egui::Frame::popup(ui.style());
                    if show_on_top {
                        frame.shadow = egui::epaint::Shadow::NONE;
                    }
                    let frame_margin = frame.total_margin().sum().y;

                    let content_h = ui
                        .with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            frame
                                .corner_radius(egui::CornerRadius {
                                    nw: if show_on_top { 4 } else { 0 },
                                    ne: if show_on_top { 4 } else { 0 },
                                    sw: if show_on_top { 0 } else { 4 },
                                    se: if show_on_top { 0 } else { 4 },
                                })
                                .show(ui, |ui| {
                                    egui::ScrollArea::vertical()
                                        .max_height((placement.max_height - frame_margin).max(0.0))
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                            ui.label(
                                                egui::RichText::new(err)
                                                    .monospace()
                                                    .color(egui::Color32::from_rgb(255, 165, 0)),
                                            )
                                        })
                                        .content_size
                                        .y
                                })
                                .inner
                        })
                        .inner;

                    // Store the unclipped height for the next frame
                    let wanted_h = content_h + frame_margin;
                    if (wanted_h - last_h).abs() > 0.1 {
                        ui.memory_mut(|mem| mem.data.insert_temp(storage_id, wanted_h));
                        ui.ctx().request_repaint();
                    }
                });
        }
    }
}

/// Vertical space taken by the window title bar above the editor content.
const ERROR_TITLE_BAR_HEIGHT: f32 = 20.0;

#[derive(Debug, PartialEq)]
struct ErrorPlacement {
    pos: egui::Pos2,
    on_top: bool,
    max_height: f32,
}

/// Places the error popup next to the editor without covering it.
///
/// The popup goes below the window when it fits, above when only the top fits,
/// and otherwise on the roomier side, clipped to the space on that side.
fn error_placement(
    window_rect: egui::Rect,
    screen_rect: egui::Rect,
    wanted_height: f32,
) -> ErrorPlacement {
    let window_top = window_rect.top() - ERROR_TITLE_BAR_HEIGHT;
    let below = (screen_rect.bottom() - window_rect.bottom()).max(0.0);
    let above = (window_top - screen_rect.top()).max(0.0);

    let on_top = wanted_height > below && (wanted_height <= above || above > below);
    if on_top {
        ErrorPlacement {
            pos: egui::pos2(window_rect.left(), window_top - wanted_height.min(above)),
            on_top,
            max_height: above,
        }
    } else {
        ErrorPlacement {
            pos: window_rect.left_bottom(),
            on_top,
            max_height: below,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ERROR_TITLE_BAR_HEIGHT, block_cursor_rect, error_placement, should_notify_editor_change,
    };
    use crate::egui::{
        self, Rect, pos2,
        text::{CCursor, CCursorRange},
    };

    #[test]
    fn enter_only_change_notifies_editor_update() {
        assert!(should_notify_editor_change(false, true, false, false));
    }

    /// Runs `check` against TextEdit output for "abc\ndef" with `range` set.
    fn with_cursor(range: CCursorRange, check: impl Fn(&egui::text_edit::TextEditOutput)) {
        // Fonts load during the first frame; lay out on the second.
        let ctx = egui::Context::default();
        for frame in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let mut text = "abc\ndef".to_owned();
                    let mut output = egui::TextEdit::multiline(&mut text).code_editor().show(ui);
                    output.cursor_range = Some(range);
                    if frame == 1 {
                        check(&output);
                    }
                });
            });
        }
    }

    #[test]
    fn block_cursor_covers_one_cell_at_primary_cursor() {
        with_cursor(CCursorRange::one(CCursor::new(5)), |output| {
            let first_row = output.galley.rows[0]
                .rect()
                .translate(output.galley_pos.to_vec2());
            let cell = block_cursor_rect(output, true, 7.0).expect("cursor shown");
            assert_eq!(cell.width(), 7.0);
            assert!(cell.height() > 0.0);
            assert!(
                cell.top() >= first_row.bottom(),
                "cell is on the second row"
            );
            assert!(
                cell.left() > output.galley_pos.x,
                "cell is past the first column"
            );
        });
    }

    #[test]
    fn block_cursor_hidden_while_selecting() {
        with_cursor(
            CCursorRange::two(CCursor::new(1), CCursor::new(3)),
            |output| {
                assert_eq!(block_cursor_rect(output, true, 7.0), None);
            },
        );
    }

    #[test]
    fn block_cursor_hidden_without_focus() {
        with_cursor(CCursorRange::one(CCursor::new(1)), |output| {
            assert_eq!(block_cursor_rect(output, false, 7.0), None);
        });
    }

    fn screen() -> Rect {
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 1000.0))
    }

    /// Asserts the popup at `wanted` height stays clear of the window and title bar.
    fn assert_clear_of_window(window: Rect, wanted: f32) {
        let placement = error_placement(window, screen(), wanted);
        let shown = wanted.min(placement.max_height);
        let top = placement.pos.y;
        let bottom = top + shown;
        let window_top = window.top() - ERROR_TITLE_BAR_HEIGHT;
        assert!(
            bottom <= window_top || top >= window.bottom(),
            "popup {top}..{bottom} overlaps window {window_top}..{}",
            window.bottom()
        );
        assert!(top >= screen().top() && bottom <= screen().bottom());
    }

    #[test]
    fn error_goes_below_when_it_fits() {
        let window = Rect::from_min_max(pos2(10.0, 100.0), pos2(500.0, 400.0));
        let placement = error_placement(window, screen(), 200.0);
        assert!(!placement.on_top);
        assert_eq!(placement.pos, window.left_bottom());
        assert_clear_of_window(window, 200.0);
    }

    #[test]
    fn error_goes_above_when_only_top_fits() {
        let window = Rect::from_min_max(pos2(10.0, 600.0), pos2(500.0, 900.0));
        let placement = error_placement(window, screen(), 300.0);
        assert!(placement.on_top);
        assert_clear_of_window(window, 300.0);
    }

    #[test]
    fn oversized_error_never_covers_the_window() {
        for top in [30.0, 200.0, 450.0, 700.0] {
            let window = Rect::from_min_max(pos2(10.0, top), pos2(500.0, top + 250.0));
            assert_clear_of_window(window, 2000.0);
        }
    }

    #[test]
    fn oversized_error_uses_the_roomier_side() {
        let window = Rect::from_min_max(pos2(10.0, 700.0), pos2(500.0, 900.0));
        let placement = error_placement(window, screen(), 2000.0);
        assert!(placement.on_top);
        assert_eq!(placement.max_height, 700.0 - ERROR_TITLE_BAR_HEIGHT);
    }
}
