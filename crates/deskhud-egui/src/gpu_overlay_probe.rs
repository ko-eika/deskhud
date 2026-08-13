//! Windows D3D11 + Direct2D + DirectComposition 宠物覆盖层后端。
//!
//! 它复用引擎契约与 prefs，承载正式 Windows 宠物运行态。

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU8, Ordering};
use std::time::Instant;

use deskhud_engine::{
    DockState, DragState, EngineRegistry, MouseState, OverlayDisplayTarget, OverlayPoint,
    OverlayScene, PetConfigBag, PetEvent, PetKey, PetModifiers, PetMouseButton, PetPaintCtx,
    PetTheme,
};
use deskhud_ui::{AnimationQuality, FpsLimit, PowerMode, UiPreferences};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, WS_EX_NOREDIRECTIONBITMAP};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GetMessageW, GetSystemMetrics, HC_ACTION, HTTRANSPARENT, HWND_NOTOPMOST,
    HWND_TOPMOST, IDC_ARROW, IDC_SIZENWSE, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LoadCursorW, MSG,
    MSLLHOOKSTRUCT, PostMessageW, PostQuitMessage, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN,
    SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SWP_SHOWWINDOW, SetCursor, SetTimer, SetWindowPos, SetWindowsHookExW, ShowWindow,
    TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCHITTEST, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SETCURSOR, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

use crate::overlay_control::{OverlayControlBus, OverlayControlCommand};
use crate::platform::{GpuCompositor, is_device_lost};

const TIMER_ID: usize = 1;
const WM_SYNC_TOPMOST: u32 = 0x8000 + 17;
const WM_ACTIVATE_EXISTING: u32 = 0x8000 + 1;
const WM_OPEN_LAYOUT_EDITOR: u32 = 0x8000 + 2;
const LAYOUT_CLASS: &[u16] = &[
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b'k' as u16,
    b'H' as u16,
    b'u' as u16,
    b'd' as u16,
    b'L' as u16,
    b'a' as u16,
    b'y' as u16,
    b'o' as u16,
    b'u' as u16,
    b't' as u16,
    b'\0' as u16,
];
// DwmFlush 会把实际提交节奏限制在桌面合成器刷新率；短计时器仅用于尽快开始下一帧。
const TIMER_INTERVAL_MS: u32 = 1;
const FRAME_STATS_WINDOW_SECS: f32 = 5.0;
const SIZE: i32 = 160;
const DIALOGUE_WIDTH: i32 = 190;
const DIALOGUE_HEIGHT: i32 = 50;
const DIALOGUE_GAP: i32 = 8;
const DRAG_THRESHOLD_PX: i32 = 6;
const SNAP_THRESHOLD_PX: i32 = 28;
const DOCK_EPS_PX: i32 = 4;
const INITIAL_LEFT: i32 = 320;
const INITIAL_TOP: i32 = 240;
const CLASS_NAME: &[u16] = &[
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b'k' as u16,
    b'H' as u16,
    b'u' as u16,
    b'd' as u16,
    b'G' as u16,
    b'p' as u16,
    b'u' as u16,
    b'P' as u16,
    b'r' as u16,
    b'o' as u16,
    b'b' as u16,
    b'e' as u16,
    0,
];

thread_local! {
    static RENDERER: RefCell<Option<GpuOverlayRenderer>> = const { RefCell::new(None) };
    static CONTROLS: RefCell<Option<OverlayControlBus>> = const { RefCell::new(None) };
    static LAYOUT_COMPOSITOR: RefCell<Option<GpuCompositor>> = const { RefCell::new(None) };
    static LAYOUT_SELECTED: RefCell<Option<usize>> = const { RefCell::new(None) };
    static LAYOUT_PREFS: RefCell<Option<UiPreferences>> = const { RefCell::new(None) };
    static LAYOUT_DRAG: RefCell<Option<LayoutDrag>> = const { RefCell::new(None) };
    static LAYOUT_RESIZING: RefCell<bool> = const { RefCell::new(false) };
}

struct LayoutDrag {
    index: usize,
    offset_x: f32,
    offset_y: f32,
    resize: Option<ResizeCorner>,
    start_layout: deskhud_ui::hud::HudSlotLayout,
    start_w: f32,
    start_h: f32,
}

#[derive(Clone, Copy)]
enum ResizeCorner {
    BottomRight,
}

static WINDOW_LEFT: AtomicI32 = AtomicI32::new(INITIAL_LEFT);
static WINDOW_TOP: AtomicI32 = AtomicI32::new(INITIAL_TOP);
static DRAG_OFFSET_X: AtomicI32 = AtomicI32::new(0);
static DRAG_OFFSET_Y: AtomicI32 = AtomicI32::new(0);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static PRIMARY_TRACKING: AtomicBool = AtomicBool::new(false);
static PRESS_CURSOR_X: AtomicI32 = AtomicI32::new(0);
static PRESS_CURSOR_Y: AtomicI32 = AtomicI32::new(0);
static WORK_LEFT: AtomicI32 = AtomicI32::new(0);
static WORK_TOP: AtomicI32 = AtomicI32::new(0);
static WORK_WIDTH: AtomicI32 = AtomicI32::new(1);
static WORK_HEIGHT: AtomicI32 = AtomicI32::new(1);
static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);
static DIALOGUE_HWND: AtomicIsize = AtomicIsize::new(0);
static RELOAD_PREFS: AtomicBool = AtomicBool::new(false);
static PET_THEME: AtomicU8 = AtomicU8::new(1);
static ALLOW_ESCAPE_EXIT: AtomicBool = AtomicBool::new(false);
static DESIRED_TOPMOST: AtomicBool = AtomicBool::new(true);

struct GpuOverlayRenderer {
    compositor: GpuCompositor,
    dialogue_compositor: GpuCompositor,
    dialogue_hwnd: HWND,
    dialogue_visible: bool,
    pet: GpuPetRuntime,
}

/// GPU 探针私有的宠物宿主。它复用引擎契约和 prefs，但不接管默认运行态。
struct GpuPetRuntime {
    host: EngineRegistry,
    prefs: UiPreferences,
    started: Instant,
    last_tick: Instant,
    pupil_smooth: [f32; 2],
    mouse: MouseState,
    dock: DockState,
    global_keys_down: HashSet<PetKey>,
    global_buttons_down: [bool; 3],
    frame_stats: FrameStats,
    diagnostics_enabled: bool,
}

struct FrameStats {
    started: Instant,
    last_present: Instant,
    frames: u32,
    slow_frames: u32,
}

impl GpuOverlayRenderer {
    unsafe fn create(hwnd: HWND, dialogue_hwnd: HWND) -> windows::core::Result<Self> {
        unsafe {
            Ok(Self {
                compositor: GpuCompositor::create(hwnd as isize, SIZE, SIZE)?,
                dialogue_compositor: GpuCompositor::create(
                    dialogue_hwnd as isize,
                    DIALOGUE_WIDTH,
                    DIALOGUE_HEIGHT,
                )?,
                dialogue_hwnd,
                dialogue_visible: false,
                pet: initialize_pet_runtime(),
            })
        }
    }

    unsafe fn render(&mut self) -> windows::core::Result<()> {
        unsafe {
            let (pet_scene, dialogue_scene) = self.pet_scenes();
            self.compositor.render(&pet_scene)?;
            if let Some(dialogue_scene) = dialogue_scene {
                self.dialogue_compositor.render(&dialogue_scene)?;
                position_dialogue_window(self.dialogue_hwnd);
                if !self.dialogue_visible {
                    let _ = ShowWindow(self.dialogue_hwnd, SW_SHOWNOACTIVATE);
                    self.dialogue_visible = true;
                    // 气泡首次显示会改变 DWM 的窗口排序；显示完成后立即
                    // 重新提交宠物与气泡的共同层级，避免冷启动只剩气泡置顶。
                    let pet_hwnd = OVERLAY_HWND.load(Ordering::Acquire) as HWND;
                    apply_topmost(
                        DESIRED_TOPMOST.load(Ordering::Acquire),
                        pet_hwnd,
                        self.dialogue_hwnd,
                    );
                }
            } else if self.dialogue_visible {
                let _ = ShowWindow(self.dialogue_hwnd, SW_HIDE);
                self.dialogue_visible = false;
            }
            self.pet.record_present();
            Ok(())
        }
    }
}

