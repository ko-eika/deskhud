//! D3D11 + Direct2D + DirectComposition 的可视覆盖层探针。
//!
//! 它复用当前宠物包的最小绘制与拖动行为，用来验收 GPU 呈现链路；绝不接管
//! 默认运行路径。

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Instant;

use deskhud_engine::{
    DockState, DragState, EngineRegistry, MouseState, OverlayDisplayTarget, OverlayPoint,
    OverlayScene, OverlayVisual, PetConfigBag, PetEvent, PetMouseButton, PetPaintCtx,
};
use deskhud_ui::UiPreferences;
use windows::core::Interface;
use windows::Win32::Foundation::{HMODULE, HWND as WinHwnd, POINT};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED,
};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, WS_EX_NOREDIRECTIONBITMAP};
use windows_numerics::{Matrix3x2, Vector2};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, LoadCursorW, PostQuitMessage, RegisterClassW, SetTimer, SetWindowPos,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, IDC_ARROW, MSG, SM_CXSCREEN,
    SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WM_DESTROY, WM_DISPLAYCHANGE,
    WM_DPICHANGED, WM_LBUTTONDOWN, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};

const TIMER_ID: usize = 1;
// DwmFlush 会把实际提交节奏限制在桌面合成器刷新率；短计时器仅用于尽快开始下一帧。
const TIMER_INTERVAL_MS: u32 = 1;
const FRAME_STATS_WINDOW_SECS: f32 = 5.0;
const SIZE: i32 = 160;
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
}

static WINDOW_LEFT: AtomicI32 = AtomicI32::new(INITIAL_LEFT);
static WINDOW_TOP: AtomicI32 = AtomicI32::new(INITIAL_TOP);
static DRAG_OFFSET_X: AtomicI32 = AtomicI32::new(0);
static DRAG_OFFSET_Y: AtomicI32 = AtomicI32::new(0);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static WORK_LEFT: AtomicI32 = AtomicI32::new(0);
static WORK_TOP: AtomicI32 = AtomicI32::new(0);
static WORK_WIDTH: AtomicI32 = AtomicI32::new(1);
static WORK_HEIGHT: AtomicI32 = AtomicI32::new(1);

struct GpuOverlayRenderer {
    composition: IDCompositionDevice,
    _d2d_device: ID2D1Device,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    surface: IDCompositionSurface,
    pet: GpuPetRuntime,
}

/// GPU 探针私有的宠物宿主。它复用引擎契约和 prefs，但不接管 eframe 运行态。
struct GpuPetRuntime {
    host: EngineRegistry,
    prefs: UiPreferences,
    started: Instant,
    last_tick: Instant,
    pupil_smooth: [f32; 2],
    mouse: MouseState,
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
    unsafe fn create(hwnd: HWND) -> windows::core::Result<Self> {
        let mut d3d: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d),
                None,
                Some(&mut context),
            )?;
            let dxgi: IDXGIDevice = d3d
                .as_ref()
                .expect("D3D11CreateDevice succeeded without a device")
                .cast()?;
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d_device = factory.CreateDevice(&dxgi)?;
            // DCompositionCreateDevice2 仍要求请求基础 IDCompositionDevice IID；
            // 传入 IDCompositionDevice2 会返回 E_NOINTERFACE。
            let composition: IDCompositionDevice = DCompositionCreateDevice2(&d2d_device)?;
            let target = composition.CreateTargetForHwnd(WinHwnd(hwnd as *mut _), true)?;
            let visual = composition.CreateVisual()?;
            let surface = composition.CreateSurface(
                SIZE as u32,
                SIZE as u32,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_PREMULTIPLIED,
            )?;
            visual.SetContent(&surface)?;
            target.SetRoot(&visual)?;
            composition.Commit()?;
            let pet = initialize_pet_runtime();
            Ok(Self {
                composition,
                _d2d_device: d2d_device,
                _target: target,
                _visual: visual,
                surface,
                pet,
            })
        }
    }

    unsafe fn render(&mut self) -> windows::core::Result<()> {
        unsafe {
            let mut offset = POINT::default();
            let context: ID2D1DeviceContext = self.surface.BeginDraw(None, &mut offset)?;
            // 该上下文由 BeginDraw 直接返回且已经绑定为目标，禁止对它调用
            // ID2D1DeviceContext::BeginDraw/EndDraw；只需结束 Composition 更新。
            // 否则下一帧 BeginDraw 必然得到 SURFACE_BEING_RENDERED。
            let draw_result = (|| -> windows::core::Result<()> {
                let transparent = D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                };
                context.Clear(Some(&transparent));
                let transform = Matrix3x2 {
                    M11: 1.0,
                    M12: 0.0,
                    M21: 0.0,
                    M22: 1.0,
                    M31: offset.x as f32,
                    M32: offset.y as f32,
                };
                context.SetTransform(&transform);
                draw_scene(&context, &self.pet_scene())?;
                Ok(())
            })();
            drop(context);
            let end_result = self.surface.EndDraw();
            draw_result?;
            end_result?;
            self.composition.Commit()?;
            // 等待 DWM 接收本帧，避免 WM_TIMER 的相位漂移造成同一合成帧内反复提交。
            let flush_result = DwmFlush();
            if flush_result < 0 {
                tracing::debug!(
                    hresult = format_args!("0x{:08x}", flush_result as u32),
                    "GPU overlay DwmFlush failed"
                );
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
    fn pet_scene(&mut self) -> OverlayScene {
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
            dock: DockState::FREE,
            drag: if DRAGGING.load(Ordering::Relaxed) {
                DragState::ACTIVE
            } else {
                DragState::IDLE
            },
            mouse: self.pet.mouse,
            config,
        });
        self.pet.pupil_smooth[0] += (paint.pupil_offset[0] - self.pet.pupil_smooth[0]) * 0.28;
        self.pet.pupil_smooth[1] += (paint.pupil_offset[1] - self.pet.pupil_smooth[1]) * 0.28;
        let scene = crate::pet_scene::scene_from_pet_paint(
            OverlayDisplayTarget::Display("primary".into()),
            OverlayPoint {
                x: SIZE as f32 / 2.0,
                y: SIZE as f32 / 2.0,
            },
            64.0,
            &paint,
            self.pet.pupil_smooth,
        );
        scene
    }
}

