use eframe::egui;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn next_global_id() -> egui::Id {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64;

    let mut rng = rand::rng();
    let entropy: u64 = rng.random();

    egui::Id::new(now).with(entropy)
}

/// Generates a fresh, globally-unique workspace id.
///
/// Workspace ids are `u64` (not `egui::Id`) so they serialize cheaply and
/// never collide with per-window `egui::Id`s. They combine the current
/// nanosecond timestamp with random entropy, matching the strategy used for
/// window ids.
pub fn next_workspace_id() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64;

    let mut rng = rand::rng();
    let entropy: u64 = rng.random();

    now.wrapping_add(entropy.rotate_left(17))
}
