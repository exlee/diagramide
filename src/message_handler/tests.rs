use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use parking_lot::RwLock;

use super::*;

fn editor_matches(window: &mini_window::Window, editor_type: crate::EditorType) -> bool {
    matches!(
        (window, editor_type),
        (
            mini_window::Window::PikchrEditor(_),
            crate::EditorType::Pikchr
        ) | (
            mini_window::Window::SvgbobEditor(_),
            crate::EditorType::Svgbob
        ) | (
            mini_window::Window::PrologEditor(_),
            crate::EditorType::Prolog
        ) | (mini_window::Window::TclEditor(_), crate::EditorType::Tcl)
            | (
                mini_window::Window::ClipsEditor(_),
                crate::EditorType::Clips
            )
            | (
                mini_window::Window::MrubyEditor(_),
                crate::EditorType::Mruby
            )
            | (
                mini_window::Window::HagoromoEditor(_),
                crate::EditorType::Hagoromo
            )
            | (
                mini_window::Window::PlainTextEditor(_),
                crate::EditorType::PlainText
            )
    )
}

#[test]
fn texture_install_waits_for_transient_state_contention() {
    let id = egui::Id::new("svg");
    let owner_id = egui::Id::new("owner");
    let state = Arc::new(RwLock::new(AppState::default()));
    state.write().windows.insert(
        id,
        mini_window::Window::SvgWindow(svg::SvgWindow::new(id, owner_id)),
    );
    let ctx = egui::Context::default();
    let texture = ctx.load_texture(
        "contention-test",
        egui::ColorImage::new([1, 1], vec![egui::Color32::WHITE]),
        egui::TextureOptions::LINEAR,
    );

    let state_guard = state.write();
    let state_for_thread = state.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let installer = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let installed = install_diagram_texture(&state_for_thread, id, texture).is_some();
        done_tx.send(installed).unwrap();
    });

    started_rx.recv().unwrap();
    assert!(
        done_rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err(),
        "texture installation should wait instead of dropping the redraw"
    );
    drop(state_guard);

    assert!(
        done_rx
            .recv_timeout(std::time::Duration::from_secs(1))
            .unwrap()
    );
    installer.join().unwrap();
    assert!(matches!(
        state.read().windows.get(&id),
        Some(mini_window::Window::SvgWindow(window))
            if window.diagram_texture.is_some()
    ));
}

