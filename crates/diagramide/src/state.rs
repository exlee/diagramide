use eframe::egui::{self};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use crate::{
    help::HelpTopic,
    mini_window::{self},
    modal::Modal,
    state_serialize::AppStatePersistent,
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct WindowState {
    pub debug: bool,
    pub log: bool,
    pub profiler: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(from = "AppStatePersistent", into = "AppStatePersistent")]
pub struct AppState {
    pub log: Vec<String>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub window_states: WindowState,
    pub windows: HashMap<egui::Id, mini_window::Window>,
    pub modals: VecDeque<Arc<RwLock<dyn Modal>>>,
    pub help_topic: Option<HelpTopic>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            editor_deps: HashMap::new(),
            modals: VecDeque::new(),
            windows: HashMap::new(),
            help_topic: None,
            window_states: WindowState {
                profiler: false,
                debug: false,
                log: true,
            },
        }
    }
}