fn initialize_pet_runtime() -> GpuPetRuntime {
    let prefs = match deskhud_ui::persist::load() {
        Ok(prefs) => prefs,
        Err(error) => {
            tracing::warn!(%error, "GPU overlay probe prefs load failed; using defaults");
            UiPreferences::default()
        }
    };
    let boot = deskhud_runtime::bootstrap_registry();
    let mut host = boot.registry;
    if !host.set_active_pet(&prefs.pet.kind) {
        tracing::warn!(id = %prefs.pet.kind, "GPU overlay probe active pet missing; using registry fallback");
    }
    let now = Instant::now();
    GpuPetRuntime {
        host,
        prefs,
        started: now,
        last_tick: now,
        pupil_smooth: [0.0, 0.0],
        mouse: MouseState::IDLE,
        dock: DockState::FREE,
        global_keys_down: HashSet::new(),
        global_buttons_down: [false; 3],
        diagnostics_enabled: read_diagnostics_setting(),
        frame_stats: FrameStats {
            started: now,
            last_present: now,
            frames: 0,
            slow_frames: 0,
        },
    }
}

impl GpuPetRuntime {
    fn reload_prefs(&mut self) {
        let Ok(prefs) = deskhud_ui::persist::load() else {
            return;
        };
        if !self.host.set_active_pet(&prefs.pet.kind) {
            tracing::warn!(id = %prefs.pet.kind, "native overlay selected pet missing");
        }
        self.host.active_pet().on_event(PetEvent::DockChanged {
            from: DockState::FREE,
            to: self.dock,
        });
        self.prefs = prefs;
    }

    fn record_present(&mut self) {
        let now = Instant::now();
        let frame_secs = now
            .duration_since(self.frame_stats.last_present)
            .as_secs_f32();
        self.frame_stats.last_present = now;
        self.frame_stats.frames += 1;
        if frame_secs > 1.0 / 40.0 {
            self.frame_stats.slow_frames += 1;
        }
        let elapsed_secs = now.duration_since(self.frame_stats.started).as_secs_f32();
        if elapsed_secs >= FRAME_STATS_WINDOW_SECS {
            if self.diagnostics_enabled {
                tracing::info!(
                    fps = self.frame_stats.frames as f32 / elapsed_secs,
                    slow_frames = self.frame_stats.slow_frames,
                    "GPU overlay frame pacing"
                );
            }
            self.frame_stats = FrameStats {
                started: now,
                last_present: now,
                frames: 0,
                slow_frames: 0,
            };
        }
    }

    fn handle_global_key(&mut self, key: PetKey, pressed: bool) {
        if !pressed && modifier_still_down(key) {
            return;
        }
        let changed = if pressed {
            self.global_keys_down.insert(key)
        } else {
            self.global_keys_down.remove(&key)
        };
        if !changed {
            return;
        }
        let modifiers = current_modifiers();
        self.host.active_pet().on_event(if pressed {
            PetEvent::GlobalKeyPressed { key, modifiers }
        } else {
            PetEvent::GlobalKeyReleased { key, modifiers }
        });
    }

    fn handle_global_button(&mut self, button: PetMouseButton, pressed: bool) {
        let index = match button {
            PetMouseButton::Primary => 0,
            PetMouseButton::Secondary => 1,
            PetMouseButton::Middle => 2,
        };
        if self.global_buttons_down[index] == pressed {
            return;
        }
        self.global_buttons_down[index] = pressed;
        if button == PetMouseButton::Primary {
            self.mouse.global_primary_down = pressed;
        }
        let modifiers = current_modifiers();
        self.host.active_pet().on_event(if pressed {
            PetEvent::GlobalMousePressed { button, modifiers }
        } else {
            PetEvent::GlobalMouseReleased { button, modifiers }
        });
    }

    fn handle_global_wheel(&mut self, delta: i8) {
        if delta != 0 {
            self.host.active_pet().on_event(PetEvent::GlobalMouseWheel {
                delta,
                modifiers: current_modifiers(),
            });
        }
    }
}

/// Low-level hooks are preferred, but some security software rejects them. Sampling the
/// documented neutral subset keeps the feature functional without producing duplicates.
fn sample_global_keyboard() {
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        for vk in 0u32..=255 {
            if pet_key_from_vk(vk).is_none() {
                continue;
            }
            let pressed = key_down(vk as i32);
            if let Some(key) = pet_key_from_vk(vk) {
                renderer.pet.handle_global_key(key, pressed);
            }
        }
    });
}

/// Capture keys already held while DeskHud starts without treating them as new presses.
fn prime_global_keyboard_state() {
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        for vk in 0u32..=255 {
            if key_down(vk as i32)
                && let Some(key) = pet_key_from_vk(vk)
            {
                renderer.pet.global_keys_down.insert(key);
            }
        }
    });
}

fn key_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

fn modifier_still_down(key: PetKey) -> bool {
    match key {
        PetKey::Shift => key_down(0x10),
        PetKey::Ctrl => key_down(0x11),
        PetKey::Alt => key_down(0x12),
        PetKey::Super => key_down(0x5B) || key_down(0x5C),
        _ => false,
    }
}

fn drag_threshold_reached(dx: i32, dy: i32) -> bool {
    dx.saturating_mul(dx) + dy.saturating_mul(dy)
        >= DRAG_THRESHOLD_PX.saturating_mul(DRAG_THRESHOLD_PX)
}

fn current_modifiers() -> PetModifiers {
    PetModifiers {
        shift: key_down(0x10),
        ctrl: key_down(0x11),
        alt: key_down(0x12),
        meta: key_down(0x5B) || key_down(0x5C),
    }
}

fn pet_key_from_vk(vk: u32) -> Option<PetKey> {
    Some(match vk {
        0x1B => PetKey::Escape,
        0x09 => PetKey::Tab,
        0x0D => PetKey::Enter,
        0x20 => PetKey::Space,
        0x08 => PetKey::Backspace,
        0x2E => PetKey::Delete,
        0x2D => PetKey::Insert,
        0x0C => PetKey::Clear,
        0x26 => PetKey::ArrowUp,
        0x28 => PetKey::ArrowDown,
        0x25 => PetKey::ArrowLeft,
        0x27 => PetKey::ArrowRight,
        0x24 => PetKey::Home,
        0x23 => PetKey::End,
        0x21 => PetKey::PageUp,
        0x22 => PetKey::PageDown,
        0x10 | 0xA0 | 0xA1 => PetKey::Shift,
        0x11 | 0xA2 | 0xA3 => PetKey::Ctrl,
        0x12 | 0xA4 | 0xA5 => PetKey::Alt,
        0x5B | 0x5C => PetKey::Super,
        0x14 => PetKey::CapsLock,
        0x90 => PetKey::NumLock,
        0x60..=0x69 => PetKey::NumpadDigit((vk - 0x60) as u8),
        0x6A => PetKey::NumpadMultiply,
        0x6B => PetKey::NumpadAdd,
        0x6C => PetKey::NumpadSeparator,
        0x6D => PetKey::NumpadSubtract,
        0x6E => PetKey::NumpadDecimal,
        0x6F => PetKey::NumpadDivide,
        0x41..=0x5A => PetKey::Letter(char::from_u32(vk)?),
        0x30..=0x39 => PetKey::Digit(char::from_u32(vk)?),
        0x70..=0x7B => PetKey::Function((vk - 0x70 + 1) as u8),
        0xBA => PetKey::Punct(';'),
        0xBB => PetKey::Punct('='),
        0xBC => PetKey::Punct(','),
        0xBD => PetKey::Punct('-'),
        0xBE => PetKey::Punct('.'),
        0xBF => PetKey::Punct('/'),
        0xC0 => PetKey::Punct('`'),
        0xDB => PetKey::Punct('['),
        0xDC => PetKey::Punct('\\'),
        0xDD => PetKey::Punct(']'),
        0xDE => PetKey::Punct('\''),
        _ => return None,
    })
}

fn pet_key_from_hook(vk: u32, flags: u32) -> Option<PetKey> {
    if vk == 0x0D && flags & LLKHF_EXTENDED != 0 {
        Some(PetKey::NumpadEnter)
    } else {
        pet_key_from_vk(vk)
    }
}

/// 仅在显式诊断时输出周期帧率，正常探针保持安静。
fn read_diagnostics_setting() -> bool {
    matches!(
        std::env::var("DESKHUD_GPU_OVERLAY_DIAGNOSTICS")
            .ok()
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1" | "true" | "on")
    )
}

