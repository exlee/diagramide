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

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq,
)]
pub enum DiagramBackground {
    Black,
    ThemeDark,
    ThemeBright,
    #[default]
    White,
}

impl DiagramBackground {
    pub fn resolve(self, visuals: &egui::Visuals) -> egui::Color32 {
        match self {
            Self::Black => egui::Color32::BLACK,
            Self::ThemeDark => visuals.panel_fill,
            Self::ThemeBright => visuals.faint_bg_color,
            Self::White => egui::Color32::WHITE,
        }
    }
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
    pub active_theme: String,
    pub diagram_background: DiagramBackground,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            editor_deps: HashMap::new(),
            modals: VecDeque::new(),
            windows: HashMap::new(),
            help_topic: None,
            active_theme: crate::theme::DEFAULT_THEME_ID.to_owned(),
            diagram_background: DiagramBackground::default(),
            window_states: WindowState {
                profiler: false,
                debug: false,
                log: true,
            },
        }
    }
}
