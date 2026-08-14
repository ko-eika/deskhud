//! Linux (non-Windows, non-macOS) desktop-pet host.
//!
//! The winit pet window, painting, dragging, snapping, keyboard input and topmost
//! are owned here so the egui shell never carries OS-specific pet code.

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

const FALLBACK_PET_SIZE: f64 = 180.0;

/// Desktop-pet host state.
pub(crate) struct PetHost {
    pet_started: Instant,
    mac_mouse: MouseState,
    mac_dock: DockState,
    mac_dragging: bool,
    mac_press_cursor: Option<(i32, i32)>,
    mac_press_window: Option<PhysicalPosition<i32>>,
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
            pet_started: Instant::now(),
            mac_mouse: MouseState::IDLE,
            mac_dock: DockState::FREE,
            mac_dragging: false,
            mac_press_cursor: None,
            mac_press_window: None,
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
        _overlay: &mut Box<dyn OverlayBackend>,
        window: Option<&Window>,
    ) {
        if let Some(window) = window {
            window.set_title("DeskHud 宠物");
            window.set_decorations(false);
            window.set_resizable(false);
            window.set_window_level(if prefs.shell.topmost {
                WindowLevel::AlwaysOnTop
            } else {
                WindowLevel::Normal
            });
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(
                FALLBACK_PET_SIZE,
                FALLBACK_PET_SIZE,
            ));
            let ppp = window.scale_factor().max(0.01);
            let pos = prefs
                .pet
                .pos()
                .unwrap_or([FALLBACK_PET_SIZE as f32, FALLBACK_PET_SIZE as f32]);
            window.set_outer_position(winit::dpi::LogicalPosition::new(
                pos[0] as f64 - FALLBACK_PET_SIZE / (2.0 * ppp),
                pos[1] as f64 - FALLBACK_PET_SIZE / (2.0 * ppp),
            ));
            window.set_visible(true);
            window.request_redraw();
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
        animate_pet: bool,
        engine: &mut EngineRegistry,
        prefs: &deskhud_ui::UiPreferences,
    ) -> Option<Instant> {
        self.update_mac_behavior(window, engine);
        if animate_pet {
            let now = Instant::now();
            if let Some(window) = window {
                window.request_redraw();
            }
            return Some(now + pet_interval_of(prefs));
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
        Some(self.mac_paint(window, engine, prefs))
    }

    #[allow(dead_code)]
    pub(crate) fn apply_topmost(
        &mut self,
        _window: Option<&Window>,
        _prefs: &deskhud_ui::UiPreferences,
        _overlay: &mut Box<dyn OverlayBackend>,
    ) {
    }

    pub(crate) fn exiting(&mut self, _overlay: &mut Box<dyn OverlayBackend>) {}

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

    fn update_mac_behavior(&mut self, window: Option<&Window>, engine: &mut EngineRegistry) {
        let (_, mouse, dock) = self.mac_pet_context(window);
        let pet = engine.active_pet();
        if mouse.hovering != self.mac_mouse.hovering {
            pet.on_event(PetEvent::MouseHover {
                inside: mouse.hovering,
            });
            self.mac_mouse.hovering = mouse.hovering;
        }
        if dock != self.mac_dock {
            let from = self.mac_dock;
            self.mac_dock = dock;
            pet.on_event(PetEvent::DockChanged { from, to: dock });
        }
    }

    fn mac_pet_context(&self, window: Option<&Window>) -> ([f32; 2], MouseState, DockState) {
        let Some(window) = window else {
            return ([0.0, 0.0], self.mac_mouse, self.mac_dock);
        };
        let scale = window.scale_factor().max(0.01) as f32;
        let Ok(position) = window.outer_position() else {
            return ([0.0, 0.0], self.mac_mouse, self.mac_dock);
        };
        let size = window.inner_size();
        let center = (
            position.x as f32 + size.width as f32 / 2.0,
            position.y as f32 + size.height as f32 / 2.0,
        );
        let cursor = crate::platform::cursor_screen_px()
            .map(|(x, y)| (x as f32, y as f32))
            .unwrap_or(center);
        let dx = (cursor.0 - center.0) / (size.width as f32 * 1.8).max(1.0);
        let dy = (cursor.1 - center.1) / (size.height as f32 * 1.8).max(1.0);
        let local_x = (cursor.0 - position.x as f32) / scale;
        let local_y = (cursor.1 - position.y as f32) / scale;
        let radius = size.width.min(size.height) as f32 / scale * 0.42;
        let local_center = (
            size.width as f32 / scale / 2.0,
            size.height as f32 / scale / 2.0,
        );
        let hovering = ((local_x - local_center.0).powi(2) + (local_y - local_center.1).powi(2))
            <= radius.powi(2);
        let monitor = window.current_monitor();
        let bounds = monitor
            .as_ref()
            .map(|monitor| {
                let origin = monitor.position();
                let size = monitor.size();
                (
                    origin.x as f32,
                    origin.y as f32,
                    (origin.x + size.width as i32) as f32,
                    (origin.y + size.height as i32) as f32,
                )
            })
            .unwrap_or_else(crate::platform::main_display_bounds_px);
        let dock = DockState {
            left: (position.x as f32 - bounds.0).abs() <= 4.0,
            right: (position.x as f32 + size.width as f32 - bounds.2).abs() <= 4.0,
            top: (position.y as f32 - bounds.1).abs() <= 4.0,
            bottom: (position.y as f32 + size.height as f32 - bounds.3).abs() <= 4.0,
        };
        (
            [dx, dy],
            MouseState {
                primary_down: false,
                hovering,
                ..self.mac_mouse
            },
            dock,
        )
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