#[tokio::test]
async fn refresh_workspace_queues_refresh_for_each_editor() {
    let pikchr_id = egui::Id::new("pikchr");
    let plain_id = egui::Id::new("plain");
    let svg_id = egui::Id::new("svg");
    let ctx = egui::Context::default();
    let state = Arc::new(RwLock::new(AppState::default()));
    {
        let mut state = state.write();
        state.windows.insert(
            pikchr_id,
            mini_window::Window::PikchrEditor(pikchr_editor::PikchrEditor::new(pikchr_id, svg_id)),
        );
        state.windows.insert(
            plain_id,
            mini_window::Window::PlainTextEditor(plain_text_editor::PlainTextEditor::new(plain_id)),
        );
        state.windows.insert(
            svg_id,
            mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, pikchr_id)),
        );
    }

    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::RefreshWorkspace(ctx),
        state,
        &mut local_queue,
    )
    .await;

    let refreshed: HashSet<egui::Id> = local_queue
        .into_iter()
        .filter_map(|msg| match msg {
            Msg::Refresh(_, id) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(refreshed, HashSet::from([pikchr_id, plain_id]));
}

#[tokio::test]
async fn render_toggle_is_owned_by_editor() {
    let editor_id = egui::Id::new("pikchr");
    let svg_id = egui::Id::new("svg");
    let ctx = egui::Context::default();
    let state = Arc::new(RwLock::new(AppState::default()));
    state.write().windows.insert(
        editor_id,
        mini_window::Window::PikchrEditor(pikchr_editor::PikchrEditor::new(editor_id, svg_id)),
    );
    let mut local_queue = VecDeque::new();

    handle_event(
        crate::logger::init_logger(),
        Msg::SetRenderEnabled(ctx.clone(), editor_id, false),
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert!(
        !state
            .read()
            .windows
            .get(&editor_id)
            .unwrap()
            .as_render_toggle()
            .unwrap()
            .render_enabled()
    );

    handle_event(
        crate::logger::init_logger(),
        Msg::SetRenderEnabled(ctx, editor_id, true),
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert!(
        state
            .read()
            .windows
            .get(&editor_id)
            .unwrap()
            .as_render_toggle()
            .unwrap()
            .render_enabled()
    );
}

#[tokio::test]
async fn creating_mruby_editor_queues_initial_refresh() {
    let ctx = egui::Context::default();
    let state = Arc::new(RwLock::new(AppState::default()));
    let mut local_queue = VecDeque::new();

    handle_event(
        crate::logger::init_logger(),
        Msg::NewWindow(ctx, crate::mini_window::WindowType::MrubyEditor),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let editor_id = state
        .read()
        .windows
        .iter()
        .find_map(|(id, window)| {
            matches!(window, mini_window::Window::MrubyEditor(_)).then_some(*id)
        })
        .expect("mruby editor should be created");

    assert!(
        local_queue
            .into_iter()
            .any(|msg| { matches!(msg, Msg::Refresh(_, id) if id == editor_id) })
    );
}

#[tokio::test]
async fn creating_svgbob_editor_queues_initial_refresh() {
    let ctx = egui::Context::default();
    let state = Arc::new(RwLock::new(AppState::default()));
    let mut local_queue = VecDeque::new();

    handle_event(
        crate::logger::init_logger(),
        Msg::NewWindow(ctx, crate::mini_window::WindowType::SvgbobEditor),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let editor_id = state
        .read()
        .windows
        .iter()
        .find_map(|(id, window)| {
            matches!(window, mini_window::Window::SvgbobEditor(_)).then_some(*id)
        })
        .expect("svgbob editor should be created");

    assert!(
        local_queue
            .iter()
            .any(|msg| { matches!(msg, Msg::Refresh(_, id) if *id == editor_id) })
    );
}

#[tokio::test]
async fn changing_svgbob_mode_persists_on_the_dedicated_editor() {
    let editor_id = egui::Id::new("svgbob");
    let svg_id = egui::Id::new("svg");
    let state = Arc::new(RwLock::new(AppState::default()));
    state.write().windows.insert(
        editor_id,
        mini_window::Window::SvgbobEditor(svgbob_editor::SvgbobEditor::new(editor_id, svg_id)),
    );

    handle_event(
        crate::logger::init_logger(),
        Msg::SetSvgbobEditMode(editor_id, crate::SvgbobEditMode::Replace),
        state.clone(),
        &mut VecDeque::new(),
    )
    .await;

    assert!(matches!(
        state.read().windows.get(&editor_id),
        Some(mini_window::Window::SvgbobEditor(editor))
            if editor.edit_mode() == crate::SvgbobEditMode::Replace
    ));
}

#[tokio::test]
async fn changing_output_type_updates_editor_preview_and_refreshes() {
    let editor_id = egui::Id::new("editor");
    let svg_id = egui::Id::new("svg");
    let ctx = egui::Context::default();
    let state = Arc::new(RwLock::new(AppState::default()));
    {
        let mut state = state.write();
        state.windows.insert(
            editor_id,
            mini_window::Window::PikchrEditor(pikchr_editor::PikchrEditor::new(editor_id, svg_id)),
        );
        state.windows.insert(
            svg_id,
            mini_window::Window::SvgWindow(svg::SvgWindow::new(svg_id, editor_id)),
        );
    }

    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::SetOutputType(ctx, editor_id, crate::OutputType::Svgbob),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let state_read = state.read();
    assert_eq!(
        state_read
            .windows
            .get(&editor_id)
            .and_then(|window| window.as_render_toggle())
            .map(|render| render.output_type()),
        Some(crate::OutputType::Svgbob)
    );
    assert!(matches!(
        state_read.windows.get(&svg_id),
        Some(mini_window::Window::SvgWindow(svg)) if svg.source_format == crate::SourceFormat::Svgbob
    ));
    assert!(
        local_queue
            .iter()
            .any(|msg| matches!(msg, Msg::Refresh(_, id) if *id == editor_id))
    );
}

#[tokio::test]
async fn updating_pikchr_dependency_refreshes_dependent_from_its_own_content() {
    let source_id = egui::Id::new("source");
    let source_svg_id = egui::Id::new("source-svg");
    let dependent_id = egui::Id::new("dependent");
    let dependent_svg_id = egui::Id::new("dependent-svg");
    let ctx = egui::Context::default();
    let state = Arc::new(RwLock::new(AppState::default()));
    {
        let mut state = state.write();
        state.windows.insert(
            source_id,
            mini_window::Window::PikchrEditor(pikchr_editor::PikchrEditor::new(
                source_id,
                source_svg_id,
            )),
        );
        state.windows.insert(
            source_svg_id,
            mini_window::Window::SvgWindow(svg::SvgWindow::new(source_svg_id, source_id)),
        );
        state.windows.insert(
            dependent_id,
            mini_window::Window::PikchrEditor(pikchr_editor::PikchrEditor::new(
                dependent_id,
                dependent_svg_id,
            )),
        );
        state.windows.insert(
            dependent_svg_id,
            mini_window::Window::SvgWindow(svg::SvgWindow::new(dependent_svg_id, dependent_id)),
        );
        state
            .editor_deps
            .entry(source_id)
            .or_default()
            .insert(dependent_id);
    }

    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateRender(ctx, source_id, "box".into()),
        state,
        &mut local_queue,
    )
    .await;

    assert!(
        local_queue
            .iter()
            .any(|msg| matches!(msg, Msg::Refresh(_, id) if *id == dependent_id))
    );
    assert!(
            !local_queue
                .iter()
                .any(|msg| matches!(msg, Msg::UpdateRender(_, id, content) if *id == dependent_id && content == "box"))
        );
}

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

#[tokio::test]
async fn saving_existing_library_path_requires_overwrite_confirmation() {
    let id = egui::Id::new("editor");
    let state = Arc::new(RwLock::new(AppState::default()));
    state.write().windows.insert(
        id,
        mini_window::Window::PlainTextEditor(plain_text_editor::PlainTextEditor::new(id)),
    );

    {
        let mut state = state.write();
        let content = state
            .windows
            .get_mut(&id)
            .and_then(|window| window.as_raw_content_mut())
            .expect("plain text has raw content");
        content.set_raw_content("first".into());
    }

    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::SaveEditorToLibrary {
            editor_id: id,
            path: "samples/plain".into(),
            overwrite: false,
        },
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert_eq!(state.read().library["samples/plain"].content, "first");

    {
        let mut state = state.write();
        let content = state
            .windows
            .get_mut(&id)
            .and_then(|window| window.as_raw_content_mut())
            .expect("plain text has raw content");
        content.set_raw_content("second".into());
    }

    handle_event(
        crate::logger::init_logger(),
        Msg::SaveEditorToLibrary {
            editor_id: id,
            path: "samples/plain".into(),
            overwrite: false,
        },
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert_eq!(state.read().library["samples/plain"].content, "first");
    assert_eq!(state.read().modals.len(), 1);

    handle_event(
        crate::logger::init_logger(),
        Msg::SaveEditorToLibrary {
            editor_id: id,
            path: "samples/plain".into(),
            overwrite: true,
        },
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert_eq!(state.read().library["samples/plain"].content, "second");
    assert_eq!(
        state
            .read()
            .window_library_paths
            .get(&id)
            .map(String::as_str),
        Some("samples/plain")
    );
}

#[tokio::test]
async fn opening_library_entries_creates_matching_editors() {
    let ctx = egui::Context::default();
    for editor_type in [
        crate::EditorType::Pikchr,
        crate::EditorType::Svgbob,
        crate::EditorType::Prolog,
        crate::EditorType::Tcl,
        crate::EditorType::Mruby,
        crate::EditorType::Hagoromo,
        crate::EditorType::PlainText,
    ] {
        let state = Arc::new(RwLock::new(AppState::default()));
        let output_type = if matches!(
            editor_type,
            crate::EditorType::Pikchr | crate::EditorType::Svgbob
        ) {
            crate::OutputType::Svgbob
        } else {
            crate::OutputType::Pikchr
        };
        let entry = LibraryEntry {
            path: format!("folder/{editor_type:?}"),
            editor_type,
            output_type,
            content: format!("content for {editor_type:?}"),
        };
        state
            .write()
            .library
            .insert(entry.path.clone(), entry.clone());

        let mut local_queue = VecDeque::new();
        handle_event(
            crate::logger::init_logger(),
            Msg::OpenLibraryEntry(ctx.clone(), entry.path.clone()),
            state.clone(),
            &mut local_queue,
        )
        .await;

        let state_read = state.read();
        let (id, window) = state_read
            .windows
            .iter()
            .find(|(_, window)| editor_matches(window, editor_type))
            .expect("matching editor should be created");
        assert_eq!(
            window
                .as_raw_content()
                .map(|content| content.get_raw_content()),
            Some(entry.content.clone())
        );
        assert_eq!(
            state_read.window_library_paths.get(id).map(String::as_str),
            Some(entry.path.as_str())
        );
        if editor_type != crate::EditorType::PlainText {
            assert_eq!(
                window.as_render_toggle().map(|render| render.output_type()),
                Some(if editor_type == crate::EditorType::Svgbob {
                    crate::OutputType::Svgbob
                } else {
                    output_type
                })
            );
        }
        assert!(
            local_queue
                .iter()
                .any(|msg| matches!(msg, Msg::Refresh(_, refresh_id) if refresh_id == id))
        );
    }
}

async fn create_hagoromo_editor(state: &Arc<RwLock<AppState>>) -> (egui::Id, egui::Id) {
    let ctx = egui::Context::default();
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::NewWindow(ctx, crate::mini_window::WindowType::HagoromoEditor),
        state.clone(),
        &mut local_queue,
    )
    .await;
    let state_read = state.read();
    let (editor_id, editor) = state_read
        .windows
        .iter()
        .filter(|(_, window)| matches!(window, mini_window::Window::HagoromoEditor(_)))
        .find(|(editor_id, _)| {
            local_queue
                .iter()
                .any(|msg| matches!(msg, Msg::Refresh(_, id) if id == *editor_id))
        })
        .expect("new hagoromo editor must be created and queue its first render");
    let svg_id = editor.as_target().unwrap().get_target();
    (*editor_id, svg_id)
}

#[tokio::test]
async fn hagoromo_editor_has_no_output_selector_and_exports_hagoromo_source() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, svg_id) = create_hagoromo_editor(&state).await;
    let state_read = state.read();
    let toggle = state_read.windows[&editor_id].as_render_toggle().unwrap();
    assert!(toggle.has_renderer());
    assert!(!toggle.has_output_selector());
    assert!(matches!(
        &state_read.windows[&svg_id],
        mini_window::Window::SvgWindow(svg) if svg.source_format == crate::SourceFormat::Hagoromo
    ));
}

#[tokio::test]
async fn refresh_routes_hagoromo_editor_to_its_own_update() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, _) = create_hagoromo_editor(&state).await;
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::Refresh(egui::Context::default(), editor_id),
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert!(
        local_queue
            .iter()
            .any(|msg| matches!(msg, Msg::UpdateHagoromo(_, id, _) if *id == editor_id))
    );
}

#[tokio::test]
async fn hagoromo_update_renders_into_the_paired_window() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, svg_id) = create_hagoromo_editor(&state).await;
    let mut local_queue = VecDeque::new();
    let script = "let { prim } = import! hagoromo\nprim.square 3.0".to_string();
    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateHagoromo(egui::Context::default(), editor_id, script.clone()),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let state_read = state.read();
    let svg = match &state_read.windows[&svg_id] {
        mini_window::Window::SvgWindow(svg) => svg.svg_string.clone().expect("svg stored"),
        other => panic!("expected render window, got {other:?}"),
    };
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<polygon"), "{svg}");
    assert!(
        local_queue
            .iter()
            .any(|msg| matches!(msg, Msg::UpdateGeneratedContent(id, content) if *id == editor_id && *content == script))
    );
    assert!(
        local_queue
            .iter()
            .any(|msg| matches!(msg, Msg::RequestRedraw(_, id) if *id == svg_id))
    );
}

