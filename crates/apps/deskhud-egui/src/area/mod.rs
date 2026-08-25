//! 屏幕活动区与禁区查询。
//!
//! Windows 使用显示器工作区作为活动区；macOS 使用 NSScreen 的
//! visibleFrame，因此任务栏、菜单栏和 Dock 等系统保留区域自动属于禁区。

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

/// 屏幕上允许 HUD 进行布局的活动区域。
#[derive(Clone, Copy)]
pub(crate) struct ActivityArea {
    /// 活动区左上角的屏幕物理坐标。
    pub(crate) position: PhysicalPosition<i32>,
    /// 活动区的物理像素尺寸。
    pub(crate) size: PhysicalSize<u32>,
}

/// 获取窗口所在显示器的活动区域。
pub(crate) fn get(window: &Window) -> Option<ActivityArea> {
    platform::get(window)
}

/// 获取指定屏幕坐标所在显示器的活动区域。
pub(crate) fn get_at(window: &Window, position: PhysicalPosition<i32>) -> Option<ActivityArea> {
    platform::get_at(window, position)
}

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
#[path = "unsupported.rs"]
mod platform;

fn fallback(window: &Window) -> Option<ActivityArea> {
    // 非 Windows 平台暂时使用 winit 提供的当前显示器完整区域。
    let monitor = window.current_monitor()?;
    Some(ActivityArea {
        position: monitor.position(),
        size: monitor.size(),
    })
}
