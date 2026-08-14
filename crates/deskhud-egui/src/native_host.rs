//! Direct `winit` + `egui_glow` host for opaque control surfaces.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
#[cfg(not(target_os = "macos"))]
use egui::Color32;
#[cfg(target_os = "macos")]
use glow::HasContext as _;
use glutin::context::NotCurrentGlContext as _;
#[cfg(not(windows))]
use glutin::context::PossiblyCurrentGlContext as _;
use glutin::display::{GetGlDisplay as _, GlDisplay as _};
use glutin::prelude::GlSurface as _;
use raw_window_handle::HasWindowHandle as _;
use winit::application::ApplicationHandler;
#[cfg(windows)]
use winit::dpi::PhysicalPosition;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(windows)]
use winit::platform::windows::{WindowAttributesExtWindows as _, WindowExtWindows as _};

use crate::overlay_control::{OverlayControlBus, OverlayControlCommand};
#[cfg(target_os = "macos")]
use crate::overlay_surface::OverlaySurface;
use crate::pet_menu::PetMenuHost;
use crate::platform::{OverlayBackend, PetHost};
use crate::settings::{SettingsHost, SettingsTab};

const MENU_INITIAL_WIDTH: f64 = 180.0;
const SETTINGS_WIDTH: f64 = 920.0;
const SETTINGS_HEIGHT: f64 = 680.0;
#[cfg(target_os = "macos")]
const MENU_FOCUS_GRACE: Duration = Duration::from_millis(220);
#[cfg(windows)]
const MENU_FOCUS_GRACE: Duration = Duration::from_millis(180);

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

#[cfg(target_os = "macos")]
fn setup_surface_repaint(surface: &OverlaySurface, proxy: &EventLoopProxy<UserEvent>) {
    let proxy = proxy.clone();
    surface
        .egui
        .egui_ctx
        .set_request_repaint_callback(move |info| {
            let _ = proxy.send_event(UserEvent::Repaint(info.delay));
        });
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
    #[allow(dead_code)]
    overlay_backend: Box<dyn OverlayBackend>,
    desk_pet: PetHost,
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
    last_gl_context_error: Option<Instant>,
}

impl NativeHost {
    fn new(
        proxy: EventLoopProxy<UserEvent>,
        prefs: deskhud_ui::UiPreferences,
        controls: OverlayControlBus,
    ) -> Self {
        let boot = deskhud_runtime::bootstrap_registry();
        let catalogs = deskhud_runtime::build_catalog_store(&boot.discovered, prefs.locale);
        let overlay_backend = crate::platform::create_backend()
            .expect("platform overlay backend construction must be infallible");
        let mut engine = boot.registry;
        let _ = engine.set_active_pet(&prefs.pet.kind);
        Self {
            proxy,
            controls,
            engine,
            catalogs,
            settings: SettingsHost::new(prefs.clone()),
            menu: PetMenuHost::new(prefs.clone()),
            overlay_backend,
            desk_pet: PetHost::new(),
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
            last_gl_context_error: None,
        }
    }

    fn window(&self) -> Option<&Window> {
        self.gl_window.as_ref().map(GlutinWindow::window)
    }