#[tokio::test]
async fn hagoromo_update_reports_script_errors() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, _) = create_hagoromo_editor(&state).await;
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateHagoromo(
            egui::Context::default(),
            editor_id,
            "let { prim } = import! hagoromo\nprim.no_such_shape 1.0".to_string(),
        ),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let state_read = state.read();
    let error = state_read.windows[&editor_id]
        .as_error()
        .unwrap()
        .get_error()
        .expect("script error must be shown on the editor");
    assert!(error.contains("no_such_shape"), "{error}");
}

#[tokio::test]
async fn hagoromo_update_expands_raw_includes() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, svg_id) = create_hagoromo_editor(&state).await;
    let ctx = egui::Context::default();
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::NewWindow(ctx.clone(), crate::mini_window::WindowType::PlainTextEditor),
        state.clone(),
        &mut local_queue,
    )
    .await;
    {
        let mut state_write = state.write();
        let text_id = *state_write
            .windows
            .iter()
            .find(|(_, window)| matches!(window, mini_window::Window::PlainTextEditor(_)))
            .map(|(id, _)| id)
            .unwrap();
        let window = state_write.windows.get_mut(&text_id).unwrap();
        window.as_name_mut().unwrap().set_name("shapes".into());
        window
            .as_raw_content_mut()
            .unwrap()
            .set_raw_content("let shape = prim.circle 2.0".into());
    }

    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateHagoromo(
            ctx,
            editor_id,
            "let { prim } = import! hagoromo\n!!shapes!!\nshape".to_string(),
        ),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let state_read = state.read();
    assert!(matches!(
        &state_read.windows[&svg_id],
        mini_window::Window::SvgWindow(svg) if svg.svg_string.as_deref().is_some_and(|s| s.contains("<circle"))
    ));
}

