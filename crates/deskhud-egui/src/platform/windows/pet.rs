//! Windows desktop-pet host.
//!
//! The real Windows pet overlay runs on its own DirectComposition thread
//! (`gpu_overlay_probe`); the egui host owns no pet surface here. Every
//! method is a no-op so the shell never branches on the pet path for Windows.
#![allow(dead_code)]

use std::time::Instant;

use winit::window::Window;

use crate::overlay_control::OverlayControlCommand;
use crate::platform::OverlayBackend;

/// Windows keeps the pet entirely on its native overlay thread; the shell has no pet.
pub(crate) struct PetHost;

impl PetHost {
    pub(crate) fn new() -> Self {
        PetHost
    }

    pub(crate) fn is_desktop_pet(&self) -> bool {
        false
    }

    pub(crate) fn resume(
        &mut self,
        _prefs: &deskhud_ui::UiPreferences,
        _overlay: &mut Box<dyn OverlayBackend>,
        _window: Option<&Window>,
    ) {
    }

    /// Route a pet scoped window event; Windows pets are driven by `gpu_overlay_probe`.
    pub(crate) fn window_event(
        &mut self,
        _window: Option<&Window>,
        _pet_active: bool,
        _event: &winit::event::WindowEvent,
        _engine: &mut deskhud_engine::EngineRegistry,
        _prefs: &mut deskhud_ui::UiPreferences,
        _overlay: &mut Box<dyn OverlayBackend>,
    ) -> bool {
        false
    }

    pub(crate) fn about_to_wait(
        &mut self,
        _window: Option<&Window>,
        _animate_pet: bool,
        _engine: &mut deskhud_engine::EngineRegistry,
        _prefs: &deskhud_ui::UiPreferences,
    ) -> Option<Instant> {
        None
    }

    pub(crate) fn apply_topmost(
        &mut self,
        _window: Option<&Window>,
        _prefs: &deskhud_ui::UiPreferences,
        _overlay: &mut Box<dyn OverlayBackend>,
    ) {
    }

    pub(crate) fn command(
        &mut self,
        _command: OverlayControlCommand,
        _engine: &mut deskhud_engine::EngineRegistry,
    ) -> Option<Instant> {
        None
    }

    pub(crate) fn frame(
        &mut self,
        _window: Option<&Window>,
        _engine: &mut deskhud_engine::EngineRegistry,
        _prefs: &deskhud_ui::UiPreferences,
    ) -> Option<deskhud_engine::PetPaint> {
        None
    }

    pub(crate) fn exiting(&mut self, _overlay: &mut Box<dyn OverlayBackend>) {}
}
