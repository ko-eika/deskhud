//! 平台相关：透明 chrome、全局输入、窗口几何。
//!
//! Windows 走原生 API；其它平台为 MVP 回退（egui 视口 + 降级全局输入）。

#[cfg(windows)]
mod windows;
#[cfg(not(windows))]
mod fallback;

#[cfg(windows)]
pub use windows::{
    capture_screen_rgba, cursor_client_px, cursor_screen_px, ensure_acrylic_popup, ensure_pet_chrome,
    fit_popup_pos_points, foreground_hwnd, foreground_is_outside, global_key_down, global_modifiers,
    global_mouse_buttons, find_window_by_title, list_displays, move_window_screen, set_click_through,
    set_window_owner, set_window_visible, take_wheel_delta, window_screen_pos,
    window_screen_rect, work_area_containing_px, DisplayInfo,
};

#[cfg(not(windows))]
pub use fallback::{
    capture_screen_rgba, cursor_client_px, cursor_screen_px, cursor_screen_px_from_ctx,
    ensure_acrylic_popup, ensure_pet_chrome, fit_popup_pos_points, foreground_hwnd,
    foreground_is_outside, global_key_down, global_modifiers, global_mouse_buttons,
    find_window_by_title, list_displays, move_viewport_points, move_window_screen, set_click_through,
    set_window_owner, set_window_visible, take_wheel_delta, window_screen_pos,
    window_screen_pos_from_ctx, window_screen_rect, work_area_containing_px, work_area_from_ctx,
    DisplayInfo,
};

/// 从 raw-window-handle 取原生窗标识（Windows HWND；其它后端取可用指针作跟踪键）。
pub fn native_window_id(handle: raw_window_handle::RawWindowHandle) -> Option<isize> {
    use raw_window_handle::RawWindowHandle;
    match handle {
        RawWindowHandle::Win32(win) => Some(win.hwnd.get() as isize),
        RawWindowHandle::AppKit(a) => Some(a.ns_view.as_ptr() as isize),
        RawWindowHandle::Xlib(x) => Some(x.window as isize),
        RawWindowHandle::Xcb(x) => Some(x.window.get() as isize),
        RawWindowHandle::Wayland(w) => Some(w.surface.as_ptr() as isize),
        _ => None,
    }
}
