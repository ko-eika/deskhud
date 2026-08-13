//! Small platform boundary used by the direct egui host.

mod backend;
use anyhow::Result;
pub(crate) use backend::OverlayBackend;

#[cfg(not(windows))]
mod fallback;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(all(not(windows), not(target_os = "macos")))]
pub use fallback::{cursor_screen_px, fit_popup_pos_points};
#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub(crate) use macos::MacosOverlayBackend;
#[cfg(target_os = "macos")]
pub(crate) use macos::{
    cursor_screen_px, fit_popup_pos_points, main_display_bounds_px, main_display_work_area_px,
    start_global_mouse_listener,
};
#[cfg(windows)]
pub(crate) use windows::{
    GpuCompositor, WindowsOverlayBackend, cursor_screen_px, fit_popup_pos_points, is_device_lost,
    primary_monitor_geometry,
};

/// Construct the platform backend behind one shell-facing boundary.
///
/// Keeping target selection here prevents the native host from accumulating
/// platform branches whose imports and lifecycle can drift independently.
pub(crate) fn create_backend() -> Result<Box<dyn OverlayBackend>> {
    #[cfg(target_os = "macos")]
    {
        return Ok(Box::new(MacosOverlayBackend::default()));
    }
    #[cfg(windows)]
    {
        return Ok(Box::new(WindowsOverlayBackend));
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        return Ok(Box::new(FallbackOverlayBackend));
    }
}
