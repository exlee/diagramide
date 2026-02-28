use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::BufReader,
    sync::Arc,
};

use eframe::egui;
use parking_lot::RwLock;
use slog::{Logger, Serde, debug, o};
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt as _;
use tokio_util::time::{DelayQueue, delay_queue::Key as DelayKey};

use crate::{
    AppState, Msg, SPACE_MONO_NAME, clean_old_deps, identifiers, mini_window,
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
        let mut state_write = $state.write();
        state_write.windows.insert(editor_id, editor_insert);
        state_write.windows.insert(svg_id, svg_insert);
    };
}
pub async fn handle(
    mut rx: tokio::sync::mpsc::Receiver<Msg>,
    logger: Logger,
    state: Arc<RwLock<AppState>>,
) {
    let mut local_queue: VecDeque<Msg> = VecDeque::new();
    let mut delay_queue: DelayQueue<(egui::Id, Msg)> = DelayQueue::new();
    let mut pending_debounces: HashMap<egui::Id, DelayKey> = HashMap::new();
    let mut cleanup_interval = tokio::time::interval(std::time::Duration::from_secs(30));
    let logger = logger.new(o!("category" => "event"));

    loop {
        tokio::select! {
            biased;
            _ = cleanup_interval.tick() => {
                local_queue.push_back(Msg::CheckDependencies);
            }

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
            let _ = handle_event(logger.clone(), msg, state.clone(), &mut local_queue).await;
        }
    }
    debug!(logger, "Handler exiting");
}