impl GpuOverlayRenderer {
    fn pet_scenes(&mut self) -> (OverlayScene, Option<OverlayScene>) {
        let now = Instant::now();
        let dt = now
            .duration_since(self.pet.last_tick)
            .as_secs_f32()
            .max(0.0);
        self.pet.last_tick = now;
        let pet = self.pet.host.active_pet();
        let id = pet.info().id;
        let pairs: Vec<_> = pet
            .config_options()
            .iter()
            .map(|option| (option.key, option.default))
            .collect();
        let config_map = self.pet.prefs.pet.short_map_for(id, &pairs);
        let config = PetConfigBag::new(&config_map);
        pet.apply_config(config);
        pet.tick(dt);
        let paint = pet.paint(PetPaintCtx {
            time_secs: now.duration_since(self.pet.started).as_secs_f64(),
            pointer_dir: pointer_direction(),
            status_line: "",
            dock: self.pet.dock,
            drag: if DRAGGING.load(Ordering::Relaxed) {
                DragState::ACTIVE
            } else {
                DragState::IDLE
            },
            mouse: self.pet.mouse,
            config,
            theme: pet_theme(),
        });
        let smoothing = match self.pet.prefs.graphics.animation_quality {
            AnimationQuality::Low => 0.12,
            AnimationQuality::Standard => 0.28,
            AnimationQuality::High => 0.42,
        };
        self.pet.pupil_smooth[0] += (paint.pupil_offset[0] - self.pet.pupil_smooth[0]) * smoothing;
        self.pet.pupil_smooth[1] += (paint.pupil_offset[1] - self.pet.pupil_smooth[1]) * smoothing;
        let target = OverlayDisplayTarget::Display("primary".into());
        let scene = crate::pet_scene::scene_from_pet_paint(
            target.clone(),
            OverlayPoint {
                x: SIZE as f32 / 2.0,
                y: SIZE as f32 / 2.0,
            },
            64.0,
            &paint,
            self.pet.pupil_smooth,
        );
        let dialogue = crate::pet_scene::dialogue_scene_from_pet_paint(
            target,
            [DIALOGUE_WIDTH as f32, DIALOGUE_HEIGHT as f32],
            &paint,
            pet_theme(),
            self.pet.prefs.graphics,
        );
        (scene, dialogue)
    }
}

fn pointer_direction() -> [f32; 2] {
    unsafe {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_err() {
            return [0.0, 0.0];
        }
        let dx = cursor.x - (WINDOW_LEFT.load(Ordering::Relaxed) + SIZE / 2);
        let dy = cursor.y - (WINDOW_TOP.load(Ordering::Relaxed) + SIZE / 2);
        let length = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
        [dx as f32 / length, dy as f32 / length]
    }
}

#[derive(Clone, Copy)]
struct WorkArea {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

/// 仅使用主显示器的工作区，避免置顶宠物拖到任务栏上。
fn primary_work_area() -> WorkArea {
    unsafe {
        let monitor = MonitorFromPoint(
            windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            MONITOR_DEFAULTTOPRIMARY,
        );
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        if !monitor.is_null() && GetMonitorInfoW(monitor, &mut info) != 0 {
            let work = info.rcWork;
            return WorkArea {
                left: work.left,
                top: work.top,
                width: (work.right - work.left).max(1),
                height: (work.bottom - work.top).max(1),
            };
        }
        WorkArea {
            left: 0,
            top: 0,
            width: GetSystemMetrics(SM_CXSCREEN).max(1),
            height: GetSystemMetrics(SM_CYSCREEN).max(1),
        }
    }
}

fn apply_work_area(work_area: WorkArea) {
    WORK_LEFT.store(work_area.left, Ordering::Relaxed);
    WORK_TOP.store(work_area.top, Ordering::Relaxed);
    WORK_WIDTH.store(work_area.width, Ordering::Relaxed);
    WORK_HEIGHT.store(work_area.height, Ordering::Relaxed);
}

unsafe fn position_dialogue_window(hwnd: HWND) {
    unsafe {
        let work_left = WORK_LEFT.load(Ordering::Relaxed);
        let work_top = WORK_TOP.load(Ordering::Relaxed);
        let work_right = work_left + WORK_WIDTH.load(Ordering::Relaxed);
        let work_bottom = work_top + WORK_HEIGHT.load(Ordering::Relaxed);
        let pet_left = WINDOW_LEFT.load(Ordering::Relaxed);
        let pet_top = WINDOW_TOP.load(Ordering::Relaxed);
        let mut left = pet_left + (SIZE - DIALOGUE_WIDTH) / 2;
        left = left.clamp(work_left, (work_right - DIALOGUE_WIDTH).max(work_left));
        let above = pet_top - DIALOGUE_HEIGHT - DIALOGUE_GAP;
        let below = pet_top + SIZE + DIALOGUE_GAP;
        let mut top = if above >= work_top { above } else { below };
        top = top.clamp(work_top, (work_bottom - DIALOGUE_HEIGHT).max(work_top));
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            left,
            top,
            DIALOGUE_WIDTH,
            DIALOGUE_HEIGHT,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
}

fn clamp_window_to_work_area() {
    let left = WORK_LEFT.load(Ordering::Relaxed);
    let top = WORK_TOP.load(Ordering::Relaxed);
    let right = left + WORK_WIDTH.load(Ordering::Relaxed).max(SIZE) - SIZE;
    let bottom = top + WORK_HEIGHT.load(Ordering::Relaxed).max(SIZE) - SIZE;
    WINDOW_LEFT.store(
        WINDOW_LEFT.load(Ordering::Relaxed).clamp(left, right),
        Ordering::Relaxed,
    );
    WINDOW_TOP.store(
        WINDOW_TOP.load(Ordering::Relaxed).clamp(top, bottom),
        Ordering::Relaxed,
    );
}

fn dock_for_position(left: i32, top: i32) -> DockState {
    let work_left = WORK_LEFT.load(Ordering::Relaxed);
    let work_top = WORK_TOP.load(Ordering::Relaxed);
    let work_right = work_left + WORK_WIDTH.load(Ordering::Relaxed);
    let work_bottom = work_top + WORK_HEIGHT.load(Ordering::Relaxed);
    DockState {
        left: (left - work_left).abs() <= DOCK_EPS_PX,
        right: (left + SIZE - work_right).abs() <= DOCK_EPS_PX,
        top: (top - work_top).abs() <= DOCK_EPS_PX,
        bottom: (top + SIZE - work_bottom).abs() <= DOCK_EPS_PX,
    }
}

fn update_pet_dock(to: DockState) {
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        let from = renderer.pet.dock;
        if from != to {
            renderer.pet.dock = to;
            renderer
                .pet
                .host
                .active_pet()
                .on_event(PetEvent::DockChanged { from, to });
        }
    });
}

/// Dragging may leave the work area; only release performs edge snapping/correction.
unsafe fn snap_window_after_drag(hwnd: HWND) -> DockState {
    unsafe {
        let work_left = WORK_LEFT.load(Ordering::Relaxed);
        let work_top = WORK_TOP.load(Ordering::Relaxed);
        let work_right = work_left + WORK_WIDTH.load(Ordering::Relaxed);
        let work_bottom = work_top + WORK_HEIGHT.load(Ordering::Relaxed);
        let left = WINDOW_LEFT.load(Ordering::Relaxed);
        let top = WINDOW_TOP.load(Ordering::Relaxed);
        let right = left + SIZE;
        let bottom = top + SIZE;

        let mut snapped_left = left;
        let mut snapped_top = top;
        let near_left = left <= work_left + SNAP_THRESHOLD_PX;
        let near_right = right >= work_right - SNAP_THRESHOLD_PX;
        if near_left && near_right {
            snapped_left = if work_left - left >= right - work_right {
                work_left
            } else {
                work_right - SIZE
            };
        } else if near_left {
            snapped_left = work_left;
        } else if near_right {
            snapped_left = work_right - SIZE;
        }
        let near_top = top <= work_top + SNAP_THRESHOLD_PX;
        let near_bottom = bottom >= work_bottom - SNAP_THRESHOLD_PX;
        if near_top && near_bottom {
            snapped_top = if work_top - top >= bottom - work_bottom {
                work_top
            } else {
                work_bottom - SIZE
            };
        } else if near_top {
            snapped_top = work_top;
        } else if near_bottom {
            snapped_top = work_bottom - SIZE;
        }

        WINDOW_LEFT.store(snapped_left, Ordering::Relaxed);
        WINDOW_TOP.store(snapped_top, Ordering::Relaxed);
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            snapped_left,
            snapped_top,
            SIZE,
            SIZE,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        dock_for_position(snapped_left, snapped_top)
    }
}

