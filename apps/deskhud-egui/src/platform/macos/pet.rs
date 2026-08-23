//! macOS desktop-pet host.
//!
//! The native AppKit pet window, painting, dragging, snapping, keyboard input and
//! topmost are owned here so the egui shell never carries OS-specific pet code.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowLevel};

use deskhud_engine::{
    DockState, DragState, EngineRegistry, MouseState, PetConfigBag, PetEvent, PetModifiers,
    PetMouseButton, PetPaint, PetPaintCtx, PetTheme,
};

use crate::overlay_control::OverlayControlCommand;
use crate::platform::OverlayBackend;

const PET_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Desktop-pet host state.
pub(crate) struct PetHost {
    pet_overlay_id: Option<deskhud_engine::OverlayWindowId>,
    native_pet_window: Option<objc2::rc::Retained<objc2_app_kit::NSWindow>>,
    native_pet_view: Option<objc2::rc::Retained<crate::platform::NativePetView>>,
    pet_started: Instant,
    mac_mouse: MouseState,
    mac_dock: DockState,
    mac_dragging: bool,
    mac_press_cursor: Option<(i32, i32)>,
    mac_press_window: Option<PhysicalPosition<i32>>,
    mac_last_tick: Instant,
}

fn pet_interval_of(prefs: &deskhud_ui::UiPreferences) -> Duration {
    let fps = match prefs.graphics.fps_limit {
        deskhud_ui::FpsLimit::Fps30 => 30,
        deskhud_ui::FpsLimit::Fps120 => 120,
        deskhud_ui::FpsLimit::Auto | deskhud_ui::FpsLimit::Fps60 => 60,
    };
    Duration::from_secs_f64(1.0 / fps as f64)
}

