//! Platform-neutral shell boundary for native overlay windows.

use anyhow::Result;
use deskhud_engine::{
    OverlayBackendCapabilities, OverlayEvent, OverlayScene, OverlayScreenArea, OverlayWindowId,
    OverlayWindowLevel, OverlayWindowRole,
};

/// Coordinates native windows without exposing OS handles to the engine.
#[allow(dead_code)]
pub(crate) trait OverlayBackend {
    fn capabilities(&self) -> OverlayBackendCapabilities;
    fn create_window(&mut self, role: OverlayWindowRole) -> Result<OverlayWindowId>;
    fn update_scene(&mut self, id: OverlayWindowId, scene: OverlayScene) -> Result<()>;
    fn set_visible(&mut self, id: OverlayWindowId, visible: bool) -> Result<()>;
    fn set_level(&mut self, id: OverlayWindowId, level: OverlayWindowLevel) -> Result<()>;
    fn destroy_window(&mut self, id: OverlayWindowId) -> Result<()>;
    fn poll_events(&mut self) -> Vec<OverlayEvent>;
    /// Resolve full display, active work area, and excluded system regions.
    fn screen_area(&self) -> Result<OverlayScreenArea>;
}