fn persist_pet_position(hwnd: HWND) {
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32;
    let scale = (dpi / 96.0).max(0.01);
    let command = OverlayControlCommand::PetMoved {
        x_points: WINDOW_LEFT.load(Ordering::Relaxed) as f32 / scale,
        y_points: WINDOW_TOP.load(Ordering::Relaxed) as f32 / scale,
    };
    CONTROLS.with(|slot| {
        if let Some(bus) = slot.borrow().as_ref() {
            bus.request(command);
        }
    });
}

unsafe fn refresh_primary_bounds(hwnd: HWND) {
    unsafe {
        apply_work_area(primary_work_area());
        if DRAGGING.swap(false, Ordering::Relaxed) {
            finish_pet_drag(hwnd);
        } else {
            clamp_window_to_work_area();
            update_pet_dock(dock_for_position(
                WINDOW_LEFT.load(Ordering::Relaxed),
                WINDOW_TOP.load(Ordering::Relaxed),
            ));
        }
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            WINDOW_LEFT.load(Ordering::Relaxed),
            WINDOW_TOP.load(Ordering::Relaxed),
            SIZE,
            SIZE,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        render(hwnd);
    }
}

pub fn spawn(
    controls: OverlayControlBus,
    topmost: bool,
    initial_pos: Option<[f32; 2]>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("deskhud-gpu-overlay".into())
        .spawn(move || {
            let _ = run_with_controls_and_level(controls, topmost, false, initial_pos);
        })
}

pub fn request_prefs_reload() {
    RELOAD_PREFS.store(true, Ordering::Release);
}

/// Open the native Windows DirectComposition layout editor.
#[allow(dead_code)]
pub fn open_layout_editor() {
    let hwnd = OVERLAY_HWND.load(Ordering::Acquire) as HWND;
    if !hwnd.is_null() {
        unsafe {
            let _ = PostMessageW(hwnd, WM_OPEN_LAYOUT_EDITOR, 0, 0);
        }
    }
}

fn open_layout_editor_window() {
    std::thread::Builder::new()
        .name("deskhud-layout-overlay".into())
        .spawn(|| unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(layout_window_proc),
                hInstance: instance,
                lpszClassName: LAYOUT_CLASS.as_ptr(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                ..std::mem::zeroed()
            };
            let _ = RegisterClassW(&class);
            let (monitor, work) = crate::platform::primary_monitor_geometry();
            WORK_WIDTH.store(work.2.max(1), Ordering::Relaxed);
            WORK_HEIGHT.store(work.3.max(1), Ordering::Relaxed);
            WORK_LEFT.store(work.0 - monitor.0, Ordering::Relaxed);
            WORK_TOP.store(work.1 - monitor.1, Ordering::Relaxed);
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP.0,
                LAYOUT_CLASS.as_ptr(),
                LAYOUT_CLASS.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                monitor.0,
                monitor.1,
                monitor.2,
                monitor.3,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return;
            }
            apply_topmost(true, hwnd, std::ptr::null_mut());
            if let Ok(mut compositor) = GpuCompositor::create(hwnd as isize, monitor.2, monitor.3) {
                let prefs = deskhud_ui::persist::load().unwrap_or_default();
                let registry = deskhud_runtime::bootstrap_registry().registry;
                let scene = layout_editor_scene(
                    monitor.2 as f32,
                    monitor.3 as f32,
                    &registry,
                    &prefs,
                    None,
                );
                let _ = compositor.render(&scene);
                LAYOUT_COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor));
                LAYOUT_PREFS.with(|slot| *slot.borrow_mut() = Some(prefs));
            }
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                let _ = TranslateMessage(&message);
                let _ = DispatchMessageW(&message);
            }
            LAYOUT_COMPOSITOR.with(|slot| slot.borrow_mut().take());
        })
        .ok();
}

