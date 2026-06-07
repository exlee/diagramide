use std::sync::Arc;

use eframe::egui::{self, Checkbox, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{AppState, Msg, Window, help::HelpTopic, mini_window::WindowType, mruby, tcl, theme};

#[cfg(target_os = "macos")]
pub fn titlebar(ctx: &egui::Context) {
    const TITLEBAR_HEIGHT: f32 = 31.0;
    egui::TopBottomPanel::top("macos_titlebar")
        .exact_height(TITLEBAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(ctx.style().visuals.panel_fill)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "DiagramIDE",
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().weak_text_color(),
            );
            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
            if response.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
        });
}

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
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save Workspace").clicked() {
                    let _ = tx.try_send(Msg::SaveWorkspace);
                }
                if ui.button("Load Workspace").clicked() {
                    let _ = tx.try_send(Msg::LoadWorkspaceRequest);
                }
                ui.separator();
                if ui.button("Reset Workspace").clicked() {
                    let _ = tx.try_send(Msg::ResetWorkspaceRequest);
                }
            });
            ui.menu_button("New", |ui| {
                if ui.button("Pikchr Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::PikchrEditor));
                };
                if ui.button("Plain text").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::PlainTextEditor));
                };
                if ui.button("Prolog Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::PrologEditor));
                };
                if tcl::is_tcl_loadable() && ui.button("Tcl Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::TclEditor));
                };
                if mruby::is_mruby_available() && ui.button("mruby Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(WindowType::MrubyEditor));
                };
            });
            ui.menu_button("Windows", |ui| {
                for window in state.read().windows.values().flat_map(|e| e.as_window()) {
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
                checkbox_buttons!(
                    state,
                    ui,
                    tx.clone(),
                    (debug, "Debug", Debugger),
                    (log, "Logger", Logger),
                )
            });
            ui.menu_button("View", |ui| {
                for zoom in [50, 75, 100, 125, 150, 200] {
                    if ui.button(format!("Scale View - {}%", zoom)).clicked() {
                        ui.ctx().set_zoom_factor(zoom as f32 / 100.0);
                    };
                }
            });
            ui.menu_button("Themes", |ui| {
                let active = state.read().active_theme.clone();
                let themes = theme::list();
                for built_in in [true, false] {
                    let section: Vec<_> = themes
                        .iter()
                        .filter(|(_, _, is_built_in)| *is_built_in == built_in)
                        .collect();
                    if section.is_empty() {
                        continue;
                    }
                    if !built_in {
                        ui.separator();
                        ui.label("Installed themes");
                    }
                    for (id, name, _) in section {
                        if ui.selectable_label(active == *id, name).clicked() {
                            let _ = tx.try_send(Msg::SelectTheme(ui.ctx().clone(), id.clone()));
                            ui.close();
                        }
                    }
                }
                ui.separator();
                if ui.button("Reload Themes").clicked() {
                    let _ = tx.try_send(Msg::ReloadThemes(ui.ctx().clone()));
                    ui.close();
                }
                if ui.button("Open Themes Folder").clicked() {
                    let _ = tx.try_send(Msg::OpenThemesFolder);
                    ui.close();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("DiagramIDE Help").clicked() {
                    let _ = tx.try_send(Msg::ShowHelp(HelpTopic::Overview));
                    ui.close();
                }
            });
            if ui
                .button("?")
                .on_hover_text("Open DiagramIDE Help")
                .clicked()
            {
                let _ = tx.try_send(Msg::ShowHelp(HelpTopic::Overview));
            }
        });
    }
}
