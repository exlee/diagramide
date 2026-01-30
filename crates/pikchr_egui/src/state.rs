use eframe::egui::{self};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use crate::{mini_window::{self, EditorMiniWindow, IndexableMiniWindow}, modal::Modal, svg::SvgWindow};

pub struct WindowState {
    pub debug: bool,
    pub pikchr_editor: bool,
    pub prolog_editor: bool,
    pub log: bool,
}

pub struct AppState {
    pub log: Vec<String>,
    pub mini_windows: HashMap<egui::Id, Arc<RwLock<dyn IndexableMiniWindow>>>,
    pub svg_windows: HashMap<egui::Id, Arc<RwLock<SvgWindow>>>,
    pub editors: HashMap<egui::Id, Arc<RwLock<dyn EditorMiniWindow>>>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub windows: WindowState,
    pub windows_enum: Arc<RwLock<HashMap<egui::Id, mini_window::Window>>>,
    pub modals: VecDeque<Arc<RwLock<dyn Modal>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            mini_windows: HashMap::new(),
            svg_windows: HashMap::new(),
            editors: HashMap::new(),
            editor_deps: HashMap::new(),
            modals: VecDeque::new(),
            windows_enum: Arc::new(RwLock::new(HashMap::new())),
            windows: WindowState {
                debug: false,
                pikchr_editor: true,
                log: false,
                prolog_editor: true,
            },
        }
    }
}
