//! macOS platform boundary for the regular application path.
//!
//! Native macOS overlay lifecycle and display geometry.

use anyhow::Result;
use core_graphics::display::CGDisplay;
use core_graphics::event::EventField;
use core_graphics::event::{
    CallbackResult, CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use deskhud_engine::{
    OverlayBackendCapabilities, OverlayEvent, OverlayPoint, OverlayRect, OverlayScene,
    OverlayScreenArea, OverlayWindowId, OverlayWindowLevel, OverlayWindowRole,
};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSEvent, NSGraphicsContext, NSScreen, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGContext;
use objc2_foundation::NSRect;
use std::sync::{Mutex, OnceLock};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::{Window, WindowLevel};

use super::OverlayBackend;
use crate::overlay_control::{OverlayControlBus, OverlayControlCommand};

mod pet;
pub(crate) use pet::PetHost;

static PET_CONTROL_BUS: std::sync::OnceLock<OverlayControlBus> = std::sync::OnceLock::new();
static PET_DRAG: OnceLock<Mutex<Option<NativeDrag>>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
struct NativeDrag {
    press_cursor: CGPoint,
    press_origin: CGPoint,
    dragging: bool,
}

pub(crate) fn set_native_pet_control_bus(bus: OverlayControlBus) {
    let _ = PET_CONTROL_BUS.set(bus);
}

define_class!(
    #[unsafe(super(NSView))]
    #[name = "DeskHudPetView"]
    pub(crate) struct NativePetView;

    impl NativePetView {
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, _event: &NSEvent) {
            if let Some(bus) = PET_CONTROL_BUS.get() {
                bus.request(OverlayControlCommand::OpenMenu);
            }
        }

        #[unsafe(method(mouseDownCanMoveWindow))]
        fn mouse_down_can_move_window(&self) -> bool {
            false
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if let Some(window) = self.window() {
                let local = event.locationInWindow();
                let frame = window.frame();
                let press_cursor = CGPoint {
                    x: frame.origin.x + local.x,
                    y: frame.origin.y + local.y,
                };
                if let Ok(mut drag) = PET_DRAG.get_or_init(|| Mutex::new(None)).lock() {
                    *drag = Some(NativeDrag {
                        press_cursor,
                        press_origin: frame.origin,
                        dragging: false,
                    });
                }
            }
            if let Some(bus) = PET_CONTROL_BUS.get() {
                bus.request(OverlayControlCommand::ActivateExisting);
                bus.request(OverlayControlCommand::PetMousePressed);
            }
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            let Some(window) = self.window() else { return };
            let local = event.locationInWindow();
            let frame = window.frame();
            let cursor = CGPoint {
                x: frame.origin.x + local.x,
                y: frame.origin.y + local.y,
            };
            let Ok(mut drag) = PET_DRAG.get_or_init(|| Mutex::new(None)).lock() else {
                return;
            };
            let Some(state) = drag.as_mut() else { return };
            let dx = cursor.x - state.press_cursor.x;
            let dy = cursor.y - state.press_cursor.y;
            if !state.dragging && dx.hypot(dy) < 4.0 {
                return;
            }
            let just_started = !state.dragging;
            state.dragging = true;
            if just_started {
                if let Some(bus) = PET_CONTROL_BUS.get() {
                    bus.request(OverlayControlCommand::PetDragStarted);
                }
            }
            window.setFrameOrigin(CGPoint {
                x: state.press_origin.x + dx,
                y: state.press_origin.y + dy,
            });
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            let Some(window) = self.window() else { return };
            let Ok(mut drag) = PET_DRAG.get_or_init(|| Mutex::new(None)).lock() else {
                return;
            };
            let Some(state) = drag.take() else { return };
            if !state.dragging {
                if let Some(bus) = PET_CONTROL_BUS.get() {
                    bus.request(OverlayControlCommand::PetMouseClicked);
                    bus.request(OverlayControlCommand::PetMouseReleased);
                }
                return;
            }
            snap_native_pet_window(&window);
            if let Some(bus) = PET_CONTROL_BUS.get() {
                bus.request(OverlayControlCommand::PetDragEnded);
                bus.request(OverlayControlCommand::PetMouseReleased);
            }
            let frame = window.frame();
            if let Some(bus) = PET_CONTROL_BUS.get() {
                bus.request(OverlayControlCommand::PetMoved {
                    x_points: (frame.origin.x + frame.size.width * 0.5) as f32,
                    y_points: (frame.origin.y + frame.size.height * 0.5) as f32,
                });
            }
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty_rect: NSRect) {
        let Some(context) = NSGraphicsContext::currentContext() else {
            return;
        };
        let cg: Retained<CGContext> = context.CGContext();
        let bounds = self.bounds();
        let cx = bounds.size.width * 0.5;
        let cy = bounds.size.height * 0.5;
        let paint = PET_PAINT
            .get_or_init(|| Mutex::new(deskhud_engine::PetPaint::default()))
            .lock()
            .map(|paint| paint.clone())
            .unwrap_or_default();
        let radius = bounds.size.width.min(bounds.size.height)
            * 0.36
            * paint.bounce.max(0.1) as f64;
        CGContext::set_rgb_fill_color(Some(&cg), paint.body_rgb[0] as f64, paint.body_rgb[1] as f64, paint.body_rgb[2] as f64, 1.0);
        CGContext::add_arc(Some(&cg), cx, cy, radius, 0.0, std::f64::consts::TAU, 1);
        CGContext::fill_path(Some(&cg));
        if !paint.draw_eyes { return; }
        let eye_open = paint.eye_open.clamp(0.0, 1.0) as f64;
        CGContext::set_rgb_fill_color(Some(&cg), paint.eye_rgb[0] as f64, paint.eye_rgb[1] as f64, paint.eye_rgb[2] as f64, 1.0);
        for eye_x in [cx - radius * 0.28, cx + radius * 0.28] {
            CGContext::add_ellipse_in_rect(
                Some(&cg),
                CGRect { origin: CGPoint { x: eye_x - radius * 0.14, y: cy + radius * 0.05 + radius * 0.14 * (1.0 - eye_open) }, size: CGSize { width: radius * 0.28, height: radius * 0.28 * eye_open.max(0.08) } },
            );
            CGContext::fill_path(Some(&cg));
        }
        if eye_open > 0.06 {
            let pupil = [paint.pupil_offset[0] as f64, paint.pupil_offset[1] as f64];
            CGContext::set_rgb_fill_color(Some(&cg), 0.11, 0.13, 0.16, 1.0);
            for eye_x in [cx - radius * 0.28, cx + radius * 0.28] {
                CGContext::add_arc(Some(&cg), eye_x + pupil[0], cy + radius * 0.18 + pupil[1], radius * 0.065 * eye_open, 0.0, std::f64::consts::TAU, 1);
                CGContext::fill_path(Some(&cg));
            }
        }
        }
    }
);

