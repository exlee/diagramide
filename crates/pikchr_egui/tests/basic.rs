use std::sync::Arc;

use egui_kittest::kittest::Queryable;
use parking_lot::RwLock;
use pikchr_egui::{Msg, PikchrEgui, message_handler, state::AppState};
use tokio::sync::mpsc;
use eframe::{egui::accesskit::Role};

type Harness<'a> = egui_kittest::Harness<'a, PikchrEgui>;
async fn build_harness<'a>() -> Harness<'a> {
    let state = Arc::new(RwLock::new(AppState::default()));
    let tx = PikchrEgui::spawn_message_handler(state.clone());
    egui_kittest::Harness::builder()
        .with_pixels_per_point(2.0)
        .with_size((800.0,600.0))
        .build_eframe(move |cc| {
        catppuccin_egui::set_theme(&cc.egui_ctx, catppuccin_egui::FRAPPE);
        PikchrEgui::new_test(&cc.egui_ctx,  tx, state)
    })
}

#[tokio::test]
async fn test_open_app() {
    let mut harness = build_harness().await;
		harness.run_steps(20);
    harness.snapshot("app_start");
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
