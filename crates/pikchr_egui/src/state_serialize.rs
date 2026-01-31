use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use eframe::egui::{self, Context};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::{
    Msg, PikchrEgui, message_handler, mini_window,
    modal::ModalItem,
    state::{AppState, WindowState},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct AppStatePersistent {
    #[serde(skip_serializing,default)]
    pub log: Vec<String>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub window_states: WindowState,
    pub windows: HashMap<egui::Id, mini_window::Window>,
    pub modals: VecDeque<ModalItem>,
}

impl From<AppState> for AppStatePersistent {
    fn from(value: AppState) -> Self {
        let windows: HashMap<egui::Id, mini_window::Window> = value.windows.read().clone();
        Self {
            log: value.log,
            editor_deps: value.editor_deps,
            window_states: value.window_states,
            windows,
            modals: value
                .modals
                .iter()
                .map(|v| v.read())
                .map(|v| v.as_item())
                .collect(),
        }
    }
}
impl From<AppStatePersistent> for AppState {
    fn from(value: AppStatePersistent) -> Self {
        let modals = value
            .modals
            .iter()
            .map(|m| m.as_modal())
            .collect::<VecDeque<_>>();
        Self {
            log: value.log,
            editor_deps: value.editor_deps,
            window_states: value.window_states,
            windows: Arc::new(RwLock::new(value.windows)),
            modals,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct PikchrEguiPersistent {
    state: AppStatePersistent,
}
impl From<PikchrEguiPersistent> for PikchrEgui {
    fn from(value: PikchrEguiPersistent) -> Self {
        let (tx, rx) = mpsc::channel::<Msg>(100);
        let app_state = AppState::from(value.state);
        let state = Arc::new(RwLock::new(app_state));
        PikchrEgui {
            tx,
            state,
            first_frame: true,
        }
    }
}
impl From<PikchrEgui> for PikchrEguiPersistent {
    fn from(value: PikchrEgui) -> Self {
        let v = value.state.read().clone();
        PikchrEguiPersistent {
            state: AppStatePersistent::from(v)
        }
    }
}
