use eframe::egui::{self};
use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{sub_window::{EditorMiniWindow, IndexableMiniWindow}, svg::SvgWindow};

#[derive(Clone)]
pub struct WindowState {
    pub debug: bool,
    pub pikchr_editor: bool,
    pub prolog_editor: bool,
    pub log: bool,
}

pub struct AppState {
    pub log: Vec<String>,
    pub diagram_texture: Option<egui::TextureHandle>,
    pub mini_windows: HashMap<egui::Id, Arc<RwLock<dyn IndexableMiniWindow>>>,
    pub svg_windows: HashMap<egui::Id, Arc<RwLock<SvgWindow>>>,
    pub editors: HashMap<egui::Id, Arc<RwLock<dyn EditorMiniWindow>>>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    pub windows: WindowState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            diagram_texture: None,
            mini_windows: HashMap::new(),
            svg_windows: HashMap::new(),
            editors: HashMap::new(),
            editor_deps: HashMap::new(),
            windows: WindowState {
                debug: false,
                pikchr_editor: true,
                log: false,
                prolog_editor: true,
            },
        }
    }
}
