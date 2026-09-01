//! Settings 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::EngineRegistry;
use deskhud_ui::{CatalogStore, LayerPreference, SettingsModel, UiPreferences};
use std::sync::Arc;
use winit::{
    dpi::PhysicalPosition,
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
    registry: Arc<EngineRegistry>,
    catalogs: CatalogStore,
    model: SettingsModel,
}

impl SettingsWindow {
    /// 创建 Settings 窗口。
    pub(crate) fn create(
        event_loop: &ActiveEventLoop,
        proxy: &EventLoopProxy<UserEvent>,
        registry: Arc<EngineRegistry>,
        catalogs: CatalogStore,
        prefs: UiPreferences,
    ) -> Self {
        Self {
            viewport: {
                let viewport = Viewport::new(event_loop, ViewportConfig::settings(), proxy);
                let size = prefs.shell.settings_size();
                viewport.request_inner_size(winit::dpi::PhysicalSize::new(
                    size[0].round() as u32,
                    size[1].round() as u32,
                ));
                if let Some(position) = prefs.shell.settings_pos() {
                    viewport.request_outer_position(winit::dpi::PhysicalPosition::new(
                        position[0].round() as i32,
                        position[1].round() as i32,
                    ));
                }
                viewport
            },
            registry,
            catalogs,
            model: SettingsModel::new(prefs),
        }
    }

    /// 显示 Settings 窗口。
    pub(crate) fn show(&mut self, prefs: &UiPreferences) {
        // 显示由渲染线程发起，具体的原生窗口调用会转回 winit 线程。
        self.model.open(prefs);
        // A decorated window may remain minimized while its taskbar button is
        // still present. Restore it before making it active so opening Settings
        // always produces a visible window.
        self.viewport.window().set_minimized(false);
        self.restore_if_offscreen();
        self.viewport.set_visible(true);
        self.viewport.window().focus_window();
    }

    /// 隐藏 Settings 窗口。
    pub(crate) fn hide(&mut self) {
        self.viewport.set_visible(false);
        self.viewport.request_surface_compaction();
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

    pub(crate) fn geometry(&mut self) -> [Option<f32>; 4] {
        self.sync_geometry();
        [
            self.model.draft.shell.settings_width,
            self.model.draft.shell.settings_height,
            self.model.draft.shell.settings_pos_x,
            self.model.draft.shell.settings_pos_y,
        ]
    }

    pub(crate) fn sync_pet_layer(&mut self, layer: LayerPreference) {
        self.model.sync_pet_layer(layer);
    }

    pub(crate) fn sync_hud_layer(&mut self, layer: LayerPreference) {
        self.model.sync_hud_layer(layer);
    }

    pub(crate) fn sync_hud_master_enabled(&mut self, enabled: bool) {
        self.model.sync_hud_master_enabled(enabled);
    }

    /// 绘制一帧并返回 UI 是否请求关闭。
    pub(crate) fn render(&mut self) -> (bool, Option<UiPreferences>) {
        // Capture native changes before the Apply command snapshots the draft.
        self.sync_geometry();
        self.viewport.apply_ui_preferences(&self.model.draft);
        self.viewport
            .set_titlebar_theme(self.model.draft.shell.ui_theme);
        let output = self.viewport.render(|context, raw_input| {
            view::setting::run(
                context,
                raw_input,
                &self.registry,
                &self.catalogs,
                &mut self.model,
            )
        });
        (output.should_close, output.applied_preferences)
    }

    pub(crate) fn maintain_surface(&mut self) {
        self.viewport.maintain_surface();
    }

    fn sync_geometry(&mut self) {
        let size = self.viewport.window().inner_size();
        if let Ok(position) = self.viewport.window().outer_position() {
            self.model.draft.shell.set_settings_geometry(
                size.width as f32,
                size.height as f32,
                position.x as f32,
                position.y as f32,
            );
        }
    }

    /// 释放 Settings 的 OpenGL 资源。
    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }

    /// Moves a previously saved settings window back onto the current monitor
    /// when its old monitor was disconnected or its saved position is invalid.
    fn restore_if_offscreen(&self) {
        let window = self.viewport.window();
        let Some(monitor) = window.current_monitor() else {
            return;
        };
        let Ok(position) = window.outer_position() else {
            return;
        };
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let window_size = window.outer_size();
        let left = i64::from(position.x);
        let top = i64::from(position.y);
        let right = left + i64::from(window_size.width);
        let bottom = top + i64::from(window_size.height);
        let monitor_left = i64::from(monitor_position.x);
        let monitor_top = i64::from(monitor_position.y);
        let monitor_right = monitor_left + i64::from(monitor_size.width);
        let monitor_bottom = monitor_top + i64::from(monitor_size.height);
        let visible = right > monitor_left
            && left < monitor_right
            && bottom > monitor_top
            && top < monitor_bottom;
        if visible {
            return;
        }
        let x =
            monitor_position.x + (monitor_size.width.saturating_sub(window_size.width) / 2) as i32;
        let y = monitor_position.y
            + (monitor_size.height.saturating_sub(window_size.height) / 2) as i32;
        self.viewport
            .request_outer_position(PhysicalPosition::new(x, y));
    }
}
