//! Direct `winit` + `egui_glow` host for opaque control surfaces.

#[cfg(not(windows))]
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
#[cfg(not(target_os = "macos"))]
use egui::Color32;
use glow::HasContext as _;
use glutin::context::NotCurrentGlContext as _;
use glutin::display::{GetGlDisplay as _, GlDisplay as _};
use glutin::prelude::GlSurface as _;
use raw_window_handle::HasWindowHandle as _;
use winit::application::ApplicationHandler;
#[cfg(windows)]
use winit::dpi::PhysicalPosition;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(windows)]
use winit::platform::windows::{WindowAttributesExtWindows as _, WindowExtWindows as _};

use crate::overlay_control::{OverlayControlBus, OverlayControlCommand};
#[cfg(target_os = "macos")]
use crate::overlay_surface::OverlaySurface;
use crate::pet_menu::PetMenuHost;
use crate::platform::OverlayBackend;
use crate::settings::{SettingsHost, SettingsTab};
#[cfg(not(windows))]
use deskhud_engine::{
    DockState, DragState, MouseState, PetConfigBag, PetEvent, PetModifiers, PetMouseButton,
    PetPaintCtx, PetTheme,
};

const MENU_INITIAL_WIDTH: f64 = 180.0;
const SETTINGS_WIDTH: f64 = 920.0;
const SETTINGS_HEIGHT: f64 = 680.0;
#[cfg(not(windows))]
const MENU_FOCUS_GRACE: Duration = Duration::from_millis(220);
#[cfg(windows)]
const MENU_FOCUS_GRACE: Duration = Duration::from_millis(180);
#[cfg(not(windows))]
const FALLBACK_PET_SIZE: f64 = 180.0;
#[cfg(not(windows))]
const PET_FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug)]
pub(crate) enum UserEvent {
    Commands,
    Repaint(Duration),
    #[cfg(target_os = "macos")]
    GlobalMouse(deskhud_engine::PetMouseButton),
    #[cfg(target_os = "macos")]
    GlobalKey {
        key: deskhud_engine::PetKey,
        pressed: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlSurface {
    Hidden,
    Menu,
    Settings,
}

pub(crate) fn run(prefs: deskhud_ui::UiPreferences, controls: OverlayControlBus) -> Result<()> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .context("create native UI event loop")?;
    let proxy = event_loop.create_proxy();
    let command_proxy = proxy.clone();
    controls.set_waker(move || {
        let _ = command_proxy.send_event(UserEvent::Commands);
    });
    let mut app = NativeHost::new(proxy, prefs, controls);
    event_loop
        .run_app(&mut app)
        .context("run native UI event loop")
}

struct NativeHost {
    proxy: EventLoopProxy<UserEvent>,
    controls: OverlayControlBus,
    prefs: deskhud_ui::UiPreferences,
    engine: deskhud_engine::EngineRegistry,
    catalogs: deskhud_ui::CatalogStore,
    settings: SettingsHost,
    menu: PetMenuHost,
    overlay_backend: Box<dyn OverlayBackend>,
    control_surface: ControlSurface,
    gl_window: Option<GlutinWindow>,
    gl: Option<Arc<glow::Context>>,
    egui: Option<egui_glow::EguiGlow>,
    #[cfg(target_os = "macos")]
    menu_surface: Option<OverlaySurface>,
    #[cfg(target_os = "macos")]
    menu_surface_dispose_pending: bool,
    #[cfg(target_os = "macos")]
    settings_surface: Option<OverlaySurface>,
    repaint_at: Option<Instant>,
    #[cfg(not(windows))]
    pet_started: Instant,
    #[cfg(not(windows))]
    mac_mouse: MouseState,
    #[cfg(not(windows))]
    mac_dock: DockState,
    #[cfg(not(windows))]
    mac_dragging: bool,
    #[cfg(target_os = "macos")]
    mac_press_cursor: Option<(i32, i32)>,
    #[cfg(target_os = "macos")]
    mac_press_window: Option<winit::dpi::PhysicalPosition<i32>>,
}

impl NativeHost {
    fn new(
        proxy: EventLoopProxy<UserEvent>,
        prefs: deskhud_ui::UiPreferences,
        controls: OverlayControlBus,
    ) -> Self {
        let boot = deskhud_runtime::bootstrap_registry();
        let catalogs = deskhud_runtime::build_catalog_store(&boot.discovered, prefs.locale);
        #[cfg(target_os = "macos")]
        let overlay_backend: Box<dyn OverlayBackend> =
            Box::new(crate::platform::MacosOverlayBackend::default());
        #[cfg(windows)]
        let overlay_backend: Box<dyn OverlayBackend> =
            Box::new(crate::platform::WindowsOverlayBackend);
        #[cfg(all(not(windows), not(target_os = "macos")))]
        let overlay_backend: Box<dyn OverlayBackend> =
            Box::new(crate::platform::FallbackOverlayBackend);
        Self {
            proxy,
            controls,
            engine: boot.registry,
            catalogs,
            settings: SettingsHost::new(prefs.clone()),
            menu: PetMenuHost::new(prefs.clone()),
            overlay_backend,
            prefs,
            control_surface: ControlSurface::Hidden,
            gl_window: None,
            gl: None,
            egui: None,
            #[cfg(target_os = "macos")]
            menu_surface: None,
            #[cfg(target_os = "macos")]
            menu_surface_dispose_pending: false,
            #[cfg(target_os = "macos")]
            settings_surface: None,
            repaint_at: None,
            #[cfg(not(windows))]
            pet_started: Instant::now(),
            #[cfg(not(windows))]
            mac_mouse: MouseState::IDLE,
            #[cfg(not(windows))]
            mac_dock: DockState::FREE,
            #[cfg(not(windows))]
            mac_dragging: false,
            #[cfg(target_os = "macos")]
            mac_press_cursor: None,
            #[cfg(target_os = "macos")]
            mac_press_window: None,
        }
    }

