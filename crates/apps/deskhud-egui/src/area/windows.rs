use core::{ffi::c_void, mem::size_of};

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use super::ActivityArea;

type HMonitor = *mut c_void;
type Hwnd = *mut c_void;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    monitor: Rect,
    work: Rect,
    flags: u32,
}

unsafe extern "system" {
    fn MonitorFromWindow(hwnd: Hwnd, flags: u32) -> HMonitor;
    fn MonitorFromPoint(point: Point, flags: u32) -> HMonitor;
    fn GetMonitorInfoW(monitor: HMonitor, info: *mut MonitorInfo) -> i32;
}

pub(super) fn get(window: &Window) -> Option<ActivityArea> {
    let raw_handle = window.window_handle().ok()?.as_raw();
    let RawWindowHandle::Win32(handle) = raw_handle else {
        return super::fallback(window);
    };
    let monitor = unsafe { MonitorFromWindow(handle.hwnd.get() as Hwnd, 2) };
    from_monitor(monitor).or_else(|| super::fallback(window))
}

pub(super) fn get_at(window: &Window, position: PhysicalPosition<i32>) -> Option<ActivityArea> {
    let monitor = unsafe {
        MonitorFromPoint(
            Point {
                x: position.x,
                y: position.y,
            },
            2,
        )
    };
    from_monitor(monitor).or_else(|| get(window))
}

fn from_monitor(monitor: HMonitor) -> Option<ActivityArea> {
    if monitor.is_null() {
        return None;
    }
    let mut info = MonitorInfo {
        cb_size: size_of::<MonitorInfo>() as u32,
        monitor: Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        work: Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        flags: 0,
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    Some(ActivityArea {
        position: PhysicalPosition::new(info.work.left, info.work.top),
        size: PhysicalSize::new(
            (info.work.right - info.work.left).max(1) as u32,
            (info.work.bottom - info.work.top).max(1) as u32,
        ),
    })
}
