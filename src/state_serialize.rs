use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};

use eframe::egui::{self};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::{
    DiagramIDE, Msg, identifiers, logger, mini_window,
    state::{AppState, DiagramBackground, LibraryEntry, WindowState, Workspace, WorkspaceId},
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct AppStatePersistent {
    #[serde(skip_serializing, default)]
    pub log: Vec<String>,
    pub editor_deps: HashMap<egui::Id, HashSet<egui::Id>>,
    #[serde(default)]
    pub window_library_paths: HashMap<egui::Id, String>,
    pub window_states: WindowState,
    pub windows: HashMap<egui::Id, mini_window::Window>,
    #[serde(default = "default_theme")]
    pub active_theme: String,
    #[serde(default)]
    pub diagram_background: DiagramBackground,
    #[serde(default)]
    pub library: BTreeMap<String, LibraryEntry>,

    // ── Multiple workspaces ───────────────────────────────────────────
    #[serde(default)]
    pub active_workspace_id: WorkspaceId,
    #[serde(default = "default_workspace_name")]
    pub active_workspace_name: String,
    /// Dormant workspaces. Absent (or empty) on pre-workspace save files,
    /// which triggers migration in `From<AppStatePersistent>`.
    #[serde(default)]
    pub workspaces: HashMap<WorkspaceId, Workspace>,
}

fn default_theme() -> String {
    crate::theme::DEFAULT_THEME_ID.to_owned()
}

fn default_workspace_name() -> String {
    String::from("Default")
}

impl From<AppState> for AppStatePersistent {
    fn from(mut value: AppState) -> Self {
        // Flush the live workspace into the dormant registry so the active
        // workspace is captured in `workspaces` alongside the others.
        value.flush_active();
        let active_ws = value
            .workspaces
            .get(&value.active_workspace_id)
            .cloned()
            .unwrap_or_default();
        Self {
            log: value.log,
            editor_deps: active_ws.editor_deps,
            window_library_paths: active_ws.window_library_paths,
            window_states: value.window_states,
            windows: active_ws.windows,
            active_theme: value.active_theme,
            diagram_background: value.diagram_background,
            active_workspace_id: value.active_workspace_id,
            active_workspace_name: value.active_workspace_name,
            workspaces: value.workspaces,
            library: value.library,
        }
    }
}
impl From<AppStatePersistent> for AppState {
    fn from(value: AppStatePersistent) -> Self {
        // Migration: pre-workspace save files have no `workspaces` map (and
        // default `active_workspace_id == 0`). Fold their legacy
        // `windows`/`editor_deps` into a single freshly-id'd "Default"
        // workspace and make it active (live fields). The dormant map stays
        // empty because there is only this one workspace.
        if value.workspaces.is_empty() {
            let id = identifiers::next_workspace_id();
            return Self {
                log: value.log,
                editor_deps: value.editor_deps,
                window_library_paths: value.window_library_paths,
                window_states: value.window_states,
                windows: value.windows,
                modals: VecDeque::new(),
                active_theme: value.active_theme,
                diagram_background: value.diagram_background,
                active_workspace_id: id,
                active_workspace_name: value.active_workspace_name,
                workspaces: HashMap::new(),
                library: value.library,
            };
        }

        // Normal path: promote the active workspace to the live fields.
        let active_id = value.active_workspace_id;
        let mut workspaces = value.workspaces;
        let active = workspaces.remove(&active_id).unwrap_or_else(|| {
            // active id missing from map — fall back to first remaining,
            // or synthesize an empty default workspace.
            if let Some((&id, ws)) = workspaces.iter().next() {
                let ws = ws.clone();
                let _ = id;
                ws
            } else {
                Workspace {
                    id: active_id,
                    name: value.active_workspace_name.clone(),
                    windows: value.windows.clone(),
                    editor_deps: value.editor_deps.clone(),
                    window_library_paths: value.window_library_paths.clone(),
                }
            }
        });

        Self {
            log: value.log,
            editor_deps: active.editor_deps,
            window_library_paths: active.window_library_paths,
            window_states: value.window_states,
            windows: active.windows,
            modals: VecDeque::new(),
            active_theme: value.active_theme,
            diagram_background: value.diagram_background,
            active_workspace_id: active.id,
            active_workspace_name: active.name,
            workspaces,
            library: value.library,
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
        let seen_workspace_id = app_state.active_workspace_id;
        let state = Arc::new(RwLock::new(app_state));
        let window_size = value.window_size;
        DiagramIDE {
            tx,
            state,
            window_size,
            first_frame: true,
            logger: logger::init_logger(),
            seen_workspace_id,
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
