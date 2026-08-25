//! Minimal Windows native backend: an ordinary blank Win32 window.

use deskhud_platform::*;

#[derive(Default)]
pub struct WindowsPlatform {
    state: HostState,
}

pub fn run_blank_window(title: &str) -> Result<(), PlatformError> {
    #[cfg(windows)]
    {
        windows_host::run(title)
    }
    #[cfg(not(windows))]
    {
        let _ = title;
        Err(PlatformError::new(
            "Windows host is only available on Windows",
        ))
    }
}

impl WindowsPlatform {
    pub fn state(&self) -> &HostState {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut HostState {
        &mut self.state
    }
}
impl SettingsHost for WindowsPlatform {
    fn open_settings(&mut self) -> Result<(), PlatformError> {
        Ok(())
    }
}
impl OverlayHost for WindowsPlatform {
    fn create_overlay(&mut self, role: WindowRole) -> Result<WindowId, PlatformError> {
        Ok(self.state.create_overlay(role))
    }
    fn set_overlay_visible(&mut self, id: WindowId, visible: bool) -> Result<(), PlatformError> {
        self.state.set_overlay_visible(id, visible)
    }
    fn set_overlay_level(&mut self, id: WindowId, level: WindowLevel) -> Result<(), PlatformError> {
        self.state.set_overlay_level(id, level)
    }
}
impl MenuHost for WindowsPlatform {
    fn show_menu(&mut self, anchor: Rect) -> Result<(), PlatformError> {
        self.state.show_menu(anchor);
        Ok(())
    }
}
impl WindowHost for WindowsPlatform {
    fn close_window(&mut self, id: WindowId) -> Result<(), PlatformError> {
        self.state.close_window(id)
    }
}
impl DisplayHost for WindowsPlatform {
    fn displays(&self) -> Result<Vec<DisplayInfo>, PlatformError> {
        #[cfg(not(windows))]
        {
            return Ok(self.state.displays());
        }
        #[cfg(windows)]
        {
            use windows::Win32::Foundation::{POINT, RECT};
            use windows::Win32::Graphics::Gdi::{
                GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
            };
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                rcMonitor: RECT::default(),
                rcWork: RECT::default(),
                dwFlags: 0,
            };
            let monitor = unsafe {
                let mut cursor = POINT { x: 0, y: 0 };
                let _ = GetCursorPos(&mut cursor);
                MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST)
            };
            if monitor.is_invalid() || unsafe { !GetMonitorInfoW(monitor, &mut info).as_bool() } {
                return Err(PlatformError::new(
                    "Windows monitor information is unavailable",
                ));
            }
            Ok(vec![DisplayInfo {
                bounds: rect(info.rcMonitor),
                work_area: rect(info.rcWork),
                scale: 1.0,
            }])
        }
    }
}
impl InputHost for WindowsPlatform {
    fn poll_events(&mut self) -> Result<Vec<PlatformEvent>, PlatformError> {
        Ok(self.state.poll_events())
    }
}

#[cfg(windows)]
fn rect(value: windows::Win32::Foundation::RECT) -> Rect {
    Rect {
        x: value.left as f64,
        y: value.top as f64,
        width: (value.right - value.left) as f64,
        height: (value.bottom - value.top) as f64,
    }
}

#[cfg(windows)]
mod windows_host {
    use super::PlatformError;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::w;
    pub fn run(title: &str) -> Result<(), PlatformError> {
        unsafe {
            let instance = GetModuleHandleW(None)
                .map_err(|e| PlatformError::new(format!("get module handle: {e}")))?;
            let class = w!("DeskHudBlankWindow");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: HINSTANCE(instance.0),
                lpszClassName: class,
                hCursor: LoadCursorW(None, IDC_ARROW)
                    .map_err(|e| PlatformError::new(format!("load cursor: {e}")))?,
                ..Default::default()
            };
            RegisterClassW(&wc);
            let title = windows::core::HSTRING::from(title);
            let _hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                &title,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                960,
                640,
                None,
                None,
                Some(HINSTANCE(instance.0)),
                None,
            )
            .map_err(|e| PlatformError::new(format!("create window: {e}")))?;
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            Ok(())
        }
    }
    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }
}