    fn window(&self) -> Option<&Window> {
        self.gl_window.as_ref().map(GlutinWindow::window)
    }

    fn process_commands(&mut self, event_loop: &ActiveEventLoop) {
        for command in self.controls.drain() {
            match command {
                OverlayControlCommand::ActivateExisting => {}
                OverlayControlCommand::OpenMenu => self.show_menu(event_loop),
                OverlayControlCommand::PetMoved { x_points, y_points } => {
                    self.prefs.pet.set_pos(x_points, y_points);
                    let mut settings = self.settings.lock();
                    settings.prefs.pet.set_pos(x_points, y_points);
                    settings.baseline.pet.set_pos(x_points, y_points);
                    drop(settings);
                    self.save_prefs();
                }
                OverlayControlCommand::Quit => event_loop.exit(),
            }
        }
    }

    fn show_menu(&mut self, event_loop: &ActiveEventLoop) {
        if self.control_surface == ControlSurface::Settings {
            if let Some(window) = self.window() {
                window.focus_window();
                window.request_redraw();
            }
            return;
        }
        #[cfg(windows)]
        let (window, ppp, cursor) = {
            let window = self.window().ok_or(()).ok();
            let Some(window) = window else { return };
            let ppp = window.scale_factor() as f32;
            let cursor = crate::platform::cursor_screen_px().unwrap_or((100, 100));
            self.menu.open_at(
                &self.prefs,
                egui::pos2(cursor.0 as f32 / ppp, cursor.1 as f32 / ppp),
                ppp,
                self.prefs.hud.is_master_enabled(),
                self.prefs.shell.topmost,
            );
            {
                let mut state = self.menu.lock();
                state.pet_topmost = self.prefs.shell.topmost;
                state.master_enabled = self.prefs.hud.is_master_enabled();
            }
            (window, ppp, cursor)
        };
        #[cfg(target_os = "macos")]
        {
            if self.menu_surface.is_none() {
                let surface = match unsafe { OverlaySurface::new(event_loop) } {
                    Ok(surface) => surface,
                    Err(error) => {
                        tracing::error!(%error, "create macOS menu surface failed");
                        return;
                    }
                };
                crate::fonts::configure_typography(
                    &surface.egui.egui_ctx,
                    &self.prefs.shell.ui_font_id,
                    self.prefs.shell.ui_font_size,
                );
                self.menu_surface = Some(surface);
            }
            let Some(surface) = self.menu_surface.as_ref() else {
                return;
            };
            // The menu surface is intentionally reused to avoid tearing down
            // GL resources during a menu action. Clear transient widget state
            // before showing it again so a previous hover cannot stick.
            surface
                .egui
                .egui_ctx
                .memory_mut(|memory| memory.data.clear());
            surface.egui.egui_ctx.request_repaint();
            let ppp = surface.window().scale_factor() as f32;
            let cursor = crate::platform::cursor_screen_px().unwrap_or((100, 100));
            self.menu.open_at(
                &self.prefs,
                egui::pos2(cursor.0 as f32 / ppp, cursor.1 as f32 / ppp),
                ppp,
                self.prefs.hud.is_master_enabled(),
                self.prefs.shell.topmost,
            );
            let state = self.menu.lock();
            let position = LogicalPosition::new(state.anchor.x as f64, state.anchor.y as f64);
            let size = LogicalSize::new(
                state.menu_width as f64,
                crate::pet_menu::menu_height() as f64,
            );
            drop(state);
            let window = surface.window();
            window.set_decorations(false);
            window.set_resizable(false);
            // The menu is a control surface, not a transparent pet surface.
            // Its opaque egui frame must remain readable over the desktop.
            window.set_title("DeskHud 菜单");
            window.set_window_level(WindowLevel::AlwaysOnTop);
            let _ = window.request_inner_size(size);
            window.set_outer_position(position);
            window.set_visible(true);
            window.focus_window();
            window.request_redraw();
            self.control_surface = ControlSurface::Menu;
            return;
        }
        #[cfg(windows)]
        {
            let menu_state = self.menu.lock();
            let position = PhysicalPosition::new(
                (menu_state.anchor.x * ppp).round() as i32,
                (menu_state.anchor.y * ppp).round() as i32,
            );
            let size = LogicalSize::new(
                menu_state.menu_width as f64,
                crate::pet_menu::menu_height() as f64,
            );
            drop(menu_state);

            self.control_surface = ControlSurface::Menu;
            let Some(window) = self.window() else { return };
            window.set_title("");
            window.set_decorations(false);
            window.set_resizable(false);
            window.set_min_inner_size(None::<LogicalSize<f64>>);
            window.set_window_level(WindowLevel::AlwaysOnTop);
            #[cfg(windows)]
            window.set_skip_taskbar(true);
            let _ = window.request_inner_size(size);
            window.set_outer_position(position);
            window.set_visible(true);
            window.focus_window();
            window.request_redraw();
        }
    }

    fn show_settings(&mut self, event_loop: &ActiveEventLoop) {
        self.show_settings_tab(event_loop, SettingsTab::General);
    }

