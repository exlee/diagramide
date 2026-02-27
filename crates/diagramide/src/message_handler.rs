use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::BufReader,
    sync::Arc,
};

use eframe::egui;
use parking_lot::RwLock;
use slog::{Logger, debug, o, Serde};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt as _;
use tokio_util::time::{DelayQueue, delay_queue::Key as DelayKey};

use crate::{
    AppState, Msg, SPACE_MONO_NAME, identifiers, mini_window,
    modal::{
        ConfirmationModal, ExportModal, FileOpenModal, FileSaveModal, RenameModal, StringEditModal,
    },
    pikchr_editor, prolog_editor, svg, tcl, tcl_editor,
};

macro_rules! push_modal {
    ($state:ident, $modal:expr) => {
        $state
            .write()
            .modals
            .push_back(Arc::new(RwLock::new($modal)));
    };
}

macro_rules! create_editor_window {
    ($state: ident, $wintype:ident, $fun:path) => {
        let editor_id = identifiers::next_global_id();
        let svg_id = identifiers::next_global_id();
        let editor_insert = mini_window::Window::$wintype($fun(editor_id, svg_id));
        let svg_insert = mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, editor_id));
        let state_write = $state.write();
        let mut windows = state_write.windows.write();
        windows.insert(editor_id, editor_insert);
        windows.insert(svg_id, svg_insert);
    };
}
pub async fn handle(
    mut rx: tokio::sync::mpsc::Receiver<Msg>,
    logger: Logger,
    state: Arc<RwLock<AppState>>
    ) {
    let mut local_queue: VecDeque<Msg> = VecDeque::new();
    let mut delay_queue: DelayQueue<(egui::Id,Msg)> = DelayQueue::new();
    let mut pending_debounces: HashMap<egui::Id, DelayKey> = HashMap::new();
    let logger = logger.new(o!("category" => "event"));

    loop {
        tokio::select!{
            biased;

            Some(expired) = delay_queue.next(), if !delay_queue.is_empty() => {
                let (id, msg) = expired.into_inner();
                pending_debounces.remove(&id);
                local_queue.push_back(msg);
            }
            maybe_msg = rx.recv() => {
                match maybe_msg {
                    Some(Msg::Debounce(dur, id, inner)) => {
                        if let Some(delay_key) = pending_debounces.get(&id) {
                            delay_queue.remove(&delay_key);
                        }
                        let queue_key = delay_queue.insert_at(
                            (id, *inner),
                            tokio::time::Instant::now() + dur,
                        );
                        pending_debounces.insert(id, queue_key);

                    }
                    Some(msg) => local_queue.push_back(msg),
                    None => break,
                }
            }

        };
        while let Some(msg) = local_queue.pop_front() {
            debug!(logger, "handle msg"; "msg" => Serde(msg.clone()));
            match msg {
                Msg::Debounce(..) => unreachable!(),
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
                Msg::UpdateContent(id, _content) => {
                    let state = state.write();
                    let mut windows_enum = state.windows.write();
                    let Some(r) = windows_enum.get_mut(&id) else {
                        continue;
                    };
                    if let Some(_c) = r.as_content_mut() {
                        //c.set_pikchr_content(content);
                    };
                },
                Msg::UpdatePikchrContent(id, content) => {
                    let state = state.write();
                    let mut windows_enum = state.windows.write();
                    let Some(r) = windows_enum.get_mut(&id) else {
                        continue;
                    };
                    if let Some(c) = r.as_content_mut() {
                        c.set_pikchr_content(content);
                    };
                },
                Msg::RequestRedraw(ctx, id) => {
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
                            false,
                        ) {
                            *reference.image = Some(im);
                            *reference.diagram_texture = Some(te);
                        }
                        let mut editor_deps = write_state.editor_deps.clone();
                        editor_deps.entry(id).or_default().iter().cloned().collect()
                    };
                    for dep_id in deps {
                        if dep_id == id {
                            continue;
                        }
                        local_queue.push_back(Msg::RequestRedraw(ctx.clone(), dep_id));
                    }

                    ctx.request_repaint();
                },
                Msg::UpdatePikchr(ctx, id) => {
                    // Logic for immediate updates
                    let (svg_maybe, svg_id) = {
                        let state_clone = state.clone();
                        let mut writable_state = state_clone.write();
                        let content = crate::replace_pikchr_content(&mut writable_state, id).clone();

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
                            if let Some(errorable) = writable_state
                                .windows
                                .write()
                                .get_mut(&id)
                                .and_then(|w| w.as_error_mut())
                            {
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
                                local_queue.push_back(Msg::RequestRedraw(ctx.clone(), svg_id));
                            } else {
                                local_queue.push_back(Msg::RecreateSvg(ctx.clone(), id))
                            }
                        },
                    }
                    for dep in writable_state
                        .editor_deps
                        .get(&id)
                        .unwrap_or(&HashSet::new())
                    {
                        local_queue.push_back(Msg::UpdatePikchr(ctx.clone(), *dep))
                    }

                    ctx.request_repaint();
                },
                Msg::RecreateSvg(ctx, id) => {
                    let svg_id = identifiers::next_global_id();
                    let svg_insert =
                        mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, id));
                    let state_write = state.write();
                    let mut windows = state_write.windows.write();
                    windows.insert(svg_id, svg_insert);

                    if let Some(targetable) = windows.get_mut(&id).and_then(|w| w.as_target_mut()) {
                        targetable.set_target(svg_id);
                    }
                    local_queue.push_back(Msg::UpdatePikchr(ctx, id));
                },
                Msg::UpdateProlog(ctx, id, content) => {
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
                            if let Some(errorable) =
                                windows.get_mut(&id).and_then(|w| w.as_error_mut())
                            {
                                errorable.set_error(Some(err.inner_string()));
                            }
                            ctx.request_repaint();
                        },
                        Ok(pikchr) => {
                            local_queue.push_back(Msg::Batch(vec![
                                Msg::ResetError(id),
                                Msg::UpdatePikchrContent(id, pikchr.into_inner()),
                                Msg::UpdatePikchr(ctx, id),
                            ]));
                        },
                    }
                },
                Msg::UpdateTcl(ctx, id, content) => {
                    // Logic for immediate updates
                    let content = crate::replace_raw_content(
                        &mut state.write(),
                        id,
                        &content
                    );

                    let pikchr_code = tcl::safe_eval_tcl(content).await;

                    match pikchr_code {
                        Err(err) => {
                            let mut state_write = state.write();
                            state_write.log.push(format!("{:?}", err));
                            let mut windows = state_write.windows.write();
                            if let Some(errorable) =
                                windows.get_mut(&id).and_then(|w| w.as_error_mut())
                            {
                                errorable.set_error(Some(err));
                            }
                            ctx.request_repaint();
                        },
                        Ok(pikchr) => {
                            local_queue.push_back(Msg::Batch(vec![
                                Msg::ResetError(id),
                                Msg::UpdatePikchrContent(id, pikchr.as_str().into()),
                                Msg::UpdatePikchr(ctx.clone(), id),
                            ]));
                        },
                    }

                    for dep in state.write()
                        .editor_deps
                        .get(&id)
                        .unwrap_or(&HashSet::new())
                    {
                        local_queue.push_back(Msg::UpdatePikchr(ctx.clone(), *dep))
                    }
                },
                Msg::ToggleWindow(crate::Window::Logger) => {
                    let mut state_write = state.write();
                    let current = state_write.window_states.log;
                    state_write.window_states.log = !current;
                },
                Msg::ToggleWindow(crate::Window::Debugger) => {
                    let mut state_write = state.write();
                    let current = state_write.window_states.debug;
                    state_write.window_states.debug = !current;
                },
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
                        create_editor_window!(
                            state,
                            PikchrEditor,
                            pikchr_editor::PikchrEditor::new
                        );
                    },
                    crate::mini_window::WindowType::PrologEditor => {
                        create_editor_window!(
                            state,
                            PrologEditor,
                            prolog_editor::PrologEditor::new
                        );
                    },
                    crate::mini_window::WindowType::TclEditor => {
                        create_editor_window!(state, TclEditor, tcl_editor::TclEditor::new);
                    },
                    crate::mini_window::WindowType::SvgWindow => (),
                },
                Msg::ExportModal(id, name, export_type) => {
                    push_modal!(state, ExportModal::new(id, name, export_type));
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
                Msg::Export(svg_id, file, crate::ExportType::PngTransparent) => {
                    let state = state.read();
                    let mut windows = state.windows.write();
                    let Some(svg_string) = windows
                        .get_mut(&svg_id)
                        .and_then(|w| w.as_svg_window())
                        .and_then(|s| s.svg_string.clone())
                    else {
                        continue;
                    };
                    let Some(image) = crate::image::render_svg_to_image(&svg_string, 2.0, true) else { continue };
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
                Msg::ExportPikchrToClipboard(ctx, id) => {
                    let pc = crate::replace_pikchr_content(
                        &mut state.write(),
                        id
                    );
                    ctx.copy_text(pc);
                },
                Msg::PopModal => {
                    state.write().modals.pop_front();
                },
                Msg::ReloadSvgs(ctx) => {
                    for w in state
                        .write()
                        .windows
                        .write()
                        .values_mut()
                        .flat_map(|m| m.as_svg_window())
                    {
                        local_queue.push_back(Msg::RequestRedraw(ctx.clone(), *w.id))
                    }
                },
                Msg::ResetError(id) => {
                    if let Some(errorable) = state
                        .write()
                        .windows
                        .write()
                        .get_mut(&id)
                        .and_then(|w| w.as_error_mut())
                    {
                        errorable.set_error(None);
                    }
                },
                Msg::ResetWorkspace => {
                    state.write().windows = Arc::new(RwLock::new(HashMap::new()));
                    state.write().editor_deps = HashMap::new();
                    local_queue.push_back(Msg::PopModal);
                },
                Msg::ResetWorkspaceRequest => {
                    push_modal!(
                        state,
                        ConfirmationModal::new(Msg::ResetWorkspace, "Reset workspace?")
                    );
                },
                Msg::SaveWorkspace => {
                    let cloned: AppState = state.clone().read().clone();
                    let payload: Box<[u8]> =
                        serde_json::to_vec(&cloned).unwrap().into_boxed_slice();
                    push_modal!(
                        state,
                        FileSaveModal::new(payload, "json", "workspace", Some("Save Workspace"))
                    );
                },
                Msg::LoadWorkspaceRequest => {
                    let modal = FileOpenModal::new(
                        "Load Workspace",
                        "json",
                        Box::new(|path, _ctx, tx: Sender<Msg>| {
                            let _ = tx.try_send(Msg::LoadWorkspace(path));
                            Ok(())
                        }),
                    );
                    push_modal!(state, modal);
                },
                Msg::LoadWorkspace(path) => {
                    let path = std::path::Path::new(&path);
                    if !path.exists() {
                        continue;
                    }
                    let Ok(file) = std::fs::File::open(path) else {
                        eprintln!("Can't open file");
                        continue;
                    };
                    let reader = BufReader::new(file);
                    let Ok(new_state) = serde_json::from_reader::<_, AppState>(reader) else {
                        eprintln!("Can't deserialize state");
                        continue;
                    };
                    let mut current_state = state.write();
                    *current_state = new_state;
                },
                Msg::FontSizeModal(_id) => {
                    let value = String::from("abc");
                    let value = Box::leak(Box::new(value));
                    let modal = StringEditModal::new("VALUE", value);
                    push_modal!(state, modal);
                },
                Msg::RequestRename(id) => {
                    let current_name = state
                        .read()
                        .with_window(id, |window| window.as_name().map(|w| w.get_name()));
                    if let Some(Some(name)) = current_name {
                        let modal = RenameModal::new(id, &name);
                        push_modal!(state, modal);
                    }
                },
                Msg::RenameWindow(id, new_title) => {
                    let svg_id_wrapped = state
                        .read()
                        .with_window(id, |window| window.as_target().map(|t| t.get_target()));
                    if let Some(Some(svg_id)) = svg_id_wrapped {
                        local_queue.push_back(Msg::RenameWindow(svg_id, new_title.clone()));
                    }
                    state.write().with_window_mut(id, move |window| {
                        window.as_name_mut().map(move |wn| wn.set_name(new_title))
                    });
                },
            }
        }
    }
}

// kakexec: <percent>s(?S)Msg::.*=<gt>.*\{<ret>mH<a-semicolon>L<space>jf,
