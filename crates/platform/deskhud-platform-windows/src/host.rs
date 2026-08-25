//! Win32 event shell. Composition rendering is intentionally isolated here;
//! no HWND crosses the platform-neutral crate boundary.

use super::PlatformError;

#[cfg(windows)]
mod win32 {
    #![allow(unsafe_op_in_unsafe_fn)]
    use super::PlatformError;
    use crate::composition;
    use std::ptr::null_mut;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, CreatePen, DeleteObject, EndPaint, GetStockObject, InvalidateRect, NULL_BRUSH,
        PAINTSTRUCT, PS_DASH, Rectangle, SelectObject, UpdateWindow,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    const ID_SETTINGS: usize = 1001;
    const ID_TOPMOST: usize = 1002;
    const ID_PLUGIN: usize = 1003;
    const ID_LAYOUT: usize = 1004;
    const ID_EXIT: usize = 1005;
    const PET_SIZE: i32 = 160;
    static HUD_HWND: AtomicIsize = AtomicIsize::new(0);
    static LAYOUT_HWND: AtomicIsize = AtomicIsize::new(0);
    static HUD_ENABLED: AtomicBool = AtomicBool::new(false);
    static TOPMOST: AtomicBool = AtomicBool::new(false);

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_RBUTTONUP => {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    let settings = wide("Settings");
                    let topmost = wide("Always on top");
                    let plugin = wide("Plugin");
                    let layout = wide("Plugin layout");
                    let exit = wide("Exit");
                    AppendMenuW(menu, MF_STRING, ID_SETTINGS, settings.as_ptr());
                    AppendMenuW(menu, MF_STRING, ID_TOPMOST, topmost.as_ptr());
                    AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
                    AppendMenuW(menu, MF_STRING, ID_PLUGIN, plugin.as_ptr());
                    AppendMenuW(menu, MF_STRING, ID_LAYOUT, layout.as_ptr());
                    AppendMenuW(menu, MF_SEPARATOR, 0, null_mut());
                    AppendMenuW(menu, MF_STRING, ID_EXIT, exit.as_ptr());
                    let mut point = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut point);
                    let command = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_RIGHTBUTTON,
                        point.x,
                        point.y,
                        0,
                        hwnd,
                        null_mut(),
                    );
                    DestroyMenu(menu);
                    SendMessageW(hwnd, WM_COMMAND, command as WPARAM, 0);
                }
                0
            }
            WM_COMMAND => {
                match (wparam & 0xffff) as usize {
                    ID_SETTINGS => super::open_settings(),
                    ID_LAYOUT => super::toggle_layout(hwnd),
                    ID_PLUGIN => super::toggle_hud(hwnd),
                    ID_TOPMOST => super::toggle_topmost(hwnd),
                    ID_EXIT => PostQuitMessage(0),
                    _ => {}
                }
                0
            }
            WM_KEYDOWN if wparam as u32 == VK_ESCAPE as u32 => {
                close_layout(hwnd);
                0
            }
            WM_PAINT => {
                let mut paint = PAINTSTRUCT {
                    hdc: null_mut(),
                    fErase: 0,
                    rcPaint: Default::default(),
                    fRestore: 0,
                    fIncUpdate: 0,
                    rgbReserved: [0; 32],
                };
                BeginPaint(hwnd, &mut paint);
                EndPaint(hwnd, &paint);
                let _ = composition::render(hwnd as isize);
                0
            }
            WM_DESTROY => {
                composition::detach(hwnd as isize);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, _lparam),
        }
    }

    unsafe extern "system" fn border_proc(
        hwnd: HWND,
        msg: u32,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_KEYDOWN if _wparam as u32 == VK_ESCAPE as u32 => {
                close_layout(hwnd);
                0
            }
            WM_PAINT => {
                let mut paint = PAINTSTRUCT {
                    hdc: null_mut(),
                    fErase: 0,
                    rcPaint: Default::default(),
                    fRestore: 0,
                    fIncUpdate: 0,
                    rgbReserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                let pen = CreatePen(PS_DASH, 2, 0x00_66_cc_ff);
                let old = SelectObject(hdc, pen as _);
                let brush = GetStockObject(NULL_BRUSH as i32);
                SelectObject(hdc, brush as _);
                let mut rect = RECT::default();
                GetClientRect(hwnd, &mut rect);
                Rectangle(hdc, 2, 2, rect.right - 2, rect.bottom - 2);
                SelectObject(hdc, old);
                DeleteObject(pen as _);
                EndPaint(hwnd, &paint);
                0
            }
            WM_DESTROY => 0,
            _ => DefWindowProcW(hwnd, msg, _wparam, _lparam),
        }
    }

    unsafe fn register_border_class(instance: HINSTANCE) -> Vec<u16> {
        let class = wide("DeskHudBorderWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(border_proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            ..Default::default()
        };
        RegisterClassW(&wc);
        class
    }

    unsafe fn create_border(instance: HINSTANCE, full_screen: bool) -> HWND {
        let class = register_border_class(instance);
        let (x, y, width, height) = if full_screen {
            (
                0,
                0,
                GetSystemMetrics(SM_CXSCREEN),
                GetSystemMetrics(SM_CYSCREEN),
            )
        } else {
            (420, 180, 420, 180)
        };
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
            class.as_ptr(),
            class.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            width,
            height,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        if !hwnd.is_null() {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            let attached =
                composition::attach(hwnd as isize, width as u32, height as u32, false).is_ok();
            SetWindowPos(
                hwnd,
                if TOPMOST.load(Ordering::Relaxed) {
                    HWND_TOPMOST
                } else {
                    HWND_TOP
                },
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            if attached {
                let _ = composition::render(hwnd as isize);
            }
        }
        hwnd
    }

    pub unsafe fn toggle_hud(_pet: HWND) {
        if HUD_ENABLED.fetch_xor(true, Ordering::AcqRel) {
            let hwnd = HUD_HWND.swap(0, Ordering::AcqRel) as HWND;
            if !hwnd.is_null() {
                DestroyWindow(hwnd);
            }
        } else {
            let hwnd = create_border(GetModuleHandleW(null_mut()), false);
            HUD_HWND.store(hwnd as isize, Ordering::Release);
        }
    }

    pub unsafe fn toggle_layout(_pet: HWND) {
        if !HUD_ENABLED.load(Ordering::Acquire) {
            return;
        }
        let hud = HUD_HWND.swap(0, Ordering::AcqRel) as HWND;
        if !hud.is_null() {
            DestroyWindow(hud);
        }
        let old = LAYOUT_HWND.swap(0, Ordering::AcqRel) as HWND;
        if !old.is_null() {
            DestroyWindow(old);
            return;
        }
        let hwnd = create_border(GetModuleHandleW(null_mut()), true);
        LAYOUT_HWND.store(hwnd as isize, Ordering::Release);
        SetForegroundWindow(hwnd);
    }

    pub unsafe fn close_layout(_source: HWND) {
        let layout = LAYOUT_HWND.swap(0, Ordering::AcqRel) as HWND;
        if !layout.is_null() {
            DestroyWindow(layout);
        }
        if HUD_ENABLED.load(Ordering::Acquire) && HUD_HWND.load(Ordering::Acquire) == 0 {
            let hwnd = create_border(GetModuleHandleW(null_mut()), false);
            HUD_HWND.store(hwnd as isize, Ordering::Release);
        }
    }

    pub unsafe fn toggle_topmost(pet: HWND) {
        let enabled = !TOPMOST.fetch_xor(true, Ordering::AcqRel);
        let z = if enabled {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        for raw in [
            pet as isize,
            HUD_HWND.load(Ordering::Acquire),
            LAYOUT_HWND.load(Ordering::Acquire),
        ] {
            let hwnd = raw as HWND;
            if !hwnd.is_null() {
                SetWindowPos(
                    hwnd,
                    z,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }

    pub fn run(title: &str) -> Result<(), PlatformError> {
        unsafe {
            let instance: HINSTANCE = GetModuleHandleW(null_mut());
            let class = wide("DeskHudPetWindow");
            let title = wide(title);
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                ..Default::default()
            };
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                class.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                100,
                100,
                PET_SIZE,
                PET_SIZE,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            );
            if hwnd.is_null() {
                return Err(PlatformError::new("CreateWindowExW failed"));
            }
            composition::attach(hwnd as isize, PET_SIZE as u32, PET_SIZE as u32, true)
                .map_err(|error| PlatformError::new(format!("attach pet compositor: {error}")))?;
            SetWindowPos(
                hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            InvalidateRect(hwnd, null_mut(), 0);
            UpdateWindow(hwnd);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            Ok(())
        }
    }
}

#[cfg(windows)]
pub fn run(title: &str) -> Result<(), PlatformError> {
    win32::run(title)
}

#[cfg(not(windows))]
pub fn run(_title: &str) -> Result<(), PlatformError> {
    Err(PlatformError::new(
        "Windows host is only available on Windows",
    ))
}

#[cfg(windows)]
fn open_settings() {
    std::thread::Builder::new()
        .name("deskhud-settings-winui".into())
        .spawn(|| {
            let _ = super::run_transparent_window("DeskHud Settings");
        })
        .ok();
}
#[cfg(windows)]
fn toggle_layout(hwnd: windows_sys::Win32::Foundation::HWND) {
    unsafe { win32::toggle_layout(hwnd) }
}
#[cfg(windows)]
fn toggle_hud(hwnd: windows_sys::Win32::Foundation::HWND) {
    unsafe { win32::toggle_hud(hwnd) }
}
#[cfg(windows)]
fn toggle_topmost(hwnd: windows_sys::Win32::Foundation::HWND) {
    unsafe { win32::toggle_topmost(hwnd) }
}
