//! Windows 原生桌面覆盖层探针。
//!
//! 它由 `DESKHUD_OVERLAY_PROBE=1` 显式启动，绝不接管正常的 eframe 运行路径。
//! 目的仅是验证 native layered window 能否稳定提供透明、局部命中和空白穿透。

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use deskhud_engine::{
    DockState, DragState, EngineRegistry, MouseState, OverlayCircle, OverlayDisplayTarget,
    OverlayPoint, OverlayScene, OverlayVisual, PetConfigBag, PetPaint, PetPaintCtx,
};
use deskhud_ui::UiPreferences;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetMonitorInfoW,
    MonitorFromPoint, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, MONITORINFO,
    MONITOR_DEFAULTTOPRIMARY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, PostQuitMessage, RegisterClassW, SetTimer,
    SetWindowLongPtrW, SetWindowPos, TranslateMessage, UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW,
    GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, MSG, SM_CXSCREEN, SM_CYSCREEN, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, ULW_ALPHA, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_LBUTTONDOWN, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

const TIMER_ID: usize = 1;
const TIMER_INTERVAL_MS: u32 = 16;
const PET_RADIUS: i32 = 64;
/// 留给呼吸动画和后续气泡的绘制边距；位图始终只覆盖宠物附近。
const PET_WINDOW_PADDING: i32 = 16;
const PET_WINDOW_RADIUS: i32 = PET_RADIUS + PET_WINDOW_PADDING;
const PET_WINDOW_SIZE: i32 = PET_WINDOW_RADIUS * 2;
const CLASS_NAME: &[u16] = &[
    b'D' as u16,
    b'e' as u16,
    b's' as u16,
    b'k' as u16,
    b'H' as u16,
    b'u' as u16,
    b'd' as u16,
    b'O' as u16,
    b'v' as u16,
    b'e' as u16,
    b'r' as u16,
    b'l' as u16,
    b'a' as u16,
    b'y' as u16,
    b'P' as u16,
    b'r' as u16,
    b'o' as u16,
    b'b' as u16,
    b'e' as u16,
    0,
];

static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);
static OVERLAY_LEFT: AtomicI32 = AtomicI32::new(0);
static OVERLAY_TOP: AtomicI32 = AtomicI32::new(0);
static OVERLAY_WIDTH: AtomicI32 = AtomicI32::new(0);
static OVERLAY_HEIGHT: AtomicI32 = AtomicI32::new(0);
static PET_X: AtomicI32 = AtomicI32::new(0);
static PET_Y: AtomicI32 = AtomicI32::new(0);
static DRAG_OFFSET_X: AtomicI32 = AtomicI32::new(0);
static DRAG_OFFSET_Y: AtomicI32 = AtomicI32::new(0);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static CLICK_THROUGH: AtomicBool = AtomicBool::new(true);
static PROBE_TOPMOST: AtomicBool = AtomicBool::new(true);
static PET_HIT_RADIUS: AtomicI32 = AtomicI32::new(PET_RADIUS);
static PET_RUNTIME: OnceLock<Mutex<ProbePetRuntime>> = OnceLock::new();

thread_local! {
    /// GDI 句柄只能在创建它的窗口线程中使用，因此不使用跨线程静态锁。
    static SURFACE: RefCell<Option<OverlaySurface>> = const { RefCell::new(None) };
}

/// 探针私有的宠物宿主；它与正式 eframe 壳隔离，但复用同一引擎契约与 prefs。
struct ProbePetRuntime {
    host: EngineRegistry,
    prefs: UiPreferences,
    started: Instant,
    last_tick: Instant,
    pupil_smooth: [f32; 2],
}

/// 可复用的 ARGB layered 位图；它仅覆盖宠物，而非整块显示器工作区。
struct OverlaySurface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    width: i32,
    height: i32,
    last_scene: Option<OverlayScene>,
}

