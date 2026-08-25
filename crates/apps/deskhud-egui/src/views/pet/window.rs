//! Pet 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::runtime::{
    viewport::{UserEvent, Viewport, WindowLayer},
    viewport_config::ViewportConfig,
};
use crate::views as view;

pub(crate) struct PetWindow {
    /// Pet 对应的通用视口运行时。
    viewport: Viewport,
}

impl PetWindow {
    /// 创建 Pet 窗口并立即显示。
    pub(crate) fn create(event_loop: &ActiveEventLoop, proxy: &EventLoopProxy<UserEvent>) -> Self {
        let mut viewport = Viewport::new(event_loop, ViewportConfig::pet(), proxy);
        viewport.set_visible(true);
        Self { viewport }
    }

    /// 返回 Pet 窗口标识。
    pub(crate) fn window_id(&self) -> WindowId {
        self.viewport.window_id()
    }

    /// 返回供主线程执行原生窗口操作的共享句柄。
    pub(crate) fn window_handle(&self) -> std::sync::Arc<winit::window::Window> {
        self.viewport.window_handle()
    }

    /// 将窗口事件交给通用视口处理器。
    pub(crate) fn handle_event(&mut self, event: &WindowEvent) {
        self.viewport.handle_event(event);
    }

    /// 返回最近一次鼠标位置对应的屏幕坐标。
    pub(crate) fn cursor_screen_position(&self) -> Option<winit::dpi::PhysicalPosition<i32>> {
        self.viewport.cursor_screen_position().or_else(|| {
            // 某些 Linux 窗口管理器在首次右键时可能不会先发送 CursorMoved。
            // 使用窗口中心作为兜底锚点，确保菜单仍然可以打开。
            let window = self.viewport.window();
            if let Ok(position) = window.outer_position() {
                let size = window.outer_size();
                return Some(winit::dpi::PhysicalPosition::new(
                    position.x + size.width.saturating_div(2) as i32,
                    position.y + size.height.saturating_div(2) as i32,
                ));
            }
            // Wayland 不提供顶层窗口的全局屏幕坐标。此时不能精确定位，
            // 但仍应打开菜单，让合成器决定其最终位置。
            Some(winit::dpi::PhysicalPosition::new(0, 0))
        })
    }

    /// 切换 Pet 的置顶状态。
    pub(crate) fn toggle_always_on_top(&mut self) {
        self.viewport.toggle_always_on_top();
    }

    /// 返回 Pet 当前的窗口层级。
    pub(crate) fn window_layer(&self) -> WindowLayer {
        self.viewport.window_layer()
    }

    /// 绘制 Pet 一帧，并返回是否请求退出应用。
    pub(crate) fn render(&mut self) -> bool {
        self.viewport.render(view::pet::run).should_close
    }

    /// 按正确的 OpenGL 资源顺序销毁 Pet 窗口。
    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }
}