    fn process_commands(&mut self, event_loop: &ActiveEventLoop) {
        for command in self.controls.drain() {
            match command {
                OverlayControlCommand::ActivateExisting => {
                    #[cfg(target_os = "macos")]
                    if self.control_surface == ControlSurface::Menu {
                        self.close_menu_surface();
                        self.control_surface = ControlSurface::Hidden;
                    }
                }
                OverlayControlCommand::OpenMenu => self.show_menu(event_loop),
                OverlayControlCommand::OpenSettings => self.show_settings(event_loop),
                OverlayControlCommand::OpenHudLayout => {
                    #[cfg(windows)]
                    crate::gpu_overlay_probe::open_layout_editor();
                    #[cfg(target_os = "macos")]
                    tracing::info!(
                        "HUD layout mode is disabled on macOS while the native pet window is active"
                    );
                }
                OverlayControlCommand::SetTopmost(enabled) => {
                    self.prefs.shell.topmost = enabled;
                    self.menu.lock().pet_topmost = enabled;
                    self.save_prefs();
                    #[cfg(windows)]
                    crate::gpu_overlay_probe::set_topmost(enabled);
                }
                OverlayControlCommand::SetHudMaster(enabled) => {
                    self.prefs.hud.set_master_enabled(enabled);
                    self.menu.lock().master_enabled = enabled;
                    self.save_prefs();
                }
                OverlayControlCommand::PetMoved { x_points, y_points } => {
                    self.prefs.pet.set_pos(x_points, y_points);
                    let mut settings = self.settings.lock();
                    settings.prefs.pet.set_pos(x_points, y_points);
                    settings.baseline.pet.set_pos(x_points, y_points);
                    drop(settings);
                    self.save_prefs();
                }
                OverlayControlCommand::PetDragStarted => {
                    self.desk_pet.command(command, &mut self.engine);
                }
                OverlayControlCommand::PetDragEnded => {
                    self.desk_pet.command(command, &mut self.engine);
                }
                OverlayControlCommand::Quit => event_loop.exit(),
            }
        }
    }