impl OverlaySurface {
    unsafe fn create(width: i32, height: i32) -> Option<Self> {
        unsafe {
            let dc = CreateCompatibleDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            let mut bits = std::ptr::null_mut();
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let bitmap = CreateDIBSection(
                dc,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                let _ = DeleteDC(dc);
                return None;
            }
            let previous = SelectObject(dc, bitmap as HGDIOBJ);
            let pixels =
                std::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);
            pixels.fill(0);
            Some(Self {
                dc,
                bitmap,
                previous,
                bits: bits as *mut u8,
                width,
                height,
                last_scene: None,
            })
        }
    }
}

impl Drop for OverlaySurface {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap as HGDIOBJ);
            let _ = DeleteDC(self.dc);
        }
    }
}

/// 运行探针。按 Escape 退出；右键会改变宠物颜色，用来确认命中没有被穿透。
pub fn run() -> eframe::Result {
    unsafe {
        initialize_pet_runtime();
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME.as_ptr(),
            ..std::mem::zeroed()
        };
        let _ = RegisterClassW(&class);

        let work_area = primary_work_area();
        apply_work_area(work_area);
        PROBE_TOPMOST.store(read_topmost_setting(), Ordering::Relaxed);
        PET_X.store(work_area.left + work_area.width / 2, Ordering::Relaxed);
        PET_Y.store(work_area.top + work_area.height / 2, Ordering::Relaxed);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW,
            CLASS_NAME.as_ptr(),
            CLASS_NAME.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            PET_X.load(Ordering::Relaxed) - PET_WINDOW_RADIUS,
            PET_Y.load(Ordering::Relaxed) - PET_WINDOW_RADIUS,
            PET_WINDOW_SIZE,
            PET_WINDOW_SIZE,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            eprintln!("DeskHud overlay probe: CreateWindowExW failed");
            return Ok(());
        }
        OVERLAY_HWND.store(hwnd as isize, Ordering::Relaxed);
        apply_window_bounds(hwnd);
        render(hwnd, &probe_scene());
        let _ = SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL_MS, None);

        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            let _ = TranslateMessage(&message);
            let _ = DispatchMessageW(&message);
        }
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_TIMER => {
                if (GetAsyncKeyState(VK_ESCAPE as i32) as u16 & 0x8000) != 0 {
                    let _ = DestroyWindow(hwnd);
                    return 0;
                }
                tick(hwnd);
                0
            }
            WM_LBUTTONDOWN => {
                let x = signed_low_word(lparam);
                let y = signed_high_word(lparam);
                DRAG_OFFSET_X.store(x - PET_WINDOW_RADIUS, Ordering::Relaxed);
                DRAG_OFFSET_Y.store(y - PET_WINDOW_RADIUS, Ordering::Relaxed);
                DRAGGING.store(true, Ordering::Relaxed);
                0
            }
            WM_RBUTTONUP => {
                // 不创建第二套 UI：仅用颜色反转确认右键确实被该区域接收。
                render(hwnd, &probe_scene());
                0
            }
            // 宠物窗是小包围区，但活动边界仍取自主显示器工作区；缩放变化后
            // 必须重新读取其物理像素范围，才能让光标和宠物保持同一坐标系。
            WM_DPICHANGED | WM_DISPLAYCHANGE => {
                refresh_primary_bounds(hwnd);
                0
            }
            WM_DESTROY => {
                OVERLAY_HWND.store(0, Ordering::Relaxed);
                SURFACE.with(|surface| {
                    surface.borrow_mut().take();
                });
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message, _wparam, lparam),
        }
    }
}

