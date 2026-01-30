use eframe::egui::{self};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use crate::{
    mini_window::{self},
    modal::Modal,
};

pub struct WindowState {
    pub debug: bool,
    pub log: bool,
}

pub struct AppState {
    pub log: Vec<String>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub windows: WindowState,
    pub windows_enum: Arc<RwLock<HashMap<egui::Id, mini_window::Window>>>,
    pub modals: VecDeque<Arc<RwLock<dyn Modal>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            editor_deps: HashMap::new(),
            modals: VecDeque::new(),
            windows_enum: Arc::new(RwLock::new(HashMap::new())),
            windows: WindowState {
                debug: false,
                log: false,
            },
        }
    }
}
