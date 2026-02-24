use std::sync::Arc;

use eframe::egui::{
    self, Context, MenuBar, Ui, Vec2,
};
use parking_lot::RwLock;
use tokio::sync::{mpsc::Sender, watch};

use crate::{AppState, Msg, pikchr_editor, prolog_editor, svg};

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
        impl $crate::mini_window::Visible for $struct {
            fn visible(&self) -> bool {
                self.$field_name
            }
            fn set_visible(&mut self, value: bool) {
                self.$field_name = value;
            }
        }
    };
}

pub trait Id: Send + Sync {
    fn get_id(&self) -> egui::Id;
}

pub trait HasMenu: Send + Sync {
    fn has_menu(&self) -> bool { false }
    fn menu(&self, _ui: &mut Ui, _tx: Sender<Msg>) {}
}
pub trait Error: Send + Sync {
    fn set_error(&mut self, error: Option<String>);
    fn get_error(&self) -> Option<String>;
}


pub trait MiniWindow: Send + Sync + Visible + Id + HasMenu {
    fn get_title(&self) -> String;
    fn inner_window(
        &mut self,
        ctx: &Context,
        ui: &mut Ui,
        tx: Sender<Msg>,
        app_state: Arc<RwLock<AppState>>,
    );

    fn should_be_listed(&self) -> bool {
        true
    }

    fn should_show(&self) -> bool {
        self.visible()
    }

    fn show(&mut self, ctx: &Context, tx: Sender<Msg>, app_state: Arc<RwLock<AppState>>) {
        if !self.should_show() {
            return;
        };
        let mut visible = self.visible();
        let window = self.outer_window(ctx).open(&mut visible);

        window.show(ctx, |ui| {
            let style_mod = |style: &mut egui::Style| {
                style.spacing.button_padding = Vec2::from((3.0, 3.0));
            };
            let style = ui.style_mut();
            style.spacing.menu_margin = egui::Margin {
                left: 10,
                right: 10,
                top: 10,
                bottom: 10,
            };
            let has_menu = self.has_menu();
            egui::Frame::new().inner_margin(0.0).show(ui, |ui| {
                if has_menu {
                    egui::Frame::new().inner_margin(0.0).show(ui, |ui| {
                        MenuBar::new().ui(ui, |ui| {
                            ui.add_space(8.0);
                            self.menu(ui, tx.clone());
                        });
                    });
                    ui.add_space(2.0 * -ui.spacing().item_spacing.y);
                    ui.separator();
                }
                self.inner_window(ctx, ui, tx.clone(), app_state)
            });
        });
        let modifiers = ctx.input(|i| i.modifiers);
        if modifiers.command_only() && self.visible() != visible {
            let _ = tx.clone().try_send(Msg::DeleteWindow(self.get_id()));
        }
        self.set_visible(visible);
    }

    fn outer_window(&self, ctx: &Context) -> egui::Window<'static> {
        egui::Window::new(self.get_title())
            .resizable(true)
            .default_size((300.0, 150.0))
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
    fn set_target(&mut self, target: egui::Id);
}

pub trait EditorType: Send + Sync {
    fn get_editor_type(&self) -> crate::EditorType;
}