#[tokio::test]
async fn hagoromo_update_binds_other_hagoromo_editors_as_references() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, svg_id) = create_hagoromo_editor(&state).await;
    let (source_id, _) = create_hagoromo_editor(&state).await;
    let ctx = egui::Context::default();
    let mut local_queue = VecDeque::new();
    {
        let mut state_write = state.write();
        let window = state_write.windows.get_mut(&source_id).unwrap();
        window.as_name_mut().unwrap().set_name("AABB".into());
        window.as_raw_content_mut().unwrap().set_raw_content(
            "let { prim } = import! hagoromo\nlet node t = prim.circle t\nnode 2.0".into(),
        );
    }

    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateHagoromo(
            ctx,
            editor_id,
            "let { prim, (|||) } = import! hagoromo\nlet node t = prim.square t\nnode 1.0 ||| !!AABB!!"
                .to_string(),
        ),
        state.clone(),
        &mut local_queue,
    )
    .await;

    let state_read = state.read();
    assert!(matches!(
        &state_read.windows[&svg_id],
        mini_window::Window::SvgWindow(svg) if svg.svg_string.as_deref().is_some_and(|s| s.contains("<circle") && s.contains("<polygon"))
    ));
    let generated = local_queue
        .iter()
        .find_map(|msg| match msg {
            Msg::UpdateGeneratedContent(id, content) if *id == editor_id => Some(content),
            _ => None,
        })
        .expect("expanded script must be stored as generated content");
    assert!(generated.contains("let ref_AABB = ("), "{generated}");
    assert!(generated.contains("||| ref_AABB"), "{generated}");
    assert!(
        state_read.editor_deps[&source_id].contains(&editor_id),
        "referencing editor must refresh when the source changes"
    );
}

