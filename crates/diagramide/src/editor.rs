use std::{sync::Arc, time::Duration};

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
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if self.get_error().is_some() {
                let t = egui::RichText::new(self.get_error().unwrap()).monospace();
                ui.label(t);
            } else {
                ui.label("");
            }
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
                        let _ = tx.try_send(Msg::RequestRename(self.get_id()));
                        true
                    } else {
                        false
                    }
                });
            }

            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                let editor = egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        //ui.add(
                        self.editor_spec(editor_id, ui)
                        //)
                    })
                    .inner;

                if editor.response.changed() {
                    self.editor_on_changed(tx.clone(), ctx);
                }
            });
        });
    }
}
