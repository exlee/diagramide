use eframe::egui::{self, Context};
use parking_lot::RwLock;
use slog::{Logger, info, o};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

use state::AppState;
use state_serialize::DiagramIDEPersistent;

use crate::mini_window::AsComponent as _;

mod editor;
mod identifiers;
mod image;
mod menubar;
pub mod message_handler;
mod mini_window;
mod modal;
pub mod logger;
mod pikchr_editor;
mod prolog_editor;
mod response_ext;
mod sender_ext;
pub mod state;
mod state_serialize;
mod svg;
mod tcl;
mod tcl_editor;
pub mod text_highlighting;
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(from = "DiagramIDEPersistent", into = "DiagramIDEPersistent")]
pub struct DiagramIDE {
    tx: mpsc::Sender<Msg>,
    state: Arc<RwLock<AppState>>,
    pub window_size: egui::Vec2,
    first_frame: bool,
    pub logger: Logger,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum ExportType {
    Svg,
    Png,
    PngTransparent,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum Msg {
    // Utilities
    Batch(Vec<Msg>),
    Debounce(Duration, egui::Id, Box<Msg>),
    PopModal,

    // Exporting
    ExportModal(egui::Id, String, ExportType),
    Export(egui::Id, String, ExportType),
    ExportPikchrToClipboard(#[serde(skip)] Context, egui::Id),

    // Editor Menu
    FontSizeModal(egui::Id),

    // Rename
    RequestRename(egui::Id),
    RenameWindow(egui::Id, String),

    // Drawing
    RequestRedraw(#[serde(skip)] Context, egui::Id),
    UpdatePikchr(#[serde(skip)] Context, egui::Id),
    UpdateProlog(#[serde(skip)] Context, egui::Id, String),
    UpdateTcl(#[serde(skip)] Context, egui::Id, String),
    ResetError(egui::Id),
    UpdateContent(egui::Id, String),
    UpdatePikchrContent(egui::Id, String),
    DeleteWindow(egui::Id),

    // Windows
    ToggleWindow(Window),
    ToggleWindowById(egui::Id),
    NewWindow(crate::mini_window::WindowType),

    // Svg Handling
    RecreateSvg(#[serde(skip)] Context, egui::Id),
    ReloadSvgs(#[serde(skip)] Context),

    // Refreshes
    Refresh(egui::Id),

    // Workspace
    /// Shows Confirmation Modal for ResetWorkspace
    ResetWorkspaceRequest,
    /// Actual Reset workspace
    ResetWorkspace,
    /// Shows FileDialog for saving Workspace
    SaveWorkspace,
    /// Shows FileFialog for opening Workspace
    LoadWorkspaceRequest,
    /// Loads workspace
    LoadWorkspace(String),
}

#[derive(PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum EditorType {
    Prolog,
    Pikchr,
    Tcl,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum Window {
    Logger,
    Debugger,
}

impl DiagramIDE {
    pub fn new_test(
        ctx: &egui::Context,
        tx: mpsc::Sender<Msg>,
        state: Arc<RwLock<AppState>>,
    ) -> Self {
        egui_extras::install_image_loaders(ctx);
        Self {
            tx,
            state,
            first_frame: true,
            window_size: egui::vec2(800.0, 600.0),
            logger: crate::logger::init_logger(),
        }
    }
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let logger = crate::logger::init_logger();
        let start_def = || {
            let blank_state = Arc::new(RwLock::new(AppState::default()));
            let tx = Self::spawn_message_handler(logger.clone(), blank_state.clone());

            Self {
                tx: tx.clone(),
                state: blank_state,
                first_frame: true,
                window_size: egui::vec2(800.0, 600.0),
                logger: logger.clone(),
            }
        };
        let pers_logger = logger.new(o!("category" => "persistence"));
        if let Some(storage) = cc.storage {
            if let Some(persistent) =
                eframe::get_value::<DiagramIDEPersistent>(storage, eframe::APP_KEY)
            {
                info!(pers_logger, "Load happening");
                let mut prev_state = DiagramIDE::from(persistent);
                let tx = Self::spawn_message_handler(prev_state.logger.clone(), prev_state.state.clone());
                prev_state.tx = tx.clone();
                let _ = tx.try_send(Msg::ReloadSvgs(cc.egui_ctx.clone()));
                prev_state
            } else {
                info!(pers_logger, "Prev state not found");
                start_def()
            }
        } else {
            info!(pers_logger, "Storage not found");
            start_def()
        }
    }
    pub fn spawn_message_handler(logger: Logger, state: Arc<RwLock<AppState>>) -> mpsc::Sender<Msg> {
        let (tx, rx) = mpsc::channel::<Msg>(100);
        let _ = tokio::spawn(message_handler::handle(rx, logger, state.clone()));
        tx
    }
    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.first_frame {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.window_size));
            self.first_frame = false;
        }
        //ctx.options_mut(|opt| opt.zoom_factor = 0.75);
        let state = self.state.clone();
        let tx_clone = self.tx.clone();
        egui::TopBottomPanel::top("top_panel").show(&ctx, menubar::widget(state, tx_clone));

        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.heading("Workspace");
        });

        {
            let state = self.state.clone();
            let tx_clone = self.tx.clone();
            if let Some(modal) = state.read().modals.front() {
                modal.write().show(&ctx, tx_clone);
            }
        }

        for window in self.state.write().windows.values_mut() {
            if let Some(mini) = window.as_mini_window_mut() {
                mini.show(ctx, self.tx.clone(), self.state.clone());
            }
        }

        if self.state.clone().read().window_states.log {
            egui::Window::new("Log")
                .resizable(true)
                .default_size((200.0, 200.0))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false]) // Key: Stop it from shrinking to fit content!
                        .stick_to_bottom(true) // Optional: Auto-scroll to new entries
                        .show(ui, |ui| {
                            for entry in &self.state.clone().read().log {
                                ui.label(entry);
                            }
                        });
                });
        }

