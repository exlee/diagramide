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
use tracing::Instrument as _;

use crate::{
    AppState,
    Msg,
    SPACE_MONO_NAME,
    clean_old_deps,
    identifiers,
    mini_window,
    modal::{
        ConfirmationModal,
        ExportModal,
        FileOpenModal,
        FileSaveModal,
        RenameModal,
        StringEditModal,
        WorkspaceNameModal,
    },
    mruby,
    mruby_editor,
    pikchr_editor,
    plain_text_editor,
    prolog_editor,
    state::Workspace,
    state_serialize::AppStatePersistent,
    svg,
    tcl,
    tcl_editor,
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
macro_rules! create_plain_text_window {
    ($state: ident) => {
        let editor_id = identifiers::next_global_id();
        let editor_insert = mini_window::Window::PlainTextEditor(
            plain_text_editor::PlainTextEditor::new(editor_id),
        );
        $state.write().windows.insert(editor_id, editor_insert);
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
                            delay_queue.remove(delay_key);
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
            #[cfg(feature = "profile")]
            {
                tracing::info!(
                    tracy.plot = "Event Local Queue Size",
                    value = local_queue.len() as f64
                );
            }
            let span = tracing::info_span!("handle_event", msg = ?msg);
            let _ = handle_event(logger.clone(), msg, state.clone(), &mut local_queue)
                .instrument(span)
                .await;
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
        Msg::ShowHelp(topic) => {
            state.write().help_topic = Some(topic);
        },
        Msg::HideHelp => {
            state.write().help_topic = None;
        },
        Msg::SelectTheme(ctx, id) => {
            if crate::theme::set_active(&id, &ctx) {
                state.write().active_theme = id;
                local_queue.push_back(Msg::ReloadSvgs(ctx));
            } else {
                state.write().log.push(format!("Theme not found: {id}"));
            }
        },
        Msg::ReloadThemes(ctx) => {
            let errors = crate::theme::reload(&ctx);
            let active = crate::theme::active_id();
            let mut state = state.write();
            state.active_theme = active;
            state.log.extend(errors);
            drop(state);
            local_queue.push_back(Msg::ReloadSvgs(ctx));
        },
        Msg::OpenThemesFolder => {
            if let Err(err) = crate::theme::open_themes_dir() {
                state.write().log.push(err);
            }
        },
        Msg::SetDiagramBackground(ctx, background) => {
            state.write().diagram_background = background;
            local_queue.push_back(Msg::ReloadSvgs(ctx));
        },
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
            let r = state.windows.get_mut(&id)?;
            if let Some(_c) = r.as_raw_content() {
                //c.set_pikchr_content(content);
            };
        },
        Msg::UpdatePikchrContent(id, content) => {
            let mut state = state.write();
            let r = state.windows.get_mut(&id)?;
            if let Some(c) = r.as_pikchr_content_mut() {
                c.set_pikchr_content(content);
            };
        },
        Msg::RequestRedraw(ctx, id) => {
            let mut state_w = state.try_write()?;
            let background = state_w.diagram_background;
            let (svg_string, scale) = {
                let reference = state_w.windows.get_mut(&id)?.as_svg_window()?;
                let svg_string = reference.svg_string.clone()?;
                let scale = *reference.scale;

                (svg_string, scale)
            };
            let background = background.resolve(&ctx.style().visuals);
            let image = crate::image::render_svg_to_image(
                &svg_string,
                scale,
                crate::image::RenderBackground::Color(background),
            )?;

            {
                let texture =
                    ctx.load_texture("pikchr_diagram", image, egui::TextureOptions::LINEAR);
                let window = state_w.windows.get_mut(&id)?.as_svg_window()?;
                *window.diagram_texture = Some(texture);
            }

            {
                let deps: Vec<egui::Id> = {
                    {
                        let mut editor_deps = state_w.editor_deps.clone();
                        editor_deps.entry(id).or_default().iter().cloned().collect()
                    }
                };
                for dep_id in deps {
                    if dep_id == id {
                        return None;
                    }
                    local_queue.push_back(Msg::RequestRedraw(ctx.clone(), dep_id));
                }
            }

            ctx.request_repaint();
        },
        Msg::UpdatePikchr(ctx, id, content) => {
            // Logic for immediate updates
            let (svg_maybe, svg_id) = {
                let state_clone = state.clone();
                let mut writable_state = state_clone.write();
                let content = crate::replace_content(&mut writable_state, id, &content);

                let windows_enum = &writable_state.windows;

                let window = windows_enum.get(&id)?;
                let svg_id = window.as_target().map(|t| t.get_target())?;
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
            let content = crate::replace_content(&mut state.write(), id, &content);
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
        Msg::UpdateMruby(ctx, id, content) => {
            let content = crate::replace_content(&mut state.write(), id, &content);
            let pikchr_code = mruby::safe_eval_mruby(content).await;

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
                        Msg::UpdatePikchrContent(id, pikchr.clone()),
                        Msg::UpdatePikchr(ctx.clone(), id, pikchr),
                    ]));
                },
            }

            for dep in state.read().editor_deps.get(&id).unwrap_or(&HashSet::new()) {
                local_queue.push_back(Msg::Refresh(ctx.clone(), *dep))
            }
        },
        Msg::UpdatePlainText(ctx, id) => {
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
            crate::mini_window::WindowType::MrubyEditor => {
                create_editor_window!(state, MrubyEditor, mruby_editor::MrubyEditor::new);
            },
            crate::mini_window::WindowType::PlainTextEditor => {
                create_plain_text_window!(state);
            },
            crate::mini_window::WindowType::SvgWindow => (),
        },
        Msg::ExportModal(id, name, export_type) => {
            push_modal!(state, ExportModal::new(id, name, export_type));
        },
        Msg::Export(svg_id, file, crate::ExportType::Png, visuals) => {
            let background = state.read().diagram_background.resolve_for_export(&visuals);
            let mut state = state.write();
            let svg_string = state
                .windows
                .get_mut(&svg_id)
                .and_then(|w| w.as_svg_window())
                .and_then(|s| s.svg_string.clone())?;
            let image = crate::image::render_svg_to_image(
                &svg_string,
                2.0,
                background,
            )?;
            let _ = crate::image::write_png(file, image);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::Export(svg_id, file, crate::ExportType::PngTransparent, _visuals) => {
            let mut state = state.write();
            let svg_string = state
                .windows
                .get_mut(&svg_id)
                .and_then(|w| w.as_svg_window())
                .and_then(|s| s.svg_string.clone())?;
            let image = crate::image::render_svg_to_image(
                &svg_string,
                2.0,
                crate::image::RenderBackground::Transparent,
            )?;
            let _ = crate::image::write_png(file, image);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::Export(svg_id, file, crate::ExportType::Svg, _visuals) => {
            let mut state = state.write();
            let svg = state
                .windows
                .get_mut(&svg_id)
                .and_then(|w| w.as_svg_window())
                .and_then(|s| s.svg_string.clone())?;
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
            // Clears the *active* workspace only; dormant ones are untouched.
            let mut state = state.write();
            state.windows = HashMap::new();
            state.editor_deps = HashMap::new();
            // keep the dormant registry entry in sync
            state.flush_active();
            drop(state);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::ResetWorkspaceRequest => {
            push_modal!(
                state,
                ConfirmationModal::new(Msg::ResetWorkspace, "Reset active workspace?")
            );
        },
        Msg::SaveWorkspace => {
            let cloned: AppState = state.clone().read().clone();
            // Export only the *active* workspace as a single-workspace file
            // (backward compatible with pre-workspace save format).
            let persisted = AppStatePersistent::from(cloned);
            let export = AppStatePersistent {
                workspaces: HashMap::new(),
                active_workspace_id: 0,
                active_workspace_name: persisted.active_workspace_name.clone(),
                ..persisted
            };
            let payload: Box<[u8]> = serde_json::to_vec(&export).unwrap().into_boxed_slice();
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
            let Ok(imported) = serde_json::from_reader::<_, AppState>(reader) else {
                eprintln!("Can't deserialize state");
                return None;
            };
            // Import the file's active workspace as a brand-new workspace
            // (fresh id ⇒ dormant ids never collide) and switch to it.
            let mut current = state.write();
            let new_id = current.new_workspace(imported.active_workspace_name.clone());
            if let Some(ws) = current.workspaces.get_mut(&new_id) {
                ws.windows = imported.windows;
                ws.editor_deps = imported.editor_deps;
            }
            current.switch_to(new_id);
            drop(current);
            local_queue.push_back(Msg::PopModal);
        },

        // ── Multiple workspaces ─────────────────────────────────────────
        Msg::SwitchWorkspace(id) => {
            state.write().switch_to(id);
            // SVG textures are refreshed by the UI loop when it notices the
            // active workspace id has changed (see DiagramIDE::ui).
        },
        Msg::NewWorkspaceRequest => {
            push_modal!(state, WorkspaceNameModal::new(None, ""));
        },
        Msg::NewWorkspace(name) => {
            let mut state = state.write();
            let id = state.new_workspace(name);
            state.switch_to(id);
        },
        Msg::RenameWorkspaceRequest(id) => {
            let initial = state
                .read()
                .workspace_listing()
                .into_iter()
                .find(|(wid, _, _)| wid == &id)
                .map(|(_, name, _)| name)
                .unwrap_or_default();
            push_modal!(state, WorkspaceNameModal::new(Some(id), &initial));
        },
        Msg::RenameWorkspace(id, name) => {
            state.write().rename_workspace(id, name);
        },
        Msg::DuplicateWorkspace(id) => {
            let mut state = state.write();
            if id == state.active_workspace_id {
                state.duplicate_active();
            } else {
                // duplicate a dormant workspace in place
                if let Some(src) = state.workspaces.get(&id).cloned() {
                    let new_id = identifiers::next_workspace_id();
                    state.workspaces.insert(
                        new_id,
                        Workspace {
                            id: new_id,
                            name: format!("{} (copy)", src.name),
                            windows: src.windows,
                            editor_deps: src.editor_deps,
                        },
                    );
                }
            }
        },
        Msg::DeleteWorkspaceRequest(id) => {
            push_modal!(
                state,
                ConfirmationModal::new(Msg::DeleteWorkspace(id), "Delete workspace?")
            );
        },
        Msg::DeleteWorkspace(id) => {
            state.write().delete_workspace(id);
            local_queue.push_back(Msg::PopModal);
        },
        Msg::FontSizeModal(_id) => {
            let value = String::from("abc");
            let value = Box::leak(Box::new(value));
            let modal = StringEditModal::new("VALUE", value);
            push_modal!(state, modal);
        },
        Msg::RequestRename(ctx, id) => {
            let name = {
                let state_r = state.read();
                state_r.windows.get(&id)?.as_name()?.get_name()
            };
            let modal = RenameModal::new(id, &name);
            push_modal!(state, modal);
            ctx.request_repaint();
        },
        Msg::RenameWindow(id, new_title) => {
            let target = {
                let state_r = state.read();
                state_r
                    .windows
                    .get(&id)?
                    .as_target()
                    .map(|target| target.get_target())
            };
            if let Some(svg_id) = target {
                local_queue.push_back(Msg::RenameWindow(svg_id, new_title.clone()));
            }
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
                crate::EditorType::Mruby => Msg::UpdateMruby(ctx, id, content),
                crate::EditorType::PlainText => Msg::UpdatePlainText(ctx, id),
            });
        },
        Msg::CheckDependencies => {
            clean_old_deps(&mut state.write());
        },
    };
    Some(())
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use parking_lot::RwLock;

    use super::*;

    #[tokio::test]
    async fn rename_request_queues_modal_repaints_and_can_be_confirmed() {
        let id = egui::Id::new("editor");
        let ctx = egui::Context::default();
        let repaint_requested = Arc::new(AtomicBool::new(false));
        let repaint_requested_clone = repaint_requested.clone();
        ctx.set_request_repaint_callback(move |_| {
            repaint_requested_clone.store(true, Ordering::SeqCst);
        });

        let state = Arc::new(RwLock::new(AppState::default()));
        state.write().windows.insert(
            id,
            mini_window::Window::PlainTextEditor(plain_text_editor::PlainTextEditor::new(id)),
        );

        let mut local_queue = VecDeque::new();
        handle_event(
            crate::logger::init_logger(),
            Msg::RequestRename(ctx, id),
            state.clone(),
            &mut local_queue,
        )
        .await;

        assert_eq!(state.read().modals.len(), 1);
        assert!(repaint_requested.load(Ordering::SeqCst));

        handle_event(
            crate::logger::init_logger(),
            Msg::RenameWindow(id, "renamed".into()),
            state.clone(),
            &mut local_queue,
        )
        .await;

        let name = state
            .read()
            .windows
            .get(&id)
            .and_then(|window| window.as_name())
            .map(|window| window.get_name());
        assert_eq!(name.as_deref(), Some("renamed"));
    }
}

// kakexec: <percent>s(?S)Msg::.*=<gt>.*\{<ret>mH<a-semicolon>L<space>jf,
