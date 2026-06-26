use eframe::egui::{self, Context, Id, Ui, text_edit::TextEditOutput};
use tokio::sync::mpsc::Sender;

use crate::{
    mini_window::{self, HasError, Id as IdTrait, InnerWindow},
    state::DiagramBackground,
    Msg,
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
/// `removals` is a slice of `(position, count)` pairs in ascending position order,
/// representing contiguous spans that were deleted.
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
    /// - **Tab** indents by 2 spaces (all selected lines, or inserts at cursor)
    /// - **Shift-Tab** dedents by 2 spaces (all selected lines, or current line)
    fn handle_tab(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, editor_id: egui::Id) {
        let is_focused = ui.memory(|mem| mem.has_focus(editor_id));
        if !is_focused {
            return;
        }

        let action = ui.input(|i| {
            if i.key_pressed(egui::Key::Tab) {
                Some(i.modifiers.shift)
            } else {
                None
            }
        });

        let Some(shift) = action else {
            return;
        };

        // Consume the Tab event so the TextEdit widget does not insert a '\t'.
        // Modifiers::NONE matches logically, so it covers both Tab and Shift-Tab
        // (but not Ctrl/Cmd+Tab, which we leave alone).
        ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
        });

        if shift {
            self.do_dedent(ctx, editor_id);
        } else {
            self.do_indent(ctx, editor_id);
        }
    }

    /// Indent: insert 2 spaces at the cursor, or at the start of every selected line.
    fn do_indent(&mut self, ctx: &egui::Context, editor_id: egui::Id) {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) else {
            return;
        };
        let Some(range) = state.cursor.char_range() else {
            return;
        };

        let mut content = self.get_raw_content();
        let primary = range.primary.index;
        let secondary = range.secondary.index;

        if primary == secondary {
            // No selection: insert 2 spaces at the cursor.
            let pos = primary.min(content.len());
            content.insert_str(pos, "  ");
            let new_cursor = pos + 2;
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(new_cursor),
            )));
        } else {
            let start = primary.min(secondary);
            let end = primary.max(secondary);

            let line_starts = collect_line_starts(&content, start, end);

            // Insert "  " right-to-left so earlier positions stay valid.
            for &pos in line_starts.iter().rev() {
                content.insert_str(pos, "  ");
            }

            let n = line_starts.len();
            let new_start = start + 2;
            let new_end = end + 2 * n;
            let (new_primary, new_secondary) = if primary < secondary {
                (new_start, new_end)
            } else {
                (new_end, new_start)
            };
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(new_primary),
                egui::text::CCursor::new(new_secondary),
            )));
        }

        state.store(ctx, editor_id);
        self.set_raw_content(content);
    }

    /// Dedent: remove up to 2 leading spaces from the current line or every selected line.
    fn do_dedent(&mut self, ctx: &egui::Context, editor_id: egui::Id) {
        let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id) else {
            return;
        };
        let Some(range) = state.cursor.char_range() else {
            return;
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
            return; // nothing to dedent
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
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(new_primary),
            )));
        } else {
            state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(new_primary),
                egui::text::CCursor::new(new_secondary),
            )));
        }

        state.store(ctx, editor_id);
        self.set_raw_content(content);
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

pub trait GenericEditor {
    fn editor_spec(&mut self, editor_id: Id, ui: &mut Ui) -> TextEditOutput;
    fn handle_enter(&mut self, ctx: &Context, ui: &mut Ui, editor_id: Id);
    fn editor_on_changed(&self, tx: Sender<Msg>, ctx: &Context);
    fn initialize(&mut self, tx: Sender<Msg>);
}

impl<T> InnerWindow for T
where
    T: GenericEditor + HasError + HandleEnter + IdTrait,
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

            HandleEnter::handle_tab(self, ctx, ui, editor_id);

            let is_focused = ui.memory(|mem| mem.has_focus(editor_id));
            if is_focused {
                ui.input_mut(|i| {
                    if i.key_pressed(egui::Key::R) && i.modifiers.command {
                        i.consume_key(egui::Modifiers::COMMAND, egui::Key::R);
                        let _ = tx.try_send(Msg::RequestRename(ctx.clone(), self.get_id()));
                        true
                    } else {
                        false
                    }
                });
            }

            let editor = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_sized(ui.available_size(), |ui: &mut egui::Ui| {
                        self.editor_spec(editor_id, ui).response
                    })
                    //)
                })
                .inner;

            if editor.changed() {
                self.editor_on_changed(tx.clone(), ctx);
            }
        });
        if let (resp, Some(err)) = (response, self.get_error()) {
            let window_rect = resp.response.rect;
            let screen_rect = ctx.content_rect();
            let storage_id = ui.id().with("err_h");

            // 1. Retrieve the height measured in the previous frame
            let last_h = ctx.memory(|mem| mem.data.get_temp::<f32>(storage_id).unwrap_or(0.0));

            // 2. Predict the bottom-position collision
            let bottom_attachment_pos = window_rect.left_bottom();
            let predicted_bottom_edge = bottom_attachment_pos.y + last_h;

            // Flip to top if the error would bleed off the screen
            let show_on_top = predicted_bottom_edge > screen_rect.bottom();

            let error_pos = if show_on_top {
                // Position at the top, shifted up by the error's own height
                window_rect.left_top() - egui::vec2(0.0, last_h + 20.0)
            } else {
                bottom_attachment_pos
            };

            egui::Area::new(ui.id().with("floating_error"))
                .fixed_pos(error_pos)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    ui.set_width(window_rect.width());

                    let frame_res = ui
                        .with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            let mut frame = egui::Frame::popup(ui.style());
                            if show_on_top {
                                frame.shadow = egui::epaint::Shadow::NONE;
                            }
                            frame
                                .corner_radius(egui::CornerRadius {
                                    nw: if show_on_top { 4 } else { 0 },
                                    ne: if show_on_top { 4 } else { 0 },
                                    sw: if show_on_top { 0 } else { 4 },
                                    se: if show_on_top { 0 } else { 4 },
                                })
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(err)
                                            .monospace()
                                            .color(egui::Color32::from_rgb(255, 165, 0)),
                                    )
                                })
                                .response
                        })
                        .inner;

                    // 3. Update the height for the next frame
                    let current_h = frame_res.rect.height();
                    if (current_h - last_h).abs() > 0.1 {
                        ui.memory_mut(|mem| mem.data.insert_temp(storage_id, current_h));
                        ui.ctx().request_repaint();
                    }
                });
        }
    }
}
