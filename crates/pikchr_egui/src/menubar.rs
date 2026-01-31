use std::sync::Arc;

use eframe::egui::{self, Checkbox, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{AppState, Msg, Window, mini_window::WindowType};

macro_rules! checkbox_buttons {
    (
        $state:ident, $ui:ident, $tx:expr,
        $(
            ($state_var:ident, $name:literal, $msg:ident)
        ),+ $(,)?
    ) => {
        {
            $(
                let mut check = $state.read().window_states.$state_var;
                let element = $ui.add(Checkbox::new(&mut check, $name));
                if element.clicked() {
                    let _ = $tx.try_send(Msg::ToggleWindow(Window::$msg));
                }
            )+
        }
    }

}

pub fn widget(state: Arc<RwLock<AppState>>, tx: Sender<Msg>) -> impl Fn(&mut Ui) {
    move |ui: &mut Ui| -> () {
        egui::MenuBar::new()
            .ui(ui, |ui| {
            ui.menu_button("New", |ui| {
                if ui.button("Pikchr Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::PikchrEditor));
                };
                if ui.button("Prolog Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::PrologEditor));
                };
                ui.separator();
                if ui.button("Reset Workspace").clicked() {
                    let _ = tx.try_send(Msg::ResetWorkspaceRequest);
                }

            });
            ui.menu_button("Windows", |ui| {
                for window in state.read().windows.read().values().flat_map(|e| e.as_window()) {
                    if window.mini_window.should_be_listed() {
                        let mut check = window.mini_window.visible();
                        let title = window.mini_window.get_title();

                        ui.horizontal(|ui| {
                            ui.set_min_width(200.0);
                            let element = ui.add(Checkbox::new(&mut check, title));
                            if element.clicked() {
                                let _ = tx.try_send(Msg::ToggleWindowById(*window.id));
                            }
                        });
                    }
                }
                checkbox_buttons!(state, ui, tx.clone(),
                    (debug, "Debug", Debugger),
                    (log, "Logger", Logger),
                )
            });
        });
    }
}

