use std::{collections::BTreeSet, sync::Arc};

use eframe::egui::{self, Checkbox, Frame, Margin, Ui};
use parking_lot::RwLock;
use tokio::sync::mpsc::Sender;

use crate::{
    AppState, Msg, Window, help::HelpTopic, mini_window::WindowType, mruby,
    state::DiagramBackground, tcl, theme,
};

#[derive(Clone, Copy)]
enum WorkspaceIcon {
    ActiveDot(bool),
    Rename,
    Duplicate,
    Delete,
}

fn workspace_icon(
    ui: &mut Ui,
    icon: WorkspaceIcon,
    color: Option<egui::Color32>,
) -> egui::Response {
    let size = match icon {
        WorkspaceIcon::ActiveDot(_) => egui::vec2(10.0, 18.0),
        _ => egui::vec2(18.0, 18.0),
    };
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let painter = ui.painter_at(rect);
        let stroke = egui::Stroke::new(1.35, color.unwrap_or(visuals.fg_stroke.color));
        let center = rect.center();

        match icon {
            WorkspaceIcon::ActiveDot(active) => {
                let dot_color = if active {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().weak_text_color()
                };
                if active {
                    painter.circle_filled(center, 3.6, dot_color);
                } else {
                    painter.circle_stroke(center, 3.2, egui::Stroke::new(1.1, dot_color));
                }
            },
            WorkspaceIcon::Rename => {
                painter.rect_filled(rect, 3.0, visuals.bg_fill);
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    "Aa",
                    egui::FontId::proportional(10.5),
                    stroke.color,
                );
            },
            WorkspaceIcon::Duplicate => {
                painter.rect_filled(rect, 3.0, visuals.bg_fill);
                let back = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + 4.0, rect.top() + 4.0),
                    egui::vec2(8.0, 8.0),
                );
                let front = back.translate(egui::vec2(3.5, 3.5));
                painter.rect_stroke(back, 1.5, stroke, egui::StrokeKind::Inside);
                painter.rect_filled(front, 1.5, visuals.bg_fill);
                painter.rect_stroke(front, 1.5, stroke, egui::StrokeKind::Inside);
            },
            WorkspaceIcon::Delete => {
                painter.rect_filled(rect, 3.0, visuals.bg_fill);
                let inset = 5.0;
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + inset, rect.top() + inset),
                        egui::pos2(rect.right() - inset, rect.bottom() - inset),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        egui::pos2(rect.right() - inset, rect.top() + inset),
                        egui::pos2(rect.left() + inset, rect.bottom() - inset),
                    ],
                    stroke,
                );
            },
        }
    }

    response
}

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

fn render_library_branch(ui: &mut Ui, paths: &[String], prefix: &str, tx: &Sender<Msg>) {
    const ROW_WIDTH: f32 = 240.0;
    const LABEL_WIDTH: f32 = 184.0;
    const BUTTON_SIZE: egui::Vec2 = egui::vec2(20.0, 20.0);

    let mut folders = BTreeSet::new();
    let mut leaves = Vec::new();
    let prefix_with_slash = if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}/")
    };

    for path in paths {
        let Some(rest) = path.strip_prefix(&prefix_with_slash) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        if let Some((folder, _)) = rest.split_once('/') {
            folders.insert(folder.to_owned());
        } else {
            leaves.push(path.clone());
        }
    }

    for folder in folders {
        let next_prefix = if prefix.is_empty() {
            folder.clone()
        } else {
            format!("{prefix}/{folder}")
        };
        ui.menu_button(folder, |ui| {
            render_library_branch(ui, paths, &next_prefix, tx);
        });
    }

    for path in leaves {
        let label = path
            .rsplit('/')
            .find(|part| !part.is_empty())
            .unwrap_or(&path)
            .to_owned();
        ui.horizontal(|ui| {
            ui.set_width(ROW_WIDTH);
            if ui
                .add_sized([LABEL_WIDTH, BUTTON_SIZE.y], egui::Button::new(label))
                .clicked()
            {
                let _ = tx.try_send(Msg::OpenLibraryEntry(ui.ctx().clone(), path.clone()));
                ui.close();
            }
            if ui
                .add_sized(BUTTON_SIZE, egui::Button::new("E"))
                .on_hover_text("Export")
                .clicked()
            {
                let _ = tx.try_send(Msg::ExportLibraryEntry(path.clone()));
                ui.close();
            }
            if ui
                .add_sized(BUTTON_SIZE, egui::Button::new("X"))
                .on_hover_text("Delete")
                .clicked()
            {
                let _ = tx.try_send(Msg::DeleteLibraryEntryRequest(path.clone()));
                ui.close();
            }
        });
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
            ui.menu_button("New", |ui| {
                if ui.button("Pikchr Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(ui.ctx().clone(), WindowType::PikchrEditor));
                };
                if ui.button("Plain text").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(
                        ui.ctx().clone(),
                        WindowType::PlainTextEditor,
                    ));
                };
                if ui.button("Prolog Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(ui.ctx().clone(), WindowType::PrologEditor));
                };
                if tcl::is_tcl_loadable() && ui.button("Tcl Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(ui.ctx().clone(), WindowType::TclEditor));
                };
                if mruby::is_mruby_available() && ui.button("mruby Editor").clicked() {
                    let _ = tx.try_send(Msg::NewWindow(ui.ctx().clone(), WindowType::MrubyEditor));
                };
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
                            let _ = tx
                                .try_send(Msg::SetDiagramBackground(ui.ctx().clone(), background));
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
            ui.menu_button("Library", |ui| {
                if ui.button("Import...").clicked() {
                    let _ = tx.try_send(Msg::ImportLibraryEntries);
                    ui.close();
                }

                let paths: Vec<String> = state.read().library.keys().cloned().collect();
                if paths.is_empty() {
                    ui.separator();
                    ui.label("Empty");
                } else {
                    ui.separator();
                    render_library_branch(ui, &paths, "", &tx);
                }
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
            ui.menu_button("Workspaces", |ui| {
                ui.set_min_width(0.0);

                let visuals = ui.visuals().clone();
                let listing = state.read().workspace_listing();
                let can_delete = listing.len() > 1;

                // Fixed, narrow row width so the menu stays compact and the
                // clickable "dead space" between name and action buttons is
                // bounded (avoids the menu expanding to screen width).
                const ROW_WIDTH: f32 = 240.0;

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
                                workspace_icon(ui, WorkspaceIcon::ActiveDot(is_active), None);
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
                                let filler = ui
                                    .allocate_at_least(
                                        egui::vec2(filler_w, 0.0),
                                        egui::Sense::click(),
                                    )
                                    .1;
                                if filler.clicked() {
                                    switch = true;
                                }

                                // Compact icon buttons on the right
                                ui.spacing_mut().item_spacing = egui::vec2(1.0, 0.0);
                                if workspace_icon(ui, WorkspaceIcon::Rename, None)
                                    .on_hover_text("Rename")
                                    .clicked()
                                {
                                    let _ = tx.try_send(Msg::RenameWorkspaceRequest(id));
                                    ui.close();
                                }
                                if workspace_icon(ui, WorkspaceIcon::Duplicate, None)
                                    .on_hover_text("Duplicate")
                                    .clicked()
                                {
                                    let _ = tx.try_send(Msg::DuplicateWorkspace(id));
                                    ui.close();
                                }
                                if can_delete
                                    && workspace_icon(
                                        ui,
                                        WorkspaceIcon::Delete,
                                        Some(egui::Color32::from_rgb(220, 90, 90)),
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
