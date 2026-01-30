use std::time::{SystemTime, UNIX_EPOCH};
use eframe::egui;
use rand::{Rng};

pub fn next_global_id() -> egui::Id {
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64;

    
    let mut rng = rand::rng();
    let entropy: u64 = rng.random();

    
    egui::Id::new(now).with(entropy)
}