fn layout_editor_scene(
    _width: f32,
    _height: f32,
    registry: &EngineRegistry,
    prefs: &deskhud_ui::UiPreferences,
    selected: Option<usize>,
) -> OverlayScene {
    let mut visuals = Vec::new();
    let color = deskhud_engine::OverlayColor {
        red: 50,
        green: 120,
        blue: 230,
        alpha: 230,
    };
    let safe_x = WORK_LEFT.load(Ordering::Relaxed) as f32;
    let safe_y = WORK_TOP.load(Ordering::Relaxed) as f32;
    let safe_w = WORK_WIDTH.load(Ordering::Relaxed) as f32;
    let safe_h = WORK_HEIGHT.load(Ordering::Relaxed) as f32;
    let inset = 2.0;
    for x in (safe_x as i32..(safe_x + safe_w - inset) as i32).step_by(12) {
        visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
            deskhud_engine::OverlayRoundedRect {
                id: format!("layout.top.{x}"),
                rect: deskhud_engine::OverlayRect {
                    origin: deskhud_engine::OverlayPoint {
                        x: x as f32,
                        y: safe_y + inset,
                    },
                    width: 6.0,
                    height: 2.0,
                },
                corner_radius: 1.0,
                color,
            },
        ));
        visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
            deskhud_engine::OverlayRoundedRect {
                id: format!("layout.bottom.{x}"),
                rect: deskhud_engine::OverlayRect {
                    origin: deskhud_engine::OverlayPoint {
                        x: x as f32,
                        y: safe_y + safe_h - inset - 2.0,
                    },
                    width: 6.0,
                    height: 2.0,
                },
                corner_radius: 1.0,
                color,
            },
        ));
    }
    for y in (safe_y as i32..(safe_y + safe_h - inset) as i32).step_by(12) {
        visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
            deskhud_engine::OverlayRoundedRect {
                id: format!("layout.left.{y}"),
                rect: deskhud_engine::OverlayRect {
                    origin: deskhud_engine::OverlayPoint {
                        x: safe_x + inset,
                        y: y as f32,
                    },
                    width: 2.0,
                    height: 6.0,
                },
                corner_radius: 1.0,
                color,
            },
        ));
        visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
            deskhud_engine::OverlayRoundedRect {
                id: format!("layout.right.{y}"),
                rect: deskhud_engine::OverlayRect {
                    origin: deskhud_engine::OverlayPoint {
                        x: safe_x + safe_w - inset - 2.0,
                        y: y as f32,
                    },
                    width: 2.0,
                    height: 6.0,
                },
                corner_radius: 1.0,
                color,
            },
        ));
    }
    let mut hud_index = 0usize;
    for (_item_index, (plugin_id, contribution)) in
        registry.all_hud_contributions().into_iter().enumerate()
    {
        if !prefs
            .hud
            .is_active(plugin_id, contribution.id, contribution.default_enabled)
        {
            continue;
        }
        let layout = prefs.hud.slot_layout(plugin_id, contribution.id, hud_index);
        let frame = registry.hud_frame(plugin_id, contribution.id, 0.0);
        let origin = deskhud_engine::OverlayPoint {
            x: safe_x + layout.x * safe_w,
            y: safe_y + layout.y * safe_h,
        };
        let scale = layout.scale;
        let mut panel = deskhud_engine::OverlayRect {
            origin,
            width: 180.0 * scale,
            height: 48.0 * scale,
        };
        if let Some(deskhud_engine::HudVisual::Panel { width, height, .. }) = frame.visuals.first()
        {
            panel.width = *width * scale;
            panel.height = *height * scale;
        }
        for (visual_index, visual) in frame.visuals.into_iter().enumerate() {
            match visual {
                deskhud_engine::HudVisual::Panel {
                    width: visual_width,
                    height: visual_height,
                    radius,
                    color: rgba,
                } => visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
                    deskhud_engine::OverlayRoundedRect {
                        id: format!("hud.{plugin_id}.{}.panel.{visual_index}", contribution.id),
                        rect: deskhud_engine::OverlayRect {
                            origin,
                            width: visual_width * scale,
                            height: visual_height * scale,
                        },
                        corner_radius: radius * scale,
                        color: deskhud_engine::OverlayColor {
                            red: rgba[0],
                            green: rgba[1],
                            blue: rgba[2],
                            alpha: rgba[3],
                        },
                    },
                )),
                deskhud_engine::HudVisual::Text {
                    text,
                    font_size,
                    color: rgba,
                } => visuals.push(deskhud_engine::OverlayVisual::Text(
                    deskhud_engine::OverlayText {
                        id: format!("hud.{plugin_id}.{}.text.{visual_index}", contribution.id),
                        rect: panel,
                        text,
                        font_size: font_size * scale,
                        color: deskhud_engine::OverlayColor {
                            red: rgba[0],
                            green: rgba[1],
                            blue: rgba[2],
                            alpha: rgba[3],
                        },
                    },
                )),
            }
        }
        if selected == Some(hud_index) {
            let border = deskhud_engine::OverlayColor {
                red: 0,
                green: 220,
                blue: 255,
                alpha: 255,
            };
            let x = origin.x - 3.0;
            let y = origin.y - 3.0;
            let w = panel.width + 6.0;
            let h = panel.height + 6.0;
            for (side, horizontal, length) in [
                ("top", true, w),
                ("bottom", true, w),
                ("left", false, h),
                ("right", false, h),
            ] {
                let count = (length / 12.0).ceil() as i32;
                for n in 0..count {
                    let (sx, sy) = match side {
                        "top" => (x + n as f32 * 12.0, y),
                        "bottom" => (x + n as f32 * 12.0, y + h - 3.0),
                        "left" => (x, y + n as f32 * 12.0),
                        _ => (x + w - 3.0, y + n as f32 * 12.0),
                    };
                    let remaining = (length - n as f32 * 12.0).max(0.0);
                    let rect = if horizontal {
                        deskhud_engine::OverlayRect {
                            origin: deskhud_engine::OverlayPoint { x: sx, y: sy },
                            width: 12.0_f32.min(remaining),
                            height: 4.0,
                        }
                    } else {
                        deskhud_engine::OverlayRect {
                            origin: deskhud_engine::OverlayPoint { x: sx, y: sy },
                            width: 4.0,
                            height: 12.0_f32.min(remaining),
                        }
                    };
                    visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
                        deskhud_engine::OverlayRoundedRect {
                            id: format!("hud.selected.{hud_index}.{side}.{n}"),
                            rect,
                            corner_radius: 1.0,
                            color: border,
                        },
                    ));
                }
            }
            // A right-corner triangle marks the only resize handle. Its right angle is
            // attached to the HUD's bottom-right corner and the diagonal faces inward.
            for (n, width) in [(0, 16.0), (1, 12.0), (2, 8.0), (3, 4.0)] {
                visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
                    deskhud_engine::OverlayRoundedRect {
                        id: format!("hud.selected.{hud_index}.resize-triangle.{n}"),
                        rect: deskhud_engine::OverlayRect {
                            origin: deskhud_engine::OverlayPoint {
                                x: x + w - width,
                                y: y + h - 4.0 - n as f32 * 4.0,
                            },
                            width,
                            height: 3.0,
                        },
                        corner_radius: 1.0,
                        color: deskhud_engine::OverlayColor {
                            red: 70,
                            green: 150,
                            blue: 255,
                            alpha: 220,
                        },
                    },
                ));
            }
            /* for (id, rect) in [
                (
                    "top",
                    deskhud_engine::OverlayRect {
                        origin: deskhud_engine::OverlayPoint { x, y },
                        width: w,
                        height: 3.0,
                    },
                ),
                (
                    "bottom",
                    deskhud_engine::OverlayRect {
                        origin: deskhud_engine::OverlayPoint { x, y: y + h - 3.0 },
                        width: w,
                        height: 3.0,
                    },
                ),
                (
                    "left",
                    deskhud_engine::OverlayRect {
                        origin: deskhud_engine::OverlayPoint { x, y },
                        width: 3.0,
                        height: h,
                    },
                ),
                (
                    "right",
                    deskhud_engine::OverlayRect {
                        origin: deskhud_engine::OverlayPoint { x: x + w - 3.0, y },
                        width: 3.0,
                        height: h,
                    },
                ),
            ] {
                visuals.push(deskhud_engine::OverlayVisual::RoundedRect(
                    deskhud_engine::OverlayRoundedRect {
                        id: format!("hud.selected.{hud_index}.{id}"),
                        rect,
                        corner_radius: 1.0,
                        color: border,
                    },
                ));
            } */
        }
        hud_index += 1;
    }
    OverlayScene {
        target: OverlayDisplayTarget::Display("primary".into()),
        visuals,
        hit_regions: Vec::new(),
    }
}

