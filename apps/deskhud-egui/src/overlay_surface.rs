//! Reusable native surface state for role-specific overlay windows.

use std::sync::Arc;

use anyhow::Result;
use egui_glow::EguiGlow;
use glow::HasContext as _;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::native_host::GlutinWindow;

/// A single native window and its renderer/UI state.
///
/// Pet, menu, bubble, and HUD surfaces own one value each. Keeping this
/// state together prevents a secondary menu surface from resizing the pet
/// surface by accident.
#[allow(dead_code)]
pub(crate) struct OverlaySurface {
    pub(crate) native: GlutinWindow,
    pub(crate) gl: Arc<glow::Context>,
    pub(crate) egui: EguiGlow,
}

impl OverlaySurface {
    #[allow(dead_code)]
    /// Create a surface with an independent native window and GL context.
    pub(crate) unsafe fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        let (native, gl) = unsafe { GlutinWindow::new(event_loop)? };
        let gl = Arc::new(gl);
        let egui = EguiGlow::new(event_loop, Arc::clone(&gl), None, None, true);
        Ok(Self { native, gl, egui })
    }

    #[allow(dead_code)]
    pub(crate) fn window(&self) -> &Window {
        self.native.window()
    }

    #[allow(dead_code)]
    pub(crate) fn id(&self) -> WindowId {
        self.window().id()
    }

    #[allow(dead_code)]
    pub(crate) fn clear_transparent(&self) {
        unsafe {
            self.gl.clear_color(0.0, 0.0, 0.0, 0.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }
    }
}