static PET_PAINT: OnceLock<Mutex<deskhud_engine::PetPaint>> = OnceLock::new();

pub(crate) fn update_native_pet_paint(paint: deskhud_engine::PetPaint) {
    if let Ok(mut current) = PET_PAINT
        .get_or_init(|| Mutex::new(deskhud_engine::PetPaint::default()))
        .lock()
    {
        *current = paint;
    }
}

impl NativePetView {
    fn new(mtm: MainThreadMarker, frame: NSRect) -> Retained<Self> {
        let this = Self::alloc(mtm);
        unsafe { msg_send![this, initWithFrame: frame] }
    }
}

/// macOS overlay backend state.
///
/// Window lifecycle state kept behind the platform-neutral overlay boundary.
#[allow(dead_code)]
pub(crate) struct MacosOverlayBackend {
    next_id: u64,
    capabilities: OverlayBackendCapabilities,
    windows: std::collections::HashMap<OverlayWindowId, MacWindowState>,
}

/// Configure the pet's native macOS surface in one place.
///
/// Rendering remains owned by the existing GL surface for now; this helper
/// makes the window policy platform-owned before AppKit hit testing is added.
#[allow(dead_code)]
pub(crate) fn configure_pet_window(window: &Window, prefs: &deskhud_ui::UiPreferences, size: f64) {
    window.set_title("DeskHud 宠物");
    window.set_decorations(false);
    window.set_resizable(false);
    window.set_window_level(if prefs.shell.topmost {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    });
    let _ = window.request_inner_size(LogicalSize::new(size, size));
    let ppp = window.scale_factor().max(0.01);
    let pos = prefs.pet.pos().unwrap_or([size as f32, size as f32]);
    window.set_outer_position(LogicalPosition::new(
        pos[0] as f64 - size / (2.0 * ppp),
        pos[1] as f64 - size / (2.0 * ppp),
    ));
    window.set_visible(true);
    window.request_redraw();
}