unsafe extern "system" fn layout_window_proc(
    hwnd: HWND,
    message: u32,
    _wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_LBUTTONDOWN => {
                let x = signed_low_word(_lparam) as f32;
                let y = signed_high_word(_lparam) as f32;
                let registry = deskhud_runtime::bootstrap_registry().registry;
                let prefs = LAYOUT_PREFS.with(|slot| slot.borrow().clone().unwrap_or_default());
                let mut index = 0usize;
                let mut selected = None;
                let mut hit = None;
                for (plugin_id, contribution) in registry.all_hud_contributions() {
                    if !prefs.hud.is_active(
                        plugin_id,
                        contribution.id,
                        contribution.default_enabled,
                    ) {
                        continue;
                    }
                    let layout = prefs.hud.slot_layout(plugin_id, contribution.id, index);
                    let left = WORK_LEFT.load(Ordering::Relaxed) as f32
                        + layout.x * WORK_WIDTH.load(Ordering::Relaxed) as f32;
                    let top = WORK_TOP.load(Ordering::Relaxed) as f32
                        + layout.y * WORK_HEIGHT.load(Ordering::Relaxed) as f32;
                    let frame = registry.hud_frame(plugin_id, contribution.id, 0.0);
                    let (base_w, base_h) = match frame.visuals.first() {
                        Some(deskhud_engine::HudVisual::Panel { width, height, .. }) => {
                            (*width, *height)
                        }
                        _ => (180.0, 48.0),
                    };
                    let item_w = base_w * layout.scale;
                    let item_h = base_h * layout.scale;
                    let hit_left = left - 6.0;
                    let hit_top = top - 6.0;
                    let hit_right = left + item_w + 6.0;
                    let hit_bottom = top + item_h + 6.0;
                    if x >= hit_left && x <= hit_right && y >= hit_top && y <= hit_bottom {
                        selected = Some(index);
                        hit = Some((
                            plugin_id.to_string(),
                            contribution.id.to_string(),
                            layout,
                            item_w,
                            item_h,
                        ));
                        break;
                    }
                    index += 1;
                }
                LAYOUT_SELECTED.with(|slot| *slot.borrow_mut() = selected);
                if let Some((plugin, contribution, layout, item_w, item_h)) = hit {
                    let corner = 18.0;
                    let left = WORK_LEFT.load(Ordering::Relaxed) as f32
                        + layout.x * WORK_WIDTH.load(Ordering::Relaxed) as f32;
                    let top = WORK_TOP.load(Ordering::Relaxed) as f32
                        + layout.y * WORK_HEIGHT.load(Ordering::Relaxed) as f32;
                    let frame_right = left + item_w + 3.0;
                    let frame_bottom = top + item_h + 3.0;
                    let resize =
                        if (x - frame_right).abs() < corner && (y - frame_bottom).abs() < corner {
                            Some(ResizeCorner::BottomRight)
                        } else {
                            None
                        };
                    LAYOUT_DRAG.with(|slot| {
                        *slot.borrow_mut() = selected.map(|index| LayoutDrag {
                            index,
                            offset_x: x - left,
                            offset_y: y - top,
                            resize,
                            start_layout: layout,
                            start_w: item_w,
                            start_h: item_h,
                        })
                    });
                    LAYOUT_RESIZING.with(|slot| *slot.borrow_mut() = resize.is_some());
                    let _ = (plugin, contribution);
                }
                if let Some(selected) = selected {
                    let scene = layout_editor_scene(
                        WORK_WIDTH.load(Ordering::Relaxed) as f32,
                        WORK_HEIGHT.load(Ordering::Relaxed) as f32,
                        &registry,
                        &prefs,
                        Some(selected),
                    );
                    LAYOUT_COMPOSITOR.with(|slot| {
                        if let Some(compositor) = slot.borrow_mut().as_mut() {
                            let _ = compositor.render(&scene);
                        }
                    });
                }
                0
            }
            WM_MOUSEMOVE => {
                let x = signed_low_word(_lparam) as f32;
                let y = signed_high_word(_lparam) as f32;
                let drag = LAYOUT_DRAG.with(|slot| {
                    slot.borrow().as_ref().map(|d| {
                        (
                            d.index,
                            d.offset_x,
                            d.offset_y,
                            d.resize,
                            d.start_layout.clone(),
                            d.start_w,
                            d.start_h,
                        )
                    })
                });
                if let Some((index, offset_x, offset_y, resize, start_layout, start_w, start_h)) =
                    drag
                {
                    let registry = deskhud_runtime::bootstrap_registry().registry;
                    let mut prefs =
                        LAYOUT_PREFS.with(|slot| slot.borrow().clone().unwrap_or_default());
                    if let Some((plugin, contribution)) = registry
                        .all_hud_contributions()
                        .into_iter()
                        .filter(|(p, c)| prefs.hud.is_active(p, c.id, c.default_enabled))
                        .nth(index)
                    {
                        let mut layout = start_layout.clone();
                        if resize.is_some() {
                            let left = WORK_LEFT.load(Ordering::Relaxed) as f32
                                + layout.x * WORK_WIDTH.load(Ordering::Relaxed) as f32;
                            let top = WORK_TOP.load(Ordering::Relaxed) as f32
                                + layout.y * WORK_HEIGHT.load(Ordering::Relaxed) as f32;
                            let factor = ((x - left) / start_w).max((y - top) / start_h);
                            layout.scale = (start_layout.scale * factor).clamp(0.5, 3.0);
                        } else {
                            layout.x = ((x - offset_x - WORK_LEFT.load(Ordering::Relaxed) as f32)
                                / WORK_WIDTH.load(Ordering::Relaxed) as f32)
                                .clamp(0.0, 1.0);
                            layout.y = ((y - offset_y - WORK_TOP.load(Ordering::Relaxed) as f32)
                                / WORK_HEIGHT.load(Ordering::Relaxed) as f32)
                                .clamp(0.0, 1.0);
                        }
                        prefs.hud.set_slot_layout(plugin, contribution.id, layout);
                        LAYOUT_PREFS.with(|slot| *slot.borrow_mut() = Some(prefs.clone()));
                        let scene = layout_editor_scene(
                            WORK_WIDTH.load(Ordering::Relaxed) as f32,
                            WORK_HEIGHT.load(Ordering::Relaxed) as f32,
                            &registry,
                            &prefs,
                            Some(index),
                        );
                        LAYOUT_COMPOSITOR.with(|slot| {
                            if let Some(c) = slot.borrow_mut().as_mut() {
                                let _ = c.render(&scene);
                            }
                        });
                    }
                }
                0
            }
            WM_LBUTTONUP => {
                LAYOUT_DRAG.with(|slot| *slot.borrow_mut() = None);
                LAYOUT_RESIZING.with(|slot| *slot.borrow_mut() = false);
                let prefs = LAYOUT_PREFS.with(|slot| slot.borrow().clone());
                if let Some(prefs) = prefs {
                    let _ = deskhud_ui::persist::save(&prefs);
                }
                0
            }
            WM_SETCURSOR => {
                if LAYOUT_RESIZING.with(|slot| *slot.borrow()) {
                    let cursor = LoadCursorW(std::ptr::null_mut(), IDC_SIZENWSE);
                    if !cursor.is_null() {
                        SetCursor(cursor);
                        return 1;
                    }
                }
                let selected = LAYOUT_SELECTED.with(|slot| *slot.borrow());
                if let Some(index) = selected {
                    let mut point = POINT::default();
                    if GetCursorPos(&mut point).is_ok() {
                        let (_, work) = crate::platform::primary_monitor_geometry();
                        let prefs =
                            LAYOUT_PREFS.with(|slot| slot.borrow().clone().unwrap_or_default());
                        let registry = deskhud_runtime::bootstrap_registry().registry;
                        if let Some((plugin, contribution)) = registry
                            .all_hud_contributions()
                            .into_iter()
                            .filter(|(p, c)| prefs.hud.is_active(p, c.id, c.default_enabled))
                            .nth(index)
                        {
                            let layout = prefs.hud.slot_layout(plugin, contribution.id, index);
                            let frame = registry.hud_frame(plugin, contribution.id, 0.0);
                            let (base_w, base_h) = match frame.visuals.first() {
                                Some(deskhud_engine::HudVisual::Panel {
                                    width, height, ..
                                }) => (*width, *height),
                                _ => (180.0, 48.0),
                            };
                            let left = work.0 as f32 + layout.x * work.2 as f32;
                            let top = work.1 as f32 + layout.y * work.3 as f32;
                            let right = left + base_w * layout.scale;
                            let bottom = top + base_h * layout.scale;
                            let tolerance = 16.0;
                            let near_right = (point.x as f32 - (right + 3.0)).abs() <= tolerance;
                            let near_bottom = (point.y as f32 - (bottom + 3.0)).abs() <= tolerance;
                            let cursor_id = if near_right && near_bottom {
                                Some(IDC_SIZENWSE)
                            } else {
                                None
                            };
                            if let Some(cursor_id) = cursor_id {
                                let cursor = LoadCursorW(std::ptr::null_mut(), cursor_id);
                                if !cursor.is_null() {
                                    SetCursor(cursor);
                                    return 1;
                                }
                            }
                        }
                    }
                }
                DefWindowProcW(hwnd, message, _wparam, _lparam)
            }
            WM_KEYDOWN if _wparam as u32 == VK_ESCAPE as u32 => {
                let _ = DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, _wparam, _lparam),
        }
    }
}

/// Publish the UI's resolved color scheme for platform-independent pet rendering.
pub fn set_pet_theme(theme: PetTheme) {
    let value = match theme {
        PetTheme::Light => 0,
        PetTheme::Dark => 1,
    };
    PET_THEME.store(value, Ordering::Release);
}

fn pet_theme() -> PetTheme {
    match PET_THEME.load(Ordering::Acquire) {
        0 => PetTheme::Light,
        _ => PetTheme::Dark,
    }
}

pub fn set_topmost(enabled: bool) {
    DESIRED_TOPMOST.store(enabled, Ordering::Release);
    let hwnd = OVERLAY_HWND.load(Ordering::Acquire) as HWND;
    if hwnd.is_null() {
        return;
    }
    unsafe {
        // 由覆盖层自己的窗口线程执行层级变更，避免跨线程 SetWindowPos
        // 在冷启动时与 DirectComposition/窗口创建时序竞争。
        let _ = PostMessageW(hwnd, WM_SYNC_TOPMOST, 0, 0);
    }
}

