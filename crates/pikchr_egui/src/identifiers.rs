use std::sync::atomic::{AtomicU64 };

use eframe::egui;

static GLOBAL_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next_global_id() -> egui::Id {
    let count = GLOBAL_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    egui::Id::new(count)
}
