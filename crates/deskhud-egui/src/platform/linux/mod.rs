//! Non-Windows geometry fallback for the direct egui host.

use super::OverlayBackend;
use anyhow::Result;
use deskhud_engine::{
    OverlayBackendCapabilities, OverlayEvent, OverlayPoint, OverlayRect, OverlayScene,
    OverlayScreenArea, OverlayWindowId, OverlayWindowLevel, OverlayWindowRole,
};

mod pet;
pub(crate) use pet::PetHost;

#[allow(dead_code)]
pub(crate) struct FallbackOverlayBackend;

impl OverlayBackend for FallbackOverlayBackend {
    fn capabilities(&self) -> OverlayBackendCapabilities {
        OverlayBackendCapabilities::default()
    }
    fn create_window(&mut self, _role: OverlayWindowRole) -> Result<OverlayWindowId> {
        Ok(OverlayWindowId(1))
    }
    fn update_scene(&mut self, _id: OverlayWindowId, _scene: OverlayScene) -> Result<()> {
        Ok(())
    }
    fn set_visible(&mut self, _id: OverlayWindowId, _visible: bool) -> Result<()> {
        Ok(())
    }
    fn set_level(&mut self, _id: OverlayWindowId, _level: OverlayWindowLevel) -> Result<()> {
        Ok(())
    }
    fn destroy_window(&mut self, _id: OverlayWindowId) -> Result<()> {
        Ok(())
    }
    fn poll_events(&mut self) -> Vec<OverlayEvent> {
        Vec::new()
    }
    fn screen_area(&self) -> Result<OverlayScreenArea> {
        let display = OverlayRect {
            origin: OverlayPoint { x: 0.0, y: 0.0 },
            width: 1920.0,
            height: 1080.0,
        };
        Ok(OverlayScreenArea {
            display,
            active: display,
            excluded: Vec::new(),
        })
    }
}

/// A portable global-cursor backend has not been wired yet.
#[allow(dead_code)]
pub fn cursor_screen_px() -> Option<(i32, i32)> {
    None
}

/// Conservative primary-display bounds in pixels. The real geometry backend is not
/// wired for Linux yet, so report a fixed 1080p desktop.
#[allow(dead_code)]
pub fn main_display_bounds_px() -> (f32, f32, f32, f32) {
    (0.0, 0.0, 1920.0, 1080.0)
}

/// Conservative primary-display work area in pixels (same fixed desktop as the bounds).
#[allow(dead_code)]
pub fn main_display_work_area_px() -> (f32, f32, f32, f32) {
    (0.0, 0.0, 1920.0, 1080.0)
}

/// Keep a popup inside a conservative primary-display work area.
#[allow(dead_code)]
pub fn fit_popup_pos_points(
    cursor_points: (f32, f32),
    menu_w: f32,
    menu_h: f32,
    _pixels_per_point: f32,
) -> (f32, f32) {
    const WORK_WIDTH: f32 = 1920.0;
    const WORK_HEIGHT: f32 = 1080.0;
    const GAP: f32 = 2.0;

    let x = if cursor_points.0 + GAP + menu_w <= WORK_WIDTH {
        cursor_points.0 + GAP
    } else {
        cursor_points.0 - menu_w - GAP
    };
    let y = if cursor_points.1 + GAP + menu_h <= WORK_HEIGHT {
        cursor_points.1 + GAP
    } else {
        cursor_points.1 - menu_h - GAP
    };

    (x.max(0.0), y.max(0.0))
}
