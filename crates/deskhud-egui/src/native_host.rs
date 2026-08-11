//! Direct `winit` + `egui_glow` host for opaque control surfaces.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use egui::Color32;
use glutin::context::NotCurrentGlContext as _;
use glutin::display::{GetGlDisplay as _, GlDisplay as _};
use glutin::prelude::GlSurface as _;
use raw_window_handle::HasWindowHandle as _;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition};
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

#[cfg(windows)]
use winit::platform::windows::{WindowAttributesExtWindows as _, WindowExtWindows as _};

use crate::overlay_control::{OverlayControlBus, OverlayControlCommand};
use crate::pet_menu::PetMenuHost;
use crate::settings::{SettingsHost, SettingsTab};

const MENU_INITIAL_WIDTH: f64 = 180.0;
const SETTINGS_WIDTH: f64 = 920.0;
const SETTINGS_HEIGHT: f64 = 680.0;
const MENU_FOCUS_GRACE: Duration = Duration::from_millis(180);

#[derive(Debug)]
enum UserEvent {
    Commands,
    Repaint(Duration),
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
    control_surface: ControlSurface,
    gl_window: Option<GlutinWindow>,
    gl: Option<Arc<glow::Context>>,
    egui: Option<egui_glow::EguiGlow>,
    repaint_at: Option<Instant>,
}

impl NativeHost {
    fn new(
        proxy: EventLoopProxy<UserEvent>,
        prefs: deskhud_ui::UiPreferences,
        controls: OverlayControlBus,
    ) -> Self {
        let boot = deskhud_runtime::bootstrap_registry();
        let catalogs = deskhud_runtime::build_catalog_store(&boot.discovered, prefs.locale);
        Self {
            proxy,
            controls,
            engine: boot.registry,
            catalogs,
            settings: SettingsHost::new(prefs.clone()),
            menu: PetMenuHost::new(prefs.clone()),
            prefs,
            control_surface: ControlSurface::Hidden,
            gl_window: None,
            gl: None,
            egui: None,
            repaint_at: None,
        }
    }

    fn window(&self) -> Option<&Window> {
        self.gl_window.as_ref().map(GlutinWindow::window)
    }

    fn process_commands(&mut self, event_loop: &ActiveEventLoop) {
        for command in self.controls.drain() {
            match command {
                OverlayControlCommand::OpenMenu => self.show_menu(),
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

    fn show_menu(&mut self) {
        if self.control_surface == ControlSurface::Settings {
            if let Some(window) = self.window() {
                window.focus_window();
                window.request_redraw();
            }
            return;
        }
        let Some(window) = self.window() else { return };
        let ppp = window.scale_factor() as f32;
        let cursor = crate::platform::cursor_screen_px().unwrap_or((100, 100));
        self.menu.open_at(
            &self.prefs,
            egui::pos2(cursor.0 as f32 / ppp, cursor.1 as f32 / ppp),
            ppp,
            self.prefs.hud.is_master_enabled(),
            self.prefs.shell.topmost,
        );
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

    fn show_settings(&mut self) {
        self.show_settings_tab(SettingsTab::General);
    }

    fn show_settings_tab(&mut self, tab: SettingsTab) {
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
        if let Some(window) = self.window() {
            window.set_visible(false);
        }
    }

    fn save_prefs(&self) {
        if let Err(error) = deskhud_ui::persist::save(&self.prefs) {
            tracing::warn!(%error, "native host prefs save failed");
            return;
        }
        #[cfg(windows)]
        crate::gpu_overlay_probe::request_prefs_reload();
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
            self.save_prefs();
            #[cfg(windows)]
            crate::gpu_overlay_probe::set_topmost(enabled);
        }
        if let Some(enabled) = toggle_master {
            self.prefs.hud.set_master_enabled(enabled);
            self.save_prefs();
        }
        if quit {
            event_loop.exit();
        } else if open_settings {
            self.show_settings();
        } else if open_hud_settings {
            self.show_settings_tab(SettingsTab::Hud);
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
        } else if pending_flush && !discard {
            self.prefs = draft;
            self.save_prefs();
        }
        if !open && self.control_surface == ControlSurface::Settings {
            self.hide_control_window();
        }
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(gl_window) = self.gl_window.as_ref() else {
            return;
        };
        let Some(egui) = self.egui.as_mut() else {
            return;
        };
        let control_surface = self.control_surface;
        let settings = self.settings.clone();
        let menu = self.menu.clone();
        egui.run(gl_window.window(), |ui| {
            crate::theme::apply(ui.ctx(), self.prefs.shell.ui_theme);
            match control_surface {
                ControlSurface::Hidden => {}
                ControlSurface::Menu => menu.draw_native(ui),
                ControlSurface::Settings => settings.draw_native(ui),
            }
        });

        unsafe {
            use glow::HasContext as _;
            let Some(gl) = self.gl.as_ref() else { return };
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
        self.show_settings();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
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
        if let (Some(egui), Some(window)) = (self.egui.as_mut(), self.gl_window.as_ref()) {
            if egui.on_window_event(window.window(), &event).repaint {
                window.window().request_redraw();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
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
    }
}

struct GlutinWindow {
    window: Window,
    context: glutin::context::PossiblyCurrentContext,
    display: glutin::display::Display,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlutinWindow {
    unsafe fn new(event_loop: &ActiveEventLoop) -> Result<(Self, glow::Context)> {
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
        #[cfg(windows)]
        {
            attributes = attributes.with_skip_taskbar(true);
        }
        let template = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false);
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

    fn window(&self) -> &Window {
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
