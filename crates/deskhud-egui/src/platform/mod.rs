//! Small platform boundary used by the direct egui host.

#[cfg(not(windows))]
mod fallback;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
pub use fallback::{cursor_screen_px, fit_popup_pos_points};
#[cfg(windows)]
pub(crate) use windows::{GpuCompositor, cursor_screen_px, fit_popup_pos_points, is_device_lost};
