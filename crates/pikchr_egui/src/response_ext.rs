use eframe::egui;

pub trait ResponseExt {
    fn on_key_escape(self, action: impl FnOnce()) -> Self;
    fn on_key_enter(self, action: impl FnOnce()) -> Self;
}

impl ResponseExt for egui::Response {
    fn on_key_escape(self, action: impl FnOnce()) -> Self {
        if self.has_focus() {
            let mut triggered = false;
            
            self.ctx.input_mut(|i| {
                if i.key_pressed(egui::Key::Escape) {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                    triggered = true;
                }
            });

            if triggered {
                action();
            }
        }
        self
    }
    fn on_key_enter(self, action: impl FnOnce()) -> Self {
        if self.has_focus() {
            let mut triggered = false;
            
            self.ctx.input_mut(|i| {
                if i.key_pressed(egui::Key::Enter) {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
                    triggered = true;
                }
            });

            if triggered {
                action();
            }
        }
        self
    }
}
