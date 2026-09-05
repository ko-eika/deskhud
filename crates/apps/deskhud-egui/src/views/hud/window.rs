//! HUD 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::EngineRegistry;
use deskhud_ui::{HudConfigValue, LayerPreference, UiPreferences};
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
use crate::views::hud::DEFAULT_SHADOW_BLUR;

use crate::area::{self, ActivityArea};
use crate::views as view;

use super::{HudLayoutTarget, HudRenderItem, LayoutState, resolved_hud_slots};

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
            layout: LayoutState {
                // Keep newly opened layout sessions aspect-ratio safe by
                // default; the adjustment panel can still toggle this off.
                lock_ratio: true,
                ..LayoutState::default()
            },
            activity_area,
            layout_restore_layer: None,
            prefs,
            registry,
            started: Instant::now(),
        };
        hud.apply_window_preset();
        hud.viewport
            .set_window_layer(window_layer(hud.prefs.hud.layer));
        // HUD 普通显示时只提供视觉叠加，不应拦截下面应用的鼠标输入。
        hud.viewport.set_cursor_hittest(false);
        hud
    }

    pub(crate) fn show(&mut self) {
        if !self.layout.layout_mode {
            self.apply_window_preset();
        }
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
        self.layout.scale_factor = scale;
        self.layout.activity_size = Some(egui::vec2(
            activity.size.width as f32 / scale,
            activity.size.height as f32 / scale,
        ));
        // Persisted HUD position and slot x/y use the same physical-pixel
        // coordinate system. Only the egui canvas projection is logical.
        let activity_origin = egui::pos2(activity.position.x as f32, activity.position.y as f32);
        let window_origin = egui::pos2(
            self.prefs.hud.window_position[0] as f32,
            self.prefs.hud.window_position[1] as f32,
        );
        // Positions are relative to the current editor canvas. Rebuild them
        // from persisted slots each time, rather than reusing stale window
        // coordinates from the previous layout session.
        self.layout.active_hud_drag = None;
        self.layout.root_dragging = false;
        self.layout.positions.clear();
        self.layout.transient_group_sizes.clear();
        let slots = resolved_hud_slots(
            &self.registry,
            &self.prefs,
            self.started.elapsed().as_secs_f32(),
            true,
        );
        self.layout.activity_origin = Some(activity_origin);
        let scale = self.layout.scale_factor.max(1.0);
        self.layout.absolute_positions = slots
            .iter()
            .map(|item| {
                let absolute = window_origin + egui::vec2(item.layout.x, item.layout.y);
                (item.key.clone(), absolute)
            })
            .collect();
        self.layout.positions = self
            .layout
            .absolute_positions
            .iter()
            .map(|(key, position)| (key.clone(), *position - activity_origin.to_vec2()))
            .map(|(key, position)| (key, position / scale))
            .collect();
        // Entering layout mode starts with no target selected. The editor
        // panels are opened only after the user selects a HUD or a group.
        self.layout.selected = None;
        self.layout.adjustment_selection = None;
        self.layout.adjustment_order.clear();
        self.layout.adjustment_reset_sizes.clear();
        self.layout.adjust_open = false;
        self.layout.hud_adjust_open = false;
        self.layout.group_adjust_open = false;
        self.layout.hud_adjust_key = None;
        self.layout.group_adjust_key = None;
        self.layout.adjust_session = self.layout.adjust_session.wrapping_add(1);
        self.layout.window_revision = self.layout.window_revision.wrapping_add(1);
        self.layout.finish_layout_requested = false;
        self.layout.layout_mode = true;
        // 布局模式需要接收鼠标，才能拖动 HUD 面板。
        self.viewport.set_cursor_hittest(true);
        self.viewport.request_outer_position(activity.position);
        self.viewport
            .request_inner_size(PhysicalSize::new(activity.size.width, activity.size.height));
    }

    fn leave_layout_mode(&mut self) {
        self.layout.finish_layout_requested = false;
        if !self.layout.layout_mode {
            return;
        }
        // A grouped HUD is detached for the duration of a drag. If layout
        // mode is closed before a pointer-up frame arrives, finish it as a
        // screen HUD so no transient drag state or stale membership remains.
        super::drawing::finish_active_hud_drag_as_screen(&mut self.layout, &mut self.prefs);
        super::drawing::sync_absolute_positions(&mut self.layout);
        // Calculate the compact window before leaving the editor. This lets
        // the next normal frame use a stable preset instead of resizing from
        // whatever content happened to be visible in the previous frame.
        let items = self.render_items();
        if let Some((position, size)) = compact_geometry(
            &items,
            &self.layout,
            self.layout.activity_size,
            self.viewport.window().scale_factor() as f32,
        ) {
            let window_origin = egui::pos2(position[0] as f32, position[1] as f32);
            for (key, absolute) in &self.layout.absolute_positions {
                let local = *absolute - window_origin.to_vec2();
                if let Some(id) = key.strip_prefix("group/") {
                    if let Some(group) = self.prefs.hud.groups.iter_mut().find(|g| g.id == id) {
                        group.layout.x = local.x.max(0.0);
                        group.layout.y = local.y.max(0.0);
                    }
                } else if let Some(id) = key.strip_prefix("instance/")
                    && let Some(instance) = self
                        .prefs
                        .hud
                        .instances
                        .iter_mut()
                        .find(|i| i.id.as_str() == id)
                {
                    instance.layout.x = local.x.max(0.0);
                    instance.layout.y = local.y.max(0.0);
                }
            }
            self.prefs.hud.window_position = position;
            self.prefs.hud.window_size = size;
            self.apply_window_preset();
        }
        self.layout.layout_mode = false;
        // The editor keeps canvas coordinates only for the duration of the
        // session. Normal mode must rebuild positions from the translated
        // persisted slots and the compact window origin.
        self.layout.positions.clear();
        self.layout.root_dragging = false;
        self.layout.absolute_positions.clear();
        self.layout.activity_origin = None;
        self.layout.transient_group_sizes.clear();
        if let Some(layer) = self.layout_restore_layer.take() {
            self.viewport.set_window_layer(layer);
        }
    }

    pub(crate) fn should_close(&mut self) -> (bool, Option<UiPreferences>) {
        if !self.layout.layout_mode {
            self.apply_window_preset();
        }
        let prefs_before = self.prefs.clone();
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
        if self.layout.finish_layout_requested {
            self.leave_layout_mode();
            self.layout.compact_pending = true;
            self.viewport.set_cursor_hittest(false);
        }
        super::drawing::sync_absolute_positions(&mut self.layout);
        // Keep the post-layout normal-mode geometry warm while the activity
        // canvas is still expanded. The native window remains full-screen;
        // only the persisted preset is updated here.
        if self.layout.layout_mode
            && let Some((position, size)) = compact_geometry(
                &items,
                &self.layout,
                self.layout.activity_size,
                self.viewport.window().scale_factor() as f32,
            )
        {
            self.prefs.hud.window_position = position;
            self.prefs.hud.window_size = size;
        }
        let applied_preferences = (self.prefs != prefs_before)
            .then(|| self.prefs.clone())
            .or(result.applied_preferences);
        (result.should_close, applied_preferences)
    }

    pub(crate) fn apply_preferences(&mut self, prefs: UiPreferences) {
        self.prefs = prefs;
        self.viewport
            .set_window_layer(window_layer(self.prefs.hud.layer));
        self.layout.positions.clear();
        if !self.layout.layout_mode {
            self.apply_window_preset();
        }
    }

    fn apply_window_preset(&self) {
        let size = PhysicalSize::new(self.prefs.hud.window_size[0], self.prefs.hud.window_size[1]);
        if self.viewport.window().inner_size() != size {
            self.viewport.request_inner_size(size);
        }
        let position = winit::dpi::PhysicalPosition::new(
            self.prefs.hud.window_position[0],
            self.prefs.hud.window_position[1],
        );
        if self.viewport.window().outer_position().ok() != Some(position) {
            self.viewport.request_outer_position(position);
        }
    }

    fn render_items(&self) -> Vec<HudRenderItem> {
        let elapsed = self.started.elapsed().as_secs_f32();
        resolved_hud_slots(
            &self.registry,
            &self.prefs,
            elapsed,
            self.layout.layout_mode,
        )
        .into_iter()
        .filter_map(|item| {
            if item.layout.display != "primary" {
                return None;
            }
            let default_border_color = item
                .frame
                .visuals
                .iter()
                .filter_map(|visual| match visual {
                    deskhud_engine::HudVisual::Text { color, .. }
                    | deskhud_engine::HudVisual::Label { color, .. } => {
                        Some([color[0], color[1], color[2]])
                    }
                    _ => None,
                })
                .max_by_key(|color| u32::from(color[0]) + u32::from(color[1]) + u32::from(color[2]))
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
            let instance_value = |name: &str, default: f32| {
                item.config
                    .get(name)
                    .and_then(config_f32)
                    .unwrap_or_else(|| {
                        self.prefs.hud.visual_value(
                            item.plugin_id,
                            item.contribution_id,
                            name,
                            default,
                        )
                    })
                    .clamp(0.0, 1.0)
            };
            let background_opacity = instance_value("background_opacity", 1.0);
            let group_color = match &item.target {
                HudLayoutTarget::Group(group_id) => self
                    .prefs
                    .hud
                    .groups
                    .iter()
                    .find(|group| &group.id == group_id)
                    .map(|group| group.color),
                HudLayoutTarget::Instance(_) => None,
            };
            let group_padding = match &item.target {
                HudLayoutTarget::Group(group_id) => self
                    .prefs
                    .hud
                    .groups
                    .iter()
                    .find(|group| &group.id == group_id)
                    .map(|group| group.inner.padding),
                HudLayoutTarget::Instance(_) => None,
            };
            let container_size = match &item.target {
                HudLayoutTarget::Group(group_id) => self
                    .prefs
                    .hud
                    .groups
                    .iter()
                    .find(|group| &group.id == group_id)
                    .and_then(|group| {
                        let transient = self.layout.transient_group_sizes.get(group_id);
                        ((group.layout.width > 0.0 && group.layout.height > 0.0)
                            || transient.is_some())
                        .then_some(egui::vec2(
                            transient.map_or(group.layout.width, |size| size.x),
                            transient.map_or(group.layout.height, |size| size.y),
                        ))
                    }),
                HudLayoutTarget::Instance(_) => None,
            };
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
            let plugin_name = self
                .registry
                .plugin_infos()
                .into_iter()
                .find(|plugin| plugin.id == item.plugin_id)
                .map_or_else(
                    || item.plugin_id.to_owned(),
                    |plugin| plugin.display_name.to_owned(),
                );
            let contribution_name = self
                .registry
                .all_hud_contributions()
                .into_iter()
                .find(|(plugin_id, contribution)| {
                    *plugin_id == item.plugin_id && contribution.id == item.contribution_id
                })
                .map_or_else(
                    || item.contribution_id.to_owned(),
                    |(_, contribution)| contribution.label.to_owned(),
                );
            Some(HudRenderItem {
                key: item.key.clone(),
                target: item.target,
                source: item.source,
                plugin_name,
                contribution_name,
                layers: item.layers,
                base_size: item.base_size,
                container_size,
                initial_position: if self.layout.layout_mode {
                    self.layout
                        .positions
                        .get(&item.key)
                        .copied()
                        .unwrap_or_else(|| egui::pos2(item.layout.x, item.layout.y))
                } else {
                    // Normal mode is already rendering in the HUD window's
                    // own coordinate system. Do not subtract the native
                    // window position here: move/resize requests are async,
                    // so outer_position() can still describe the previous
                    // layout window for one or more frames.
                    let scale = self.viewport.window().scale_factor() as f32;
                    egui::pos2(item.layout.x / scale, item.layout.y / scale)
                },
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
                window_shadow: shadow_opacity,
                window_shadow_blur: self.prefs.hud.visual_value(
                    item.plugin_id,
                    item.contribution_id,
                    "shadow_blur",
                    self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        "window_shadow_blur",
                        DEFAULT_SHADOW_BLUR,
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
                    DEFAULT_SHADOW_BLUR,
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
                    DEFAULT_SHADOW_BLUR,
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
                // Keep the editor outline in sync with the instance-owned
                // radius used by the renderer. Falling back to source-level
                // preferences preserves legacy HUDs without an override.
                corner_radius: instance_value("corner_radius", legacy_corner_radius),
                border_color: std::array::from_fn(|channel| {
                    (self.prefs.hud.visual_value(
                        item.plugin_id,
                        item.contribution_id,
                        ["border_red", "border_green", "border_blue"][channel],
                        default_border_color[channel] as f32 / 255.0,
                    ) * 255.0)
                        .round() as u8
                }),
                group_color,
                group_padding,
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

/// Returns the smallest normal-mode window containing all editor items.
/// Coordinates are kept in physical screen pixels, matching `hud.position`.
fn compact_geometry(
    items: &[HudRenderItem],
    layout: &LayoutState,
    activity: Option<egui::Vec2>,
    scale: f32,
) -> Option<([i32; 2], [u32; 2])> {
    let activity = activity?;
    let mut bounds = egui::Rect::NOTHING;
    for item in items {
        let size = item.container_size.unwrap_or_else(|| {
            egui::vec2(
                item.base_size.width * item.width,
                item.base_size.height * item.height,
            )
        }) * scale;
        let position = layout
            .absolute_positions
            .get(&item.key)
            .copied()
            .unwrap_or(egui::pos2(
                layout.activity_origin?.x + item.initial_position.x * scale,
                layout.activity_origin?.y + item.initial_position.y * scale,
            ));
        bounds = bounds.union(egui::Rect::from_min_size(position, size));
    }
    if !bounds.is_positive() {
        return None;
    }
    let padding = 12.0 * scale;
    let min = bounds.min - egui::vec2(padding, padding);
    let size = bounds.size() + egui::vec2(padding * 2.0, padding * 2.0);
    let max_size = activity * scale;
    let size = size.min(max_size).max(egui::vec2(160.0, 100.0));
    Some((
        [min.x.round() as i32, min.y.round() as i32],
        [
            size.x.round().max(1.0) as u32,
            size.y.round().max(1.0) as u32,
        ],
    ))
}

fn config_f32(value: &HudConfigValue) -> Option<f32> {
    match value {
        HudConfigValue::Float(value) => Some(*value as f32),
        HudConfigValue::Int(value) => Some(*value as f32),
        _ => None,
    }
}

fn window_layer(layer: LayerPreference) -> WindowLayer {
    match layer {
        LayerPreference::Top => WindowLayer::AlwaysOnTop,
        LayerPreference::Normal => WindowLayer::Normal,
        LayerPreference::Bottom => WindowLayer::AlwaysOnBottom,
    }
}
