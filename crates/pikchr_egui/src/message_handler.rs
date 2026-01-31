use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use eframe::egui;
use parking_lot::RwLock;

use crate::{
    AppState, Msg, SPACE_MONO_NAME, identifiers, mini_window, modal::ExportModal, pikchr_editor,
    prolog_editor, svg,
};

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
                Msg::DeleteWindow(id) => {
                    let mut state = state.write();
                    let dkeys: Vec<egui::Id> = state.editor_deps.keys().cloned().collect();
                    {
                        let mut windows = state.windows.write();
                        if let Some(targetable) = windows.get(&id).and_then(|w| w.as_target()) {
                            let target_id = targetable.get_target();
                           	windows.remove(&target_id);
                        }
                        windows.remove(&id);
                    }
                    for dkey in dkeys {
                        state.editor_deps.entry(dkey).and_modify(|e| {
                            e.remove(&id);
                        });
                    }
                },
                Msg::UpdateContent(id, content) => {
                    let state = state.write();
                    let mut windows_enum = state.windows.write();
                    let Some(r) = windows_enum.get_mut(&id) else {
                        continue;
                    };
                    if let Some(c) = r.as_content_mut() {
                        c.set_content(content);
                    };
                },
                Msg::RequestRedraw(id) => {
                    let deps: Vec<egui::Id> = {
                        let write_state = state.write();
                        let mut windows_enum = write_state.windows.write();
                        let reference =
                            match windows_enum.get_mut(&id).and_then(|s| s.as_svg_window()) {
                                Some(window) => window,
                                None => continue,
                            };
                        let Some(svg_string) = reference.svg_string.clone() else {
                            continue;
                        };
                        let scale = *reference.scale;
                        if let Some((im, te)) = crate::image::render_svg_to_texture(
                            &ctx,
                            &svg_string,
                            "pikchr_diagram",
                            scale,
                        ) {
                            *reference.image = Some(im);
                            *reference.diagram_texture = Some(te);
                        }
                        let mut editor_deps = write_state.editor_deps.clone();
                        editor_deps.entry(id).or_default().iter().cloned().collect()
                    };
                    for dep_id in deps {
                        local_queue.push_back(Msg::RequestRedraw(dep_id));
                    }

                    ctx.request_repaint();
                },
                Msg::UpdatePikchr(id) => {
                    // Logic for immediate updates
                    let (svg_maybe, svg_id) = {
                        let state_clone = state.clone();
                        let mut writable_state = state_clone.write();
                        let content = crate::replace_content(&mut writable_state, id).clone();

                        let windows_enum = &writable_state.windows.read();

                        let window = windows_enum.get(&id);
                        if window.is_none() {
                            continue;
                        }
                        let window = window.unwrap();
                        let Some(svg_id) = window.as_target().map(|t| t.get_target()) else {
                            continue;
                        };
                        (
                            pikchr_pro::pikchr::render_pikchr(pikchr_pro::types::PikchrCode::new(
                                content.clone(),
                            )),
                            svg_id,
                        )
                    };

                    let state_clone = state.clone();
                    let mut writable_state = state_clone.write();
                    match svg_maybe {
                        Err(err) => {
                            if let Some(errorable) = writable_state.windows.write().get_mut(&id).and_then(|w| w.as_error_mut()) {
                                errorable.set_error(Some(err.inner_string()))
                            };
                            writable_state.log.push(format!("{:?}", err));
                        },
                        Ok(svg) => {
                            let svg_string = svg.inject_svg_style(SPACE_MONO_NAME).into_inner();
                            let mut windows_enum = writable_state.windows.write();
                            local_queue.push_back(Msg::ResetError(id));
                            if let Some(reference) = windows_enum
                                .get_mut(&svg_id)
                                .and_then(|s| s.as_svg_window())
                            {
                                *reference.svg_string = Some(svg_string);
                                local_queue.push_back(Msg::RequestRedraw(svg_id));
                            } else {
                                local_queue.push_back(Msg::RecreateSvg(id))
                            }
                        },
                    }
                    for dep in writable_state
                        .editor_deps
                        .get(&id)
                        .unwrap_or(&HashSet::new())
                    {
                        local_queue.push_back(Msg::UpdatePikchr(*dep))
                    }

                    ctx.request_repaint();
                },
                Msg::RecreateSvg(id) => {
                    let svg_id = identifiers::next_global_id();
                    let svg_insert =
                        mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, id));
                    let state_write = state.write();
                    let mut windows = state_write.windows.write();
                    windows.insert(svg_id, svg_insert);

                    if let Some (targetable) = windows.get_mut(&id).and_then(|w| w.as_target_mut()) {
                        targetable.set_target(svg_id);
                    }
                    local_queue.push_back(Msg::UpdatePikchr(id));
                }
                Msg::UpdateProlog(id, _svg_id, content) => {
                    // Logic for immediate updates
                    let pikchr_code =
                        pikchr_pro::prolog::engine::trealla::EngineAsync::process_diagram(vec![
                            content,
                        ])
                        .await;

                    match pikchr_code {
                        Err(err) => {
                            let mut state_write = state.write();
                            state_write.log.push(format!("{:?}", err));
                            let mut windows = state_write.windows.write();
                            if let Some(errorable) = windows.get_mut(&id).and_then(|w| w.as_error_mut()) {
                                errorable.set_error(Some(err.inner_string()));
                            }
                            ctx.request_repaint();
                        },
                        Ok(pikchr) => {
                            local_queue.push_back(Msg::Batch(vec![
                                Msg::ResetError(id),
                                Msg::UpdateContent(id, pikchr.into_inner()),
                                Msg::UpdatePikchr(id),
                            ]));
                        },
                    }
                },
                Msg::Process(_content) => {
                    // This awaits, ensuring sequential execution order
                },
                Msg::ToggleWindow(crate::Window::Logger) => {
                    let mut state_write = state.write();
                    let current = state_write.window_states.log;
                    state_write.window_states.log = !current;
                },
                Msg::ToggleWindow(_) => (),
                Msg::ToggleWindowById(id) => {
                    if let Some(window) = state
                        .write()
                        .windows
                        .write()
                        .get_mut(&id)
                        .and_then(|w| w.as_mini_window_mut())
                    {
                        window.toggle_visible();
                    }
                },
                Msg::NewWindow(window_type) => match window_type {
                    crate::mini_window::WindowType::PikchrEditor => {
                        let editor_id = identifiers::next_global_id();
                        let svg_id = identifiers::next_global_id();
                        let editor_insert = mini_window::Window::PikchrEditor(
                            pikchr_editor::PikchrEditor::new(editor_id, svg_id),
                        );
                        let svg_insert =
                            mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id,editor_id));
                        let state_write = state.write();
                        let mut windows = state_write.windows.write();
                        windows.insert(editor_id, editor_insert);
                        windows.insert(svg_id, svg_insert);
                    },
                    crate::mini_window::WindowType::PrologEditor => {
                        let editor_id = identifiers::next_global_id();
                        let svg_id = identifiers::next_global_id();
                        let editor_insert = mini_window::Window::PrologEditor(
                            prolog_editor::PrologEditor::new(editor_id, svg_id),
                        );
                        let svg_insert =
                            mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, editor_id));
                        let state_write = state.write();
                        let mut windows = state_write.windows.write();
                        windows.insert(editor_id, editor_insert);
                        windows.insert(svg_id, svg_insert);
                    },
                    crate::mini_window::WindowType::SvgWindow => (),
                },
                Msg::ExportModal(id, name, export_type) => {
                    let modal = ExportModal::new(id, name, export_type);
                    state.write().modals.push_back(Arc::new(RwLock::new(modal)));
                },
                Msg::Export(svg_id, file, crate::ExportType::Png) => {
                    let state = state.read();
                    let mut windows = state.windows.write();
                    let Some(image) = windows
                        .get_mut(&svg_id)
                        .and_then(|w| w.as_svg_window())
                        .and_then(|s| s.image.clone())
                    else {
                        continue;
                    };
                    let _ = crate::image::write_png(file, image);
                    local_queue.push_back(Msg::PopModal);
                },
                Msg::Export(svg_id, file, crate::ExportType::Svg) => {
                    let state = state.read();
                    let mut windows = state.windows.write();
                    let Some(svg) = windows
                        .get_mut(&svg_id)
                        .and_then(|w| w.as_svg_window())
                        .and_then(|s| s.svg_string.clone())
                    else {
                        continue;
                    };
                    let _ = crate::image::write_svg(file, svg);
                    local_queue.push_back(Msg::PopModal);
                },
                Msg::PopModal => {
                    state.write().modals.pop_front();
                },
                Msg::ReloadSvgs => {
                    for w in state.write().windows.write().values_mut().flat_map(|m| m.as_svg_window()) {
                        local_queue.push_back(Msg::RequestRedraw(*w.id))
                    }
                }
                Msg::ResetError(id) => {
                    if let Some(errorable) = state.write().windows.write().get_mut(&id).and_then(|w| w.as_error_mut()) {
                        errorable.set_error(None);
                    }
                }
            }
        }
    }
}

// kakexec: <percent>s(?S)Msg::.*=<gt>.*\{<ret>mH<a-semicolon>L<space>jf,
