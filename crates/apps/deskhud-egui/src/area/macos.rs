use core::{
    ffi::{CStr, c_char, c_void},
    mem,
};

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

use super::ActivityArea;

pub(super) fn get(window: &Window) -> Option<ActivityArea> {
    visible_frame(window)
}

pub(super) fn get_at(window: &Window, _position: PhysicalPosition<i32>) -> Option<ActivityArea> {
    // 菜单定位运行在渲染线程，不能在这里访问 AppKit/NSScreen；菜单只需要
    // 一个显示器边界，使用 winit 的显示器区域即可。
    super::fallback(window)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NsPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NsSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NsRect {
    origin: NsPoint,
    size: NsSize,
}

type Id = *mut c_void;
type Sel = *mut c_void;

#[link(name = "AppKit", kind = "framework")]
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
    fn sel_registerName(name: *const c_char) -> Sel;
}

unsafe fn msg_send_id(receiver: Id, selector: &CStr) -> Id {
    unsafe {
        let send: unsafe extern "C" fn(Id, Sel) -> Id = mem::transmute(objc_msgSend as *const ());
        send(receiver, sel_registerName(selector.as_ptr()))
    }
}

unsafe fn msg_send_rect(receiver: Id, selector: &CStr) -> NsRect {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        {
            let send: unsafe extern "C" fn(*mut NsRect, Id, Sel) =
                mem::transmute(objc_msgSend_stret as *const ());
            let mut result = NsRect {
                origin: NsPoint { x: 0.0, y: 0.0 },
                size: NsSize {
                    width: 0.0,
                    height: 0.0,
                },
            };
            send(&mut result, receiver, sel_registerName(selector.as_ptr()));
            return result;
        }

        #[cfg(not(target_arch = "x86_64"))]
        let send: unsafe extern "C" fn(Id, Sel) -> NsRect =
            mem::transmute(objc_msgSend as *const ());
        #[cfg(not(target_arch = "x86_64"))]
        return send(receiver, sel_registerName(selector.as_ptr()));
    }
}

fn visible_frame(window: &Window) -> Option<ActivityArea> {
    let raw_handle = window.window_handle().ok()?.as_raw();
    let RawWindowHandle::AppKit(handle) = raw_handle else {
        return super::fallback(window);
    };

    // winit 的 AppKit handle 指向 NSView；NSView.window 即其 NSWindow。
    let ns_view = handle.ns_view.as_ptr();
    let screen = unsafe {
        let ns_window = msg_send_id(ns_view, c"window");
        if !ns_window.is_null() {
            let screen = msg_send_id(ns_window, c"screen");
            if !screen.is_null() {
                screen
            } else {
                // 隐藏窗口可能暂时没有 screen，使用主屏作为创建阶段的兜底。
                msg_send_id(class_named(c"NSScreen"), c"mainScreen")
            }
        } else {
            msg_send_id(class_named(c"NSScreen"), c"mainScreen")
        }
    };
    if screen.is_null() {
        return super::fallback(window);
    }

    let frame = unsafe { msg_send_rect(screen, c"frame") };
    let visible = unsafe { msg_send_rect(screen, c"visibleFrame") };
    let scale = window.scale_factor();
    let monitor = window.current_monitor()?;
    let monitor_position = monitor.position();

    // NSScreen 使用左下角为原点，winit 使用左上角为原点；同时将 point
    // 转换为物理像素，以匹配 Window::set_outer_position 的坐标。
    let left = ((visible.origin.x - frame.origin.x) * scale).round() as i32;
    let top = ((frame.origin.y + frame.size.height - (visible.origin.y + visible.size.height))
        * scale)
        .round() as i32;
    let width = (visible.size.width * scale).round().max(1.0) as u32;
    let height = (visible.size.height * scale).round().max(1.0) as u32;

    Some(ActivityArea {
        position: PhysicalPosition::new(
            monitor_position.x.saturating_add(left),
            monitor_position.y.saturating_add(top),
        ),
        size: PhysicalSize::new(width, height),
    })
}

fn class_named(name: &CStr) -> Id {
    unsafe {
        let class = objc_getClass(name.as_ptr());
        class
    }
}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
}
