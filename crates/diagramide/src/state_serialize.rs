use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use eframe::egui::{self};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::{
    Msg, DiagramIDE, mini_window,
    state::{AppState, WindowState},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppStatePersistent {
    #[serde(skip_serializing, default)]
    pub log: Vec<String>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub window_states: WindowState,
    pub windows: HashMap<egui::Id, mini_window::Window>,
}

impl From<AppState> for AppStatePersistent {
    fn from(value: AppState) -> Self {
        let windows: HashMap<egui::Id, mini_window::Window> = value.windows.read().clone();
        Self {
            log: value.log,
            editor_deps: value.editor_deps,
            window_states: value.window_states,
            windows,
        }
    }
}
impl From<AppStatePersistent> for AppState {
    fn from(value: AppStatePersistent) -> Self {
        Self {
            log: value.log,
            editor_deps: value.editor_deps,
            window_states: value.window_states,
            windows: Arc::new(RwLock::new(value.windows)),
            modals: VecDeque::new(),
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
