use eframe::egui::{self};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{identifiers::next_global_id, mini_window::{Id as _, Indexable}, modal::ExportModal};
use state::AppState;

mod identifiers;
mod menubar;
pub mod message_handler;
mod mini_window;
mod pikchr_editor;
mod prolog_editor;
pub mod state;
mod modal;
mod svg;

pub struct PikchrEgui {
    tx: mpsc::Sender<Msg>,
    state: Arc<RwLock<AppState>>,
    first_frame: bool,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum ExportType {
    SVG,
    PNG
}
#[derive(Debug, serde::Serialize,serde::Deserialize)]
pub enum Msg {
    Batch(Vec<Msg>),
    Export(egui::Id, ExportType),
    RequestRedraw(egui::Id),
    UpdatePikchr(egui::Id),
    UpdateProlog(egui::Id, egui::Id, String),
    Process(String),
    ToggleWindow(Window),
    ToggleWindowById(egui::Id),
    NewWindow(crate::mini_window::WindowType),
    NewEditor(EditorType),
    UpdateContent(egui::Id, String),
    DeleteWindow(egui::Id),
    PopModal,
}
#[derive(PartialEq, Debug, serde::Serialize,serde::Deserialize)]
pub enum EditorType {
    Prolog,
    Pikchr,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Window {
    PrologEditor,
    Logger,
    PikchrEditor,
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
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rx: mpsc::Receiver<Msg>,
        tx: mpsc::Sender<Msg>,
        state: Arc<RwLock<AppState>>,
    ) -> Self {
        let state_clone = state.clone();
        state.write().modals.push_back(Arc::new(RwLock::new(ExportModal::new(next_global_id(), ExportType::PNG))));
        let ctx = &cc.egui_ctx;
        tokio::spawn(message_handler::handle(rx, state_clone, ctx.clone()));
        egui_extras::install_image_loaders(ctx);
        Self {
            tx,
            state,
            first_frame: true,
        }
    }
    pub fn ui(&mut self, ctx: &egui::Context) {
        if self.first_frame {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize([1200.0, 800.0].into()));
            self.first_frame = false;
        }
        let state = self.state.clone();
        let tx_clone = self.tx.clone();
        egui::TopBottomPanel::top("top_panel").show(ctx, menubar::widget(state, tx_clone));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Workspace");
        });
        {
            let state = self.state.clone();
            let tx_clone = self.tx.clone();
            if let Some(modal) = state.read().modals.front() {
                modal.read().show(ctx, tx_clone);
            }
        }

				for window in self.state.write().windows_enum.write().values_mut() {
    				if let Some(mini) = window.as_mini_window_mut() {
        				mini.show(ctx, self.tx.clone(), self.state.clone());
    				}
				}
        for window in self.state.write().mini_windows.values_mut() {
            window
                .write()
                .show(ctx, self.tx.clone(), self.state.clone())
        }


        if self.state.clone().read().windows.log {
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

        if self.state.read().windows.debug {
            egui::Window::new("FPS").show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });
        }
    }
}

impl eframe::App for PikchrEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

fn replace_content(state: &mut AppState, id: egui::Id) -> String {
    let content = state
        .editors
        .get(&id)
        .expect("ID should exist")
        .read()
        .get_content();
    let editors: Vec<(egui::Id, String, String)> = state
        .editors
        .values()
        .filter(|&e| e.read().get_editor_type() == EditorType::Pikchr)
        .filter(|&e| e.read().get_id() != id)
        .map(|e| {
            (
                e.read().get_id(),
                format!("$${}$$", e.read().get_index()),
                e.read().get_content(),
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
            content = content.replace(repl, value);
        }
    }
    content
}

const SPACE_MONO_BYTES: &[u8] = include_bytes!("../../pikchr_pl//fonts/SpaceMono-Regular.ttf");
const SPACE_MONO_NAME: &str = "Space Mono"; // Must match the internal TTF Name