async fn create_clips_editor(state: &Arc<RwLock<AppState>>) -> (egui::Id, egui::Id) {
    let ctx = egui::Context::default();
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::NewWindow(ctx, crate::mini_window::WindowType::ClipsEditor),
        state.clone(),
        &mut local_queue,
    )
    .await;
    let state_read = state.read();
    let (editor_id, editor) = state_read
        .windows
        .iter()
        .filter(|(_, window)| matches!(window, mini_window::Window::ClipsEditor(_)))
        .find(|(editor_id, _)| {
            local_queue
                .iter()
                .any(|msg| matches!(msg, Msg::Refresh(_, id) if id == *editor_id))
        })
        .expect("new CLIPS editor must be created and queue its first render");
    let svg_id = editor.as_target().unwrap().get_target();
    (*editor_id, svg_id)
}

#[tokio::test]
async fn clips_editor_renders_pikchr_without_an_output_selector() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, svg_id) = create_clips_editor(&state).await;
    let state_read = state.read();
    let toggle = state_read.windows[&editor_id].as_render_toggle().unwrap();
    assert!(toggle.has_renderer());
    assert!(!toggle.has_output_selector());
    assert_eq!(toggle.output_type(), crate::OutputType::Pikchr);
    assert!(matches!(
        &state_read.windows[&svg_id],
        mini_window::Window::SvgWindow(svg) if svg.source_format == crate::SourceFormat::Pikchr
    ));
}

