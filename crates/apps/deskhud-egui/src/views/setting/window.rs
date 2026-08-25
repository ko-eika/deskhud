//! Settings 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::runtime::{
    viewport::{UserEvent, Viewport},
    viewport_config::ViewportConfig,
};
use crate::views as view;

pub(crate) struct SettingsWindow {
    /// Settings 对应的通用视口运行时。
    viewport: Viewport,
}

impl SettingsWindow {
    /// 创建 Settings 窗口。
    pub(crate) fn create(event_loop: &ActiveEventLoop, proxy: &EventLoopProxy<UserEvent>) -> Self {
        Self {
            viewport: Viewport::new(event_loop, ViewportConfig::settings(), proxy),
        }
    }

    /// 显示 Settings 窗口。
    pub(crate) fn show(&mut self) {
        // 显示由渲染线程发起，具体的原生窗口调用会转回 winit 线程。
        self.viewport.set_visible(true);
    }

    /// 隐藏 Settings 窗口。
    pub(crate) fn hide(&mut self) {
        self.viewport.set_visible(false);
    }

    /// 判断 Settings 是否可见。
    pub(crate) fn is_visible(&self) -> bool {
        self.viewport.is_visible()
    }

    /// 返回 Settings 窗口标识。
    pub(crate) fn window_id(&self) -> WindowId {
        self.viewport.window_id()
    }

    /// 返回供主线程使用的共享窗口句柄。
    pub(crate) fn window_handle(&self) -> std::sync::Arc<winit::window::Window> {
        self.viewport.window_handle()
    }

    /// 将原生窗口事件转交给通用视口。
    pub(crate) fn handle_event(&mut self, event: &WindowEvent) {
        self.viewport.handle_event(event);
    }

    /// 绘制一帧并返回 UI 是否请求关闭。
    pub(crate) fn should_close(&mut self) -> bool {
        self.viewport.render(view::setting::run).should_close
    }

    /// 释放 Settings 的 OpenGL 资源。
    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }
}