    fn show_settings_tab(&mut self, event_loop: &ActiveEventLoop, tab: SettingsTab) {
        #[cfg(target_os = "macos")]
        self.close_menu_surface();
        #[cfg(target_os = "macos")]
        if let Some(surface) = self.settings_surface.as_ref() {
            surface.window().set_visible(false);
        }
        let pet_options = self
            .engine
            .pets()
            .into_iter()
            .map(|pet| (pet.info().id.to_owned(), pet.config_options().to_vec()))
            .collect();
        self.settings.open(
            &self.prefs,
            self.engine.pet_infos(),
            pet_options,
            self.engine.plugin_infos(),
            self.engine.all_hud_contributions(),
            self.catalogs.clone(),
            tab,
        );
        #[cfg(target_os = "macos")]
        if self.settings_surface.is_none() {
            let surface = match unsafe { OverlaySurface::new(event_loop) } {
                Ok(surface) => surface,
                Err(error) => {
                    tracing::error!(%error, "create macOS settings surface failed");
                    return;
                }
            };
            crate::fonts::configure_typography(
                &surface.egui.egui_ctx,
                &self.prefs.shell.ui_font_id,
                self.prefs.shell.ui_font_size,
            );
            self.settings_surface = Some(surface);
        }
        #[cfg(target_os = "macos")]
        if let Some(surface) = self.settings_surface.as_ref() {
            let window = surface.window();
            window.set_title("DeskHud 设置");
            window.set_decorations(true);
            window.set_resizable(true);
            window.set_min_inner_size(Some(LogicalSize::new(720.0, 520.0)));
            window.set_window_level(WindowLevel::Normal);
            let size = self.prefs.shell.settings_size();
            let _ = window.request_inner_size(LogicalSize::new(size[0] as f64, size[1] as f64));
            window.set_visible(true);
            window.focus_window();
            window.request_redraw();
            self.control_surface = ControlSurface::Settings;
            return;
        }
        self.control_surface = ControlSurface::Settings;
        let Some(window) = self.window() else { return };
        window.set_title("DeskHud 设置");
        window.set_decorations(true);
        window.set_resizable(true);
        window.set_min_inner_size(Some(LogicalSize::new(720.0, 520.0)));
        // 设置是普通工具窗口；宠物置顶偏好不得把设置也提升为系统置顶。
        window.set_window_level(WindowLevel::Normal);
        #[cfg(windows)]
        window.set_skip_taskbar(false);
        let size = self.prefs.shell.settings_size();
        let _ = window.request_inner_size(LogicalSize::new(size[0] as f64, size[1] as f64));
        if let Some([x, y]) = self.prefs.shell.settings_pos() {
            window.set_outer_position(LogicalPosition::new(x as f64, y as f64));
        } else {
            let _ = window.request_inner_size(LogicalSize::new(SETTINGS_WIDTH, SETTINGS_HEIGHT));
        }
        window.set_visible(true);
        window.focus_window();
        window.request_redraw();
    }

    fn hide_control_window(&mut self) {
        self.control_surface = ControlSurface::Hidden;
        #[cfg(target_os = "macos")]
        self.close_menu_surface();
        if let Some(window) = self.window() {
            #[cfg(windows)]
            window.set_visible(false);
            #[cfg(not(windows))]
            self.show_fallback_pet(window);
        }
    }

    #[cfg(target_os = "macos")]
    fn close_menu_surface(&mut self) {
        if let Some(surface) = self.menu_surface.as_ref() {
            surface.window().set_visible(false);
        }
        self.menu_surface_dispose_pending = true;
        self.menu.lock().open = false;
    }

    #[cfg(target_os = "macos")]
    fn dispose_menu_surface_if_pending(&mut self) {
        // Do not destroy a live GL surface during the event loop. egui_glow
        // owns VAO/texture resources tied to that context; destroying it while
        // another surface is current can produce GL_INVALID_VALUE. Reuse the
        // hidden surface and reset its transient input state on next open.
        self.menu_surface_dispose_pending = false;
    }

    #[cfg(not(windows))]
    fn show_fallback_pet(&self, window: &Window) {
        window.set_title("DeskHud 宠物");
        window.set_decorations(false);
        window.set_resizable(false);
        window.set_window_level(if self.prefs.shell.topmost {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        });
        let _ = window.request_inner_size(LogicalSize::new(FALLBACK_PET_SIZE, FALLBACK_PET_SIZE));
        let ppp = window.scale_factor() as f32;
        let pos = self
            .prefs
            .pet
            .pos()
            .unwrap_or([FALLBACK_PET_SIZE as f32, FALLBACK_PET_SIZE as f32]);
        window.set_outer_position(LogicalPosition::new(
            pos[0] as f64 - FALLBACK_PET_SIZE / (2.0 * ppp as f64),
            pos[1] as f64 - FALLBACK_PET_SIZE / (2.0 * ppp as f64),
        ));
        window.set_visible(true);
        window.request_redraw();
    }

    fn save_prefs(&self) {
        if let Err(error) = deskhud_ui::persist::save(&self.prefs) {
            tracing::warn!(%error, "native host prefs save failed");
            return;
        }
        #[cfg(windows)]
        crate::gpu_overlay_probe::request_prefs_reload();
    }

