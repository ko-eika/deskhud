//! HUD 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::EngineRegistry;
use deskhud_ui::UiPreferences;
use std::{sync::Arc, time::Instant};
use winit::{
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::runtime::{
    viewport::{UserEvent, Viewport, WindowLayer},
    viewport_config::ViewportConfig,
};

use crate::area::{self, ActivityArea};
use crate::views as view;

use super::{HudRenderItem, LayoutState, active_hud_frames};

pub(crate) struct HudWindow {
    /// HUD 对应的通用视口运行时。
    viewport: Viewport,
    /// HUD 当前的布局状态。
    layout: LayoutState,
    /// HUD 所在显示器的活动区域缓存。
    activity_area: Option<ActivityArea>,
    /// 布局模式期间暂存的用户选择层级；布局模式本身临时强制置顶。
    layout_restore_layer: Option<WindowLayer>,
    /// 所有窗口共享的已应用外观与语言偏好。
    prefs: UiPreferences,
    /// 提供实际插件 HUD contribution 和逐帧内容。
    registry: Arc<EngineRegistry>,
    started: Instant,
}

impl HudWindow {
    /// 创建 HUD 窗口，并缓存其初始活动区域。
    pub(crate) fn create(
        event_loop: &ActiveEventLoop,
        proxy: &EventLoopProxy<UserEvent>,
        registry: Arc<EngineRegistry>,
        prefs: UiPreferences,
    ) -> Self {
        let viewport = Viewport::new(event_loop, ViewportConfig::hud(), proxy);
        let activity_area = area::get(viewport.window());
        let mut hud = Self {
            viewport,
            layout: LayoutState::default(),
            activity_area,
            layout_restore_layer: None,
            prefs,
            registry,
            started: Instant::now(),
        };
        // HUD 普通显示时只提供视觉叠加，不应拦截下面应用的鼠标输入。
        hud.viewport.set_cursor_hittest(false);
        hud
    }

    pub(crate) fn show(&mut self) {
        self.viewport.set_visible(true);
        self.viewport.set_cursor_hittest(self.layout.layout_mode);
    }

    pub(crate) fn hide(&mut self) {
        self.leave_layout_mode();
        self.layout.activity_size = None;
        self.layout.compact_pending = false;
        self.viewport.set_visible(false);
        self.viewport.set_cursor_hittest(false);
        self.viewport.request_surface_compaction();
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.viewport.is_visible()
    }

    pub(crate) fn window_layer(&self) -> WindowLayer {
        self.layout_restore_layer
            .unwrap_or_else(|| self.viewport.window_layer())
    }

    pub(crate) fn set_window_layer(&mut self, layer: WindowLayer) {
        if self.layout.layout_mode {
            // 布局期间原生窗口必须保持置顶，但菜单中的层级选择仍应
            // 成为退出布局后的目标层级。
            self.layout_restore_layer = Some(layer);
        } else {
            self.viewport.set_window_layer(layer);
        }
    }

    pub(crate) fn window_id(&self) -> WindowId {
        self.viewport.window_id()
    }

    pub(crate) fn window_handle(&self) -> std::sync::Arc<winit::window::Window> {
        self.viewport.window_handle()
    }

    pub(crate) fn handle_event(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state == winit::event::ElementState::Pressed
                && event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
            {
                self.leave_layout_mode();
                self.layout.compact_pending = true;
                self.viewport.set_cursor_hittest(false);
            }
        }
        self.viewport.handle_event(event);
    }

    pub(crate) fn enter_layout_mode(&mut self) {
        if self.layout.layout_mode {
            return;
        }
        // 活动区域在窗口创建时由 winit 主线程读取。这里运行在渲染线程，
        // 不能再次访问 macOS AppKit/NSScreen；如果平台查询失败则退回
        // winit 的显示器完整区域，保证布局功能仍可用。
        let Some(activity) = self.activity_area.or_else(|| {
            self.viewport
                .window()
                .current_monitor()
                .map(|monitor| ActivityArea {
                    position: monitor.position(),
                    size: monitor.size(),
                })
        }) else {
            return;
        };
        self.layout_restore_layer = Some(self.viewport.window_layer());
        self.viewport.set_window_layer(WindowLayer::AlwaysOnTop);
        let scale = self.viewport.window().scale_factor() as f32;
        if let Ok(previous_position) = self.viewport.window().inner_position() {
            let delta = egui::vec2(
                (previous_position.x - activity.position.x) as f32 / scale,
                (previous_position.y - activity.position.y) as f32 / scale,
            );
            for position in self.layout.positions.values_mut() {
                *position += delta;
            }
        }
        self.layout.activity_size = Some(egui::vec2(
            activity.size.width as f32 / scale,
            activity.size.height as f32 / scale,
        ));
        self.layout.layout_mode = true;
        // 布局模式需要接收鼠标，才能拖动 HUD 面板。
        self.viewport.set_cursor_hittest(true);
        self.viewport.request_outer_position(activity.position);
        self.viewport
            .request_inner_size(PhysicalSize::new(activity.size.width, activity.size.height));
    }

    fn leave_layout_mode(&mut self) {
        if !self.layout.layout_mode {
            return;
        }
        self.layout.layout_mode = false;
        if let Some(layer) = self.layout_restore_layer.take() {
            self.viewport.set_window_layer(layer);
        }
    }

    pub(crate) fn should_close(&mut self) -> bool {
        self.viewport.apply_ui_preferences(&self.prefs);
        let items = self.render_items();
        self.viewport
            .render(|context, raw_input| {
                view::hud::run(context, raw_input, &mut self.layout, &items)
            })
            .should_close
    }

    pub(crate) fn apply_preferences(&mut self, prefs: UiPreferences) {
        self.prefs = prefs;
        self.layout.positions.clear();
    }

    fn render_items(&self) -> Vec<HudRenderItem> {
        let elapsed = self.started.elapsed().as_secs_f32();
        let window_position = self.viewport.window().outer_position().unwrap_or_default();
        let scale_factor = self.viewport.window().scale_factor() as f32;
        let activity = self.activity_area.unwrap_or(ActivityArea {
            position: window_position,
            size: self.viewport.window().inner_size(),
        });
        active_hud_frames(&self.registry, &self.prefs, elapsed)
            .into_iter()
            .filter_map(|item| {
                if item.layout.display != "primary" {
                    return None;
                }
                let target_x =
                    activity.position.x as f32 + item.layout.x * activity.size.width as f32;
                let target_y =
                    activity.position.y as f32 + item.layout.y * activity.size.height as f32;
                Some(HudRenderItem {
                    key: format!("{}/{}", item.plugin_id, item.contribution_id),
                    frame: item.frame,
                    initial_position: egui::pos2(
                        (target_x - window_position.x as f32) / scale_factor,
                        (target_y - window_position.y as f32) / scale_factor,
                    ),
                    scale: item.layout.scale,
                })
            })
            .collect()
    }

    pub(crate) fn maintain_surface(&mut self) {
        self.viewport.maintain_surface();
    }

    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }
}
