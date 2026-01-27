use std::sync::Arc;

use eframe::egui::{self, Checkbox, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{AppState, EditorType, Msg, Window};

macro_rules! checkbox_buttons {
    (
        $state:ident, $ui:ident, $tx:expr,
        $(
            ($state_var:ident, $name:literal, $msg:ident)
        ),+ $(,)?
    ) => {
        {
            $(
                let mut check = $state.read().windows.$state_var;
                let element = $ui.add(Checkbox::new(&mut check, $name));
                if element.clicked() {
                    let _ = $tx.try_send(Msg::ToggleWindow(Window::$msg));
                }
            )+
        }
    }

}

pub fn widget(state: Arc<RwLock<AppState>>, tx: Sender<Msg>) -> impl Fn(&mut Ui) {
    //let message_on_click = move |element: egui::Response, message| {
    //    if element.clicked() {
    //        let _ = tx.try_send(message);
    //    }
    //};
    //let new_checkbox =
    //    |ui: &mut Ui, name: &str, variable: &bool| {
    //        let mut check = *variable;
    //        ui.add(Checkbox::new(&mut check, name))
    //    };

    move |ui: &mut Ui| -> () {
        egui::MenuBar::new()
            .ui(ui, |ui| {
            ui.menu_button("New", |ui| {
                if ui.button("Pikchr Editor").clicked() {
                    tx.try_send(Msg::NewEditor(EditorType::Pikchr));
                };
                if ui.button("Prolog Editor").clicked() {
                    tx.try_send(Msg::NewEditor(EditorType::Prolog));
                };
            });
            ui.menu_button("Windows", |ui| {
                for window in state.read().mini_windows.values() {
                    let window = window.read();
                    if window.should_be_listed() {
                        let mut check = window.visible();
                        let title = window.get_title();
                        let element = ui.add(Checkbox::new(&mut check, title));
                        if element.clicked() {
                            let _ = tx.try_send(Msg::ToggleWindowById(window.get_id()));
                        }
                    }
                }
                checkbox_buttons!(state, ui, tx.clone(),
                    (debug, "Debug", Debugger),
                    (log, "Logger", Logger),
                )
                //let debug_win = new_checkbox(ui, "Debug", &state.read().windows.debug);
                //message_on_click(debug_win, Msg::ToggleDebugWindow);

                //let editor_win = new_checkbox(ui, "Pickhr editor", &state.read().windows.pikchr_editor);
                //message_on_click(editor_win, Msg::ToggleEditorWindow);

                //let editor_win = new_checkbox(ui, "Prolog editor", &state.read().windows.pikchr_editor);
                //message_on_click(editor_win, Msg::ToggleEditorWindow);

                //let log_win = new_checkbox(ui, "Logger window", &state.read().windows.log);
                //message_on_click(log_win, Msg::ToggleLogWindow);
            });
        });
    }
}

