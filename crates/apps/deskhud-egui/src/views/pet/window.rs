//! Pet 原生窗口生命周期。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::{
    DockState, EngineRegistry, PetConfigBag, PetEvent, PetKind, PetModifiers, PetMouseButton,
};
use deskhud_ui::{LayerPreference, UiPreferences};
use std::{sync::Arc, time::Instant};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::area;
use crate::runtime::{
    viewport::{UserEvent, Viewport, WindowLayer},
    viewport_config::ViewportConfig,
};
use crate::views as view;

pub(crate) struct PetWindow {
    /// Pet 对应的通用视口运行时。
    viewport: Viewport,
    activity_area: Option<area::ActivityArea>,
    native_position: Option<winit::dpi::PhysicalPosition<i32>>,
    native_size: winit::dpi::PhysicalSize<u32>,
    pet: Arc<dyn PetKind>,
    prefs: UiPreferences,
    started: Instant,
    last_tick: Instant,
    last_hit: bool,
    last_drag: bool,
    position_dirty: bool,
    snap_frames: u8,
    last_dock: DockState,
    last_scene: deskhud_engine::PetScene,
    last_global_mouse: crate::input::GlobalMouseButtons,
    local_modifiers: PetModifiers,
    last_click: Option<(PetMouseButton, Instant)>,
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
        let activity_area = area::get(viewport.window());
        let pet = registry
            .pets()
            .into_iter()
            .find(|pet| pet.info().id == prefs.pet.kind)
            .unwrap_or_else(|| registry.active_pet());
        viewport.set_window_layer(window_layer(prefs.pet.layer));
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
            activity_area,
            native_position: prefs.pet.position().map(|position| {
                winit::dpi::PhysicalPosition::new(
                    position.x.round() as i32,
                    position.y.round() as i32,
                )
            }),
            native_size: winit::dpi::PhysicalSize::new(
                prefs.pet.width.max(1.0) as u32,
                prefs.pet.height.max(1.0) as u32,
            ),
            pet,
            prefs,
            started: Instant::now(),
            last_tick: Instant::now(),
            last_hit: false,
            last_drag: false,
            position_dirty: false,
            snap_frames: 0,
            last_dock: DockState::FREE,
            last_scene: deskhud_engine::PetScene::default(),
            last_global_mouse: crate::input::GlobalMouseButtons::default(),
            local_modifiers: PetModifiers::NONE,
            last_click: None,
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
        match event {
            WindowEvent::Moved(position) => {
                self.native_position = Some(*position);
            }
            WindowEvent::Resized(size) => {
                self.native_size = *size;
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.local_modifiers = PetModifiers {
                    shift: modifiers.state().shift_key(),
                    ctrl: modifiers.state().control_key(),
                    alt: modifiers.state().alt_key(),
                    meta: modifiers.state().super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
                    && let Some(key) = crate::input::winit_key_to_pet_key(code)
                {
                    self.pet.on_event(if event.state == ElementState::Pressed {
                        PetEvent::KeyPressed {
                            key,
                            modifiers: self.local_modifiers,
                        }
                    } else {
                        PetEvent::KeyReleased {
                            key,
                            modifiers: self.local_modifiers,
                        }
                    });
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y.signum() as i8,
                    MouseScrollDelta::PixelDelta(position) => position.y.signum() as i8,
                };
                if delta != 0 {
                    self.pet.on_event(PetEvent::MouseWheel {
                        delta,
                        modifiers: self.local_modifiers,
                    });
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(position) = self.viewport.cursor_position() else {
                    self.viewport.handle_event(event);
                    return;
                };
                let size = self.viewport.window_handle().inner_size();
                let center = [size.width as f32 / 2.0, size.height as f32 / 2.0];
                let base = size.width.min(size.height) as f32 * 0.32;
                let point = [
                    (position.x as f32 - center[0]) / base.max(1.0),
                    (position.y as f32 - center[1]) / base.max(1.0),
                ];
                if self.last_scene.hit_test(point) {
                    let pet_button = match button {
                        MouseButton::Left => Some(PetMouseButton::Primary),
                        MouseButton::Right | MouseButton::Other(3) => {
                            Some(PetMouseButton::Secondary)
                        }
                        MouseButton::Middle => Some(PetMouseButton::Middle),
                        _ => None,
                    };
                    if let Some(button) = pet_button {
                        let event = match state {
                            ElementState::Pressed => PetEvent::MousePressed {
                                button,
                                modifiers: self.local_modifiers,
                            },
                            ElementState::Released => PetEvent::MouseReleased {
                                button,
                                modifiers: self.local_modifiers,
                            },
                        };
                        self.pet.on_event(event);
                        if *state == ElementState::Released {
                            let now = Instant::now();
                            let double_click = self.last_click.is_some_and(|(last, at)| {
                                last == button && now.duration_since(at).as_millis() <= 500
                            });
                            self.pet.on_event(if double_click {
                                PetEvent::MouseDoubleClicked {
                                    button,
                                    modifiers: self.local_modifiers,
                                }
                            } else {
                                PetEvent::MouseClicked {
                                    button,
                                    modifiers: self.local_modifiers,
                                }
                            });
                            self.last_click = Some((button, now));
                        }
                    }
                }
            }
            _ => {}
        }
        self.viewport.handle_event(event);
    }

    /// 转发已经由平台壳归一化的全局输入事件。
    pub(crate) fn dispatch_event(&self, event: PetEvent) {
        self.pet.on_event(event);
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

    pub(crate) fn bubble_content(&self) -> Option<crate::views::bubble::BubbleContent> {
        if !self.prefs.pet.bubbles {
            return None;
        }
        self.last_scene
            .items
            .iter()
            .find_map(|item| match &item.node {
                deskhud_engine::SceneNode::Bubble {
                    text,
                    color,
                    background,
                    corner_radius,
                } => Some(crate::views::bubble::BubbleContent {
                    text: text.clone(),
                    color: *color,
                    background: *background,
                    corner_radius: *corner_radius,
                }),
                _ => None,
            })
    }

    pub(crate) fn bubble_anchor(&self) -> Option<winit::dpi::PhysicalPosition<i32>> {
        let position = self.native_position?;
        let size = self.native_size;
        let center = winit::dpi::PhysicalPosition::new(
            position.x + size.width as i32 / 2,
            position.y + size.height as i32 / 2,
        );
        // 气泡位置在渲染线程中计算。macOS 上 `NSScreen` 只能在主线程访问，
        // 因此这里使用 `get_at`：它会回退到 winit 的显示器范围，而不是让
        // 锚点丢失并把气泡错误地放到屏幕左上角。
        let Some(area) = area::get_at(self.viewport.window(), center) else {
            return Some(winit::dpi::PhysicalPosition::new(
                center.x,
                position.y - 52_i32 / 2 - 12,
            ));
        };
        let bubble_width = 180_i32;
        let bubble_height = 52_i32;
        let min_x = area.position.x + bubble_width / 2;
        let max_x = area.position.x + area.size.width as i32 - bubble_width / 2;
        let x = center.x.clamp(min_x, max_x.max(min_x));
        let above = position.y - bubble_height - 12 >= area.position.y;
        let y = if above {
            position.y - bubble_height / 2 - 12
        } else {
            (position.y + size.height as i32 + bubble_height / 2 + 12)
                .min(area.position.y + area.size.height as i32 - bubble_height / 2)
        };
        Some(winit::dpi::PhysicalPosition::new(x, y))
    }

    /// 应用设置页刚提交的宠物选择、尺寸、层级和位置。
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
        self.native_size = winit::dpi::PhysicalSize::new(
            prefs.pet.width.max(1.0) as u32,
            prefs.pet.height.max(1.0) as u32,
        );
        self.native_position = prefs.pet.position().map(|position| {
            winit::dpi::PhysicalPosition::new(position.x.round() as i32, position.y.round() as i32)
        });
        self.viewport
            .request_inner_size(winit::dpi::PhysicalSize::new(
                prefs.pet.width.max(48.0) as u32,
                prefs.pet.height.max(48.0) as u32,
            ));
        self.viewport
            .set_window_layer(window_layer(prefs.pet.layer));
        if let Some(position) = prefs.pet.position() {
            self.viewport
                .request_outer_position(winit::dpi::PhysicalPosition::new(
                    position.x.round() as i32,
                    position.y.round() as i32,
                ));
        }
    }

    /// 返回原生窗口的最新位置，供设置 Apply 保留拖拽产生的几何状态。
    pub(crate) fn current_position(&self) -> Option<winit::dpi::PhysicalPosition<i32>> {
        self.viewport.window().outer_position().ok()
    }

    /// Copies the final native position into the applied preferences before
    /// the renderer is shut down.
    pub(crate) fn sync_position(&mut self) {
        if let Some(position) = self.current_position() {
            self.prefs.pet.set_pos(position.x as f32, position.y as f32);
        }
    }

    /// Returns whether a completed drag changed the persisted native position.
    pub(crate) fn take_position_dirty(&mut self) -> bool {
        let dirty = self.position_dirty;
        self.position_dirty = false;
        dirty
    }

    /// 返回 Pet 当前的窗口层级。
    pub(crate) fn window_layer(&self) -> WindowLayer {
        self.viewport.window_layer()
    }

    pub(crate) fn set_window_layer(&mut self, layer: WindowLayer) {
        self.viewport.set_window_layer(layer);
    }

    /// 绘制 Pet 一帧，并返回是否请求退出应用。
    pub(crate) fn render(&mut self) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f32().min(0.25);
        self.last_tick = now;
        let was_dragging = self.last_drag;
        let dock = self.current_dock();
        let info = self.pet.info();
        let options: Vec<(&str, bool)> = self
            .pet
            .config_options()
            .iter()
            .map(|option| (option.key, option.default))
            .collect();
        let config = self.prefs.pet.short_map_for(info.id, &options);
        self.pet.apply_config(PetConfigBag::new(&config));
        if dock != self.last_dock {
            self.pet.on_event(PetEvent::DockChanged {
                from: self.last_dock,
                to: dock,
            });
            self.last_dock = dock;
        }
        self.viewport.apply_ui_preferences(&self.prefs);
        let window = self.viewport.window();
        let window_size = window.outer_size();
        let screen_center = window.outer_position().ok().map(|position| {
            [
                position.x as f64 + window_size.width as f64 / 2.0,
                position.y as f64 + window_size.height as f64 / 2.0,
            ]
        });
        let result = self.viewport.render(|context, raw_input| {
            view::pet::run(
                context,
                raw_input,
                self.pet.as_ref(),
                &self.prefs,
                self.started.elapsed().as_secs_f32(),
                &mut self.last_hit,
                dock,
                &mut self.last_drag,
                dt,
                &mut self.last_scene,
                &mut self.last_global_mouse,
                screen_center,
                [window_size.width as f64, window_size.height as f64],
                self.prefs.pet.global_mouse_input,
            )
        });
        if was_dragging && !self.last_drag {
            self.snap_frames = 4;
        }
        if !self.last_drag && self.snap_frames > 0 {
            self.snap_to_activity_area();
            self.snap_frames -= 1;
        }
        // 透明区域慢速拖动时，窗口移动可能让 egui 丢失释放边沿；以全局
        // 鼠标快照兜底，确保释放后仍会执行严格回弹。
        if !self.last_drag
            && !crate::input::global_mouse_buttons().primary_down
            && self.is_outside_activity_area()
        {
            self.snap_to_activity_area();
        }
        // Native drag can lose the release edge when the pointer crosses a
        // transparent region. The global snapshot is authoritative on every
        // platform that provides it, so recover the state and persist the
        // final native position as soon as the drag ends.
        let recovered_drag = self.last_drag && !crate::input::global_mouse_buttons().primary_down;
        if recovered_drag {
            self.pet.on_event(PetEvent::DragEnded {
                drag: deskhud_engine::DragState::IDLE,
            });
            self.last_drag = false;
            self.snap_frames = 4;
        }
        // Do not sample and save on ordinary startup frames: the asynchronous
        // initial Move command may not have reached the native window yet.
        // Only a completed drag (or its short snap settling period) authorizes
        // replacing the persisted position.
        if !self.last_drag && (was_dragging || recovered_drag || self.snap_frames > 0) {
            if let Some(position) = self.current_position() {
                let current = deskhud_ui::PetPosition {
                    x: position.x as f32,
                    y: position.y as f32,
                };
                if self.prefs.pet.position() != Some(current) {
                    self.prefs.pet.set_pos(current.x, current.y);
                    self.position_dirty = true;
                }
            }
        }
        result.should_close
    }