#[tokio::test]
async fn refresh_routes_clips_editor_to_its_own_update() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, _) = create_clips_editor(&state).await;
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::Refresh(egui::Context::default(), editor_id),
        state.clone(),
        &mut local_queue,
    )
    .await;
    assert!(
        local_queue
            .iter()
            .any(|msg| matches!(msg, Msg::UpdateClips(_, id, _) if *id == editor_id))
    );
}

#[tokio::test]
async fn clips_update_queues_generated_pikchr_for_rendering() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, _) = create_clips_editor(&state).await;
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateClips(
            egui::Context::default(),
            editor_id,
            "(box (id b) (label \"Hi\"))".to_string(),
        ),
        state.clone(),
        &mut local_queue,
    )
    .await;
    let generated = local_queue.iter().find_map(|msg| match msg {
        Msg::Batch(batch) => batch.iter().find_map(|msg| match msg {
            Msg::UpdateGeneratedContent(id, content) if *id == editor_id => Some(content.clone()),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(generated.as_deref(), Some("B: box \"Hi\"\n"));
    assert!(local_queue.iter().any(|msg| matches!(
        msg,
        Msg::Batch(batch) if batch.iter().any(|msg| matches!(msg, Msg::UpdateRender(_, id, _) if *id == editor_id))
    )));
}

#[tokio::test]
async fn clips_update_reports_program_errors_on_the_editor() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, _) = create_clips_editor(&state).await;
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateClips(
            egui::Context::default(),
            editor_id,
            "(box (nosuch 1))".to_string(),
        ),
        state.clone(),
        &mut local_queue,
    )
    .await;
    let state_read = state.read();
    let error = state_read.windows[&editor_id]
        .as_error()
        .unwrap()
        .get_error()
        .expect("error recorded on the editor");
    assert!(error.contains("nosuch"), "{error}");
}