async fn handle_event(
    logger: Logger,
    msg: Msg,
    state: Arc<RwLock<AppState>>,
    local_queue: &mut VecDeque<Msg>,
) -> Option<()> {
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
                if let Some(targetable) = state.windows.get(&id).and_then(|w| w.as_target()) {
                    let target_id = targetable.get_target();
                    state.windows.remove(&target_id);
                }
                state.windows.remove(&id);
            }
            for dkey in dkeys {
                state.editor_deps.entry(dkey).and_modify(|e| {
                    e.remove(&id);
                });
            }
        },
        Msg::UpdateContent(id, _content) => {
            let mut state = state.write();
            let Some(r) = state.windows.get_mut(&id) else {
                return None;
            };
            if let Some(_c) = r.as_raw_content() {
                //c.set_pikchr_content(content);
            };
        },
        Msg::UpdatePikchrContent(id, content) => {
            let mut state = state.write();
            let Some(r) = state.windows.get_mut(&id) else {
                return None;
            };
            if let Some(c) = r.as_pikchr_content_mut() {
                c.set_pikchr_content(content);
            };
        },
        Msg::RequestRedraw(ctx, id) => {
            let deps: Vec<egui::Id> = {
                let mut write_state = state.write();
                let reference = match write_state
                    .windows
                    .get_mut(&id)
                    .and_then(|s| s.as_svg_window())
                {
                    Some(window) => window,
                    None => return None,
                };
                let Some(svg_string) = reference.svg_string.clone() else {
                    return None;
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
                    return None;
                }
                local_queue.push_back(Msg::RequestRedraw(ctx.clone(), dep_id));
            }

            ctx.request_repaint();
        },
        Msg::UpdatePikchr(ctx, id, content) => {
            // Logic for immediate updates
            let (svg_maybe, svg_id) = {
                let state_clone = state.clone();
                let mut writable_state = state_clone.write();
                let content =
                    crate::replace_pikchr_content(&mut writable_state, id, &content).clone();

                let windows_enum = &writable_state.windows;

                let window = windows_enum.get(&id);
                if window.is_none() {
                    return None;
                }
                let window = window.unwrap();
                let Some(svg_id) = window.as_target().map(|t| t.get_target()) else {
                    return None;
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
                        .get_mut(&id)
                        .and_then(|w| w.as_error_mut())
                    {
                        errorable.set_error(Some(err.inner_string()))
                    };
                    writable_state.log.push(format!("{:?}", err));
                },
                Ok(svg) => {
                    let svg_string = svg.inject_svg_style(SPACE_MONO_NAME).into_inner();
                    local_queue.push_back(Msg::ResetError(id));
                    if let Some(reference) = writable_state
                        .windows
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
                let content = writable_state
                    .windows
                    .get(&id)?
                    .as_pikchr_content()?
                    .get_pikchr_content();
                local_queue.push_back(Msg::UpdatePikchr(ctx.clone(), *dep, content))
            }

            ctx.request_repaint();
        },
        Msg::RecreateSvg(ctx, id) => {
            let svg_id = identifiers::next_global_id();
            let svg_insert = mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, id));
            let mut state_write = state.write();
            state_write.windows.insert(svg_id, svg_insert);

            let content = state_write
                .windows
                .get(&id)?
                .as_pikchr_content()?
                .get_pikchr_content();
            if let Some(targetable) = state_write
                .windows
                .get_mut(&id)
                .and_then(|w| w.as_target_mut())
            {
                targetable.set_target(svg_id);
            }
            local_queue.push_back(Msg::UpdatePikchr(ctx, id, content));
        },
        Msg::UpdateProlog(ctx, id, content) => {
            // Logic for immediate updates
            let pikchr_code =
                pikchr_pro::prolog::engine::trealla::EngineAsync::process_diagram(vec![content])
                    .await;

            match pikchr_code {
                Err(err) => {
                    let mut state_write = state.write();
                    state_write.log.push(format!("{:?}", err));
                    if let Some(errorable) = state_write
                        .windows
                        .get_mut(&id)
                        .and_then(|w| w.as_error_mut())
                    {
                        errorable.set_error(Some(err.inner_string()));
                    }
                    ctx.request_repaint();
                },
                Ok(pikchr) => {
                    local_queue.push_back(Msg::Batch(vec![
                        Msg::ResetError(id),
                        Msg::UpdatePikchrContent(id, pikchr.clone().into_inner()),
                        Msg::UpdatePikchr(ctx.clone(), id, pikchr.into_inner()),
                    ]));
                },
            }
            for dep in state
                .write()
                .editor_deps
                .get(&id)
                .unwrap_or(&HashSet::new())
            {
                local_queue.push_back(Msg::Refresh(ctx.clone(), *dep))
            }
        },
        Msg::UpdateTcl(ctx, id, content) => {
            // Logic for immediate updates
            let content = crate::replace_content(&mut state.write(), id, &content);

            let pikchr_code = tcl::safe_eval_tcl(content).await;

            match pikchr_code {
                Err(err) => {
                    let mut state_write = state.write();
                    state_write.log.push(format!("{:?}", err));
                    if let Some(errorable) = state_write
                        .windows
                        .get_mut(&id)
                        .and_then(|w| w.as_error_mut())
                    {
                        errorable.set_error(Some(err));
                    }
                    ctx.request_repaint();
                },
                Ok(pikchr) => {
                    local_queue.push_back(Msg::Batch(vec![
                        Msg::ResetError(id),
                        Msg::UpdatePikchrContent(id, pikchr.as_str().into()),
                        Msg::UpdatePikchr(ctx.clone(), id, pikchr),
                    ]));
                },
            }

            let deps = state.read().editor_deps.clone();
            debug!(logger, "Dependency handling"; "id" => id.short_debug_format(),
                "deps" => Serde(deps));
            for dep in state.read().editor_deps.get(&id).unwrap_or(&HashSet::new()) {
                local_queue.push_back(Msg::Refresh(ctx.clone(), *dep))
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
                .get_mut(&id)
                .and_then(|w| w.as_mini_window_mut())
            {
                window.toggle_visible();
            }
        },
        Msg::NewWindow(window_type) => match window_type {
            crate::mini_window::WindowType::PikchrEditor => {
                create_editor_window!(state, PikchrEditor, pikchr_editor::PikchrEditor::new);
            },
            crate::mini_window::WindowType::PrologEditor => {
                create_editor_window!(state, PrologEditor, prolog_editor::PrologEditor::new);
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
            let mut state = state.write();
            let Some(image) = state
                .windows
                .get_mut(&svg_id)
                .and_then(|w| w.as_svg_window())
                .and_then(|s| s.image.clone())
            else {
                return None;
            };
            let _ = crate::image::write_png(file, image);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::Export(svg_id, file, crate::ExportType::PngTransparent) => {
            let mut state = state.write();
            let Some(svg_string) = state
                .windows
                .get_mut(&svg_id)
                .and_then(|w| w.as_svg_window())
                .and_then(|s| s.svg_string.clone())
            else {
                return None;
            };
            let Some(image) = crate::image::render_svg_to_image(&svg_string, 2.0, true) else {
                return None;
            };
            let _ = crate::image::write_png(file, image);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::Export(svg_id, file, crate::ExportType::Svg) => {
            let mut state = state.write();
            let Some(svg) = state
                .windows
                .get_mut(&svg_id)
                .and_then(|w| w.as_svg_window())
                .and_then(|s| s.svg_string.clone())
            else {
                return None;
            };
            let _ = crate::image::write_svg(file, svg);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::ExportPikchrToClipboard(ctx, id) => {
            let content = state
                .read()
                .windows
                .get(&id)?
                .as_pikchr_content()?
                .get_pikchr_content();

            let pc = crate::replace_pikchr_content(&mut state.write(), id, &content);
            ctx.copy_text(pc);
        },
        Msg::PopModal => {
            state.write().modals.pop_front();
        },
        Msg::ReloadSvgs(ctx) => {
            for w in state
                .write()
                .windows
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
                .get_mut(&id)
                .and_then(|w| w.as_error_mut())
            {
                errorable.set_error(None);
            }
        },
        Msg::ResetWorkspace => {
            state.write().windows = HashMap::new();
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
            let payload: Box<[u8]> = serde_json::to_vec(&cloned).unwrap().into_boxed_slice();
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
                return None;
            }
            let Ok(file) = std::fs::File::open(path) else {
                eprintln!("Can't open file");
                return None;
            };
            let reader = BufReader::new(file);
            let Ok(new_state) = serde_json::from_reader::<_, AppState>(reader) else {
                eprintln!("Can't deserialize state");
                return None;
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
            let state_r = state.read();
            let name = state_r.windows.get(&id)?.as_name()?.get_name();
            let modal = RenameModal::new(id, &name);
            push_modal!(state, modal);
        },
        Msg::RenameWindow(id, new_title) => {
            let state_r = state.read();
            let svg_id = state_r.windows.get(&id)?.as_target()?.get_target();
            local_queue.push_back(Msg::RenameWindow(svg_id, new_title.clone()));
            let mut state = state.write();
            let as_name = state.windows.get_mut(&id)?.as_name_mut()?;
            as_name.set_name(new_title);
        },
        Msg::Refresh(ctx, id) => {
            let state = state.read();
            let window = state.windows.get(&id)?;
            let et = window.as_editor_type()?.get_editor_type();

            let content = window.as_raw_content()?.get_raw_content();

            local_queue.push_back(match et {
                crate::EditorType::Prolog => Msg::UpdateProlog(ctx, id, content),
                crate::EditorType::Pikchr => Msg::UpdatePikchr(ctx, id, content),
                crate::EditorType::Tcl => Msg::UpdateTcl(ctx, id, content),
            });
        },
        Msg::CheckDependencies => {
            clean_old_deps(&mut state.write());
        },
    };
    Some(())
}

// kakexec: <percent>s(?S)Msg::.*=<gt>.*\{<ret>mH<a-semicolon>L<space>jf,