unsafe fn draw_scene(
    context: &ID2D1DeviceContext,
    scene: &OverlayScene,
) -> windows::core::Result<()> {
    unsafe {
        for visual in &scene.visuals {
            let OverlayVisual::Circle(circle) = visual;
            let color = D2D1_COLOR_F {
                r: circle.color.red as f32 / 255.0,
                g: circle.color.green as f32 / 255.0,
                b: circle.color.blue as f32 / 255.0,
                a: circle.color.alpha as f32 / 255.0,
            };
            let brush = context.CreateSolidColorBrush(&color, None)?;
            let ellipse = D2D1_ELLIPSE {
                point: Vector2 {
                    X: circle.center.x,
                    Y: circle.center.y,
                },
                radiusX: circle.radius,
                radiusY: circle.radius,
            };
            context.FillEllipse(&ellipse, &brush);
        }
        Ok(())
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

unsafe fn refresh_primary_bounds(hwnd: HWND) {
    unsafe {
        apply_work_area(primary_work_area());
        clamp_window_to_work_area();
        if DRAGGING.swap(false, Ordering::Relaxed) {
            finish_pet_drag();
        }
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            WINDOW_LEFT.load(Ordering::Relaxed),
            WINDOW_TOP.load(Ordering::Relaxed),
            SIZE,
            SIZE,
            SWP_NOACTIVATE,
        );
        render(hwnd);
    }
}

pub fn run() -> eframe::Result {
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: CLASS_NAME.as_ptr(),
            // 原生窗口不经过 eframe 的光标管理；避免启动时沿用忙碌光标。
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            ..std::mem::zeroed()
        };
        let _ = RegisterClassW(&class);
        let work_area = primary_work_area();
        apply_work_area(work_area);
        WINDOW_LEFT.store(
            work_area.left + (work_area.width - SIZE) / 2,
            Ordering::Relaxed,
        );
        WINDOW_TOP.store(
            work_area.top + (work_area.height - SIZE) / 2,
            Ordering::Relaxed,
        );
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
        // 探针只在创建时设一次层级，避免每帧改变窗口层级；正式运行态仍由 prefs 决定。
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        match GpuOverlayRenderer::create(hwnd) {
            Ok(renderer) => RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer)),
            Err(error) => {
                eprintln!("DeskHud GPU overlay probe initialization failed: {error}");
                let _ = DestroyWindow(hwnd);
                return Ok(());
            }
        }
        render(hwnd);
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
                } else {
                    update_input_state(hwnd);
                    render(hwnd);
                }
                0
            }
            WM_LBUTTONDOWN => {
                DRAG_OFFSET_X.store(signed_low_word(lparam) - SIZE / 2, Ordering::Relaxed);
                DRAG_OFFSET_Y.store(signed_high_word(lparam) - SIZE / 2, Ordering::Relaxed);
                DRAGGING.store(true, Ordering::Relaxed);
                begin_pet_drag();
                0
            }
            WM_DPICHANGED | WM_DISPLAYCHANGE => {
                refresh_primary_bounds(hwnd);
                0
            }
            WM_DESTROY => {
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
        if DRAGGING.load(Ordering::Relaxed) {
            if left_button_down {
                WINDOW_LEFT.store(
                    cursor.x - SIZE / 2 - DRAG_OFFSET_X.load(Ordering::Relaxed),
                    Ordering::Relaxed,
                );
                WINDOW_TOP.store(
                    cursor.y - SIZE / 2 - DRAG_OFFSET_Y.load(Ordering::Relaxed),
                    Ordering::Relaxed,
                );
                clamp_window_to_work_area();
                let _ = SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    WINDOW_LEFT.load(Ordering::Relaxed),
                    WINDOW_TOP.load(Ordering::Relaxed),
                    SIZE,
                    SIZE,
                    SWP_NOACTIVATE,
                );
            } else {
                DRAGGING.store(false, Ordering::Relaxed);
                finish_pet_drag();
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

fn begin_pet_drag() {
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
        renderer
            .pet
            .host
            .active_pet()
            .on_event(PetEvent::DragStarted);
    });
}

fn finish_pet_drag() {
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
    match unsafe { GpuOverlayRenderer::create(_hwnd) } {
        Ok(renderer) => {
            RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer));
            tracing::info!("GPU overlay renderer recovered after device loss");
        }
        Err(recreate_error) => {
            eprintln!(
                "DeskHud GPU overlay recovery failed: {recreate_error}; close the probe and restart without DESKHUD_GPU_OVERLAY_PROBE"
            );
            let _ = unsafe { DestroyWindow(_hwnd) };
        }
    }
}

fn is_device_lost(error: &windows::core::Error) -> bool {
    matches!(
        error.code(),
        DXGI_ERROR_DEVICE_HUNG | DXGI_ERROR_DEVICE_REMOVED | DXGI_ERROR_DEVICE_RESET
    )
}
