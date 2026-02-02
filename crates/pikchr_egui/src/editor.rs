use eframe::egui;

use crate::mini_window;

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

pub trait HandleEnter: mini_window::RawContent{
    fn handle_enter<F>(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        editor_id: egui::Id,
        get_indent: F,
    ) where
        F: Fn(&str) -> String,
    {
        let is_focused = ui.memory(|mem| mem.has_focus(editor_id));
        if is_focused {
            let indent_requested = ui.input_mut(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
                    true
                } else {
                    false
                }
            });
            if indent_requested
                && let Some(mut state) = egui::TextEdit::load_state(ctx, editor_id)
                && let Some(range) = state.cursor.char_range()
            {
                let mut content = self.get_raw_content();
                let mut cursor = range.primary.index;

                let cursor_line = get_last_line(&content[..range.primary.index]);

                content.insert(range.primary.index, '\n');
                cursor += 1;

                let indent = get_indent(&cursor_line);

                if !indent.is_empty() {
                    content.insert_str(cursor, indent.as_str());
                    cursor += indent.chars().count();
                }
                let ch_range = egui::text::CCursorRange::one(egui::text::CCursor::new(cursor));
                state.cursor.set_char_range(Some(ch_range));
                state.store(ui.ctx(), editor_id);
                self.set_raw_content(content);
            }
        }
    }
}
impl <T>HandleEnter for T where T: Editor + mini_window::RawContent{}
pub trait Editor {}

