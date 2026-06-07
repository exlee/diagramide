use std::sync::Arc;

use egui_kittest::kittest::Queryable;
use parking_lot::RwLock;
use diagramide::{DiagramIDE, state::AppState};
use eframe::egui::accesskit::Role;

type Harness<'a> = egui_kittest::Harness<'a, DiagramIDE>;
async fn build_harness<'a>() -> Harness<'a> {
    let state = Arc::new(RwLock::new(AppState::default()));
    let logger = diagramide::logger::init_logger();
    let tx = DiagramIDE::spawn_message_handler(logger, state.clone());
    egui_kittest::Harness::builder()
        .with_pixels_per_point(2.0)
        .with_size((800.0,600.0))
        .build_eframe(move |cc| {
        catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::FRAPPE);
        DiagramIDE::new_test(&cc.egui_ctx,  tx, state)
    })
}

#[tokio::test]
async fn test_open_app() {
    let mut harness = build_harness().await;
		harness.run_steps(20);
    harness.snapshot("app_start");
}

#[tokio::test]
async fn test_help_opens_from_main_menu() {
    let mut harness = build_harness().await;
    harness.run_steps(10);
    harness.get_by_label("Help").click_accesskit();
    harness.run_ok();
    harness.get_by_label("DiagramIDE Help").click_accesskit();
    poll(&mut harness, |h| {
        h.query_by_label("Cross-window references").is_some()
    }).await;

    assert!(harness.query_by_label("!!NAME!!").is_some());
    assert!(harness.query_by_label("$$NAME$$").is_some());
}

async fn poll<'a>(harness: &mut Harness<'a>, mut condition: impl FnMut(&mut Harness<'a>) -> bool) {
    		loop {
        		harness.run();
        		tokio::task::yield_now().await;
        		if condition(harness) { break; }
    		}
}
#[tokio::test]
async fn test_new_editor() {
    let mut harness: Harness = build_harness().await;
		//tokio::task::yield_now().await;
		harness.run_steps(10);
		harness.get_by_label("New").click_accesskit();
		//tokio::task::yield_now().await;
		harness.run_ok();
		harness.get_by_label("Pikchr Editor").click_accesskit();
    harness.run_steps(10);
		//tokio::task::yield_now().await;
		//tokio::task::yield_now().await;
		harness.run_steps(10);
		harness.get_by_label("New").click_accesskit();
		harness.run_steps(10);
    harness.snapshot("new_editor");
    poll(&mut harness,|h| {
        h.query_by_role(Role::MultilineTextInput).is_some()
    }).await;
    let editor = harness.get_by_role(Role::MultilineTextInput);
    editor.focus();
    editor.type_text("box \"abc\"");
		harness.step();
		//tokio::task::yield_now().await;
		let _ = harness.try_run_realtime();
		//tokio::task::yield_now().await;
    harness.snapshot("new_editor");
}