#[tokio::test]
async fn clips_editor_template_renders() {
    let state = Arc::new(RwLock::new(AppState::default()));
    let (editor_id, svg_id) = create_clips_editor(&state).await;
    let template = state.read().windows[&editor_id]
        .as_raw_content()
        .unwrap()
        .get_raw_content();
    let mut local_queue = VecDeque::new();
    handle_event(
        crate::logger::init_logger(),
        Msg::UpdateClips(egui::Context::default(), editor_id, template),
        state.clone(),
        &mut local_queue,
    )
    .await;
    let render = local_queue
        .iter()
        .find_map(|msg| match msg {
            Msg::Batch(batch) => batch.iter().find_map(|msg| match msg {
                Msg::UpdateRender(_, id, pikchr) if *id == editor_id => Some(pikchr.clone()),
                _ => None,
            }),
            _ => None,
        })
        .expect("template produced Pikchr");
    let svg = crate::render::render(crate::OutputType::Pikchr, &render)
        .unwrap_or_else(|error| panic!("{error}\n{render}"));
    assert!(svg.starts_with("<svg"));
    assert!(state.read().windows.contains_key(&svg_id));
}

#[test]
fn clips_sources_merge_an_editor_with_its_render_window() {
    use mini_window::{HasName as _, RawContent as _};
    let clips_id = egui::Id::new("clips");
    let other_id = egui::Id::new("other");
    let other_svg_id = egui::Id::new("other-svg");
    let mut state = AppState::default();
    state.windows.insert(
        clips_id,
        mini_window::Window::ClipsEditor(clips_editor::ClipsEditor::new(
            clips_id,
            egui::Id::new("clips-svg"),
        )),
    );
    let mut other = clips_editor::ClipsEditor::new(other_id, other_svg_id);
    other.set_raw_content("(box (id o))".to_string());
    let name = other.get_name();
    state
        .windows
        .insert(other_id, mini_window::Window::ClipsEditor(other));
    // The Render window is inserted after the editor and carries the same name.
    state.windows.insert(
        other_svg_id,
        mini_window::Window::SvgWindow(svg::SvgWindow::new(other_svg_id, other_id)),
    );

    let sources = clips_sources(&state, clips_id);
    let source = sources.get(&name).expect("editor listed under its name");
    assert_eq!(source.raw.as_deref(), Some("(box (id o))"));
    assert_eq!(source.output_type, Some(crate::OutputType::Pikchr));
    assert!(!sources.contains_key(&state.windows[&clips_id].as_name().unwrap().get_name()));
}

#[test]
fn clips_sources_keep_a_ruby_editors_text_output_type() {
    use mini_window::{GeneratedContent as _, HasName as _, RenderToggle as _};
    let clips_id = egui::Id::new("clips");
    let ruby_id = egui::Id::new("ruby");
    let ruby_svg_id = egui::Id::new("ruby-svg");
    let mut state = AppState::default();
    let mut ruby = crate::mruby_editor::MrubyEditor::new(ruby_id, ruby_svg_id);
    ruby.set_output_type(crate::OutputType::Text);
    ruby.set_generated_content("(box (id r))\n".to_string());
    let name = ruby.get_name();
    state
        .windows
        .insert(ruby_id, mini_window::Window::MrubyEditor(ruby));
    state.windows.insert(
        ruby_svg_id,
        mini_window::Window::SvgWindow(svg::SvgWindow::new(ruby_svg_id, ruby_id)),
    );

    let sources = clips_sources(&state, clips_id);
    let source = sources.get(&name).expect("editor listed under its name");
    assert_eq!(source.output_type, Some(crate::OutputType::Text));
    assert_eq!(source.generated.as_deref(), Some("(box (id r))\n"));
}
