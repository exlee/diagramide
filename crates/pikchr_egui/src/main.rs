use eframe::egui::{self, Id, MenuBar, Vec2, ViewportCommand};
use parking_lot::RwLock;
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize},
    },
};
use tokio::sync::mpsc;

use crate::{
    pikchr_editor::PikchrEditor,
    prolog_editor::PrologEditor,
    sub_window::{EditorMiniWindow, Id as _, Indexable, IndexableMiniWindow},
    svg::SvgWindow,
};
use state::AppState;


struct PikchrEgui {
    tx: mpsc::Sender<Msg>,
    state: Arc<RwLock<AppState>>,
    first_frame: bool,
}
#[derive(Debug)]
pub enum Msg {
    Batch(Vec<Msg>),
    RequestRedraw(egui::Id),
    UpdatePikchr(egui::Id),
    UpdateProlog(egui::Id, egui::Id, String),
    Process(String),
    ToggleWindow(Window),
    ToggleWindowById(egui::Id),
    NewEditor(EditorType),
    UpdateContent(egui::Id, String),
}
#[derive(PartialEq, Debug)]
pub enum EditorType {
    Prolog,
    Pikchr,
}
#[derive(Debug)]
pub enum Window {
    PrologEditor,
    Logger,
    PikchrEditor,
    Debugger,
}

mod identifiers;
mod menubar;
mod message_handler;
mod pikchr_editor;
mod prolog_editor;
mod sub_window;
mod svg;
mod state;

#[tokio::main]
async fn main() -> eframe::Result<()> {
    let (tx, rx) = mpsc::channel::<Msg>(100);
    let state = Arc::new(RwLock::new(AppState::new()));
    let native_options = eframe::NativeOptions::default();

    let ui_state = state.clone();
    eframe::run_native(
        "Pikchr.pl",
        native_options,
        Box::new(|cc| {
            catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::FRAPPE);
            Ok(Box::new(PikchrEgui::new(cc, rx, tx, ui_state)))
        }),
    )
}

impl PikchrEgui {
    fn new(
        cc: &eframe::CreationContext<'_>,
        rx: mpsc::Receiver<Msg>,
        tx: mpsc::Sender<Msg>,
        state: Arc<RwLock<AppState>>,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        let state_clone = state.clone();

        egui_extras::install_image_loaders(&cc.egui_ctx);
        tokio::spawn(message_handler::handle(rx, state_clone, ctx));
        Self {
            tx,
            state,
            first_frame: true,
        }
    }
}

impl eframe::App for PikchrEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
    for _ in 1..=3 {
        for (repl_id, repl, value) in &editors {
            let entry = state.editor_deps.entry(*repl_id).or_default();
            if content.contains(repl) {
                entry.insert(id);
            } else {
                entry.remove(&id);
            };
            content = content.replace(repl, value);
        }
    }
    content
}

const SPACE_MONO_BYTES: &[u8] = include_bytes!("../../pikchr_pl//fonts/SpaceMono-Regular.ttf");
const SPACE_MONO_NAME: &str = "Space Mono"; // Must match the internal TTF Name
