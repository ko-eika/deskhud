//! Windows single-instance guard and activation signal.

#[cfg(windows)]
use std::ptr::null_mut;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_APP};

#[cfg(windows)]
const INSTANCE_NAME: &[u16] = &[
    b'L' as u16,
    b'o' as u16,
    b'c' as u16,
    b'a' as u16,
    b'l' as u16,
    b'\\' as u16,
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b'k' as u16,
    b'H' as u16,
    b'u' as u16,
    b'd' as u16,
    b'S' as u16,
    b'i' as u16,
    b'n' as u16,
    b'g' as u16,
    b'l' as u16,
    b'e' as u16,
    b'I' as u16,
    b'n' as u16,
    b's' as u16,
    b't' as u16,
    b'a' as u16,
    b'n' as u16,
    b'c' as u16,
    b'e' as u16,
    0,
];

#[cfg(windows)]
const OVERLAY_CLASS: &[u16] = &[
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b'k' as u16,
    b'H' as u16,
    b'u' as u16,
    b'd' as u16,
    b'G' as u16,
    b'p' as u16,
    b'u' as u16,
    b'P' as u16,
    b'r' as u16,
    b'o' as u16,
    b'b' as u16,
    b'e' as u16,
    0,
];

#[cfg(windows)]
pub(crate) struct Guard(isize);

#[cfg(windows)]
pub(crate) fn acquire_or_activate() -> Option<Guard> {
    let handle = unsafe { CreateMutexW(null_mut(), 0, INSTANCE_NAME.as_ptr()) };
    if handle.is_null() {
        tracing::error!("failed to create DeskHud single-instance mutex; refusing startup");
        return None;
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let hwnd = unsafe { FindWindowW(OVERLAY_CLASS.as_ptr(), std::ptr::null()) };
        if !hwnd.is_null() {
            unsafe { PostMessageW(hwnd, WM_APP + 1, 0, 0) };
        }
        unsafe { windows_sys::Win32::Foundation::CloseHandle(handle as _) };
        None
    } else {
        Some(Guard(handle as isize))
    }
}

#[cfg(not(windows))]
pub(crate) struct Guard;

#[cfg(not(windows))]
pub(crate) fn acquire_or_activate() -> Option<Guard> {
    Some(Guard)
}

#[cfg(windows)]
impl Drop for Guard {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0 as _) };
        }
    }
}