pub trait PikchrContent: Send + Sync + Indexable {
    fn get_pikchr_content(&self) -> String;
    fn set_pikchr_content(&mut self, value: String);
}
pub trait RawContent: Send + Sync + Indexable {
    fn get_raw_content(&self) -> String;
    fn set_raw_content(&mut self, value: String);
}
pub trait InitializeWatchTx: Send + Sync + Initialize {
    type ChangeData: Clone + Send + Sync + 'static;
    fn watch_change_fn(data: Self::ChangeData) -> Msg;
    fn set_watch_tx(&mut self, tx: watch::Sender<Self::ChangeData>);
    fn empty_change_data() -> Self::ChangeData;
    fn initialize(&mut self, event_tx: Sender<Msg>) {
        if !self.is_initialized() {
            self.set_initialized();
            let (tx, mut rx) = tokio::sync::watch::channel(Self::empty_change_data());
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
                }
            });
        }
    }
}
#[macro_export]
macro_rules! impl_initialize {
    ($name:ident, $field:ident) => {
        impl $crate::mini_window::Initialize for $name {
            fn set_initialized(&mut self) {
                self.$field = true;
            }
            fn is_initialized(&self) -> bool {
                self.$field
            }
        }
    };
}
#[macro_export]
macro_rules! impl_initialize_tx {
    ($name:ident, $field:ident, on_change: $closure:expr, data: $data:tt, empty: $empty:tt) => {
        impl $crate::mini_window::InitializeWatchTx for $name {
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
    };
}

#[macro_export]
macro_rules! impl_indexable {
    ($name:ident) => {
        impl $crate::mini_window::Indexable for $name {
            fn set_index(&mut self, value: usize) {
                self.index = value;
            }
            fn get_index(&self) -> usize {
                self.index
            }
        }
    };
}
#[macro_export]
macro_rules! impl_id {
    ($name:ident, $field:ident) => {
        impl $crate::mini_window::Id for $name {
            fn get_id(&self) -> egui::Id {
                self.$field
            }
        }
    };
}
#[macro_export]
macro_rules! impl_target {
    ($name:ident, $field:ident) => {
        impl $crate::mini_window::Target for $name {
            fn get_target(&self) -> egui::Id {
                self.$field
            }
            fn set_target(&mut self, value: egui::Id) {
                self.$field = value
            }
        }
    };
}

#[macro_export]
macro_rules! impl_pikchr_content {
    ($name:ident, $field:ident) => {
        impl $crate::mini_window::PikchrContent for $name {
            fn get_pikchr_content(&self) -> String {
                self.$field.clone()
            }
            fn set_pikchr_content(&mut self, value: String) {
                self.$field = value;
            }
        }
    };
}

#[derive(Debug,serde::Serialize,serde::Deserialize, Clone)]
#[serde(tag = "type", content="fields")]
pub enum Window {
    PikchrEditor(pikchr_editor::PikchrEditor),
    PrologEditor(prolog_editor::PrologEditor),
    SvgWindow(svg::SvgWindow),
}
#[derive(Debug, serde::Serialize,serde::Deserialize, Clone, Copy)]
pub enum WindowType {
    PikchrEditor,
    PrologEditor,
    SvgWindow,
}

#[macro_export]
macro_rules! setter_getter_for_trait {
		{($infield:ident => $intype:ty | $outfield:ident $(.$outmethod:ident ())?=> $outtype:ty ) for $struct:ty as $name:ident for $trait:ty} => {
    		paste::paste! {
        		impl $trait for $struct {
            		fn [<get_ $name>](&self) -> $outtype{
                		self.$outfield $(.$outmethod())?
            		}
            		fn [<set_ $name>](&mut self, value: $intype) {
                		self.$infield = value;
            		}
        		}
    		}
		}
}
macro_rules! trait_getter {
    (
        $tr:ty, $name:ident,
        $(some => [$( $some_variant:ident $(,)? ),*] $(,)?)?
        $(none => [$( $none_variant:ident $(,)? ),*] $(,)?)?
    ) => {
        paste::paste! {
            pub fn $name(&self) -> Option<&dyn $tr> {
                match self {
                    $($( Self::$some_variant(e) =>  Some(e as &dyn $tr),  )*)?
                    $($( Self::$none_variant(..) =>  None,  )*)?
                }
            }
            pub fn [<$name _mut>](&mut self) -> Option<&mut dyn $tr> {
                match self {
                    $($( Self::$some_variant(e) =>  Some(e as &mut dyn $tr),  )*)?
                    $($( Self::$none_variant(..) => None,  )*)?
                }
            }
        }
    };
    (
        view $view:ty, $name:ident, $fun:ident,
        $(some => [$( $some_variant:ident $(,)? ),*] $(,)?)?
        $(none => [$( $none_variant:ident $(,)? ),*] $(,)?)?
    ) => {
        paste::paste! {
            pub fn $name(&self) -> Option<$view> {
                match self {
                    $($( Self::$some_variant(e) =>  Some(e.$fun()),  )*)?
                    $($( Self::$none_variant(..) =>  None,  )*)?
                }
            }
        }
    };
    (
        mut_view $view:ty, $name:ident, $fun:ident,
        $(some => [$( $some_variant:ident $(,)? ),*] $(,)?)?
        $(none => [$( $none_variant:ident $(,)? ),*] $(,)?)?
    ) => {
        paste::paste! {
            pub fn $name(&mut self) -> Option<$view> {
                match self {
                    $($( Self::$some_variant(e) =>  Some(e.$fun()),  )*)?
                    $($( Self::$none_variant(..) =>  None,  )*)?
                }
            }
        }
    };
}


impl Window {
    trait_getter!(PikchrContent, as_content,
        some => [PikchrEditor, PrologEditor],
        none => [SvgWindow],
    );
    trait_getter!(Target, as_target,
        some => [PikchrEditor, PrologEditor],
        none => [SvgWindow],
    );
    trait_getter!(
        Id, as_id,
        some => [PikchrEditor,PrologEditor,SvgWindow]
    );
    trait_getter!(
        Indexable, as_indexable,
        some => [PikchrEditor,PrologEditor,SvgWindow]
    );
    trait_getter!(
        Initialize, as_initialize,
        some => [PikchrEditor,SvgWindow],
        none => [PrologEditor],
    );
    trait_getter!(
        MiniWindow, as_mini_window,
        some => [PikchrEditor,PrologEditor,SvgWindow]
    );
    trait_getter!(
        EditorType, as_editor_type,
        some => [PikchrEditor,PrologEditor],
        none => [SvgWindow],
    );
    trait_getter!(
        view EditorWindowView<'_>, as_editor_window, get_editor_window,
        some => [PikchrEditor,PrologEditor],
        none => [SvgWindow],
    );
    trait_getter!(
        mut_view svg::SvgWindowView<'_>, as_svg_window, get_svg_window,
        some => [SvgWindow],
        none => [PikchrEditor,PrologEditor],
    );
    trait_getter!(
        view WindowView<'_>, as_window, get_window,
        some => [SvgWindow,PikchrEditor,PrologEditor],
    );
    trait_getter!(
        Error, as_error,
        some => [PikchrEditor,PrologEditor],
        none => [SvgWindow],
    );

}

pub trait SvgWindow {
    fn get_svg_window(&mut self) -> svg::SvgWindowView<'_>;
}

pub trait NormalWindow {
    fn get_window(&self) -> WindowView<'_>;
}

pub trait EditorWindow {
    fn get_editor_window(&self) -> EditorWindowView<'_>;
}

impl <T>NormalWindow for T where T: EditorWindow {
    fn get_window(&self) -> WindowView<'_> {
        let value = self.get_editor_window();
        WindowView {
            index: value.index,
            id: value.id,
            mini_window: value.mini_window,
        }
    }
}

pub struct WindowView<'a> {
    pub index: &'a usize,
    pub id: &'a egui::Id,
    pub mini_window: &'a dyn MiniWindow,
}
pub struct EditorWindowView<'a> {
    pub index: &'a usize,
    pub id: &'a egui::Id,
    pub content: &'a dyn PikchrContent,
    pub editor_type: &'a dyn EditorType,
    pub mini_window: &'a dyn MiniWindow,
}

