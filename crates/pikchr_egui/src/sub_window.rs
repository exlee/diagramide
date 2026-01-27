use std::sync::Arc;

use eframe::egui::{self, Context, Ui, Window};
use parking_lot::RwLock;
use tokio::sync::{mpsc::Sender, watch};

use crate::{AppState, Msg};

pub trait Visible {
    fn visible(&self) -> bool;
    fn set_visible(&mut self, new: bool);
    fn toggle_visible(&mut self) {
        self.set_visible(!self.visible());
    }
}

#[macro_export]
macro_rules! impl_visible {
    ($struct:ident,$field_name:ident) => {
        impl crate::sub_window::Visible for $struct {
            fn visible(&self) -> bool {
                self.$field_name
            }
            fn set_visible(&mut self, value: bool) {
                self.$field_name = value;
            }
        }
    }
}

pub trait Id: Send + Sync {
    fn get_id(&self) -> egui::Id;
}
pub trait MiniWindow: Send + Sync + Visible + Id {

    //fn widget(&mut self, ctx: &Context, tx: Sender<Msg>, app_state: Arc<RwLock<AppState>>);

    fn get_title(&self) -> String;

    fn should_be_listed(&self) -> bool {
        true
    }

    fn should_show(&self) -> bool {
        self.visible()
    }
    fn change_handler(&self, _ctx: &Context, _tx: Sender<Msg>, _app_state: Arc<RwLock<AppState>>) {
    }

    fn show(&mut self, ctx: &Context, tx: Sender<Msg>, app_state: Arc<RwLock<AppState>>) {
        if self.should_show() {
            //self.widget(ctx, tx, app_state);
            let window = self.outer_window(ctx);

            window.show(ctx, |ui| {
                self.inner_window(ctx, ui, tx, app_state);
            });

        }
    }
    fn inner_window(&mut self, ctx: &Context, ui: &mut Ui, tx: Sender<Msg>, app_state: Arc<RwLock<AppState>>);

    fn outer_window(&self, ctx: &Context) -> Window<'static> {
        egui::Window::new(self.get_title()).resizable(true)
            .id(self.get_id())
            .frame(egui::Frame::window(&ctx.style()).inner_margin(0.0))
    }

}

pub trait Indexable: Send + Sync {
    fn set_index(&mut self, value: usize);
    fn get_index(&self) -> usize;
}

pub trait Initialize: Send + Sync + Id {
    fn is_initialized(&self) -> bool;
    fn set_initialized(&mut self);
}

pub trait Target: Send + Sync {
    fn get_target(&self) -> egui::Id;
}

pub trait EditorType: Send + Sync {
    fn get_editor_type(&self) -> crate::EditorType;
}

pub trait Content: Send + Sync + Indexable {
    fn get_content(&self) -> String;
    fn set_content(&mut self,value: String);
}
pub trait InitializeWatchTx: Send + Sync + Initialize {
    type ChangeData: Clone + Send + Sync + 'static;
    fn watch_change_fn(data: Self::ChangeData) -> Msg;
    fn set_watch_tx(&mut self, tx: watch::Sender<Self::ChangeData>);
    fn empty_change_data() -> Self::ChangeData;
    fn initialize(&mut self, event_tx: Sender<Msg>) {
        if !self.is_initialized() {
						self.set_initialized();
            let (tx,mut rx) = tokio::sync::watch::channel(Self::empty_change_data());
            self.set_watch_tx(tx);

            tokio::task::spawn(async move {
                let duration = tokio::time::Duration::from_millis(100);
                let mut interval = tokio::time::interval(duration);
                loop {
                    interval.tick().await;
                    if rx.has_changed().unwrap_or_default() {
                        let data: Self::ChangeData = rx.borrow_and_update().clone();
                        let _ = event_tx.try_send(Self::watch_change_fn(data));
                    }
                };
            });
        }
    }
}
#[macro_export]
macro_rules! impl_initialize {
    ($name:ident, $field:ident) => {
        impl crate::sub_window::Initialize for $name {
            fn set_initialized(&mut self) {
                self.$field = true;
            }
            fn is_initialized(&self) -> bool {
                self.$field
            }
        }
    }
}
#[macro_export]
macro_rules! impl_initialize_tx {
    ($name:ident, $field:ident, on_change: $closure:expr, data: $data:ty, empty: $empty:expr) => {
        impl crate::sub_window::InitializeWatchTx for $name {
            type ChangeData = $data;
            fn set_watch_tx(&mut self, tx: tokio::sync::watch::Sender<Self::ChangeData>) {
                self.$field = Some(tx);
            }
            fn empty_change_data() -> Self::ChangeData {
                $empty
            }
            fn watch_change_fn(data: Self::ChangeData) -> Msg {
                let closure = $closure;
                closure(data)
            }
        }
    }
}

#[macro_export]
macro_rules! impl_indexable {
    ($name:ident) => {
        impl crate::sub_window::Indexable for $name {
            fn set_index(&mut self, value: usize) {
                self.index = value;
            }
            fn get_index(&self) -> usize {
                self.index
            }
        }
    }
}
#[macro_export]
macro_rules! impl_id {
    ($name:ident, $field:ident) => {
        impl crate::sub_window::Id for $name {
            fn get_id(&self) -> egui::Id {
                self.$field
            }
        }
    }
}
#[macro_export]
macro_rules! impl_target {
    ($name:ident, $field:ident) => {
        impl crate::sub_window::Target for $name {
            fn get_target(&self) -> egui::Id {
                self.$field
            }
        }
    }
}

#[macro_export]
macro_rules! impl_content {
    ($name:ident, $field:ident) => {
        impl crate::sub_window::Content for $name {
            fn get_content(&self) -> String {
                self.$field.clone()
            }
            fn set_content(&mut self, value: String)  {
                self.$field = value;
            }
        }
    }
}

pub trait IndexableMiniWindow: MiniWindow + Indexable {}
impl<T> IndexableMiniWindow for T where T: MiniWindow + Indexable {}
pub trait EditorMiniWindow: MiniWindow + Content + EditorType + Target {}
impl<T> EditorMiniWindow for T where T: MiniWindow + Content + EditorType + Target {}
