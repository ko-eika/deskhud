//! 非 Windows MVP 回退：无 DWM/全局钩子；几何与拖移靠 egui 视口信息。

use eframe::egui;

/// 无原生 chrome（透明/去标题栏由 ViewportBuilder 处理）。
pub fn ensure_pet_chrome(_hwnd: isize) {}

/// 无 Acrylic。
pub fn ensure_acrylic_popup(_hwnd: isize, _pet_hwnd: Option<isize>) {}

/// 无 Mica。
#[allow(dead_code)]
pub fn ensure_mica_window(_hwnd: isize, _pet_hwnd: Option<isize>) {}

/// 无前台 HWND 概念。
pub fn foreground_hwnd() -> Option<isize> {
    None
}

/// 无可靠前台检测时不自动关菜单（由焦点逻辑处理）。
pub fn foreground_is_outside(_pet: Option<isize>, _menu: Option<isize>) -> bool {
    false
}

/// 无客户区映射；调用方用 egui 指针。
pub fn cursor_client_px(_hwnd: isize) -> Option<(i32, i32)> {
    None
}

/// 无全局光标 API。
pub fn cursor_screen_px() -> Option<(i32, i32)> {
    None
}

/// 全局鼠标：降级为空（避免假边沿）。
pub fn global_mouse_buttons() -> (bool, bool, bool) {
    (false, false, false)
}

/// 全局修饰键：降级为空。
pub fn global_modifiers() -> (bool, bool, bool) {
    (false, false, false)
}

/// 全局按键：降级。
pub fn global_key_down(_vk: i32) -> bool {
    false
}

/// 无低级滚轮钩。
pub fn take_wheel_delta() -> i32 {
    0
}

/// 优先从 egui 视口读外接矩形左上角（物理像素近似：points × ppp）。
pub fn window_screen_pos_from_ctx(ctx: &egui::Context) -> Option<(i32, i32)> {
    let (rect, ppp) = ctx.input(|i| {
        let v = i.viewport();
        (v.outer_rect, i.pixels_per_point())
    });
    let rect = rect?;
    Some(((rect.min.x * ppp).round() as i32, (rect.min.y * ppp).round() as i32))
}

/// 无 HWND 时的空实现。
pub fn window_screen_pos(_hwnd: isize) -> Option<(i32, i32)> {
    None
}

pub fn window_screen_rect(_hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    None
}

/// 用 ViewportCommand 移动（逻辑点）。
pub fn move_viewport_points(ctx: &egui::Context, x_points: f32, y_points: f32) {
    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x_points, y_points)));
}

/// 兼容旧签名：无原生 move。
pub fn move_window_screen(_hwnd: isize, _x: i32, _y: i32) {}

/// 工作区：尽量用 egui 监视器尺寸，否则 1920×1080。
pub fn work_area_containing_px(x: i32, y: i32) -> (i32, i32, i32, i32) {
    let _ = (x, y);
    (0, 0, 1920, 1080)
}

/// 带 ctx 的工作区（points→px）。
pub fn work_area_from_ctx(ctx: &egui::Context) -> (i32, i32, i32, i32) {
    let (size, ppp) = ctx.input(|i| {
        let v = i.viewport();
        (v.monitor_size, i.pixels_per_point())
    });
    if let Some(size) = size {
        let w = (size.x * ppp).round().max(320.0) as i32;
        let h = (size.y * ppp).round().max(240.0) as i32;
        (0, 0, w, h)
    } else {
        (0, 0, 1920, 1080)
    }
}

pub fn fit_popup_pos_points(
    cursor_points: (f32, f32),
    menu_w: f32,
    menu_h: f32,
    _ppp: f32,
) -> (f32, f32) {
    let mut x = cursor_points.0 + 2.0;
    let mut y = cursor_points.1 + 2.0;
    if x + menu_w > 1920.0 {
        x = cursor_points.0 - menu_w - 2.0;
    }
    if y + menu_h > 1080.0 {
        y = cursor_points.1 - menu_h - 2.0;
    }
    (x.max(0.0), y.max(0.0))
}

/// egui 指针 → 屏幕物理像素（视口外接矩形 + 局部点）。
pub fn cursor_screen_px_from_ctx(ctx: &egui::Context) -> Option<(i32, i32)> {
    let (outer, pointer, ppp) = ctx.input(|i| {
        (
            i.viewport().outer_rect,
            i.pointer.latest_pos(),
            i.pixels_per_point(),
        )
    });
    let outer = outer?;
    let pointer = pointer?;
    let sx = ((outer.min.x + pointer.x) * ppp).round() as i32;
    let sy = ((outer.min.y + pointer.y) * ppp).round() as i32;
    Some((sx, sy))
}
