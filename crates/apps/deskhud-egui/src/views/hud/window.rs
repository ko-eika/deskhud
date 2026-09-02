//! HUD 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::EngineRegistry;
use deskhud_ui::{LayerPreference, UiPreferences};
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
        hud.viewport
            .set_window_layer(window_layer(hud.prefs.hud.layer));
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
        if let WindowEvent::KeyboardInput { event, .. } = event
            && event.state == winit::event::ElementState::Pressed
            && event.logical_key == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
        {
            self.leave_layout_mode();
            self.layout.compact_pending = true;
            self.viewport.set_cursor_hittest(false);
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
        self.layout.selected = active_hud_frames(
            &self.registry,
            &self.prefs,
            self.started.elapsed().as_secs_f32(),
        )
        .first()
        .map(|item| format!("{}/{}", item.plugin_id, item.contribution_id));
        self.layout.adjust_open = true;
        self.layout.adjust_session = self.layout.adjust_session.wrapping_add(1);
        self.layout.window_revision = self.layout.window_revision.wrapping_add(1);
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

    pub(crate) fn should_close(&mut self) -> (bool, Option<UiPreferences>) {
        self.viewport.apply_ui_preferences(&self.prefs);
        let items = self.render_items();
        let result = self.viewport.render(|context, raw_input| {
            view::hud::run(
                context,
                raw_input,
                &mut self.layout,
                &items,
                &mut self.prefs,
            )
        });
        (result.should_close, result.applied_preferences)
    }

    pub(crate) fn apply_preferences(&mut self, prefs: UiPreferences) {
        self.prefs = prefs;
        self.viewport
            .set_window_layer(window_layer(self.prefs.hud.layer));
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
                let default_border_color = item
                    .frame
                    .visuals
                    .iter()
                    .find_map(|visual| match visual {
                        deskhud_engine::HudVisual::Text { color, .. } => {
                            Some([color[0], color[1], color[2]])
                        }
                        _ => None,
                    })
                    .unwrap_or([255; 3]);
                let default_content_color = default_border_color;
                let default_corner_radius = item
                    .frame
                    .visuals
                    .iter()
                    .find_map(|visual| match visual {
                        deskhud_engine::HudVisual::Panel { radius, .. } => {
                            Some((*radius / 160.0).clamp(0.0, 1.0))
                        }
                        _ => None,
                    })
                    .unwrap_or(6.0 / 160.0);
                let legacy_corner_radius = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "border_radius",
                    default_corner_radius,
                );
                let target_x =
                    activity.position.x as f32 + item.layout.x * activity.size.width as f32;
                let target_y =
                    activity.position.y as f32 + item.layout.y * activity.size.height as f32;
                let background_opacity = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "background_opacity",
                    1.0,
                );
                let border_opacity = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "border_opacity",
                    1.0,
                );
                let border_width = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "border_width",
                    1.0 / 6.0,
                );
                let legacy_window_shadow = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "window_shadow",
                    0.0,
                );
                let legacy_content_shadow = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "content_shadow",
                    0.0,
                );
                let shadow_opacity = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "shadow_opacity",
                    0.75_f32.max(legacy_window_shadow.max(legacy_content_shadow)),
                );
                let window_shadow_global = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "window_shadow_mode",
                    0.0,
                ) < 0.5;
                let content_shadow_global = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "content_shadow_mode",
                    0.0,
                ) < 0.5;
                let window_shadow_enabled = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "window_shadow_enabled",
                    1.0,
                ) >= 0.5;
                let content_shadow_enabled = self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "content_shadow_enabled",
                    1.0,
                ) >= 0.5;
                Some(HudRenderItem {
                    key: format!("{}/{}", item.plugin_id, item.contribution_id),
                    frame: item.frame,
                    initial_position: egui::pos2(
                        (target_x - window_position.x as f32) / scale_factor,
                        (target_y - window_position.y as f32) / scale_factor,
                    ),
                    width: item.layout.width,
                    height: item.layout.height,
                    background_enabled: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "background_enabled",
                        1.0,
                    ) >= 0.5,
                    background_opacity,
                    background_blur: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "background_blur",
                        0.0,
                    ),
                    content_opacity: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "content_opacity",
                        1.0,
                    ),
                    shadow_enabled: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "shadow_enabled",
                        if shadow_opacity > f32::EPSILON {
                            1.0
                        } else {
                            0.0
                        },
                    ) >= 0.5,
                    window_shadow_global,
                    content_shadow_global,
                    window_shadow_enabled,
                    content_shadow_enabled,
                    window_shadow: shadow_opacity,
                    window_shadow_blur: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "shadow_blur",
                        self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            "window_shadow_blur",
                            1.0,
                        ),
                    ),
                    window_shadow_distance: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "shadow_distance",
                        self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            "window_shadow_distance",
                            5.0 / 12.0,
                        ),
                    ),
                    window_shadow_angle: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "shadow_angle",
                        self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            "window_shadow_angle",
                            0.125,
                        ),
                    ),
                    window_shadow_color: std::array::from_fn(|channel| {
                        (self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            ["shadow_red", "shadow_green", "shadow_blue"][channel],
                            self.prefs.hud.visual_value(
                                item.plugin_id,
                                item.contribution_id,
                                [
                                    "window_shadow_red",
                                    "window_shadow_green",
                                    "window_shadow_blue",
                                ][channel],
                                0.0,
                            ),
                        ) * 255.0)
                            .round() as u8
                    }),
                    window_custom_shadow: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "window_shadow",
                        0.75,
                    ),
                    window_custom_shadow_blur: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "window_shadow_blur",
                        1.0,
                    ),
                    window_custom_shadow_distance: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "window_shadow_distance",
                        5.0 / 12.0,
                    ),
                    window_custom_shadow_angle: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "window_shadow_angle",
                        0.125,
                    ),
                    window_custom_shadow_color: std::array::from_fn(|channel| {
                        (self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            [
                                "window_shadow_red",
                                "window_shadow_green",
                                "window_shadow_blue",
                            ][channel],
                            0.0,
                        ) * 255.0)
                            .round() as u8
                    }),
                    content_custom_shadow: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "content_shadow",
                        0.75,
                    ),
                    content_custom_shadow_blur: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "content_shadow_blur",
                        1.0,
                    ),
                    content_custom_shadow_distance: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "content_shadow_distance",
                        5.0 / 12.0,
                    ),
                    content_custom_shadow_angle: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "content_shadow_angle",
                        0.125,
                    ),
                    content_custom_shadow_color: std::array::from_fn(|channel| {
                        (self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            [
                                "content_shadow_red",
                                "content_shadow_green",
                                "content_shadow_blue",
                            ][channel],
                            0.0,
                        ) * 255.0)
                            .round() as u8
                    }),
                    content_color: std::array::from_fn(|channel| {
                        (self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            ["content_red", "content_green", "content_blue"][channel],
                            default_content_color[channel] as f32 / 255.0,
                        ) * 255.0)
                            .round() as u8
                    }),
                    border_enabled: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "border_enabled",
                        1.0,
                    ) >= 0.5,
                    border_opacity,
                    border_width,
                    corner_radius: self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "corner_radius",
                        legacy_corner_radius,
                    ),
                    border_color: std::array::from_fn(|channel| {
                        (self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            ["border_red", "border_green", "border_blue"][channel],
                            default_border_color[channel] as f32 / 255.0,
                        ) * 255.0)
                            .round() as u8
                    }),
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

fn window_layer(layer: LayerPreference) -> WindowLayer {
    match layer {
        LayerPreference::Top => WindowLayer::AlwaysOnTop,
        LayerPreference::Normal => WindowLayer::Normal,
        LayerPreference::Bottom => WindowLayer::AlwaysOnBottom,
    }
}
