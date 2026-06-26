use std::sync::Arc;

use eframe::egui::{self, Checkbox, Frame, Margin, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{
    AppState, Msg, Window, help::HelpTopic, mini_window::WindowType, mruby,
    state::DiagramBackground, tcl, theme,
};

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
            });
            ui.menu_button("Workspace", |ui| {
                ui.set_min_width(0.0);

                let visuals = ui.visuals().clone();
                let listing = state.read().workspace_listing();
                let can_delete = listing.len() > 1;

                // Fixed, narrow row width so the menu stays compact and the
                // clickable "dead space" between name and action buttons is
                // bounded (avoids the menu expanding to screen width).
                const ROW_WIDTH: f32 = 160.0;

                for (id, name, is_active) in listing {
                    let bg_fill = if is_active {
                        visuals.selection.bg_fill.gamma_multiply(0.25)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    Frame::new()
                        .fill(bg_fill)
                        .corner_radius(4.0)
                        .inner_margin(Margin::symmetric(4i8, 2i8))
                        .show(ui, |ui| {
                            ui.set_width(ROW_WIDTH);
                            ui.horizontal(|ui| {
                                // Active indicator dot
                                let dot = if is_active {
                                    egui::RichText::new("\u{25CF}")
                                        .size(9.0)
                                        .color(visuals.selection.stroke.color)
                                } else {
                                    egui::RichText::new("\u{25CB}")
                                        .size(9.0)
                                        .color(visuals.weak_text_color())
                                };
                                ui.label(dot);
                                ui.add_space(3.0);

                                // Clickable name — standard selectable_label (no text cursor)
                                let text = if is_active {
                                    egui::RichText::new(&name).size(12.0).strong()
                                } else {
                                    egui::RichText::new(&name).size(12.0)
                                };
                                let mut switch = ui.selectable_label(is_active, text).clicked();

                                // Dead space between name and buttons — also switches.
                                // Bounded because the row width is fixed above.
                                let reserved = if can_delete { 78.0 } else { 52.0 };
                                let filler_w = (ui.available_width() - reserved).max(0.0);
                                let filler = ui.allocate_at_least(
                                    egui::vec2(filler_w, 0.0),
                                    egui::Sense::click(),
                                ).1;
                                if filler.clicked() {
                                    switch = true;
                                }

                                // Compact icon buttons on the right
                                ui.spacing_mut().item_spacing = egui::vec2(1.0, 0.0);
                                if ui
                                    .small_button(egui::RichText::new("\u{270E}").size(12.0))
                                    .on_hover_text("Rename")
                                    .clicked()
                                {
                                    let _ = tx.try_send(Msg::RenameWorkspaceRequest(id));
                                    ui.close();
                                }
                                if ui
                                    .small_button(egui::RichText::new("\u{29C9}").size(12.0))
                                    .on_hover_text("Duplicate")
                                    .clicked()
                                {
                                    let _ = tx.try_send(Msg::DuplicateWorkspace(id));
                                    ui.close();
                                }
                                if can_delete
                                    && ui
                                        .small_button(
                                            egui::RichText::new("\u{2715}")
                                                .size(12.0)
                                                .color(egui::Color32::from_rgb(220, 90, 90)),
                                        )
                                        .on_hover_text("Delete")
                                        .clicked()
                                {
                                    let _ = tx.try_send(Msg::DeleteWorkspaceRequest(id));
                                    ui.close();
                                }

                                if switch {
                                    let _ = tx.try_send(Msg::SwitchWorkspace(id));
                                    ui.close();
                                }
                            });
                        });

                    ui.add_space(1.0);
                }

                ui.separator();
                ui.add_space(2.0);

                if ui.button("New Workspace").clicked() {
                    let _ = tx.try_send(Msg::NewWorkspaceRequest);
                    ui.close();
                }

                if ui
                    .button("Reset Active")
                    .on_hover_text("Delete all editors and windows in the active workspace")
                    .clicked()
                {
                    let _ = tx.try_send(Msg::ResetWorkspaceRequest);
                    ui.close();
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
                ui.menu_button("Diagram Background", |ui| {
                    let active = state.read().diagram_background;
                    for (background, label) in [
                        (DiagramBackground::Black, "Black"),
                        (DiagramBackground::ThemeDark, "Theme Dark"),
                        (DiagramBackground::ThemeBright, "Theme Bright"),
                        (DiagramBackground::White, "White"),
                    ] {
                        if ui.selectable_label(active == background, label).clicked() {
                            let _ = tx.try_send(Msg::SetDiagramBackground(
                                ui.ctx().clone(),
                                background,
                            ));
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Scale", |ui| {
                    let current = (ui.ctx().zoom_factor() * 100.0).round() as i32;
                    for zoom in [50, 75, 100, 125, 150, 200] {
                        if ui
                            .selectable_label(current == zoom, format!("{zoom}%"))
                            .clicked()
                        {
                            ui.ctx().set_zoom_factor(zoom as f32 / 100.0);
                            ui.close();
                        };
                    }
                });
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