    #[allow(clippy::needless_return)]
    fn show_menu(&mut self, _event_loop: &ActiveEventLoop) {
        if self.control_surface == ControlSurface::Settings {
            if let Some(window) = self.window() {
                window.focus_window();
                window.request_redraw();
            }
            return;
        }
        #[cfg(windows)]
        let ppp = {
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
            ppp
        };
        #[cfg(target_os = "macos")]
        {
            if self.menu_surface.is_none() {
                let surface = match unsafe { OverlaySurface::new(_event_loop) } {
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
                setup_surface_repaint(&surface, &self.proxy);
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

    fn show_settings_tab(&mut self, _event_loop: &ActiveEventLoop, tab: SettingsTab) {
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
            let surface = match unsafe { OverlaySurface::new(_event_loop) } {
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
            setup_surface_repaint(&surface, &self.proxy);
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
        #[cfg(windows)]
        if let Some(window) = self.window() {
            window.set_visible(false);
        }
        #[cfg(target_os = "macos")]
        self.close_menu_surface();
        #[allow(unused_variables)]
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let window = self.gl_window.as_ref().map(GlutinWindow::window);
            self.desk_pet
                .resume(&self.prefs, &mut self.overlay_backend, window);
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

    #[allow(clippy::needless_return)]
    fn save_prefs(&self) {
        if let Err(error) = deskhud_ui::persist::save(&self.prefs) {
            tracing::warn!(%error, "native host prefs save failed");
            return;
        }
        #[cfg(windows)]
        crate::gpu_overlay_probe::request_prefs_reload();
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
            {
                let window = self.gl_window.as_ref().map(GlutinWindow::window);
                self.desk_pet
                    .apply_topmost(window, &self.prefs, &mut self.overlay_backend);
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
            #[cfg(windows)]
            crate::gpu_overlay_probe::open_layout_editor();
            #[cfg(not(target_os = "macos"))]
            self.hide_control_window();
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
        let layout_request = {
            let mut state = self.settings.lock();
            let request = state.hud_layout_request;
            state.hud_layout_request = false;
            request
        };
        if layout_request {
            self.settings.lock().open = false;
            self.hide_control_window();
            #[cfg(windows)]
            crate::gpu_overlay_probe::open_layout_editor();
        }
        if apply {
            #[cfg(windows)]
            let topmost_changed = self.prefs.shell.topmost != draft.shell.topmost;
            self.prefs = draft.clone();
            let _ = self.engine.set_active_pet(&self.prefs.pet.kind);
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
            {
                let window = self.gl_window.as_ref().map(GlutinWindow::window);
                self.desk_pet
                    .apply_topmost(window, &self.prefs, &mut self.overlay_backend);
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
        #[cfg(windows)]
        if self.control_surface == ControlSurface::Hidden
            || self.window().and_then(Window::is_visible) != Some(true)
        {
            // Windows can deliver one queued RedrawRequested after the native
            // control window has been hidden. Do not make an invalid surface
            // current during that teardown race.
            return;
        }
        #[cfg(target_os = "macos")]
        let paint = {
            let window = self.gl_window.as_ref().map(GlutinWindow::window);
            self.desk_pet.frame(window, &mut self.engine, &self.prefs)
        };
        #[cfg(target_os = "linux")]
        {
            let window = self.gl_window.as_ref().map(GlutinWindow::window);
            let _ = self.desk_pet.frame(window, &mut self.engine, &self.prefs);
        }
        let Some(gl_window) = self.gl_window.as_mut() else {
            return;
        };
        #[cfg(not(windows))]
        if let Err(error) = gl_window.make_current() {
            let should_log = self
                .last_gl_context_error
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(1));
            if should_log {
                tracing::warn!(%error, "activate native UI GL context skipped; native surface is temporarily unavailable");
                self.last_gl_context_error = Some(Instant::now());
            }
            return;
        }
        // Windows owns a single UI GL context for this host. It is activated
        // during construction and remains current on the event-loop thread;
        // re-activating it while a just-shown/resized window surface is still
        // settling can produce transient WGL/EGL errors such as os error 2004.
        self.last_gl_context_error = None;
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
            if let Some(paint) = &paint {
                draw_pet_paint(ui, paint.clone());
            }
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
    }

    #[cfg(target_os = "macos")]
    fn draw_menu_surface(&mut self, event_loop: &ActiveEventLoop) {
        let Some(surface) = self.menu_surface.as_mut() else {
            return;
        };
        let menu = self.menu.clone();
        if let Err(error) = surface.native.make_current() {
            tracing::error!(%error, "activate macOS menu GL context failed");
            return;
        }
        let (native, egui) = (&surface.native, &mut surface.egui);
        let window = native.window();
        let theme = self.prefs.shell.ui_theme;
        if let (Some((cursor_x, cursor_y)), Ok(origin)) =
            (crate::platform::cursor_screen_px(), window.outer_position())
        {
            let local = egui::pos2((cursor_x - origin.x) as f32, (cursor_y - origin.y) as f32);
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
        if let Err(error) = surface.native.make_current() {
            tracing::error!(%error, "activate macOS settings GL context failed");
            return;
        }
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
        {
            let window = self.gl_window.as_ref().map(GlutinWindow::window);
            self.desk_pet
                .resume(&self.prefs, &mut self.overlay_backend, window);
        }
        #[cfg(target_os = "macos")]
        {
            crate::platform::set_native_pet_control_bus(self.controls.clone());
            if let Some(window) = self.window() {
                window.set_visible(false);
            }
        }
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
        {
            let window = self.gl_window.as_ref().map(GlutinWindow::window);
            if self.desk_pet.window_event(
                window,
                self.control_surface == ControlSurface::Hidden,
                &event,
                &mut self.engine,
                &mut self.prefs,
                &mut self.overlay_backend,
            ) {
                self.show_menu(event_loop);
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
                use deskhud_engine::{PetEvent, PetModifiers};
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
                use deskhud_engine::{PetEvent, PetModifiers};
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
        if matches!(cause, StartCause::ResumeTimeReached { .. })
            && self.repaint_at.is_some_and(|at| at <= Instant::now())
        {
            self.repaint_at = None;
            if let Some(window) = self.window() {
                window.request_redraw();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_commands(event_loop);
        #[cfg(target_os = "macos")]
        self.dispose_menu_surface_if_pending();
        let window = self.gl_window.as_ref().map(GlutinWindow::window);
        if let Some(wake) = self.desk_pet.about_to_wait(
            window,
            self.control_surface == ControlSurface::Hidden,
            &mut self.engine,
            &self.prefs,
        ) {
            self.repaint_at = Some(self.repaint_at.map_or(wake, |current| current.min(wake)));
        }
        if let Some(wake_at) = self.repaint_at {
            event_loop.set_control_flow(ControlFlow::WaitUntil(wake_at));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.controls.request_shutdown();
        #[cfg(not(windows))]
        self.desk_pet.exiting(&mut self.overlay_backend);
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

#[cfg(target_os = "macos")]
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

    #[cfg(not(windows))]
    pub(crate) fn make_current(&mut self) -> glutin::error::Result<()> {
        self.context.make_current(&self.surface)
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
