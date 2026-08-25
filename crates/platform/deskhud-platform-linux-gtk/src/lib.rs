//! Linux GTK backend boundary. GTK lifecycle is added behind this API.
use deskhud_platform::*;

#[derive(Default)]
pub struct GtkPlatform {
    state: HostState,
}
impl GtkPlatform {
    pub fn state(&self) -> &HostState {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut HostState {
        &mut self.state
    }
}

impl SettingsHost for GtkPlatform {
    fn open_settings(&mut self) -> Result<(), PlatformError> {
        Ok(())
    }
}
impl OverlayHost for GtkPlatform {
    fn create_overlay(&mut self, _role: WindowRole) -> Result<WindowId, PlatformError> {
        Ok(self.state.create_overlay(_role))
    }
    fn set_overlay_visible(&mut self, _id: WindowId, _visible: bool) -> Result<(), PlatformError> {
        self.state.set_overlay_visible(_id, _visible)
    }
    fn set_overlay_level(
        &mut self,
        _id: WindowId,
        _level: WindowLevel,
    ) -> Result<(), PlatformError> {
        self.state.set_overlay_level(_id, _level)
    }
}
impl MenuHost for GtkPlatform {
    fn show_menu(&mut self, anchor: Rect) -> Result<(), PlatformError> {
        self.state.show_menu(anchor);
        Ok(())
    }
}
impl WindowHost for GtkPlatform {
    fn close_window(&mut self, id: WindowId) -> Result<(), PlatformError> {
        self.state.close_window(id)
    }
}
impl DisplayHost for GtkPlatform {
    fn displays(&self) -> Result<Vec<DisplayInfo>, PlatformError> {
        Ok(self.state.displays())
    }
}
impl InputHost for GtkPlatform {
    fn poll_events(&mut self) -> Result<Vec<PlatformEvent>, PlatformError> {
        Ok(self.state.poll_events())
    }
}

/// Run the temporary ordinary native window used as the Linux baseline.
pub fn run_blank_window(title: &str) -> Result<(), PlatformError> {
    let event_loop = winit::event_loop::EventLoop::new()
        .map_err(|error| PlatformError::new(format!("create Linux event loop: {error}")))?;
    let attributes = winit::window::Window::default_attributes().with_title(title);
    event_loop
        .run_app(&mut BlankApp {
            attributes,
            window: None,
        })
        .map_err(|error| PlatformError::new(format!("run Linux window: {error}")))
}

struct BlankApp {
    attributes: winit::window::WindowAttributes,
    window: Option<winit::window::Window>,
}
impl winit::application::ApplicationHandler for BlankApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            self.window = event_loop.create_window(self.attributes.clone()).ok();
            if self.window.is_none() {
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if matches!(event, winit::event::WindowEvent::CloseRequested) {
            event_loop.exit();
        }
    }
}
