#![doc = include_str!("../README.md")]
pub mod actions;
pub mod build_pattern;
mod clipboard;
mod conversion;
mod error;
mod event;
mod multi_window;
mod proxy;
mod user_interface;

pub mod settings;

pub mod reexport {
    pub use layershellev::NewInputPanelSettings;
    pub use layershellev::NewLayerShellSettings;
    pub use layershellev::OutputOption;
    pub use layershellev::PopupPlacement;
    pub use layershellev::WithConnection;
    pub use layershellev::reexport::Anchor;
    pub use layershellev::reexport::KeyboardInteractivity;
    pub use layershellev::reexport::Layer;
    pub use layershellev::reexport::wayland_client::{WlRegion, wl_keyboard};
    pub use layershellev::reexport::xdg_positioner::{
        Anchor as PopupAnchor, ConstraintAdjustment as PopupConstraintAdjustment,
        Gravity as PopupGravity,
    };
    pub mod core {
        pub use iced_core::*;
    }
    pub use iced_core::window::Id as IcedId;
    pub use iced_runtime::Task;
}

mod ime_preedit;

pub use iced_layershell_macros::to_layer_message;

pub use error::Error;

/// Opt-out for clipboard initialization. Call this before starting the
/// runtime when your app has no text input and doesn't need paste/copy —
/// this skips spawning the always-on smithay-clipboard worker thread.
pub fn disable_clipboard() {
    clipboard::set_disabled();
}

/// Graphics backend actually obtained by the compositor (hyprdesk addition):
/// `ICED_BACKEND` lists what to try, and wgpu silently falls back to
/// tiny-skia when no adapter works. `None` until the first surface exists.
pub fn renderer_information() -> Option<RendererInformation> {
    RENDERER.lock().ok()?.clone()
}

/// Copy of `iced_graphics::compositor::Information` (which is not `Clone`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RendererInformation {
    /// Graphics adapter (`CPU` for tiny-skia).
    pub adapter: String,
    /// Backend: a wgpu backend (`Vulkan`, `Gl`…) or `tiny-skia`.
    pub backend: String,
}

static RENDERER: std::sync::Mutex<Option<RendererInformation>> = std::sync::Mutex::new(None);

static BACKEND: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// Backends to try, in order (`"wgpu,tiny-skia"`), instead of the
/// `ICED_BACKEND` environment variable — which child processes would
/// inherit (hyprdesk addition). Call before `run`.
pub fn set_backend(backends: &'static str) {
    let _ = BACKEND.set(backends);
}

pub(crate) fn backend() -> Option<&'static str> {
    BACKEND.get().copied()
}

pub(crate) fn set_renderer_information(info: iced_graphics::compositor::Information) {
    if let Ok(mut r) = RENDERER.lock() {
        *r = Some(RendererInformation {
            adapter: info.adapter,
            backend: info.backend,
        });
    }
}

/// Surfaces to redraw after a batch of messages (hyprdesk addition).
///
/// Upstream refreshes every surface after any message. The application may
/// instead declare, for each message it handles, which surfaces it touched:
/// [`damage::all`], [`damage::only`] or [`damage::none`]. The union over the
/// batch is refreshed; a batch where a message declared nothing, or declared
/// `all`, refreshes every surface as upstream does. Widgets that request a
/// redraw themselves are unaffected.
pub mod damage {
    use iced_core::window::Id;
    use std::sync::Mutex;

    struct Acc {
        /// Every message of the batch declared its damage.
        declared: usize,
        all: bool,
        ids: Vec<Id>,
    }

    static ACC: Mutex<Acc> = Mutex::new(Acc {
        declared: 0,
        all: false,
        ids: Vec::new(),
    });

    fn with(f: impl FnOnce(&mut Acc)) {
        if let Ok(mut a) = ACC.lock() {
            f(&mut a);
        }
    }

    /// The message may have changed any surface.
    pub fn all() {
        with(|a| {
            a.declared += 1;
            a.all = true;
        });
    }

    /// The message changed nothing on screen.
    pub fn none() {
        with(|a| a.declared += 1);
    }

    /// The message changed only these surfaces.
    pub fn only(ids: impl IntoIterator<Item = Id>) {
        with(|a| {
            a.declared += 1;
            a.ids.extend(ids);
        });
    }

    pub(crate) fn begin() {
        with(|a| {
            a.declared = 0;
            a.all = false;
            a.ids.clear();
        });
    }

    /// Surfaces to refresh after `messages` messages: `None` means all.
    pub(crate) fn take(messages: usize) -> Option<Vec<Id>> {
        let mut out = None;
        with(|a| {
            if !a.all && a.declared >= messages && messages > 0 {
                out = Some(std::mem::take(&mut a.ids));
            }
            a.declared = 0;
            a.all = false;
            a.ids.clear();
        });
        out
    }
}

pub type Result = std::result::Result<(), error::Error>;
use iced_core::theme::Style as Appearance;

use iced_core::theme::Base as DefaultStyle;

// layershell application
pub use build_pattern::application;
pub use build_pattern::daemon;

pub use settings::Settings;