impl PetHost {
    pub(crate) fn new() -> Self {
        PetHost {
            pet_overlay_id: None,
            native_pet_window: None,
            native_pet_view: None,
            pet_started: Instant::now(),
            mac_mouse: MouseState::IDLE,
            mac_dock: DockState::FREE,
            mac_dragging: false,
            mac_press_cursor: None,
            mac_press_window: None,
            mac_last_tick: Instant::now(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_desktop_pet(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    pub(crate) fn resume(
        &mut self,
        prefs: &deskhud_ui::UiPreferences,
        overlay: &mut Box<dyn OverlayBackend>,
        _window: Option<&Window>,
    ) {
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            let (win, view) = crate::platform::create_native_pet_window(
                mtm,
                prefs.pet.width.max(48.0) as f64,
                prefs.shell.topmost,
            );
            self.native_pet_window = Some(win);
            self.native_pet_view = Some(view);
            if let Some(win) = self.native_pet_window.as_ref() {
                crate::platform::position_native_pet_window(
                    win,
                    prefs,
                    prefs.pet.width.max(48.0) as f64,
                );
            }
        } else {
            tracing::error!("macOS native pet window requires the AppKit main thread");
        }
        match overlay.create_window(deskhud_engine::OverlayWindowRole::Pet) {
            Ok(id) => {
                self.pet_overlay_id = Some(id);
                let level = if prefs.shell.topmost {
                    deskhud_engine::OverlayWindowLevel::AlwaysOnTop
                } else {
                    deskhud_engine::OverlayWindowLevel::Normal
                };
                let _ = overlay.set_level(id, level);
                let _ = overlay.set_visible(id, true);
            }
            Err(error) => {
                tracing::warn!(%error, "create macOS pet overlay window failed");
            }
        }
    }

    /// Route pet-scoped window input; returns true when a menu was requested.
    pub(crate) fn window_event(
        &mut self,
        window: Option<&Window>,
        pet_active: bool,
        event: &WindowEvent,
        engine: &mut EngineRegistry,
        prefs: &mut deskhud_ui::UiPreferences,
        overlay: &mut Box<dyn OverlayBackend>,
    ) -> bool {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if let Some(key) = mac_key_from_physical(event.physical_key) {
                let pet_event = if event.state == ElementState::Pressed {
                    PetEvent::KeyPressed {
                        key,
                        modifiers: PetModifiers::NONE,
                    }
                } else {
                    PetEvent::KeyReleased {
                        key,
                        modifiers: PetModifiers::NONE,
                    }
                };
                engine.active_pet().on_event(pet_event);
                if let Some(window) = window {
                    window.request_redraw();
                }
            }
        }
        if pet_active {
            match event {
                WindowEvent::CursorMoved { .. } if self.mac_mouse.primary_down => {
                    let Some((cursor_x, cursor_y)) = crate::platform::cursor_screen_px() else {
                        return false;
                    };
                    let Some((press_x, press_y)) = self.mac_press_cursor else {
                        return false;
                    };
                    let dx = cursor_x - press_x;
                    let dy = cursor_y - press_y;
                    let threshold = 4.0 * window.map(|w| w.scale_factor()).unwrap_or(1.0);
                    if !self.mac_dragging && ((dx as f64).hypot(dy as f64)) >= threshold {
                        self.mac_dragging = true;
                        engine.active_pet().on_event(PetEvent::DragStarted);
                    }
                    if self.mac_dragging {
                        if let Some(window) = window {
                            if let Some(origin) = self.mac_press_window {
                                window.set_outer_position(PhysicalPosition::new(
                                    origin.x + dx,
                                    origin.y + dy,
                                ));
                            }
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: winit::event::MouseButton::Right,
                    ..
                } => return true,
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    self.mac_mouse.primary_down = true;
                    self.mac_press_cursor = crate::platform::cursor_screen_px();
                    self.mac_press_window = window.and_then(|w| w.outer_position().ok());
                    engine.active_pet().on_event(PetEvent::MousePressed {
                        button: PetMouseButton::Primary,
                        modifiers: PetModifiers::NONE,
                    });
                    if let Some(window) = window {
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    self.mac_mouse.primary_down = false;
                    if self.mac_dragging {
                        self.mac_dragging = false;
                        engine.active_pet().on_event(PetEvent::DragEnded {
                            drag: DragState::ACTIVE,
                        });
                        self.snap_mac_pet_window(window, prefs, overlay);
                    } else {
                        engine.active_pet().on_event(PetEvent::MouseClicked {
                            button: PetMouseButton::Primary,
                            modifiers: PetModifiers::NONE,
                        });
                    }
                    engine.active_pet().on_event(PetEvent::MouseReleased {
                        button: PetMouseButton::Primary,
                        modifiers: PetModifiers::NONE,
                    });
                    self.mac_press_cursor = None;
                    self.mac_press_window = None;
                    self.save_pet_window_position(window, prefs);
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn about_to_wait(
        &mut self,
        window: Option<&Window>,
        _animate_pet: bool,
        engine: &mut EngineRegistry,
        prefs: &deskhud_ui::UiPreferences,
    ) -> Option<Instant> {
        self.update_mac_behavior(window, engine);
        if self.native_pet_window.is_some() {
            let interval = pet_interval_of(prefs);
            if let Some(view) = self.native_pet_view.as_ref() {
                let now = Instant::now();
                let dt = now
                    .saturating_duration_since(self.mac_last_tick)
                    .min(PET_FRAME_INTERVAL * 2);
                if dt < interval {
                    crate::platform::request_native_pet_redraw(view);
                    return Some(now + interval.saturating_sub(dt));
                }
                self.mac_last_tick = now;
                engine.active_pet().tick(dt.as_secs_f32().clamp(0.0, 0.05));
                let paint = self.mac_paint(window, engine, prefs);
                crate::platform::update_native_pet_paint(paint);
                crate::platform::request_native_pet_redraw(view);
            }
            return Some(Instant::now() + pet_interval_of(prefs));
        }
        None
    }

    pub(crate) fn command(
        &mut self,
        command: OverlayControlCommand,
        engine: &mut EngineRegistry,
    ) -> Option<Instant> {
        match command {
            OverlayControlCommand::PetDragStarted => {
                self.mac_dragging = true;
                engine.active_pet().on_event(PetEvent::DragStarted);
            }
            OverlayControlCommand::PetDragEnded => {
                self.mac_dragging = false;
                engine.active_pet().on_event(PetEvent::DragEnded {
                    drag: DragState::ACTIVE,
                });
            }
            _ => {}
        }
        None
    }

    #[allow(unused_variables)]
    pub(crate) fn frame(
        &mut self,
        window: Option<&Window>,
        engine: &mut EngineRegistry,
        prefs: &deskhud_ui::UiPreferences,
    ) -> Option<PetPaint> {
        let paint = self.mac_paint(window, engine, prefs);
        crate::platform::update_native_pet_paint(paint.clone());
        Some(paint)
    }

    pub(crate) fn apply_topmost(
        &mut self,
        window: Option<&Window>,
        prefs: &deskhud_ui::UiPreferences,
        overlay: &mut Box<dyn OverlayBackend>,
    ) {
        if let Some(window) = window {
            window.set_window_level(if prefs.shell.topmost {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            });
        }
        if let Some(window) = self.native_pet_window.as_ref() {
            crate::platform::set_native_pet_topmost(window, prefs.shell.topmost);
        }
        if let Some(id) = self.pet_overlay_id {
            let level = if prefs.shell.topmost {
                deskhud_engine::OverlayWindowLevel::AlwaysOnTop
            } else {
                deskhud_engine::OverlayWindowLevel::Normal
            };
            let _ = overlay.set_level(id, level);
        }
    }

    pub(crate) fn exiting(&mut self, overlay: &mut Box<dyn OverlayBackend>) {
        if let Some(id) = self.pet_overlay_id.take() {
            let _ = overlay.set_visible(id, false);
            let _ = overlay.destroy_window(id);
        }
        if let Some(window) = self.native_pet_window.take() {
            window.orderOut(None);
        }
        self.native_pet_view = None;
    }

    fn snap_mac_pet_window(
        &mut self,
        window_opt: Option<&Window>,
        _prefs: &deskhud_ui::UiPreferences,
        overlay: &mut Box<dyn OverlayBackend>,
    ) {
        let Some(window) = window_opt else {
            return;
        };
        let Ok(mut position) = window.outer_position() else {
            return;
        };
        let size = window.outer_size();
        let work = self
            .mac_screen_area(overlay.as_ref())
            .map(|area| {
                (
                    area.active.origin.x,
                    area.active.origin.y,
                    area.active.origin.x + area.active.width,
                    area.active.origin.y + area.active.height,
                )
            })
            .unwrap_or_else(crate::platform::main_display_work_area_px);
        let left = work.0.round() as i32;
        let top = work.1.round() as i32;
        let right = work.2.round() as i32;
        let bottom = work.3.round() as i32;
        let tolerance = 16;
        if (position.x - left).abs() <= tolerance {
            position.x = left;
        }
        if (position.y - top).abs() <= tolerance {
            position.y = top;
        }
        position.x = position.x.clamp(left, right - size.width as i32);
        position.y = position.y.clamp(top, bottom - size.height as i32);
        if (right - (position.x + size.width as i32)).abs() <= tolerance {
            position.x = right - size.width as i32;
        }
        if (bottom - (position.y + size.height as i32)).abs() <= tolerance {
            position.y = bottom - size.height as i32;
        }
        window.set_outer_position(position);
    }

    fn mac_screen_area(
        &self,
        overlay: &dyn OverlayBackend,
    ) -> Option<deskhud_engine::OverlayScreenArea> {
        overlay.screen_area().ok()
    }

    fn update_mac_behavior(&mut self, _window: Option<&Window>, engine: &mut EngineRegistry) {
        if let Some(native) = self.native_pet_window.as_ref() {
            let frame = native.frame();
            let cursor = objc2_app_kit::NSEvent::mouseLocation();
            let cursor = (cursor.x as f32, cursor.y as f32);
            let center = (
                (frame.origin.x + frame.size.width * 0.5) as f32,
                (frame.origin.y + frame.size.height * 0.5) as f32,
            );
            let dx = (cursor.0 - center.0) / (frame.size.width as f32 * 1.8).max(1.0);
            let dy = (cursor.1 - center.1) / (frame.size.height as f32 * 1.8).max(1.0);
            let local_x = cursor.0 - frame.origin.x as f32;
            let local_y = cursor.1 - frame.origin.y as f32;
            let radius = frame.size.width.min(frame.size.height) as f32 * 0.42;
            let hovering = ((local_x - frame.size.width as f32 * 0.5).powi(2)
                + (local_y - frame.size.height as f32 * 0.5).powi(2))
                <= radius.powi(2);
            let was_hovering = self.mac_mouse.hovering;
            self.mac_mouse.hovering = hovering;
            let visible = native
                .screen()
                .or_else(|| objc2_app_kit::NSScreen::mainScreen(objc2::MainThreadMarker::new()?))
                .map(|screen| screen.visibleFrame())
                .unwrap_or(frame);
            let tolerance = 16.0;
            let dock = DockState {
                left: (frame.origin.x - visible.origin.x).abs() <= tolerance,
                right: (visible.origin.x + visible.size.width
                    - (frame.origin.x + frame.size.width))
                    .abs()
                    <= tolerance,
                top: (visible.origin.y + visible.size.height
                    - (frame.origin.y + frame.size.height))
                    .abs()
                    <= tolerance,
                bottom: (frame.origin.y - visible.origin.y).abs() <= tolerance,
            };
            let pet = engine.active_pet();
            if hovering != was_hovering {
                pet.on_event(PetEvent::MouseHover {
                    inside: hovering,
                });
            }
            if dock != self.mac_dock {
                let from = self.mac_dock;
                self.mac_dock = dock;
                pet.on_event(PetEvent::DockChanged { from, to: dock });
            }
            let _ = (dx, dy);
        }
    }

    fn mac_paint(
        &self,
        window: Option<&Window>,
        engine: &EngineRegistry,
        prefs: &deskhud_ui::UiPreferences,
    ) -> PetPaint {
        let pet = engine.active_pet();
        let info = pet.info();
        let options = pet
            .config_options()
            .iter()
            .map(|option| {
                (
                    option.key.to_string(),
                    prefs.pet.get_option(info.id, option.key, option.default),
                )
            })
            .collect::<HashMap<_, _>>();
        let config = PetConfigBag::new(&options);
        let (pointer_dir, mouse, dock) = self.mac_pet_context(window);
        pet.paint(PetPaintCtx {
            time_secs: self.pet_started.elapsed().as_secs_f64(),
            pointer_dir,
            status_line: "",
            dock,
            drag: if self.mac_dragging {
                DragState::ACTIVE
            } else {
                DragState::IDLE
            },
            mouse,
            config,
            theme: PetTheme::Dark,
        })
    }

    fn mac_pet_context(&self, _window: Option<&Window>) -> ([f32; 2], MouseState, DockState) {
        let Some(native) = self.native_pet_window.as_ref() else {
            return ([0.0, 0.0], self.mac_mouse, self.mac_dock);
        };
        let frame = native.frame();
        let cursor = objc2_app_kit::NSEvent::mouseLocation();
        let cursor = (cursor.x as f32, cursor.y as f32);
        let center = (
            (frame.origin.x + frame.size.width * 0.5) as f32,
            (frame.origin.y + frame.size.height * 0.5) as f32,
        );
        let dx = (cursor.0 - center.0) / (frame.size.width as f32 * 1.8).max(1.0);
        let dy = (cursor.1 - center.1) / (frame.size.height as f32 * 1.8).max(1.0);
        (
            [dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0)],
            self.mac_mouse,
            self.mac_dock,
        )
    }

    fn save_pet_window_position(
        &mut self,
        window: Option<&Window>,
        prefs: &mut deskhud_ui::UiPreferences,
    ) {
        let Some(window) = window else {
            return;
        };
        let Ok(position) = window.outer_position() else {
            return;
        };
        let scale = window.scale_factor().max(0.01) as f32;
        let size = window.inner_size();
        let center_x = position.x as f32 / scale + size.width as f32 / scale / 2.0;
        let center_y = position.y as f32 / scale + size.height as f32 / scale / 2.0;
        prefs.pet.set_pos(center_x, center_y);
        if let Err(error) = deskhud_ui::persist::save(prefs) {
            tracing::warn!(%error, "pet position prefs save failed");
        }
    }
}

fn mac_key_from_physical(key: PhysicalKey) -> Option<deskhud_engine::PetKey> {
    use deskhud_engine::PetKey;
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    Some(match code {
        KeyCode::Escape => PetKey::Escape,
        KeyCode::Tab => PetKey::Tab,
        KeyCode::Enter => PetKey::Enter,
        KeyCode::Space => PetKey::Space,
        KeyCode::Backspace => PetKey::Backspace,
        KeyCode::Delete => PetKey::Delete,
        KeyCode::ArrowUp => PetKey::ArrowUp,
        KeyCode::ArrowDown => PetKey::ArrowDown,
        KeyCode::ArrowLeft => PetKey::ArrowLeft,
        KeyCode::ArrowRight => PetKey::ArrowRight,
        _ => return None,
    })
}