    #[cfg(not(windows))]
    fn mac_paint(&self) -> deskhud_engine::PetPaint {
        let pet = self.engine.active_pet();
        let info = pet.info();
        let options = pet
            .config_options()
            .iter()
            .map(|option| {
                (
                    option.key.to_string(),
                    self.prefs
                        .pet
                        .get_option(info.id, option.key, option.default),
                )
            })
            .collect::<HashMap<_, _>>();
        let config = PetConfigBag::new(&options);
        pet.apply_config(config);
        let elapsed = self.pet_started.elapsed().as_secs_f64();
        pet.tick(1.0 / 60.0);
        let (pointer_dir, mouse, dock) = self.mac_pet_context();
        pet.paint(PetPaintCtx {
            time_secs: elapsed,
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

    #[cfg(not(windows))]
    fn mac_pet_context(&self) -> ([f32; 2], MouseState, DockState) {
        let Some(window) = self.window() else {
            return ([0.0, 0.0], MouseState::IDLE, DockState::FREE);
        };
        let scale = window.scale_factor().max(0.01) as f32;
        let Ok(position) = window.outer_position() else {
            return ([0.0, 0.0], MouseState::IDLE, DockState::FREE);
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
        let work_area = crate::platform::main_display_work_area_px();
        let tolerance = (16.0 * scale).max(8.0);
        let dock = DockState {
            left: (position.x as f32 - bounds.0).abs() <= tolerance,
            right: (bounds.2 - (position.x as f32 + size.width as f32)).abs() <= tolerance,
            top: (position.y as f32 - work_area.1).abs() <= tolerance,
            bottom: (work_area.3 - (position.y as f32 + size.height as f32)).abs() <= tolerance,
        };
        (
            [dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0)],
            MouseState {
                hovering,
                ..self.mac_mouse
            },
            dock,
        )
    }

    #[cfg(not(windows))]
    fn update_mac_behavior(&mut self) {
        let (_, mouse, dock) = self.mac_pet_context();
        let pet = self.engine.active_pet();
        if mouse.hovering != self.mac_mouse.hovering {
            self.mac_mouse.hovering = mouse.hovering;
            pet.on_event(PetEvent::MouseHover {
                inside: mouse.hovering,
            });
        }
        if dock != self.mac_dock {
            let from = self.mac_dock;
            self.mac_dock = dock;
            pet.on_event(PetEvent::DockChanged { from, to: dock });
        }
    }

    #[cfg(not(windows))]
    fn save_pet_window_position(&mut self) {
        let Some(window) = self.window() else { return };
        let Ok(position) = window.outer_position() else {
            return;
        };
        let scale = window.scale_factor().max(0.01) as f32;
        let size = window.inner_size();
        let center_x = position.x as f32 / scale + size.width as f32 / scale / 2.0;
        let center_y = position.y as f32 / scale + size.height as f32 / scale / 2.0;
        self.prefs.pet.set_pos(center_x, center_y);
        self.save_prefs();
    }

    #[cfg(target_os = "macos")]
    fn mac_screen_area(&self) -> Option<deskhud_engine::OverlayScreenArea> {
        self.overlay_backend.screen_area().ok()
    }

    #[cfg(target_os = "macos")]
    fn snap_mac_pet_window(&mut self) {
        let Some(window) = self.window() else { return };
        let Ok(mut position) = window.outer_position() else {
            return;
        };
        let size = window.outer_size();
        let work = self
            .mac_screen_area()
            .map(|area| {
                (
                    area.active.origin.x,
                    area.active.origin.y,
                    area.active.origin.x + area.active.width,
                    area.active.origin.y + area.active.height,
                )
            })
            .unwrap_or_else(crate::platform::main_display_work_area_px);
        // `visibleFrame` changes automatically when Dock auto-hide changes.
        // Use it as the recovery area so a pet dropped into Dock/menu-bar
        // space is brought back into the currently active desktop area.
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
        // A desktop pet must remain fully visible; never leave half the window
        // outside the screen after a manual drag.
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

    #[cfg(target_os = "macos")]
    fn save_settings_surface_geometry(&mut self) {
        let Some(surface) = self.settings_surface.as_ref() else {
            return;
        };
        let Ok(position) = surface.window().outer_position() else {
            return;
        };
        let scale = surface.window().scale_factor().max(0.01) as f32;
        let size = surface.window().inner_size();
        let width = size.width as f32 / scale;
        let height = size.height as f32 / scale;
        let x = position.x as f32 / scale;
        let y = position.y as f32 / scale;
        self.prefs.shell.set_settings_geometry(width, height, x, y);
        self.save_prefs();
    }

    fn sync_pet_theme(&self) {
        #[cfg(windows)]
        {
            let theme = match self.prefs.shell.ui_theme {
                deskhud_ui::UiTheme::Light => deskhud_engine::PetTheme::Light,
                deskhud_ui::UiTheme::Dark => deskhud_engine::PetTheme::Dark,
                deskhud_ui::UiTheme::System => match self.window().and_then(Window::theme) {
                    Some(winit::window::Theme::Light) => deskhud_engine::PetTheme::Light,
                    Some(winit::window::Theme::Dark) | None => deskhud_engine::PetTheme::Dark,
                },
            };
            crate::gpu_overlay_probe::set_pet_theme(theme);
        }
    }

    fn pull_menu(&mut self, event_loop: &ActiveEventLoop) {
        let (open, open_settings, open_hud_settings, toggle_topmost, toggle_master, quit) = {
            let mut state = self.menu.lock();
            let actions = (
                state.open,
                state.open_settings,
                state.begin_hud_layout,
                state.toggle_topmost.take(),
                state.toggle_master.take(),
                state.quit,
            );
            state.open_settings = false;
            state.begin_hud_layout = false;
            state.quit = false;
            actions
        };
        if let Some(enabled) = toggle_topmost {
            self.prefs.shell.topmost = enabled;
            {
                let mut state = self.menu.lock();
                state.pet_topmost = enabled;
            }
            self.save_prefs();
            #[cfg(target_os = "macos")]
            if let Some(window) = self.window() {
                window.set_window_level(if enabled {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                });
            }
            #[cfg(windows)]
            crate::gpu_overlay_probe::set_topmost(enabled);
        }
        if let Some(enabled) = toggle_master {
            self.prefs.hud.set_master_enabled(enabled);
            {
                let mut state = self.menu.lock();
                state.master_enabled = enabled;
            }
            self.save_prefs();
        }
        let action_taken = toggle_topmost.is_some()
            || toggle_master.is_some()
            || quit
            || open_settings
            || open_hud_settings;
        if action_taken {
            #[cfg(target_os = "macos")]
            self.close_menu_surface();
        }
        if quit {
            event_loop.exit();
        } else if open_settings {
            self.show_settings(event_loop);
        } else if open_hud_settings {
            self.show_settings_tab(event_loop, SettingsTab::Hud);
        } else if !open && self.control_surface == ControlSurface::Menu {
            self.hide_control_window();
        }
    }

    fn pull_settings(&mut self) {
        let (open, apply, pending_flush, discard, draft) = {
            let mut state = self.settings.lock();
            let values = (
                state.open,
                state.apply_requested,
                state.pending_flush,
                state.discard_draft,
                state.prefs.clone(),
            );
            state.apply_requested = false;
            state.pending_flush = false;
            state.discard_draft = false;
            values
        };
        if apply {
            #[cfg(windows)]
            let topmost_changed = self.prefs.shell.topmost != draft.shell.topmost;
            self.prefs = draft.clone();
            self.save_prefs();
            self.sync_pet_theme();
            #[cfg(windows)]
            if topmost_changed {
                crate::gpu_overlay_probe::set_topmost(self.prefs.shell.topmost);
            }
            let mut state = self.settings.lock();
            state.baseline = draft;
            state.prefs = self.prefs.clone();
            if let Some(window) = self.window() {
                window.request_redraw();
            }
            #[cfg(target_os = "macos")]
            if let Some(window) = self.window() {
                window.set_window_level(if self.prefs.shell.topmost {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                });
            }
        } else if pending_flush && !discard {
            self.prefs = draft;
            self.save_prefs();
        }
        if !open && self.control_surface == ControlSurface::Settings {
            self.hide_control_window();
        }
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "macos")]
        let paint = self.mac_paint();
        let Some(gl_window) = self.gl_window.as_ref() else {
            return;
        };
        let Some(egui) = self.egui.as_mut() else {
            return;
        };
        #[cfg(windows)]
        let control_surface = self.control_surface;
        #[cfg(windows)]
        let settings = self.settings.clone();
        #[cfg(windows)]
        let menu = self.menu.clone();
        egui.run(gl_window.window(), |ui| {
            crate::theme::apply(ui.ctx(), self.prefs.shell.ui_theme);
            // macOS owns menu/settings in their own surfaces. The pet surface
            // must never switch its content based on control-surface state.
            #[cfg(windows)]
            match control_surface {
                ControlSurface::Hidden => {}
                ControlSurface::Menu => menu.draw_native(ui),
                ControlSurface::Settings => settings.draw_native(ui),
            }
            #[cfg(target_os = "macos")]
            draw_pet_paint(ui, paint.clone());
        });

        unsafe {
            use glow::HasContext as _;
            let Some(gl) = self.gl.as_ref() else { return };
            #[cfg(target_os = "macos")]
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            #[cfg(not(target_os = "macos"))]
            let color = self
                .egui
                .as_ref()
                .map(|egui| {
                    egui.egui_ctx
                        .style_of(egui.egui_ctx.theme())
                        .visuals
                        .window_fill()
                })
                .unwrap_or(Color32::BLACK);
            #[cfg(not(target_os = "macos"))]
            gl.clear_color(
                color.r() as f32 / 255.0,
                color.g() as f32 / 255.0,
                color.b() as f32 / 255.0,
                1.0,
            );
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        if let Some(egui) = self.egui.as_mut() {
            egui.paint(gl_window.window());
        }
        if let Err(error) = gl_window.swap_buffers() {
            tracing::error!(%error, "native UI swap buffers failed");
            event_loop.exit();
        }
        self.pull_menu(event_loop);
        self.pull_settings();
        #[cfg(not(windows))]
        {
            if let Some(window) = self.window() {
                window.request_redraw();
            }
            self.repaint_at = Some(Instant::now() + PET_FRAME_INTERVAL);
        }
    }

    #[cfg(target_os = "macos")]
    fn draw_menu_surface(&mut self, event_loop: &ActiveEventLoop) {
        let Some(surface) = self.menu_surface.as_mut() else {
            return;
        };
        let menu = self.menu.clone();
        let (native, egui) = (&surface.native, &mut surface.egui);
        let window = native.window();
        let theme = self.prefs.shell.ui_theme;
        if let (Some((cursor_x, cursor_y)), Ok(origin)) =
            (crate::platform::cursor_screen_px(), window.outer_position())
        {
            let scale = window.scale_factor().max(0.01) as f32;
            let local = egui::pos2(
                (cursor_x - origin.x) as f32 / scale,
                (cursor_y - origin.y) as f32 / scale,
            );
            egui.egui_ctx.input_mut(|input| {
                input.events.push(egui::Event::PointerMoved(local));
            });
        }
        egui.run(window, |ui| {
            crate::theme::apply(ui.ctx(), theme);
            menu.draw_native(ui);
        });
        unsafe {
            let fill = match theme {
                deskhud_ui::UiTheme::Light | deskhud_ui::UiTheme::System => {
                    (0.973, 0.973, 0.988, 1.0)
                }
                deskhud_ui::UiTheme::Dark => (0.169, 0.176, 0.192, 1.0),
            };
            surface.gl.clear_color(fill.0, fill.1, fill.2, fill.3);
            surface.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        egui.paint(window);
        if let Err(error) = surface.native.swap_buffers() {
            tracing::error!(%error, "macOS menu surface swap failed");
            event_loop.exit();
        }
        self.pull_menu(event_loop);
    }

    #[cfg(target_os = "macos")]
    fn draw_settings_surface(&mut self, event_loop: &ActiveEventLoop) {
        let Some(surface) = self.settings_surface.as_mut() else {
            return;
        };
        let settings = self.settings.clone();
        let (native, egui) = (&surface.native, &mut surface.egui);
        let window = native.window();
        egui.run(window, |ui| settings.draw_native(ui));
        unsafe {
            surface.gl.clear_color(0.12, 0.12, 0.14, 1.0);
            surface.gl.clear(glow::COLOR_BUFFER_BIT);
        }
        egui.paint(window);
        if let Err(error) = surface.native.swap_buffers() {
            tracing::error!(%error, "macOS settings surface swap failed");
            event_loop.exit();
        }
        self.pull_settings();
        if !self.settings.lock().open {
            if let Some(surface) = self.settings_surface.as_ref() {
                surface.window().set_visible(false);
            }
            self.control_surface = ControlSurface::Hidden;
            if let Some(window) = self.window() {
                window.request_redraw();
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for NativeHost {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (gl_window, gl) = match unsafe { GlutinWindow::new(event_loop) } {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(%error, "native UI initialization failed");
                event_loop.exit();
                return;
            }
        };
        let gl = Arc::new(gl);
        let egui = egui_glow::EguiGlow::new(event_loop, Arc::clone(&gl), None, None, true);
        let proxy = self.proxy.clone();
        egui.egui_ctx.set_request_repaint_callback(move |info| {
            let _ = proxy.send_event(UserEvent::Repaint(info.delay));
        });
        crate::fonts::configure_typography(
            &egui.egui_ctx,
            &self.prefs.shell.ui_font_id,
            self.prefs.shell.ui_font_size,
        );
        self.gl_window = Some(gl_window);
        self.gl = Some(gl);
        self.egui = Some(egui);
        self.sync_pet_theme();
        #[cfg(not(windows))]
        self.show_fallback_pet(self.window().expect("native window initialized"));
        #[cfg(target_os = "macos")]
        crate::platform::start_global_mouse_listener(self.proxy.clone());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        #[cfg(target_os = "macos")]
        if self
            .menu_surface
            .as_ref()
            .is_some_and(|surface| surface.id() == _window_id)
        {
            if matches!(event, WindowEvent::RedrawRequested) {
                self.draw_menu_surface(event_loop);
                return;
            }
            if let Some(surface) = self.menu_surface.as_mut() {
                if matches!(event, WindowEvent::CloseRequested)
                    || (matches!(event, WindowEvent::Focused(false))
                        && self.menu.lock().opened_at.elapsed() >= MENU_FOCUS_GRACE)
                {
                    self.close_menu_surface();
                    self.control_surface = ControlSurface::Hidden;
                    return;
                }
                if let WindowEvent::Resized(size) = event {
                    if size.width > 0 && size.height > 0 {
                        surface.native.resize(size);
                    }
                }
                let (native, egui) = (&surface.native, &mut surface.egui);
                let window = native.window();
                if egui.on_window_event(window, &event).repaint {
                    window.request_redraw();
                }
                if matches!(
                    event,
                    WindowEvent::CursorMoved { .. }
                        | WindowEvent::MouseInput { .. }
                        | WindowEvent::MouseWheel { .. }
                ) {
                    window.request_redraw();
                }
            }
            return;
        }
        #[cfg(target_os = "macos")]
        if self
            .settings_surface
            .as_ref()
            .is_some_and(|surface| surface.id() == _window_id)
        {
            if matches!(event, WindowEvent::RedrawRequested) {
                self.draw_settings_surface(event_loop);
                return;
            }
            if let Some(surface) = self.settings_surface.as_mut() {
                if matches!(event, WindowEvent::CloseRequested) {
                    surface.window().set_visible(false);
                    self.settings.lock().open = false;
                    self.control_surface = ControlSurface::Hidden;
                    return;
                }
                if let WindowEvent::Resized(size) = event {
                    if size.width > 0 && size.height > 0 {
                        surface.native.resize(size);
                    }
                }
                let geometry_changed =
                    matches!(event, WindowEvent::Resized(_) | WindowEvent::Moved(_));
                let (native, egui) = (&surface.native, &mut surface.egui);
                let window = native.window();
                if egui.on_window_event(window, &event).repaint {
                    window.request_redraw();
                }
                let _ = surface;
                if matches!(
                    event,
                    WindowEvent::CursorMoved { .. }
                        | WindowEvent::MouseInput { .. }
                        | WindowEvent::MouseWheel { .. }
                ) {
                    window.request_redraw();
                }
                if geometry_changed {
                    self.save_settings_surface_geometry();
                }
            }
            return;
        }
        if matches!(event, WindowEvent::Destroyed) {
            event_loop.exit();
            return;
        }
        if matches!(event, WindowEvent::ThemeChanged(_)) {
            self.sync_pet_theme();
        }
        if matches!(event, WindowEvent::CloseRequested) {
            self.hide_control_window();
            return;
        }
        #[cfg(windows)]
        if matches!(event, WindowEvent::Focused(false))
            && self.control_surface == ControlSurface::Menu
            && self.menu.lock().opened_at.elapsed() >= MENU_FOCUS_GRACE
        {
            self.menu.lock().open = false;
            self.hide_control_window();
            return;
        }
        if matches!(event, WindowEvent::RedrawRequested) {
            self.draw(event_loop);
            return;
        }
        if let WindowEvent::Resized(size) = event {
            if size.width > 0 && size.height > 0 {
                if let Some(window) = self.gl_window.as_ref() {
                    window.resize(size);
                }
            }
        }
        #[cfg(not(windows))]
        if let WindowEvent::KeyboardInput { event, .. } = &event {
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
                self.engine.active_pet().on_event(pet_event);
                if let Some(window) = self.window() {
                    window.request_redraw();
                }
            }
        }
        #[cfg(not(windows))]
        if self.control_surface == ControlSurface::Hidden {
            match &event {
                #[cfg(target_os = "macos")]
                WindowEvent::CursorMoved { .. } if self.mac_mouse.primary_down => {
                    let Some((cursor_x, cursor_y)) = crate::platform::cursor_screen_px() else {
                        return;
                    };
                    let Some((press_x, press_y)) = self.mac_press_cursor else {
                        return;
                    };
                    let dx = cursor_x - press_x;
                    let dy = cursor_y - press_y;
                    let threshold = 4.0 * self.window().map(|w| w.scale_factor()).unwrap_or(1.0);
                    if !self.mac_dragging && ((dx as f64).hypot(dy as f64)) >= threshold {
                        self.mac_dragging = true;
                        self.engine.active_pet().on_event(PetEvent::DragStarted);
                    }
                    if self.mac_dragging {
                        if let Some(window) = self.window() {
                            if let Some(origin) = self.mac_press_window {
                                window.set_outer_position(winit::dpi::PhysicalPosition::new(
                                    origin.x + dx,
                                    origin.y + dy,
                                ));
                            }
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Right,
                    ..
                } => self.show_menu(event_loop),
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    self.mac_mouse.primary_down = true;
                    self.mac_press_cursor = crate::platform::cursor_screen_px();
                    self.mac_press_window = self.window().and_then(|w| w.outer_position().ok());
                    self.engine.active_pet().on_event(PetEvent::MousePressed {
                        button: PetMouseButton::Primary,
                        modifiers: PetModifiers::NONE,
                    });
                    if let Some(window) = self.window() {
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Released,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    self.mac_mouse.primary_down = false;
                    if self.mac_dragging {
                        self.mac_dragging = false;
                        self.engine.active_pet().on_event(PetEvent::DragEnded {
                            drag: DragState::ACTIVE,
                        });
                        self.snap_mac_pet_window();
                    } else {
                        self.engine.active_pet().on_event(PetEvent::MouseClicked {
                            button: PetMouseButton::Primary,
                            modifiers: PetModifiers::NONE,
                        });
                    }
                    self.engine.active_pet().on_event(PetEvent::MouseReleased {
                        button: PetMouseButton::Primary,
                        modifiers: PetModifiers::NONE,
                    });
                    self.mac_press_cursor = None;
                    self.mac_press_window = None;
                    self.save_pet_window_position();
                }
                _ => {}
            }
        }
        if let (Some(egui), Some(window)) = (self.egui.as_mut(), self.gl_window.as_ref()) {
            if egui.on_window_event(window.window(), &event).repaint {
                window.window().request_redraw();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            #[cfg(target_os = "macos")]
            UserEvent::GlobalMouse(button) => {
                self.engine
                    .active_pet()
                    .on_event(PetEvent::GlobalMousePressed {
                        button,
                        modifiers: PetModifiers::NONE,
                    });
                if let Some(window) = self.window() {
                    window.request_redraw();
                }
            }
            #[cfg(target_os = "macos")]
            UserEvent::GlobalKey { key, pressed } => {
                self.engine.active_pet().on_event(if pressed {
                    PetEvent::GlobalKeyPressed {
                        key,
                        modifiers: PetModifiers::NONE,
                    }
                } else {
                    PetEvent::GlobalKeyReleased {
                        key,
                        modifiers: PetModifiers::NONE,
                    }
                });
                if let Some(window) = self.window() {
                    window.request_redraw();
                }
            }
            UserEvent::Commands => self.process_commands(event_loop),
            UserEvent::Repaint(delay) if delay.is_zero() => {
                self.repaint_at = None;
                if let Some(window) = self.window() {
                    window.request_redraw();
                }
            }
            UserEvent::Repaint(delay) => {
                if let Some(at) = Instant::now().checked_add(delay) {
                    self.repaint_at = Some(self.repaint_at.map_or(at, |current| current.min(at)));
                }
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            if self.repaint_at.is_some_and(|at| at <= Instant::now()) {
                self.repaint_at = None;
                if let Some(window) = self.window() {
                    window.request_redraw();
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_commands(event_loop);
        #[cfg(not(windows))]
        self.update_mac_behavior();
        #[cfg(target_os = "macos")]
        self.dispose_menu_surface_if_pending();
        #[cfg(target_os = "macos")]
        if self
            .menu_surface
            .as_ref()
            .is_some_and(|surface| surface.window().is_visible().unwrap_or(false))
            || self
                .settings_surface
                .as_ref()
                .is_some_and(|surface| surface.window().is_visible().unwrap_or(false))
        {
            // The settings surface owns the focused event stream, but the
            // transparent pet must keep receiving animation frames.
            if let Some(window) = self.window() {
                window.request_redraw();
            }
            self.repaint_at = Some(Instant::now() + PET_FRAME_INTERVAL);
        }
        #[cfg(not(windows))]
        if matches!(
            self.control_surface,
            ControlSurface::Menu | ControlSurface::Settings
        ) {
            if let Some(window) = self.window() {
                window.request_redraw();
            }
            self.repaint_at = Some(Instant::now() + PET_FRAME_INTERVAL);
        }
        if let Some(wake_at) = self.repaint_at {
            event_loop.set_control_flow(ControlFlow::WaitUntil(wake_at));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.controls.request_shutdown();
        if let Some(egui) = self.egui.as_mut() {
            egui.destroy();
        }
        #[cfg(target_os = "macos")]
        if let Some(surface) = self.menu_surface.as_mut() {
            surface.egui.destroy();
        }
        #[cfg(target_os = "macos")]
        if let Some(surface) = self.settings_surface.as_mut() {
            surface.egui.destroy();
        }
    }
}

#[cfg(not(windows))]
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
        KeyCode::ShiftLeft | KeyCode::ShiftRight => PetKey::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => PetKey::Ctrl,
        KeyCode::AltLeft | KeyCode::AltRight => PetKey::Alt,
        KeyCode::SuperLeft | KeyCode::SuperRight => PetKey::Super,
        KeyCode::F1 => PetKey::Function(1),
        KeyCode::F2 => PetKey::Function(2),
        KeyCode::F3 => PetKey::Function(3),
        KeyCode::F4 => PetKey::Function(4),
        KeyCode::F5 => PetKey::Function(5),
        KeyCode::F6 => PetKey::Function(6),
        KeyCode::F7 => PetKey::Function(7),
        KeyCode::F8 => PetKey::Function(8),
        KeyCode::F9 => PetKey::Function(9),
        KeyCode::F10 => PetKey::Function(10),
        KeyCode::F11 => PetKey::Function(11),
        KeyCode::F12 => PetKey::Function(12),
        _ => return None,
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn mac_key_from_keycode(code: u16) -> Option<deskhud_engine::PetKey> {
    use deskhud_engine::PetKey;
    Some(match code {
        36 => PetKey::Enter,
        48 => PetKey::Tab,
        49 => PetKey::Space,
        51 => PetKey::Backspace,
        53 => PetKey::Escape,
        123 => PetKey::ArrowLeft,
        124 => PetKey::ArrowRight,
        125 => PetKey::ArrowDown,
        126 => PetKey::ArrowUp,
        122 => PetKey::Function(1),
        120 => PetKey::Function(2),
        99 => PetKey::Function(3),
        118 => PetKey::Function(4),
        96 => PetKey::Function(5),
        97 => PetKey::Function(6),
        98 => PetKey::Function(7),
        100 => PetKey::Function(8),
        101 => PetKey::Function(9),
        109 => PetKey::Function(10),
        103 => PetKey::Function(11),
        111 => PetKey::Function(12),
        56 | 60 => PetKey::Shift,
        59 | 62 => PetKey::Ctrl,
        58 | 61 => PetKey::Alt,
        55 | 54 => PetKey::Super,
        _ => return None,
    })
}

#[cfg(not(windows))]
fn draw_pet_paint(ui: &egui::Ui, paint: deskhud_engine::PetPaint) {
    let rect = ui.max_rect();
    #[cfg(windows)]
    let response = ui.interact(
        rect,
        egui::Id::new("macos.pet.interaction"),
        egui::Sense::click_and_drag(),
    );
    #[cfg(windows)]
    if response.secondary_clicked() {
        _controls.request(OverlayControlCommand::OpenMenu);
    }
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.36 * paint.bounce.max(0.1);
    let blink = paint.eye_open.clamp(0.0, 1.25);
    let pupil = egui::vec2(paint.pupil_offset[0], paint.pupil_offset[1]);
    let body = egui::Color32::from_rgb(
        (paint.body_rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
        (paint.body_rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
        (paint.body_rgb[2].clamp(0.0, 1.0) * 255.0) as u8,
    );
    let painter = ui.painter();
    painter.circle_filled(center, radius, body);
    let eye_size = egui::vec2(radius * 0.14, radius * 0.14 * blink.max(0.08));
    let left_eye = center + egui::vec2(-radius * 0.28, -radius * 0.12);
    let right_eye = center + egui::vec2(radius * 0.28, -radius * 0.12);
    painter.add(egui::Shape::ellipse_filled(
        left_eye,
        eye_size,
        egui::Color32::WHITE,
    ));
    painter.add(egui::Shape::ellipse_filled(
        right_eye,
        eye_size,
        egui::Color32::WHITE,
    ));
    painter.circle_filled(
        left_eye + pupil,
        radius * 0.065 * blink.max(0.08),
        egui::Color32::from_rgb(28, 32, 40),
    );
    painter.circle_filled(
        right_eye + pupil,
        radius * 0.065 * blink.max(0.08),
        egui::Color32::from_rgb(28, 32, 40),
    );
    if let Some(text) = paint.bubble_text.as_deref().filter(|text| !text.is_empty()) {
        let bubble_width = (text.chars().count() as f32 * 8.5 + 24.0)
            .min(rect.width() - 8.0)
            .max(70.0);
        let bubble_rect = egui::Rect::from_min_size(
            egui::pos2(rect.center().x - bubble_width / 2.0, rect.top() + 4.0),
            egui::vec2(bubble_width, 28.0),
        );
        painter.rect_filled(
            bubble_rect,
            10.0,
            egui::Color32::from_rgba_unmultiplied(248, 248, 252, 242),
        );
        painter.text(
            bubble_rect.center(),
            egui::Align2::CENTER_CENTER,
            text.chars().take(18).collect::<String>(),
            egui::FontId::proportional(12.0),
            egui::Color32::from_rgb(32, 34, 40),
        );
    }
}

pub(crate) struct GlutinWindow {
    window: Window,
    context: glutin::context::PossiblyCurrentContext,
    display: glutin::display::Display,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlutinWindow {
    pub(crate) unsafe fn new(event_loop: &ActiveEventLoop) -> Result<(Self, glow::Context)> {
        #[allow(unused_mut)]
        let mut attributes = WindowAttributes::default()
            .with_title("DeskHud")
            .with_visible(false)
            .with_decorations(false)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(
                MENU_INITIAL_WIDTH,
                crate::pet_menu::menu_height() as f64,
            ))
            .with_position(LogicalPosition::new(-32_000.0, -32_000.0));
        #[cfg(target_os = "macos")]
        {
            attributes = attributes.with_transparent(true);
        }
        #[cfg(windows)]
        {
            attributes = attributes.with_skip_taskbar(true);
        }
        let template = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(cfg!(target_os = "macos"));
        let (window, config) = glutin_winit::DisplayBuilder::new()
            .with_preference(glutin_winit::ApiPreference::FallbackEgl)
            .with_window_attributes(Some(attributes.clone()))
            .build(event_loop, template, |mut configs| {
                configs.next().expect("no OpenGL configuration available")
            })
            .map_err(|error| anyhow::anyhow!("create OpenGL display: {error}"))?;
        let display = config.display();
        let raw = window
            .as_ref()
            .and_then(|window| window.window_handle().ok())
            .map(|handle| handle.as_raw());
        let context_attributes = glutin::context::ContextAttributesBuilder::new().build(raw);
        let fallback_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(raw);
        let not_current = unsafe {
            display
                .create_context(&config, &context_attributes)
                .or_else(|_| display.create_context(&config, &fallback_attributes))
        }
        .context("create OpenGL context")?;
        let window = match window {
            Some(window) => window,
            None => glutin_winit::finalize_window(event_loop, attributes, &config)
                .context("finalize native UI window")?,
        };
        let size = window.inner_size();
        let width = NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN);
        let raw = window
            .window_handle()
            .context("get native UI window handle")?
            .as_raw();
        let surface_attributes =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(raw, width, height);
        let surface = unsafe { display.create_window_surface(&config, &surface_attributes) }
            .context("create OpenGL window surface")?;
        let context = not_current
            .make_current(&surface)
            .context("activate OpenGL context")?;
        surface
            .set_swap_interval(
                &context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            )
            .context("enable UI vsync")?;
        let host = Self {
            window,
            context,
            display,
            surface,
        };
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                let Ok(name) = std::ffi::CString::new(name) else {
                    return std::ptr::null();
                };
                host.display.get_proc_address(&name)
            })
        };
        Ok((host, gl))
    }

    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&self, size: winit::dpi::PhysicalSize<u32>) {
        let width = NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN);
        self.surface.resize(&self.context, width, height);
    }

    fn swap_buffers(&self) -> glutin::error::Result<()> {
        self.surface.swap_buffers(&self.context)
    }
}
