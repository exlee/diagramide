use std::{sync::Arc};

use eframe::egui::{self, Context, Id, Ui, text_edit::TextEditOutput};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{
    Msg,
    mini_window::{self, HasError, Id as IdTrait, InnerWindow},
    state::AppState,
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
        _app_state: Arc<RwLock<AppState>>,
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

            let is_focused = ui.memory(|mem| mem.has_focus(editor_id));
            if is_focused {
                ui.input_mut(|i| {
                    if i.key_pressed(egui::Key::R) && i.modifiers.command {
                        i.consume_key(egui::Modifiers::COMMAND, egui::Key::R);
                        //println!("Will rename: {}", self.get_id().short_debug_format());
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
