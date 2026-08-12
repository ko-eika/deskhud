//! Small platform boundary used by the direct egui host.

mod backend;
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
    GpuCompositor, cursor_screen_px, fit_popup_pos_points, is_device_lost, primary_monitor_geometry,
};