/// Create the native AppKit pet window and attach the native view.
///
/// This is kept separate from the winit control window so the settings surface
/// can continue using egui while the pet migrates to AppKit.
#[allow(dead_code)]
pub(crate) fn create_native_pet_window(
    mtm: MainThreadMarker,
    size: f64,
    topmost: bool,
) -> (Retained<NSWindow>, Retained<NativePetView>) {
    let frame = NSRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: size,
            height: size,
        },
    };
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setOpaque(false);
    window.setBackgroundColor(Some(&NSColor::clearColor()));
    window.setHasShadow(false);
    // Do not let AppKit classify pet dragging as window tiling/Stage Manager
    // placement. Dragging is handled by the native view and host contract.
    window.setMovableByWindowBackground(false);
    window.setMovable(false);
    window.setLevel(if topmost { 3 } else { 0 });
    let view = NativePetView::new(mtm, frame);
    view.setWantsLayer(true);
    window.setContentView(Some(&view));
    unsafe { window.setReleasedWhenClosed(false) };
    window.orderFrontRegardless();
    (window, view)
}

#[allow(dead_code)]
pub(crate) fn request_native_pet_redraw(view: &NativePetView) {
    view.setNeedsDisplay(true);
}

#[allow(dead_code)]
pub(crate) fn position_native_pet_window(
    window: &NSWindow,
    prefs: &deskhud_ui::UiPreferences,
    size: f64,
) {
    let pos = prefs.pet.pos().unwrap_or([size as f32, size as f32]);
    window.setFrameOrigin(objc2_foundation::NSPoint {
        x: pos[0] as f64 - size / 2.0,
        y: pos[1] as f64 - size / 2.0,
    });
}

pub(crate) fn set_native_pet_topmost(window: &NSWindow, enabled: bool) {
    window.setLevel(if enabled { 3 } else { 0 });
    window.orderFrontRegardless();
}

#[allow(dead_code)]
pub(crate) fn resize_native_pet_window(window: &NSWindow, width: f64, height: f64) {
    let frame = window.frame();
    window.setContentSize(CGSize { width, height });
    window.setFrameOrigin(CGPoint {
        x: frame.origin.x + (frame.size.width - width) / 2.0,
        y: frame.origin.y + (frame.size.height - height) / 2.0,
    });
}

fn snap_native_pet_window(window: &NSWindow) {
    let frame = window.frame();
    let visible = window
        .screen()
        .map(|screen| screen.visibleFrame())
        .unwrap_or_else(|| {
            NSScreen::mainScreen(MainThreadMarker::new().expect("main thread"))
                .map(|screen| screen.visibleFrame())
                .unwrap_or(frame)
        });
    let tolerance = 16.0;
    let mut x = frame.origin.x;
    let mut y = frame.origin.y;
    if (x - visible.origin.x).abs() <= tolerance {
        x = visible.origin.x;
    }
    if (y - visible.origin.y).abs() <= tolerance {
        y = visible.origin.y;
    }
    if (x + frame.size.width - (visible.origin.x + visible.size.width)).abs() <= tolerance {
        x = visible.origin.x + visible.size.width - frame.size.width;
    }
    if (y + frame.size.height - (visible.origin.y + visible.size.height)).abs() <= tolerance {
        y = visible.origin.y + visible.size.height - frame.size.height;
    }
    x = x.clamp(
        visible.origin.x,
        visible.origin.x + visible.size.width - frame.size.width,
    );
    y = y.clamp(
        visible.origin.y,
        visible.origin.y + visible.size.height - frame.size.height,
    );
    window.setFrameOrigin(CGPoint { x, y });
}