        if self.state.read().window_states.debug {
            egui::Window::new("FPS").show(&ctx, |ui| {
                ctx.inspection_ui(ui);
            });
        }
        egui::Area::new(egui::Id::new("bottom_right_status"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0))
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Non-mandated use only. Contact for commercial license.").weak());
            });
    }
}

impl eframe::App for DiagramIDE {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.window_size = ctx.content_rect().size();
        self.ui(ctx);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eprintln!("Saving!");
        let persistent = DiagramIDEPersistent::from(self.clone());
        eframe::set_value(storage, eframe::APP_KEY, &persistent);
        storage.flush();
    }
}

fn replace_raw_content(state: &mut AppState, id: egui::Id, content: &str) -> String {
    let editors_ew = state.windows
        .values()
        .filter(|e| e.as_id().unwrap().get_id() != id)
        .flat_map(|e| e.as_editor_window());

    let editors_rc = state.windows
        .values()
        .filter(|e| e.as_id().unwrap().get_id() != id)
        .flat_map(|e| e.get_as());

    let editors: Vec<(egui::Id, String, String)> = editors_ew
        .zip(editors_rc)
        .map(
            |(e, rc): (mini_window::EditorWindowView, &dyn mini_window::RawContent)| {
                (*e.id, format!("!!{}!!", e.name), rc.get_raw_content())
            },
        )
        .collect();
    let mut content = String::from(content);
    for (repl_id, repl, _value) in &editors {
        let entry = state.editor_deps.entry(*repl_id).or_default();
        if content.contains(repl) {
            entry.insert(id);
        } else {
            entry.remove(&id);
        };
    }
    for _ in 1..=3 {
        for (_repl_id, repl, value) in &editors {
            let wrapped_value = format!("{value};");
            content = content.replace(repl, &wrapped_value);
        }
    }
    content
}
fn replace_pikchr_content(state: &mut AppState, id: egui::Id) -> String {
    let content = state
        .windows
        .get(&id)
        .and_then(|w| w.as_editor_window())
        .map(|c| c.content.get_pikchr_content())
        .unwrap_or_default();
    let editors: Vec<(egui::Id, String, String)> = state
        .windows
        .values()
        .flat_map(|e| e.as_editor_window())
        .filter(|e| e.id != &id)
        .map(|e| {
            (
                *e.id,
                format!("$${}$$", e.name),
                e.content.get_pikchr_content(),
            )
        })
        .collect();
    let mut content = content;

    for (repl_id, repl, _value) in &editors {
        let entry = state.editor_deps.entry(*repl_id).or_default();
        if content.contains(repl) {
            entry.insert(id);
        } else {
            entry.remove(&id);
        };
    }
    for _ in 1..=3 {
        for (_repl_id, repl, value) in &editors {
            let wrapped_value = format!("{value};");
            content = content.replace(repl, &wrapped_value);
        }
    }
    content
}

pub const SPACE_MONO_BYTES: &[u8] = include_bytes!("../../pikchr_pl//fonts/SpaceMono-Regular.ttf");
pub const SPACE_MONO_NAME: &str = "Space Mono"; // Must match the internal TTF Name