unsafe fn tick(hwnd: HWND) {
    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor) == 0 {
            return;
        }
        let primary_down = (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0;
        if DRAGGING.load(Ordering::Relaxed) {
            if primary_down {
                let radius = current_pet_radius();
                PET_X.store(
                    (cursor.x - DRAG_OFFSET_X.load(Ordering::Relaxed)).clamp(
                        OVERLAY_LEFT.load(Ordering::Relaxed) + radius,
                        OVERLAY_LEFT.load(Ordering::Relaxed)
                            + OVERLAY_WIDTH.load(Ordering::Relaxed)
                            - radius,
                    ),
                    Ordering::Relaxed,
                );
                PET_Y.store(
                    (cursor.y - DRAG_OFFSET_Y.load(Ordering::Relaxed)).clamp(
                        OVERLAY_TOP.load(Ordering::Relaxed) + radius,
                        OVERLAY_TOP.load(Ordering::Relaxed)
                            + OVERLAY_HEIGHT.load(Ordering::Relaxed)
                            - radius,
                    ),
                    Ordering::Relaxed,
                );
            } else {
                DRAGGING.store(false, Ordering::Relaxed);
            }
        }

        let interactive = in_pet(
            cursor.x - PET_X.load(Ordering::Relaxed) + PET_WINDOW_RADIUS,
            cursor.y - PET_Y.load(Ordering::Relaxed) + PET_WINDOW_RADIUS,
        );
        set_click_through(hwnd, !interactive && !DRAGGING.load(Ordering::Relaxed));
        // 动画与指针方向来自 `tick` / `paint`，因此每个探针帧都必须提交新位图。
        render(hwnd, &probe_scene());
    }
}

/// 缩放或显示器配置变化后，重新对齐主显示器物理坐标与 layered 位图尺寸。
unsafe fn refresh_primary_bounds(hwnd: HWND) {
    unsafe {
        let work_area = primary_work_area();
        apply_work_area(work_area);
        clamp_pet_to_bounds(work_area.width, work_area.height);
        DRAGGING.store(false, Ordering::Relaxed);
        apply_window_bounds(hwnd);
        render(hwnd, &probe_scene());
    }
}

