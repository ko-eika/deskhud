//! 应用图标和 macOS Dock 显示策略。

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) fn set_visibility(visible: bool) {
    platform::set_visibility(visible);
}
