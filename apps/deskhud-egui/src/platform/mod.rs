//! Small platform boundary used by the direct egui host.

mod backend;
use anyhow::Result;
pub(crate) use backend::OverlayBackend;

#[cfg(all(not(windows), not(target_os = "macos")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(all(not(windows), not(target_os = "macos")))]
pub(crate) use linux::PetHost;
#[cfg(all(not(windows), not(target_os = "macos")))]
pub(crate) use linux::{
    FallbackOverlayBackend, cursor_screen_px, fit_popup_pos_points, main_display_bounds_px,
    main_display_work_area_px,
};
#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub(crate) use macos::MacosOverlayBackend;
#[cfg(target_os = "macos")]
pub(crate) use macos::PetHost;
#[cfg(target_os = "macos")]
pub(crate) use macos::{
    NativePetView, create_native_pet_window, cursor_screen_px, fit_popup_pos_points,
    main_display_bounds_px, main_display_work_area_px, position_native_pet_window,
    request_native_pet_redraw, set_native_pet_control_bus, set_native_pet_topmost,
    start_global_mouse_listener, update_native_pet_paint,
};
#[cfg(windows)]
pub(crate) use windows::{
    GpuCompositor, PetHost, WindowsOverlayBackend, cursor_screen_px, fit_popup_pos_points,
    is_device_lost, primary_monitor_geometry,
};

/// Construct the platform backend behind one shell-facing boundary.
///
/// Keeping target selection here prevents the native host from accumulating
/// platform branches whose imports and lifecycle can drift independently.
pub(crate) fn create_backend() -> Result<Box<dyn OverlayBackend>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(MacosOverlayBackend::default()))
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
