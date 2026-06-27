use std::time::Duration;

use eframe::egui;
use tokio::sync::mpsc::{Sender, error::TrySendError};

use crate::Msg;

pub trait DebouncedTrySend {
    fn try_send_debounced(
        &self,
        id: egui::Id,
        debounce_millis: u64,
        msg: Msg,
    ) -> Result<(), TrySendError<Msg>>;
}

impl DebouncedTrySend for Sender<Msg> {
    fn try_send_debounced(
        &self,
        id: egui::Id,
        debounce_millis: u64,
        msg: Msg,
    ) -> Result<(), TrySendError<Msg>> {
        let msg = Msg::Debounce(Duration::from_millis(debounce_millis), id, Box::new(msg));
        self.try_send(msg)
    }
}
