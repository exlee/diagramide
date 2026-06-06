use eframe::egui::{self, Context};
use parking_lot::RwLock;
use slog::{Logger, debug, info, o};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

use state::AppState;
use state_serialize::DiagramIDEPersistent;


mod editor;
mod identifiers;
mod image;
pub mod logger;
mod menubar;
pub mod message_handler;
mod mini_window;
mod modal;
mod mruby;
mod mruby_editor;
mod pikchr_editor;
mod plain_text_editor;
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
    CheckDependencies,

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
    UpdatePikchr(#[serde(skip)] Context, egui::Id, String),
    UpdateProlog(#[serde(skip)] Context, egui::Id, String),
    UpdateTcl(#[serde(skip)] Context, egui::Id, String),
    UpdateMruby(#[serde(skip)] Context, egui::Id, String),
    UpdatePlainText(#[serde(skip)] Context, egui::Id),
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
    Refresh(#[serde(skip)] Context, egui::Id),

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
    Mruby,
    PlainText,
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
                let tx = Self::spawn_message_handler(
                    prev_state.logger.clone(),
                    prev_state.state.clone(),
                );
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
    pub fn spawn_message_handler(
        logger: Logger,
        state: Arc<RwLock<AppState>>,
    ) -> mpsc::Sender<Msg> {
        debug!(logger, "Spawning logger");
        let (tx, rx) = mpsc::channel::<Msg>(100);
        tokio::spawn(message_handler::handle(rx, logger, state.clone()));
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
        egui::TopBottomPanel::top("top_panel").show(ctx, menubar::widget(state, tx_clone));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Workspace");
        });

        {
            let state = self.state.clone();
            let tx_clone = self.tx.clone();
            if let Some(modal) = state.read().modals.front() {
                modal.write().show(ctx, tx_clone);
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
            egui::Window::new("FPS").show(ctx, |ui| {
                ctx.inspection_ui(ui);
            });
        }
        egui::Area::new(egui::Id::new("bottom_right_status"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0))
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Non-mandated use only. Contact for commercial license.")
                        .weak(),
                );
            });
    }
}

impl eframe::App for DiagramIDE {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        tracing::info!(tracy.frame_mark = true);
        let _span = tracing::info_span!("ui_update").entered();

        self.window_size = ctx.content_rect().size();
        self.ui(ctx);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {

        info!(slog_scope::logger(), "Saving!"; "category" => "persistence");
        let persistent = DiagramIDEPersistent::from(self.clone());
        eframe::set_value(storage, eframe::APP_KEY, &persistent);
        storage.flush();
    }
}

fn has_dependency(content: &str, name: &str) -> bool {
    let options = vec![format!("!!{}!!", name), format!("$${}$$", name)];
    for o in options {
        if content.contains(&o) {
            return true;
        }
    }
    false
}
fn clean_old_deps(state: &mut AppState) {
    let span = tracing::info_span!("clean_old_deps", deps_cleaned = tracing::field::Empty);
    let _enter = span.enter();
    let mut cleared_deps = 0;
    let dkeys: Vec<egui::Id> = state.editor_deps.keys().cloned().collect();
    for dkey in dkeys {
        let editor_deps = &mut state.editor_deps;
        let Some(dname) = (|| {
            let v = state.windows.get(&dkey)?.as_name()?.get_name();
            Some(v)
        })() else {
            continue;
        };
        let ids = editor_deps.entry(dkey).or_default();
        for id in ids.clone().into_iter() {
            let pik_content = 
                state.windows.get(&id)
                .and_then(|w| w.as_pikchr_content())
                .map(|pc| pc.get_pikchr_content())
                .unwrap_or_default();

            let raw_content = 
                state.windows.get(&id)
                .and_then(|w| w.as_raw_content())
                .map(|pc| pc.get_raw_content())
                .unwrap_or_default();

            let dep_count: usize = vec![pik_content, raw_content]
                .into_iter()
                .map(|c| has_dependency(&c, &dname))
                .map(|b| if b { 1 } else { 0 })
                .sum();
            if dep_count == 0 {
                tracing::debug!(from = ?&dkey, to = ?&id, "removing dependency");

                slog_scope::debug!("removing dep"; "payload" => format!("{:?} -x- {:?}", &dkey, &id), "category" => "clean_old_deps");
                cleared_deps += 1;
                ids.remove(&id);
            }
        }
    }
    span.record("deps_cleaned", cleared_deps);
}
fn replace_raw_content(state: &mut AppState, id: egui::Id, content: &str) -> String {
    let editors: Vec<(egui::Id, String, String, String)> = state
        .windows
        .values()
        .filter_map(|window| {
            let editor_id = window.as_id()?.get_id();
            if editor_id == id {
                return None;
            }
            let name = window.as_name()?.get_name();
            let raw_content = window.as_raw_content()?.get_raw_content();
            Some((
                editor_id,
                name.clone(),
                format!("!!{name}!!"),
                raw_content,
            ))
        })
        .collect();
    let mut content = String::from(content);
    for (repl_id, name, _repl, _value) in &editors {
        let entry = state.editor_deps.entry(*repl_id).or_default();
        if has_dependency(&content, name) {
            slog_scope::debug!("new dependency"; "type" => "raw", "payload" => format!("{:?} -> {:?}", repl_id, id));
            entry.insert(id);
        }
    }
    for _ in 1..=3 {
        for (_repl_id, _name, repl, value) in &editors {
            let wrapped_value = value.clone();
            content = content.replace(repl, &wrapped_value);
        }
    }
    content
}
fn replace_content(state: &mut AppState, id: egui::Id, content: &str) -> String {
    let content = replace_pikchr_content(state, id, content);
    replace_raw_content(state, id, &content)
}
fn replace_pikchr_content(state: &mut AppState, id: egui::Id, content: &str) -> String {
    let editors: Vec<(egui::Id, &str, String, String)> = state
        .windows
        .values()
        .flat_map(|e| e.as_editor_window())
        .filter(|e| e.id != &id)
        .map(|e| {
            (
                *e.id,
                e.name,
                format!("$${}$$", e.name),
                e.content.get_pikchr_content(),
            )
        })
        .collect();
    let mut content = String::from(content);

    for (repl_id, name, _repl, _value) in &editors {
        let entry = state.editor_deps.entry(*repl_id).or_default();
        if has_dependency(&content, name) {
            slog_scope::debug!("new dependency"; "type" => "pikchr", "payload" => format!("{:?} -> {:?}", repl_id, id));
            entry.insert(id);
        };
    }
    for _ in 1..=3 {
        for (_repl_id, _name, repl, value) in &editors {
            let wrapped_value = format!("{value};");
            content = content.replace(repl, &wrapped_value);
        }
    }
    content
}

pub const SPACE_MONO_BYTES: &[u8] = include_bytes!("../../pikchr_pl//fonts/SpaceMono-Regular.ttf");
pub const SPACE_MONO_NAME: &str = "Space Mono"; // Must match the internal TTF Name

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::RwLock;

    use crate::{
        DiagramIDE, Msg,
        mini_window::{HasName, RawContent, Window, WindowType},
        pikchr_editor::PikchrEditor,
        plain_text_editor::PlainTextEditor,
        state::AppState,
    };

    #[test]
    fn plain_text_is_only_available_as_raw_content() {
        let plain_id = crate::egui::Id::new("plain");
        let pikchr_id = crate::egui::Id::new("pikchr");
        let svg_id = crate::egui::Id::new("svg");
        let mut plain = PlainTextEditor::new(plain_id);
        plain.set_name("REF".into());
        plain.set_raw_content("embedded text".into());

        let mut state = AppState::default();
        state
            .windows
            .insert(plain_id, Window::PlainTextEditor(plain));
        state.windows.insert(
            pikchr_id,
            Window::PikchrEditor(PikchrEditor::new(pikchr_id, svg_id)),
        );

        assert_eq!(
            crate::replace_content(&mut state, pikchr_id, "before !!REF!! after"),
            "before embedded text after"
        );
        assert_eq!(
            crate::replace_pikchr_content(&mut state, pikchr_id, "$$REF$$"),
            "$$REF$$"
        );
        assert!(
            state
                .windows
                .get(&plain_id)
                .and_then(Window::as_pikchr_content)
                .is_none()
        );
        assert!(state.editor_deps[&plain_id].contains(&pikchr_id));
    }

    #[tokio::test]
    async fn creating_plain_text_does_not_create_an_svg_window() {
        let state = Arc::new(RwLock::new(AppState::default()));
        let tx = DiagramIDE::spawn_message_handler(crate::logger::init_logger(), state.clone());

        tx.send(Msg::NewWindow(WindowType::PlainTextEditor))
            .await
            .unwrap();
        while state.read().windows.is_empty() {
            tokio::task::yield_now().await;
        }

        let state = state.read();
        assert_eq!(state.windows.len(), 1);
        assert!(matches!(
            state.windows.values().next(),
            Some(Window::PlainTextEditor(_))
        ));
    }
}
