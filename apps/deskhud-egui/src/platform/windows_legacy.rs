//! Windows helpers kept outside product and extension contracts.

mod gpu_compositor;

pub(crate) use gpu_compositor::{GpuCompositor, is_device_lost};

use super::OverlayBackend;
use anyhow::Result;
use deskhud_engine::{
    OverlayBackendCapabilities, OverlayEvent, OverlayPoint, OverlayRect, OverlayScene,
    OverlayScreenArea, OverlayWindowId, OverlayWindowLevel, OverlayWindowRole,
};

use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub fn primary_monitor_geometry() -> ((i32, i32, i32, i32), (i32, i32, i32, i32)) {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
            let m = info.rcMonitor;
            let w = info.rcWork;
            return (
                (m.left, m.top, m.right - m.left, m.bottom - m.top),
                (w.left, w.top, w.right - w.left, w.bottom - w.top),
            );
        }
    }
    ((0, 0, 1, 1), (0, 0, 1, 1))
}

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
        let preferred_x = if cursor.x + width <= info.rcWork.right {
            cursor.x
        } else {
            cursor.x - width
        };
        let preferred_y = if cursor.y + height <= info.rcWork.bottom {
            cursor.y
        } else {
            cursor.y - height
        };
        let x = preferred_x.clamp(
            info.rcWork.left,
            (info.rcWork.right - width).max(info.rcWork.left),
        );
        let y = preferred_y.clamp(
            info.rcWork.top,
            (info.rcWork.bottom - height).max(info.rcWork.top),
        );
        (x as f32 / scale, y as f32 / scale)
    }
}

/// Windows implementation of the common overlay backend boundary.
#[allow(dead_code)]
pub(crate) struct WindowsOverlayBackend;

impl OverlayBackend for WindowsOverlayBackend {
    fn capabilities(&self) -> OverlayBackendCapabilities {
        OverlayBackendCapabilities {
            desktop_transparency: true,
            per_region_passthrough: true,
            selected_display: true,
            virtual_desktop: true,
        }
    }
    fn create_window(&mut self, _role: OverlayWindowRole) -> Result<OverlayWindowId> {
        Ok(OverlayWindowId(1))
    }
    fn update_scene(&mut self, _id: OverlayWindowId, _scene: OverlayScene) -> Result<()> {
        Ok(())
    }
    fn set_visible(&mut self, _id: OverlayWindowId, _visible: bool) -> Result<()> {
        Ok(())
    }
    fn set_level(&mut self, _id: OverlayWindowId, _level: OverlayWindowLevel) -> Result<()> {
        Ok(())
    }
    fn destroy_window(&mut self, _id: OverlayWindowId) -> Result<()> {
        Ok(())
    }
    fn poll_events(&mut self) -> Vec<OverlayEvent> {
        Vec::new()
    }
    fn screen_area(&self) -> Result<OverlayScreenArea> {
        let cursor = cursor_screen_px().unwrap_or((0, 0));
        let monitor = unsafe {
            MonitorFromPoint(
                POINT {
                    x: cursor.0,
                    y: cursor.1,
                },
                MONITOR_DEFAULTTONEAREST,
            )
        };
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
        unsafe {
            GetMonitorInfoW(monitor, &mut info);
        }
        let display = rect(info.rcMonitor);
        let active = rect(info.rcWork);
        Ok(OverlayScreenArea {
            display,
            active,
            excluded: Vec::new(),
        })
    }
}

fn rect(value: RECT) -> OverlayRect {
    OverlayRect {
        origin: OverlayPoint {
            x: value.left as f32,
            y: value.top as f32,
        },
        width: (value.right - value.left) as f32,
        height: (value.bottom - value.top) as f32,
    }
}