fn apply_topmost(enabled: bool, hwnd: HWND, dialogue_hwnd: HWND) {
    unsafe {
        let insert_after = if enabled {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        // 将两个窗口作为一次层级事务处理：先处理气泡，再处理宠物，
        // 让最终层级锚定在宠物窗口，避免气泡更新后宠物仍停留在旧层级。
        for window in [dialogue_hwnd, hwnd] {
            if !window.is_null() {
                let _ = SetWindowPos(
                    window,
                    insert_after,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }
}

fn run_with_controls_and_level(
    controls: OverlayControlBus,
    topmost: bool,
    allow_escape_exit: bool,
    initial_pos: Option<[f32; 2]>,
) -> anyhow::Result<()> {
    CONTROLS.with(|slot| *slot.borrow_mut() = Some(controls));
    DESIRED_TOPMOST.store(topmost, Ordering::Release);
    ALLOW_ESCAPE_EXIT.store(allow_escape_exit, Ordering::Relaxed);
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME.as_ptr(),
            // 原生窗口显式设置普通箭头，避免启动时沿用忙碌光标。
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            ..std::mem::zeroed()
        };
        let _ = RegisterClassW(&class);
        let work_area = primary_work_area();
        apply_work_area(work_area);
        let scale = (GetDpiForSystem().max(96) as f32 / 96.0).max(0.01);
        let initial_left = initial_pos
            .map(|pos| (pos[0] * scale).round() as i32)
            .unwrap_or(work_area.left + (work_area.width - SIZE) / 2);
        let initial_top = initial_pos
            .map(|pos| (pos[1] * scale).round() as i32)
            .unwrap_or(work_area.top + (work_area.height - SIZE) / 2);
        WINDOW_LEFT.store(initial_left, Ordering::Relaxed);
        WINDOW_TOP.store(initial_top, Ordering::Relaxed);
        clamp_window_to_work_area();
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP.0,
            CLASS_NAME.as_ptr(),
            CLASS_NAME.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            WINDOW_LEFT.load(Ordering::Relaxed),
            WINDOW_TOP.load(Ordering::Relaxed),
            SIZE,
            SIZE,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            eprintln!("DeskHud GPU overlay probe: CreateWindowExW failed");
            return Ok(());
        }
        OVERLAY_HWND.store(hwnd as isize, Ordering::Release);
        let dialogue_hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_NOREDIRECTIONBITMAP.0,
            CLASS_NAME.as_ptr(),
            CLASS_NAME.as_ptr(),
            WS_POPUP,
            0,
            0,
            DIALOGUE_WIDTH,
            DIALOGUE_HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if dialogue_hwnd.is_null() {
            eprintln!("DeskHud dialogue overlay: CreateWindowExW failed");
            let _ = DestroyWindow(hwnd);
            return Ok(());
        }
        DIALOGUE_HWND.store(dialogue_hwnd as isize, Ordering::Release);
        // 探针只在创建时设一次层级，避免每帧改变窗口层级；正式运行态仍由 prefs 决定。
        apply_topmost(topmost, hwnd, dialogue_hwnd);
        match GpuOverlayRenderer::create(hwnd, dialogue_hwnd) {
            Ok(renderer) => {
                RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer));
                prime_global_keyboard_state();
                update_pet_dock(dock_for_position(
                    WINDOW_LEFT.load(Ordering::Relaxed),
                    WINDOW_TOP.load(Ordering::Relaxed),
                ));
                apply_topmost(topmost, hwnd, dialogue_hwnd);
            }
            Err(error) => {
                eprintln!("DeskHud GPU overlay probe initialization failed: {error}");
                let _ = DestroyWindow(dialogue_hwnd);
                let _ = DestroyWindow(hwnd);
                return Ok(());
            }
        }
        render(hwnd);
        // 等窗口首次提交后再由其所属线程重放一次，覆盖冷启动时层级被 DWM
        // 或首次显示流程改写的情况。
        let _ = PostMessageW(hwnd, WM_SYNC_TOPMOST, 0, 0);
        let keyboard_hook =
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), instance, 0);
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), instance, 0);
        if keyboard_hook.is_null() || mouse_hook.is_null() {
            tracing::warn!("global keyboard/mouse hook initialization was incomplete");
        }
        let _ = SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL_MS, None);
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            let _ = TranslateMessage(&message);
            let _ = DispatchMessageW(&message);
        }
        if !keyboard_hook.is_null() {
            let _ = UnhookWindowsHookEx(keyboard_hook);
        }
        if !mouse_hook.is_null() {
            let _ = UnhookWindowsHookEx(mouse_hook);
        }
    }
    CONTROLS.with(|slot| slot.borrow_mut().take());
    Ok(())
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code == HC_ACTION as i32 && lparam != 0 {
            let message = wparam as u32;
            let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
            let released = matches!(message, WM_KEYUP | WM_SYSKEYUP);
            if pressed || released {
                let event = &*(lparam as *const KBDLLHOOKSTRUCT);
                RENDERER.with(|slot| {
                    if let Some(renderer) = slot.borrow_mut().as_mut() {
                        if let Some(key) = pet_key_from_hook(event.vkCode, event.flags) {
                            renderer.pet.handle_global_key(key, pressed);
                        }
                    }
                });
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code == HC_ACTION as i32 {
            let message = wparam as u32;
            RENDERER.with(|slot| {
                let mut slot = slot.borrow_mut();
                let Some(renderer) = slot.as_mut() else {
                    return;
                };
                match message {
                    WM_LBUTTONDOWN => renderer
                        .pet
                        .handle_global_button(PetMouseButton::Primary, true),
                    WM_LBUTTONUP => renderer
                        .pet
                        .handle_global_button(PetMouseButton::Primary, false),
                    WM_RBUTTONDOWN => renderer
                        .pet
                        .handle_global_button(PetMouseButton::Secondary, true),
                    WM_RBUTTONUP => renderer
                        .pet
                        .handle_global_button(PetMouseButton::Secondary, false),
                    WM_MBUTTONDOWN => renderer
                        .pet
                        .handle_global_button(PetMouseButton::Middle, true),
                    WM_MBUTTONUP => renderer
                        .pet
                        .handle_global_button(PetMouseButton::Middle, false),
                    WM_MOUSEWHEEL if lparam != 0 => {
                        let event = &*(lparam as *const MSLLHOOKSTRUCT);
                        let raw = (event.mouseData >> 16) as u16 as i16;
                        renderer.pet.handle_global_wheel(
                            (raw / 120).clamp(i8::MIN as i16, i8::MAX as i16) as i8,
                        );
                    }
                    _ => {}
                }
            });
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_OPEN_LAYOUT_EDITOR => {
                open_layout_editor_window();
                0
            }
            WM_ACTIVATE_EXISTING => {
                CONTROLS.with(|slot| {
                    if let Some(bus) = slot.borrow().as_ref() {
                        bus.request(OverlayControlCommand::ActivateExisting);
                    }
                });
                let dialogue = DIALOGUE_HWND.load(Ordering::Acquire) as HWND;
                apply_topmost(DESIRED_TOPMOST.load(Ordering::Acquire), hwnd, dialogue);
                0
            }
            WM_SYNC_TOPMOST => {
                let enabled = DESIRED_TOPMOST.load(Ordering::Acquire);
                let dialogue = DIALOGUE_HWND.load(Ordering::Acquire) as HWND;
                apply_topmost(enabled, hwnd, dialogue);
                0
            }
            WM_NCHITTEST if hwnd as isize == DIALOGUE_HWND.load(Ordering::Acquire) => {
                HTTRANSPARENT as LRESULT
            }
            WM_TIMER => {
                if RELOAD_PREFS.swap(false, Ordering::AcqRel) {
                    RENDERER.with(|slot| {
                        if let Some(renderer) = slot.borrow_mut().as_mut() {
                            renderer.pet.reload_prefs();
                        }
                    });
                }
                let shutdown = CONTROLS.with(|slot| {
                    slot.borrow()
                        .as_ref()
                        .is_some_and(OverlayControlBus::shutdown_requested)
                });
                let escape_exit = ALLOW_ESCAPE_EXIT.load(Ordering::Relaxed)
                    && (GetAsyncKeyState(VK_ESCAPE as i32) as u16 & 0x8000) != 0;
                if shutdown || escape_exit {
                    CONTROLS.with(|slot| {
                        if let Some(bus) = slot.borrow().as_ref() {
                            bus.request(OverlayControlCommand::Quit);
                        }
                    });
                    let _ = DestroyWindow(hwnd);
                } else {
                    sample_global_keyboard();
                    update_input_state(hwnd);
                    render(hwnd);
                }
                0
            }
            WM_LBUTTONDOWN => {
                let local_x = signed_low_word(lparam);
                let local_y = signed_high_word(lparam);
                DRAG_OFFSET_X.store(local_x - SIZE / 2, Ordering::Relaxed);
                DRAG_OFFSET_Y.store(local_y - SIZE / 2, Ordering::Relaxed);
                PRESS_CURSOR_X.store(
                    WINDOW_LEFT.load(Ordering::Relaxed) + local_x,
                    Ordering::Relaxed,
                );
                PRESS_CURSOR_Y.store(
                    WINDOW_TOP.load(Ordering::Relaxed) + local_y,
                    Ordering::Relaxed,
                );
                PRIMARY_TRACKING.store(true, Ordering::Relaxed);
                DRAGGING.store(false, Ordering::Relaxed);
                begin_pet_press();
                0
            }
            0x0205 => {
                CONTROLS.with(|slot| {
                    if let Some(bus) = slot.borrow().as_ref() {
                        bus.request(OverlayControlCommand::OpenMenu);
                    }
                });
                0
            }
            WM_DPICHANGED | WM_DISPLAYCHANGE => {
                refresh_primary_bounds(hwnd);
                0
            }
            WM_DESTROY => {
                if hwnd as isize == DIALOGUE_HWND.load(Ordering::Acquire) {
                    DIALOGUE_HWND.store(0, Ordering::Release);
                    return 0;
                }
                let dialogue = DIALOGUE_HWND.load(Ordering::Acquire) as HWND;
                if !dialogue.is_null() {
                    let _ = DestroyWindow(dialogue);
                }
                OVERLAY_HWND.store(0, Ordering::Release);
                RENDERER.with(|slot| slot.borrow_mut().take());
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, _wparam, lparam),
        }
    }
}