#[derive(Clone, Copy)]
struct WorkArea {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

/// 读取主显示器工作区，而不是完整显示器矩形，避免透明置顶窗口覆盖任务栏。
fn primary_work_area() -> WorkArea {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
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
    OVERLAY_LEFT.store(work_area.left, Ordering::Relaxed);
    OVERLAY_TOP.store(work_area.top, Ordering::Relaxed);
    OVERLAY_WIDTH.store(work_area.width, Ordering::Relaxed);
    OVERLAY_HEIGHT.store(work_area.height, Ordering::Relaxed);
}

fn clamp_pet_to_bounds(width: i32, height: i32) {
    let radius = current_pet_radius();
    PET_X.store(
        PET_X.load(Ordering::Relaxed).clamp(
            OVERLAY_LEFT.load(Ordering::Relaxed) + radius,
            OVERLAY_LEFT.load(Ordering::Relaxed) + width - radius,
        ),
        Ordering::Relaxed,
    );
    PET_Y.store(
        PET_Y.load(Ordering::Relaxed).clamp(
            OVERLAY_TOP.load(Ordering::Relaxed) + radius,
            OVERLAY_TOP.load(Ordering::Relaxed) + height - radius,
        ),
        Ordering::Relaxed,
    );
}

fn current_pet_radius() -> i32 {
    PET_HIT_RADIUS
        .load(Ordering::Relaxed)
        .max(1)
        .min(PET_WINDOW_RADIUS)
}

/// 探针用环境变量模拟正式运行时由 prefs 控制的置顶状态。
///
/// 未设置或非关闭值时置顶；`0`、`false`、`off` 表示不置顶。
fn read_topmost_setting() -> bool {
    !matches!(
        std::env::var("DESKHUD_OVERLAY_PROBE_TOPMOST")
            .ok()
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("0" | "false" | "off")
    )
}

unsafe fn apply_window_bounds(hwnd: HWND) {
    unsafe {
        let level = if PROBE_TOPMOST.load(Ordering::Relaxed) {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        let _ = SetWindowPos(
            hwnd,
            level,
            PET_X.load(Ordering::Relaxed) - PET_WINDOW_RADIUS,
            PET_Y.load(Ordering::Relaxed) - PET_WINDOW_RADIUS,
            PET_WINDOW_SIZE,
            PET_WINDOW_SIZE,
            SWP_NOACTIVATE,
        );
    }
}

fn in_pet(x: i32, y: i32) -> bool {
    probe_hit_shape().contains(OverlayPoint {
        x: x as f32,
        y: y as f32,
    })
}

unsafe fn set_click_through(hwnd: HWND, enabled: bool) {
    unsafe {
        if CLICK_THROUGH.swap(enabled, Ordering::Relaxed) == enabled {
            return;
        }
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let desired = if enabled {
            style | WS_EX_TRANSPARENT as isize
        } else {
            style & !(WS_EX_TRANSPARENT as isize)
        };
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

unsafe fn render(hwnd: HWND, scene: &OverlayScene) {
    unsafe {
        let width = PET_WINDOW_SIZE;
        let height = PET_WINDOW_SIZE;
        SURFACE.with(|slot| {
            let mut slot = slot.borrow_mut();
            let recreate = slot
                .as_ref()
                .is_none_or(|surface| surface.width != width || surface.height != height);
            if recreate {
                *slot = OverlaySurface::create(width, height);
            }
            let Some(surface) = slot.as_mut() else {
                return;
            };
            let pixels = std::slice::from_raw_parts_mut(
                surface.bits,
                (surface.width * surface.height * 4) as usize,
            );
            if let Some(previous) = surface.last_scene.as_ref() {
                clear_scene(pixels, surface.width, surface.height, previous);
            }
            draw_scene(pixels, surface.width, surface.height, scene);

            let destination = POINT {
                x: PET_X.load(Ordering::Relaxed) - PET_WINDOW_RADIUS,
                y: PET_Y.load(Ordering::Relaxed) - PET_WINDOW_RADIUS,
            };
            let size = SIZE {
                cx: surface.width,
                cy: surface.height,
            };
            let source = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = UpdateLayeredWindow(
                hwnd,
                std::ptr::null_mut(),
                &destination,
                &size,
                surface.dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            );
            surface.last_scene = Some(scene.clone());
        });
    }
}

fn initialize_pet_runtime() {
    let _ = PET_RUNTIME.get_or_init(|| {
        let prefs = match deskhud_ui::persist::load() {
            Ok(prefs) => prefs,
            Err(error) => {
                tracing::warn!(%error, "overlay probe prefs load failed; using defaults");
                UiPreferences::default()
            }
        };
        let boot = deskhud_runtime::bootstrap_registry();
        let mut host = boot.registry;
        if !host.set_active_pet(&prefs.pet.kind) {
            tracing::warn!(id = %prefs.pet.kind, "overlay probe active pet missing; using registry fallback");
        }
        let now = Instant::now();
        Mutex::new(ProbePetRuntime {
            host,
            prefs,
            started: now,
            last_tick: now,
            pupil_smooth: [0.0, 0.0],
        })
    });
}

fn probe_scene() -> OverlayScene {
    let Some(runtime) = PET_RUNTIME.get() else {
        return fallback_scene();
    };
    let Ok(mut runtime) = runtime.lock() else {
        return fallback_scene();
    };
    let now = Instant::now();
    let dt = now.duration_since(runtime.last_tick).as_secs_f32().max(0.0);
    runtime.last_tick = now;

    let pet = runtime.host.active_pet();
    let id = pet.info().id;
    let pairs: Vec<_> = pet
        .config_options()
        .iter()
        .map(|option| (option.key, option.default))
        .collect();
    let config_map = runtime.prefs.pet.short_map_for(id, &pairs);
    let config = PetConfigBag::new(&config_map);
    pet.apply_config(config);
    pet.tick(dt);

    let paint = pet.paint(PetPaintCtx {
        time_secs: now.duration_since(runtime.started).as_secs_f64(),
        pointer_dir: pointer_direction(),
        status_line: "",
        dock: DockState::FREE,
        drag: if DRAGGING.load(Ordering::Relaxed) {
            DragState::ACTIVE
        } else {
            DragState::IDLE
        },
        mouse: MouseState::IDLE,
        config,
    });
    runtime.pupil_smooth[0] += (paint.pupil_offset[0] - runtime.pupil_smooth[0]) * 0.28;
    runtime.pupil_smooth[1] += (paint.pupil_offset[1] - runtime.pupil_smooth[1]) * 0.28;
    let radius = (PET_RADIUS as f32 * paint.bounce.max(0.0)).round().max(1.0) as i32;
    PET_HIT_RADIUS.store(radius, Ordering::Relaxed);
    clamp_pet_to_bounds(
        OVERLAY_WIDTH.load(Ordering::Relaxed),
        OVERLAY_HEIGHT.load(Ordering::Relaxed),
    );
    crate::pet_scene::scene_from_pet_paint(
        OverlayDisplayTarget::Display("primary".into()),
        OverlayPoint {
            x: PET_WINDOW_RADIUS as f32,
            y: PET_WINDOW_RADIUS as f32,
        },
        PET_RADIUS as f32,
        &paint,
        runtime.pupil_smooth,
    )
}

fn fallback_scene() -> OverlayScene {
    let paint = PetPaint::default();
    crate::pet_scene::scene_from_pet_paint(
        OverlayDisplayTarget::Display("primary".into()),
        OverlayPoint {
            x: PET_WINDOW_RADIUS as f32,
            y: PET_WINDOW_RADIUS as f32,
        },
        PET_RADIUS as f32,
        &paint,
        paint.pupil_offset,
    )
}

fn pointer_direction() -> [f32; 2] {
    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor) == 0 {
            return [0.0, 0.0];
        }
        let dx = cursor.x - PET_X.load(Ordering::Relaxed);
        let dy = cursor.y - PET_Y.load(Ordering::Relaxed);
        let length = ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
        [dx as f32 / length, dy as f32 / length]
    }
}

fn probe_hit_shape() -> deskhud_engine::OverlayHitShape {
    crate::pet_scene::pet_hit_shape(
        OverlayPoint {
            x: PET_WINDOW_RADIUS as f32,
            y: PET_WINDOW_RADIUS as f32,
        },
        PET_HIT_RADIUS.load(Ordering::Relaxed) as f32,
    )
}

fn draw_scene(pixels: &mut [u8], width: i32, height: i32, scene: &OverlayScene) {
    for visual in &scene.visuals {
        match visual {
            OverlayVisual::Circle(circle) => draw_circle(pixels, width, height, circle),
        }
    }
}

fn clear_scene(pixels: &mut [u8], width: i32, height: i32, scene: &OverlayScene) {
    for visual in &scene.visuals {
        match visual {
            OverlayVisual::Circle(circle) => clear_circle(pixels, width, height, circle),
        }
    }
}

fn draw_circle(pixels: &mut [u8], width: i32, height: i32, circle: &OverlayCircle) {
    let center_x = circle.center.x.round() as i32;
    let center_y = circle.center.y.round() as i32;
    let radius = circle.radius.round() as i32;
    for y in (center_y - radius)..=(center_y + radius) {
        if !(0..height).contains(&y) {
            continue;
        }
        for x in (center_x - radius)..=(center_x + radius) {
            if !(0..width).contains(&x) {
                continue;
            }
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let offset = ((y * width + x) * 4) as usize;
            let alpha = circle.color.alpha as u16;
            pixels[offset] = (circle.color.blue as u16 * alpha / 255) as u8;
            pixels[offset + 1] = (circle.color.green as u16 * alpha / 255) as u8;
            pixels[offset + 2] = (circle.color.red as u16 * alpha / 255) as u8;
            pixels[offset + 3] = circle.color.alpha;
        }
    }
}

fn clear_circle(pixels: &mut [u8], width: i32, height: i32, circle: &OverlayCircle) {
    let center_x = circle.center.x.round() as i32;
    let center_y = circle.center.y.round() as i32;
    let radius = circle.radius.round() as i32;
    for y in (center_y - radius)..=(center_y + radius) {
        if !(0..height).contains(&y) {
            continue;
        }
        for x in (center_x - radius)..=(center_x + radius) {
            if !(0..width).contains(&x) {
                continue;
            }
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let offset = ((y * width + x) * 4) as usize;
            pixels[offset..offset + 4].fill(0);
        }
    }
}

fn signed_low_word(value: LPARAM) -> i32 {
    (value as i32 & 0xffff) as i16 as i32
}

fn signed_high_word(value: LPARAM) -> i32 {
    ((value as i32 >> 16) & 0xffff) as i16 as i32
}
