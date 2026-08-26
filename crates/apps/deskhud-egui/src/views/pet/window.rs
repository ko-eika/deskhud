//! Pet 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::{EngineRegistry, PetKind};
use deskhud_ui::UiPreferences;
use std::{sync::Arc, time::Instant};
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
    pet: Arc<dyn PetKind>,
    prefs: UiPreferences,
    started: Instant,
}

impl PetWindow {
    /// 创建 Pet 窗口并立即显示。
    pub(crate) fn create(
        event_loop: &ActiveEventLoop,
        proxy: &EventLoopProxy<UserEvent>,
        registry: Arc<EngineRegistry>,
        prefs: UiPreferences,
    ) -> Self {
        let mut viewport = Viewport::new(event_loop, ViewportConfig::pet(), proxy);
        let pet = registry
            .pets()
            .into_iter()
            .find(|pet| pet.info().id == prefs.pet.kind)
            .unwrap_or_else(|| registry.active_pet());
        if !prefs.shell.topmost {
            viewport.set_window_layer(WindowLayer::Normal);
        }
        viewport.request_inner_size(winit::dpi::PhysicalSize::new(
            prefs.pet.width as u32,
            prefs.pet.height as u32,
        ));
        viewport.set_visible(true);
        if let Some(position) = prefs.pet.position() {
            viewport.request_outer_position(winit::dpi::PhysicalPosition::new(
                position.x.round() as i32,
                position.y.round() as i32,
            ));
        }
        Self {
            viewport,
            pet,
            prefs,
            started: Instant::now(),
        }
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

    /// 应用设置页刚提交的宠物选择、尺寸、置顶和位置。
    pub(crate) fn apply_preferences(&mut self, registry: &EngineRegistry, prefs: UiPreferences) {
        if self.pet.info().id != prefs.pet.kind {
            if let Some(pet) = registry
                .pets()
                .into_iter()
                .find(|pet| pet.info().id == prefs.pet.kind)
            {
                self.pet = pet;
            }
        }
        self.prefs = prefs.clone();
        self.viewport
            .request_inner_size(winit::dpi::PhysicalSize::new(
                prefs.pet.width.max(48.0) as u32,
                prefs.pet.height.max(48.0) as u32,
            ));
        self.viewport.set_window_layer(if prefs.shell.topmost {
            WindowLayer::AlwaysOnTop
        } else {
            WindowLayer::Normal
        });
        if let Some(position) = prefs.pet.position() {
            self.viewport
                .request_outer_position(winit::dpi::PhysicalPosition::new(
                    position.x.round() as i32,
                    position.y.round() as i32,
                ));
        }
    }

    /// 返回 Pet 当前的窗口层级。
    pub(crate) fn window_layer(&self) -> WindowLayer {
        self.viewport.window_layer()
    }

    pub(crate) fn is_always_on_top(&self) -> bool {
        self.viewport.window_layer() == WindowLayer::AlwaysOnTop
    }

    /// 绘制 Pet 一帧，并返回是否请求退出应用。
    pub(crate) fn render(&mut self) -> bool {
        self.pet.tick(1.0 / 60.0);
        self.viewport
            .render(|context, raw_input| {
                view::pet::run(
                    context,
                    raw_input,
                    self.pet.as_ref(),
                    &self.prefs,
                    self.started.elapsed().as_secs_f32(),
                )
            })
            .should_close
    }

    /// 按正确的 OpenGL 资源顺序销毁 Pet 窗口。
    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }
}