unsafe fn update_input_state(hwnd: HWND) {
    unsafe {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_err() {
            return;
        }
        let left_button_down = (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0;
        sync_pet_mouse(cursor, left_button_down);
        if PRIMARY_TRACKING.load(Ordering::Relaxed) {
            if left_button_down {
                if !DRAGGING.load(Ordering::Relaxed) {
                    let dx = cursor.x - PRESS_CURSOR_X.load(Ordering::Relaxed);
                    let dy = cursor.y - PRESS_CURSOR_Y.load(Ordering::Relaxed);
                    if drag_threshold_reached(dx, dy) {
                        DRAGGING.store(true, Ordering::Relaxed);
                        begin_pet_drag();
                    }
                }
                if DRAGGING.load(Ordering::Relaxed) {
                    WINDOW_LEFT.store(
                        cursor.x - SIZE / 2 - DRAG_OFFSET_X.load(Ordering::Relaxed),
                        Ordering::Relaxed,
                    );
                    WINDOW_TOP.store(
                        cursor.y - SIZE / 2 - DRAG_OFFSET_Y.load(Ordering::Relaxed),
                        Ordering::Relaxed,
                    );
                    let _ = SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        WINDOW_LEFT.load(Ordering::Relaxed),
                        WINDOW_TOP.load(Ordering::Relaxed),
                        SIZE,
                        SIZE,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    );
                }
            } else {
                PRIMARY_TRACKING.store(false, Ordering::Relaxed);
                if DRAGGING.swap(false, Ordering::Relaxed) {
                    finish_pet_drag(hwnd);
                } else {
                    finish_pet_click();
                }
            }
        }
    }
}

fn sync_pet_mouse(cursor: POINT, global_primary_down: bool) {
    let hovering = cursor.x >= WINDOW_LEFT.load(Ordering::Relaxed)
        && cursor.x < WINDOW_LEFT.load(Ordering::Relaxed) + SIZE
        && cursor.y >= WINDOW_TOP.load(Ordering::Relaxed)
        && cursor.y < WINDOW_TOP.load(Ordering::Relaxed) + SIZE;
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        if renderer.pet.mouse.hovering != hovering {
            renderer.pet.mouse.hovering = hovering;
            renderer
                .pet
                .host
                .active_pet()
                .on_event(PetEvent::MouseHover { inside: hovering });
        }
        renderer.pet.mouse.global_primary_down = global_primary_down;
    });
}

fn begin_pet_press() {
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        renderer.pet.mouse.primary_down = true;
        renderer
            .pet
            .host
            .active_pet()
            .on_event(PetEvent::MousePressed {
                button: PetMouseButton::Primary,
                modifiers: deskhud_engine::PetModifiers::NONE,
            });
    });
}

fn begin_pet_drag() {
    update_pet_dock(DockState::FREE);
    RENDERER.with(|slot| {
        if let Some(renderer) = slot.borrow_mut().as_mut() {
            renderer
                .pet
                .host
                .active_pet()
                .on_event(PetEvent::DragStarted);
        }
    });
}

fn finish_pet_click() {
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        renderer.pet.mouse.primary_down = false;
        let modifiers = current_modifiers();
        renderer
            .pet
            .host
            .active_pet()
            .on_event(PetEvent::MouseReleased {
                button: PetMouseButton::Primary,
                modifiers,
            });
        renderer
            .pet
            .host
            .active_pet()
            .on_event(PetEvent::MouseClicked {
                button: PetMouseButton::Primary,
                modifiers,
            });
    });
}

fn finish_pet_drag(hwnd: HWND) {
    RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(renderer) = slot.as_mut() else {
            return;
        };
        renderer.pet.mouse.primary_down = false;
        renderer
            .pet
            .host
            .active_pet()
            .on_event(PetEvent::MouseReleased {
                button: PetMouseButton::Primary,
                modifiers: deskhud_engine::PetModifiers::NONE,
            });
        renderer
            .pet
            .host
            .active_pet()
            .on_event(PetEvent::DragEnded {
                drag: DragState::IDLE,
            });
    });
    let dock = unsafe { snap_window_after_drag(hwnd) };
    update_pet_dock(dock);
    persist_pet_position(hwnd);
}

fn signed_low_word(value: LPARAM) -> i32 {
    (value as u32 & 0xffff) as u16 as i16 as i32
}

fn signed_high_word(value: LPARAM) -> i32 {
    ((value as u32 >> 16) & 0xffff) as u16 as i16 as i32
}

fn render(_hwnd: HWND) {
    let result = RENDERER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(renderer) = slot.as_ref() {
            let mut fps = match renderer.pet.prefs.graphics.fps_limit {
                FpsLimit::Auto => None,
                FpsLimit::Fps30 => Some(30.0),
                FpsLimit::Fps60 => Some(60.0),
                FpsLimit::Fps120 => Some(120.0),
            };
            if matches!(renderer.pet.prefs.graphics.power_mode, PowerMode::Saving) {
                fps = Some(fps.unwrap_or(60.0_f32).min(30.0_f32));
            }
            if let Some(fps) = fps {
                if renderer.pet.frame_stats.last_present.elapsed()
                    < std::time::Duration::from_secs_f32(1.0 / fps)
                {
                    return Ok(());
                }
            }
        }
        slot.as_mut()
            .map(|renderer| unsafe { renderer.render() })
            .unwrap_or(Ok(()))
    });
    let Err(error) = result else {
        return;
    };
    if !is_device_lost(&error) {
        eprintln!("DeskHud GPU overlay probe render failed: {error}");
        return;
    }

    tracing::warn!(%error, "GPU overlay device lost; recreating renderer");
    let dialogue_hwnd = DIALOGUE_HWND.load(Ordering::Acquire) as HWND;
    if dialogue_hwnd.is_null() {
        eprintln!("DeskHud GPU overlay recovery failed: dialogue window is unavailable");
        let _ = unsafe { DestroyWindow(_hwnd) };
        return;
    }
    match unsafe { GpuOverlayRenderer::create(_hwnd, dialogue_hwnd) } {
        Ok(renderer) => {
            RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer));
            tracing::info!("GPU overlay renderer recovered after device loss");
        }
        Err(recreate_error) => {
            eprintln!(
                "DeskHud GPU overlay recovery failed: {recreate_error}; restart DeskHud to retry"
            );
            let _ = unsafe { DestroyWindow(_hwnd) };
        }
    }
}

#[cfg(test)]
mod tests {
    use deskhud_engine::PetKey;

    use super::{drag_threshold_reached, pet_key_from_hook, pet_key_from_vk};

    #[test]
    fn maps_global_keyboard_subset_to_neutral_keys() {
        assert_eq!(pet_key_from_vk(0x43), Some(PetKey::Letter('C')));
        assert_eq!(pet_key_from_vk(0x31), Some(PetKey::Digit('1')));
        assert_eq!(pet_key_from_vk(0x70), Some(PetKey::Function(1)));
        assert_eq!(pet_key_from_vk(0xA2), Some(PetKey::Ctrl));
        assert_eq!(pet_key_from_vk(0x60), Some(PetKey::NumpadDigit(0)));
        assert_eq!(pet_key_from_vk(0x69), Some(PetKey::NumpadDigit(9)));
        assert_eq!(pet_key_from_vk(0x6B), Some(PetKey::NumpadAdd));
        assert_eq!(pet_key_from_vk(0x6F), Some(PetKey::NumpadDivide));
        assert_eq!(pet_key_from_hook(0x0D, 1), Some(PetKey::NumpadEnter));
        assert_eq!(pet_key_from_hook(0x0D, 0), Some(PetKey::Enter));
        assert_eq!(pet_key_from_vk(0xFF), None);
    }

    #[test]
    fn distinguishes_click_jitter_from_drag_motion() {
        assert!(!drag_threshold_reached(0, 0));
        assert!(!drag_threshold_reached(3, 4));
        assert!(drag_threshold_reached(6, 0));
        assert!(drag_threshold_reached(5, 4));
    }
}