    fn current_dock(&self) -> deskhud_engine::DockState {
        let Some(position) = self.viewport.window().outer_position().ok() else {
            return deskhud_engine::DockState::FREE;
        };
        let Some(area) = self.activity_area else {
            return deskhud_engine::DockState::FREE;
        };
        let size = self.viewport.window().outer_size();
        let right = position.x + size.width as i32;
        let bottom = position.y + size.height as i32;
        let tolerance = 16;
        deskhud_engine::DockState {
            left: position.x <= area.position.x + tolerance,
            top: position.y <= area.position.y + tolerance,
            right: right >= area.position.x + area.size.width as i32 - tolerance,
            bottom: bottom >= area.position.y + area.size.height as i32 - tolerance,
        }
    }

    fn snap_to_activity_area(&self) {
        let window = self.viewport.window();
        let Some(area) = self.activity_area else {
            return;
        };
        let Ok(position) = window.outer_position() else {
            return;
        };
        let size = window.outer_size();
        let right = position.x + size.width as i32;
        let bottom = position.y + size.height as i32;
        let area_right = area.position.x + area.size.width as i32;
        let area_bottom = area.position.y + area.size.height as i32;
        let mut snapped = position;
        if position.x <= area.position.x + 16 {
            snapped.x = area.position.x;
        } else if right >= area_right - 16 {
            snapped.x = area_right - size.width as i32;
        }
        if position.y <= area.position.y + 16 {
            snapped.y = area.position.y;
        } else if bottom >= area_bottom - 16 {
            snapped.y = area_bottom - size.height as i32;
        }
        if snapped != position {
            self.viewport.request_outer_position(snapped);
        }
    }

    fn is_outside_activity_area(&self) -> bool {
        let Some(area) = self.activity_area else {
            return false;
        };
        let window = self.viewport.window();
        let Ok(position) = window.outer_position() else {
            return false;
        };
        let size = window.outer_size();
        position.x < area.position.x
            || position.y < area.position.y
            || position.x + size.width as i32 > area.position.x + area.size.width as i32
            || position.y + size.height as i32 > area.position.y + area.size.height as i32
    }

    /// 按正确的 OpenGL 资源顺序销毁 Pet 窗口。
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
