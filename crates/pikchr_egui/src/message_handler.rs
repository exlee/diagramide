use std::{collections::{HashSet, VecDeque}, sync::Arc};

use eframe::egui;
use parking_lot::RwLock;

use crate::{AppState, EditorType, Msg, SPACE_MONO_NAME, Window, identifiers, pikchr_editor::PikchrEditor, prolog_editor::PrologEditor, sub_window::EditorMiniWindow, svg::{self, SvgWindow}};
use crate::sub_window::{Indexable,Id};

macro_rules! handle_toggle {
    ($ctx:ident, $state:ident, $var:ident) => {{
        let prev_state = $state.read().windows.$var;
        $state.write().windows.$var = !prev_state;

        $ctx.request_repaint();
    }};
}
macro_rules! make_window {
    ($state:ident,$init:expr) => {{
        let window = $init;
        let id = window.get_id();
        let state_write = $state.clone();
        let reference = Arc::new(RwLock::new(window));
        state_write
            .write()
            .mini_windows
            .insert(id, reference.clone());
        (id, reference)
    }};
}

pub async fn handle(
    mut rx: tokio::sync::mpsc::Receiver<Msg>,
    state: Arc<RwLock<AppState>>,
    ctx: egui::Context,
) {
    let mut local_queue: VecDeque<Msg> = VecDeque::new();
    while let Some(msg) = rx.recv().await {
        local_queue.push_back(msg);
        while let Some(msg) = local_queue.pop_front() {
            dbg!(&msg);
            match msg {
                Msg::Batch(msgs) => {
                    for m in msgs {
                        local_queue.push_back(m);
                    }
                },
                Msg::UpdateContent(id, content) => {
                    let state = state.write();
                    eprintln!("1");
                    let r = state.editors.get(&id);
                    if r.is_none() {
                        continue;
                    }
                    eprintln!("2");
                    r.unwrap().write().set_content(content);
                },
                Msg::RequestRedraw(id) => {
                    let deps: Vec<egui::Id> = {
                        let write_state = state.write();
                        let mut reference = match write_state.svg_windows.get(&id) {
                            Some(window) => window.write(),
                            None => continue,
                        };
                        let svg_string = reference.svg_string.clone();
                        let scale = reference.scale;
                        reference.diagram_texture = svg::render_svg_to_texture(
                            &ctx,
                            &svg_string.unwrap(),
                            "pikchr_diagram",
                            scale,
                        );
                        let mut editor_deps = write_state.editor_deps.clone();
                        editor_deps.entry(id).or_default().iter().cloned().collect()
                    };
                    for dep_id in deps {
                        dbg!("Trying send!");
                        local_queue.push_back(Msg::RequestRedraw(dep_id));
                    }

                    ctx.request_repaint();
                },
                Msg::UpdatePikchr(id) => {
                    eprintln!("UpdatePikchr");
                    // Logic for immediate updates
                    let mut state = state.write();
                    let Some(window) = state.editors.get(&id) else {
                        eprintln!("Editor not found");
                        continue;
                    };
                    let svg_id = window.read().get_target();

                    let content = crate::replace_content(&mut state, id);

                    let svg_maybe = pikchr_pro::pikchr::render_pikchr(
                        pikchr_pro::types::PikchrCode::new(content),
                    );
                    match svg_maybe {
                        Err(err) => {
                            state.log.push(format!("{:?}", err));
                        },
                        Ok(svg) => {
                            let svg_string = svg.inject_svg_style(SPACE_MONO_NAME).into_inner();
                            if let Some(reference) = state.svg_windows.get(&svg_id) {
                                reference.write().svg_string = Some(svg_string);
                                local_queue.push_back(Msg::RequestRedraw(svg_id));
                            }
                        },
                    }
                    for dep in state.editor_deps.get(&id).unwrap_or(&HashSet::new()) {
                        local_queue.push_back(Msg::UpdatePikchr(*dep))
                    }

                    ctx.request_repaint();
                },
                Msg::UpdateProlog(id, svg_id, content) => {
                    // Logic for immediate updates
                    let pikchr_code =
                        pikchr_pro::prolog::engine::trealla::EngineAsync::process_diagram(vec![
                            content,
                        ])
                        .await;

                    match pikchr_code {
                        Err(err) => {
                            state.write().log.push(format!("{:?}", err));
                            ctx.request_repaint();
                        },
                        Ok(pikchr) => {
                            local_queue.push_back(Msg::Batch(vec![
                                Msg::UpdateContent(id, pikchr.into_inner()),
                                Msg::UpdatePikchr(id),
                            ]));
                        },
                    }
                },
                Msg::Process(_content) => {
                    // This awaits, ensuring sequential execution order
                },
                Msg::ToggleWindow(Window::PikchrEditor) => {
                    handle_toggle!(ctx, state, pikchr_editor)
                },
                Msg::ToggleWindow(Window::PrologEditor) => {
                    handle_toggle!(ctx, state, prolog_editor)
                },
                Msg::ToggleWindow(Window::Debugger) => handle_toggle!(ctx, state, debug),
                Msg::ToggleWindow(Window::Logger) => handle_toggle!(ctx, state, log),
                Msg::ToggleWindowById(id) => {
                    if let Some(window) = state.write().mini_windows.get_mut(&id) {
                        window.write().toggle_visible();
                    }
                },
                Msg::NewEditor(editor_type) => {
                    let counter = identifiers::next_counter();
                    let svg_id = identifiers::next_global_id();
                    let editor_id = identifiers::next_global_id();
                    let (_, svg) = make_window!(state, SvgWindow::new(svg_id));
                    svg.clone().write().set_index(counter);
                    state.clone().write().svg_windows.insert(svg_id, svg);
                    let result: (egui::Id, Arc<RwLock<dyn EditorMiniWindow>>) = match editor_type {
                        EditorType::Prolog => {
                            make_window!(state, PrologEditor::new(editor_id, svg_id))
                        },
                        EditorType::Pikchr => {
                            make_window!(state, PikchrEditor::new(editor_id, svg_id))
                        },
                    };
                    state
                        .clone()
                        .write()
                        .editors
                        .insert(editor_id, result.1.clone());
                    result.1.write().set_index(counter);
                },
            }
        }
    }
}
