use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use eframe::egui::{self};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::{
    DiagramIDE, Msg,
    help::HelpTopic,
    logger, mini_window,
    state::{AppState, DiagramBackground, WindowState},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppStatePersistent {
    #[serde(skip_serializing, default)]
    pub log: Vec<String>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub window_states: WindowState,
    pub windows: HashMap<egui::Id, mini_window::Window>,
    #[serde(default)]
    pub help_topic: Option<HelpTopic>,
    #[serde(default = "default_theme")]
    pub active_theme: String,
    #[serde(default)]
    pub diagram_background: DiagramBackground,
}

fn default_theme() -> String {
    crate::theme::DEFAULT_THEME_ID.to_owned()
}

impl From<AppState> for AppStatePersistent {
    fn from(value: AppState) -> Self {
        let windows: HashMap<egui::Id, mini_window::Window> = value.windows.clone();
        Self {
            log: value.log,
            editor_deps: value.editor_deps,
            window_states: value.window_states,
            windows,
            help_topic: value.help_topic,
            active_theme: value.active_theme,
            diagram_background: value.diagram_background,
        }
    }
}
impl From<AppStatePersistent> for AppState {
    fn from(value: AppStatePersistent) -> Self {
        Self {
            log: value.log,
            editor_deps: value.editor_deps,
            window_states: value.window_states,
            windows: value.windows,
            modals: VecDeque::new(),
            help_topic: value.help_topic,
            active_theme: value.active_theme,
            diagram_background: value.diagram_background,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DiagramIDEPersistent {
    state: AppStatePersistent,
    window_size: egui::Vec2,
}
impl From<DiagramIDEPersistent> for DiagramIDE {
    fn from(value: DiagramIDEPersistent) -> Self {
        let (tx, _rx) = mpsc::channel::<Msg>(100);
        let app_state = AppState::from(value.state);
        let state = Arc::new(RwLock::new(app_state));
        let window_size = value.window_size;
        DiagramIDE {
            tx,
            state,
            window_size,
            first_frame: true,
            logger: logger::init_logger(),
        }
    }
}
impl From<DiagramIDE> for DiagramIDEPersistent {
    fn from(value: DiagramIDE) -> Self {
        let v = value.state.read().clone();
        DiagramIDEPersistent {
            state: AppStatePersistent::from(v),
            window_size: value.window_size,
        }
    }
}