#[derive(Debug, Clone)]
struct MacWindowState {
    visible: bool,
    level: OverlayWindowLevel,
    scene: Option<OverlayScene>,
}

impl Default for MacosOverlayBackend {
    fn default() -> Self {
        Self {
            next_id: 1,
            capabilities: OverlayBackendCapabilities {
                desktop_transparency: true,
                per_region_passthrough: true,
                selected_display: true,
                virtual_desktop: false,
            },
            windows: std::collections::HashMap::new(),
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
        self.windows.insert(
            id,
            MacWindowState {
                visible: false,
                level: OverlayWindowLevel::Normal,
                scene: None,
            },
        );
        Ok(id)
    }

    fn update_scene(&mut self, id: OverlayWindowId, scene: OverlayScene) -> Result<()> {
        if let Some(window) = self.windows.get_mut(&id) {
            window.scene = Some(scene);
        }
        Ok(())
    }

    fn set_visible(&mut self, id: OverlayWindowId, visible: bool) -> Result<()> {
        if let Some(window) = self.windows.get_mut(&id) {
            window.visible = visible;
        }
        Ok(())
    }

    fn set_level(&mut self, id: OverlayWindowId, level: OverlayWindowLevel) -> Result<()> {
        if let Some(window) = self.windows.get_mut(&id) {
            window.level = level;
        }
        Ok(())
    }

    fn destroy_window(&mut self, id: OverlayWindowId) -> Result<()> {
        self.windows.remove(&id);
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
                CGEventType::LeftMouseUp,
                CGEventType::RightMouseUp,
                CGEventType::OtherMouseUp,
                CGEventType::ScrollWheel,
                CGEventType::KeyDown,
                CGEventType::KeyUp,
            ],
            move |_proxy, kind, event| {
                if matches!(kind, CGEventType::KeyDown | CGEventType::KeyUp) {
                    let code =
                        event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
                    if let Some(key) = mac_key_from_keycode(code) {
                        let _ = proxy.send_event(crate::native_host::UserEvent::GlobalKey {
                            key,
                            pressed: matches!(kind, CGEventType::KeyDown),
                        });
                    }
                    return CallbackResult::Keep;
                }
                let button = match kind {
                    CGEventType::LeftMouseDown | CGEventType::LeftMouseUp => {
                        deskhud_engine::PetMouseButton::Primary
                    }
                    CGEventType::RightMouseDown | CGEventType::RightMouseUp => {
                        deskhud_engine::PetMouseButton::Secondary
                    }
                    CGEventType::OtherMouseDown | CGEventType::OtherMouseUp => {
                        deskhud_engine::PetMouseButton::Middle
                    }
                    _ => return CallbackResult::Keep,
                };
                let pressed = matches!(
                    kind,
                    CGEventType::LeftMouseDown
                        | CGEventType::RightMouseDown
                        | CGEventType::OtherMouseDown
                );
                let _ = proxy.send_event(crate::native_host::UserEvent::GlobalMouse {
                    button,
                    pressed,
                });
                CallbackResult::Keep
            },
        );
        let Ok(tap) = tap else {
            tracing::warn!(
                "macOS global mouse tap unavailable; grant Accessibility/Input Monitoring permission"
            );
            return;
        };
        let run_loop = core_foundation::runloop::CFRunLoop::get_current();
        let Ok(source) = tap.mach_port().create_runloop_source(0) else {
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
    let scale_x = full.size.width / screen.frame().size.width.max(1.0);
    let scale_y = full.size.height / screen.frame().size.height.max(1.0);
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

/// Translate a macOS virtual keycode to the neutral pet key contract.
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
