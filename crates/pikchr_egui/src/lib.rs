use eframe::egui::{self, Context};
use parking_lot::RwLock;
use tracing::Instrument as _;
use std::sync::Arc;
use tokio::sync::mpsc;

use state::AppState;
use state_serialize::PikchrEguiPersistent;

mod identifiers;
mod image;
mod menubar;
pub mod message_handler;
mod mini_window;
mod modal;
mod pikchr_editor;
mod prolog_editor;
pub mod state;
pub mod text_highlighting;
mod editor;
mod state_serialize;
mod svg;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(from = "PikchrEguiPersistent", into = "PikchrEguiPersistent")]
pub struct PikchrEgui {
    tx: mpsc::Sender<Msg>,
    state: Arc<RwLock<AppState>>,
    first_frame: bool,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum ExportType {
    Svg,
    Png,
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum Msg {
    // Utilities
    Batch(Vec<Msg>),
    PopModal,

    // Exporting
    ExportModal(egui::Id, String, ExportType),
    Export(egui::Id, String, ExportType),

    // Editor Menu
    FontSizeModal(egui::Id),

    // Drawing
    RequestRedraw(#[serde(skip)] Context, egui::Id),
    UpdatePikchr(#[serde(skip)]  Context, egui::Id),
    UpdateProlog(#[serde(skip)]  Context, egui::Id, String),
    ResetError(egui::Id),
    UpdateContent(egui::Id, String),
    DeleteWindow(egui::Id),

    // Windows
    ToggleWindow(Window),
    ToggleWindowById(egui::Id),
    NewWindow(crate::mini_window::WindowType),

		// Svg Handling
    RecreateSvg(#[serde(skip)] Context, egui::Id),
    ReloadSvgs(#[serde(skip)] Context),

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
}
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum Window {
    Logger,
    Debugger,
}

impl PikchrEgui {
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
        }
    }
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let start_def = || {
            let blank_state = Arc::new(RwLock::new(AppState::default()));
            let tx = Self::spawn_message_handler(blank_state.clone());

            Self {
                tx: tx.clone(),
                state: blank_state,
                first_frame: true,
            }
        };
        if let Some(storage) = cc.storage {
            if let Some(persistent) =
                eframe::get_value::<PikchrEguiPersistent>(storage, eframe::APP_KEY)
            {
                eprintln!("Load happening");
                let mut prev_state = PikchrEgui::from(persistent);
                let tx = Self::spawn_message_handler(prev_state.state.clone());
                prev_state.tx = tx.clone();
                let _ = tx.try_send(Msg::ReloadSvgs(cc.egui_ctx.clone()));
                prev_state
            } else {
                eprintln!("Prev state not found");
                start_def()
            }
        } else {
            eprintln!("Storage not found");
            start_def()
        }
    }
    pub fn spawn_message_handler(state: Arc<RwLock<AppState>>)-> mpsc::Sender<Msg> {
        let (tx, rx) = mpsc::channel::<Msg>(100);
        let span = tracing::info_span!("message_handler");
        let _ = tokio::spawn(message_handler::handle(
            rx,
            state.clone(),
        )).instrument(span);
        tx
    }
    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.first_frame {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([1200.0, 800.0].into()));
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

        for window in self.state.write().windows.write().values_mut() {
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
    }
}

impl eframe::App for PikchrEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eprintln!("Saving!");
        let persistent = PikchrEguiPersistent::from(self.clone());
        eframe::set_value(storage, eframe::APP_KEY, &persistent);
        storage.flush();
    }
}

fn replace_content(state: &mut AppState, id: egui::Id) -> String {
    let content = state
        .windows
        .write()
        .get(&id)
        .and_then(|w| w.as_editor_window())
        .map(|c| c.content.get_pikchr_content())
        .unwrap_or_default();
    let editors: Vec<(egui::Id, String, String)> = state
        .windows
        .read()
        .values()
        .flat_map(|e| e.as_editor_window())
        //.filter(|e| e.editor_type.get_editor_type() == EditorType::Pikchr)
        .filter(|e| e.id != &id)
        .map(|e| (*e.id, format!("$${:?}$$", e.id), e.content.get_pikchr_content()))
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
        for (repl_id, repl, value) in &editors {
            let wrapped_value = format!("G{}: [{value};right]", repl_id.short_debug_format());
            content = content.replace(repl, &wrapped_value);
        }
    }
    content
}

pub const SPACE_MONO_BYTES: &[u8] = include_bytes!("../../pikchr_pl//fonts/SpaceMono-Regular.ttf");
pub const SPACE_MONO_NAME: &str = "Space Mono"; // Must match the internal TTF Name
