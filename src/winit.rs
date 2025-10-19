use i_slint_backend_winit::winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::Window;

pub trait WinitWindow {
    fn has_focus(&self) -> bool;
    fn hide_cursor(&self);
    fn display_server(&self) -> anyhow::Result<DisplayServer>;
}

pub enum DisplayServer {
    #[cfg(feature = "wayland")]
    Wayland,
    #[cfg(feature = "x11")]
    X11,
}

impl WinitWindow for Window {
    fn has_focus(&self) -> bool {
        self.with_winit_window(|ww| ww.has_focus()).unwrap_or(false)
    }

    fn hide_cursor(&self) {
        self.with_winit_window(|ww| ww.set_cursor_visible(false));
    }

    fn display_server(&self) -> anyhow::Result<DisplayServer> {
        let handle = self
            .with_winit_window(|ww| ww.display_handle().map(|h| h.as_raw()))
            .unwrap()?;

        match handle {
            #[cfg(feature = "x11")]
            RawDisplayHandle::Xlib(_) => Ok(DisplayServer::X11),

            #[cfg(feature = "x11")]
            RawDisplayHandle::Xcb(_) => Ok(DisplayServer::X11),

            #[cfg(feature = "wayland")]
            RawDisplayHandle::Wayland(_) => Ok(DisplayServer::Wayland),

            _ => anyhow::bail!("Unsupported display server {:?}", handle),
        }
    }
}
