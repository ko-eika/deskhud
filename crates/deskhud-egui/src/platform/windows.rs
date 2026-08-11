//! Windows helpers kept outside product and extension contracts.

mod gpu_compositor;

pub(crate) use gpu_compositor::{GpuCompositor, is_device_lost};

use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub fn cursor_screen_px() -> Option<(i32, i32)> {
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        (GetCursorPos(&mut point) != 0).then_some((point.x, point.y))
    }
}

pub fn fit_popup_pos_points(
    cursor_points: (f32, f32),
    width_points: f32,
    height_points: f32,
    pixels_per_point: f32,
) -> (f32, f32) {
    let scale = pixels_per_point.max(0.01);
    let cursor = POINT {
        x: (cursor_points.0 * scale).round() as i32,
        y: (cursor_points.1 * scale).round() as i32,
    };
    unsafe {
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        if monitor.is_null() || GetMonitorInfoW(monitor, &mut info) == 0 {
            return cursor_points;
        }
        let width = (width_points * scale).round() as i32;
        let height = (height_points * scale).round() as i32;
        let x = cursor.x.clamp(
            info.rcWork.left,
            (info.rcWork.right - width).max(info.rcWork.left),
        );
        let y = cursor.y.clamp(
            info.rcWork.top,
            (info.rcWork.bottom - height).max(info.rcWork.top),
        );
        (x as f32 / scale, y as f32 / scale)
    }
}
