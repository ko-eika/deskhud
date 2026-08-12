//! macOS platform boundary for the regular application path.
//!
//! Native transparent composition can be added here later without changing
//! the engine or package contracts, and without affecting Windows.

use anyhow::Result;
use core_graphics::display::CGDisplay;
use core_graphics::event::EventField;
use core_graphics::event::{
    CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use deskhud_engine::{
    OverlayBackendCapabilities, OverlayEvent, OverlayPoint, OverlayRect, OverlayScene,
    OverlayScreenArea, OverlayWindowId, OverlayWindowLevel, OverlayWindowRole,
};
use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;

use super::OverlayBackend;

/// macOS overlay backend state.
///
/// The native AppKit window objects are intentionally the next implementation
/// step. Keeping this state behind the common backend contract lets the shell
/// stop growing macOS-specific window branches in the meantime.
#[allow(dead_code)]
pub(crate) struct MacosOverlayBackend {
    next_id: u64,
    capabilities: OverlayBackendCapabilities,
}

impl Default for MacosOverlayBackend {
    fn default() -> Self {
        Self {
            next_id: 1,
            capabilities: OverlayBackendCapabilities {
                desktop_transparency: true,
                per_region_passthrough: false,
                selected_display: true,
                virtual_desktop: false,
            },
        }
    }
}

impl OverlayBackend for MacosOverlayBackend {
    fn capabilities(&self) -> OverlayBackendCapabilities {
        self.capabilities
    }

    fn create_window(&mut self, _role: OverlayWindowRole) -> Result<OverlayWindowId> {
        let id = OverlayWindowId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        Ok(id)
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
        let display = crate::platform::main_display_bounds_px();
        let active = crate::platform::main_display_work_area_px();
        let display_rect = rect_from_bounds(display);
        let active_rect = rect_from_bounds(active);
        let mut excluded = Vec::new();
        if active_rect.origin.y > display_rect.origin.y {
            excluded.push(OverlayRect {
                origin: display_rect.origin,
                width: display_rect.width,
                height: active_rect.origin.y - display_rect.origin.y,
            });
        }
        if active_rect.origin.y + active_rect.height < display_rect.origin.y + display_rect.height {
            excluded.push(OverlayRect {
                origin: OverlayPoint {
                    x: display_rect.origin.x,
                    y: active_rect.origin.y + active_rect.height,
                },
                width: display_rect.width,
                height: display_rect.origin.y + display_rect.height
                    - (active_rect.origin.y + active_rect.height),
            });
        }
        Ok(OverlayScreenArea {
            display: display_rect,
            active: active_rect,
            excluded,
        })
    }
}

fn rect_from_bounds(bounds: (f32, f32, f32, f32)) -> OverlayRect {
    OverlayRect {
        origin: OverlayPoint {
            x: bounds.0,
            y: bounds.1,
        },
        width: (bounds.2 - bounds.0).max(0.0),
        height: (bounds.3 - bounds.1).max(0.0),
    }
}

/// Return the global cursor in Quartz display coordinates.
pub(crate) fn cursor_screen_px() -> Option<(i32, i32)> {
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let point = event.location();
    Some((point.x.round() as i32, point.y.round() as i32))
}

/// Start a passive global mouse tap. macOS requires Accessibility/Input
/// Monitoring permission; failure is intentionally non-fatal.
pub(crate) fn start_global_mouse_listener(
    proxy: winit::event_loop::EventLoopProxy<crate::native_host::UserEvent>,
) {
    std::thread::spawn(move || {
        let tap = CGEventTap::new(
            CGEventTapLocation::Session,
            CGEventTapPlacement::TailAppendEventTap,
            CGEventTapOptions::ListenOnly,
            vec![
                CGEventType::LeftMouseDown,
                CGEventType::RightMouseDown,
                CGEventType::OtherMouseDown,
                CGEventType::ScrollWheel,
                CGEventType::KeyDown,
                CGEventType::KeyUp,
            ],
            move |_proxy, kind, event| {
                if matches!(kind, CGEventType::KeyDown | CGEventType::KeyUp) {
                    let code =
                        event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                    if let Some(key) = crate::native_host::mac_key_from_keycode(code) {
                        let _ = proxy.send_event(crate::native_host::UserEvent::GlobalKey {
                            key,
                            pressed: matches!(kind, CGEventType::KeyDown),
                        });
                    }
                    return None;
                }
                let button = match kind {
                    CGEventType::LeftMouseDown => deskhud_engine::PetMouseButton::Primary,
                    CGEventType::RightMouseDown => deskhud_engine::PetMouseButton::Secondary,
                    CGEventType::OtherMouseDown => deskhud_engine::PetMouseButton::Middle,
                    _ => return None,
                };
                let _ = proxy.send_event(crate::native_host::UserEvent::GlobalMouse(button));
                None
            },
        );
        let Ok(tap) = tap else {
            tracing::warn!(
                "macOS global mouse tap unavailable; grant Accessibility/Input Monitoring permission"
            );
            return;
        };
        let run_loop = core_foundation::runloop::CFRunLoop::get_current();
        let Ok(source) = tap.mach_port.create_runloop_source(0) else {
            return;
        };
        unsafe {
            run_loop.add_source(&source, core_foundation::runloop::kCFRunLoopCommonModes);
        }
        tap.enable();
        core_foundation::runloop::CFRunLoop::run_current();
    });
}

/// Main display bounds in Quartz global pixel coordinates.
pub(crate) fn main_display_bounds_px() -> (f32, f32, f32, f32) {
    let bounds = CGDisplay::main().bounds();
    (
        bounds.origin.x as f32,
        bounds.origin.y as f32,
        (bounds.origin.x + bounds.size.width) as f32,
        (bounds.origin.y + bounds.size.height) as f32,
    )
}

/// AppKit visible frame in Quartz-like global coordinates. This excludes the
/// menu bar and Dock and is the safe area reserved for future HUD layout.
pub(crate) fn main_display_work_area_px() -> (f32, f32, f32, f32) {
    let Some(mtm) = MainThreadMarker::new() else {
        return main_display_bounds_px();
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return main_display_bounds_px();
    };
    let frame = screen.visibleFrame();
    let full = CGDisplay::main().bounds();
    // AppKit reports points with a bottom-left origin; Quartz reports pixels
    // with a top-left origin. Convert both scale and Y origin explicitly.
    let scale_x = full.size.width as f64 / screen.frame().size.width.max(1.0);
    let scale_y = full.size.height as f64 / screen.frame().size.height.max(1.0);
    let full_bottom = full.origin.y + full.size.height;
    let top = full_bottom - (frame.origin.y + frame.size.height) * scale_y;
    let bottom = full_bottom - frame.origin.y * scale_y;
    (
        (full.origin.x + frame.origin.x * scale_x) as f32,
        top as f32,
        (full.origin.x + (frame.origin.x + frame.size.width) * scale_x) as f32,
        bottom as f32,
    )
}

/// Keep a popup inside the main display's global bounds.
///
/// Core Graphics reports global display bounds, including negative origins for
/// displays placed to the left or above the main display. The menu bar is not
/// subtracted here; AppKit can refine that policy when the native overlay
/// backend is introduced.
pub(crate) fn fit_popup_pos_points(
    cursor_points: (f32, f32),
    menu_w: f32,
    menu_h: f32,
    pixels_per_point: f32,
) -> (f32, f32) {
    const GAP: f32 = 2.0;

    let scale = pixels_per_point.max(0.01);
    let cursor = (cursor_points.0 * scale, cursor_points.1 * scale);
    let bounds = CGDisplay::main().bounds();
    let width = menu_w * scale;
    let height = menu_h * scale;
    let left = bounds.origin.x as f32;
    let top = bounds.origin.y as f32;
    let right = left + bounds.size.width as f32;
    let bottom = top + bounds.size.height as f32;
    let x = (cursor.0 + GAP * scale).min(right - width).max(left);
    let y = (cursor.1 + GAP * scale).min(bottom - height).max(top);

    (x / scale, y / scale)
}
