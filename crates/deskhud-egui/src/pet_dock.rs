//! 宠窗贴边：相对工作区检测与松手吸附（仅 UI 壳；结果以 [`DockState`] 交给宠物包）。

use deskhud_host::DockState;

use crate::platform;

/// 靠近边缘多少逻辑像素时松手吸附。
pub const SNAP_THRESHOLD_POINTS: f32 = 28.0;
/// 判定「已贴边」的容差（物理像素）。
const DOCKED_EPS_PX: i32 = 3;

fn window_rect_px(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    platform::window_screen_rect(hwnd)
}

#[cfg(not(windows))]
fn window_rect_from_ctx(ctx: &eframe::egui::Context) -> Option<(i32, i32, i32, i32)> {
    let (rect, ppp) = ctx.input(|i| (i.viewport().outer_rect, i.pixels_per_point()));
    let rect = rect?;
    let l = (rect.min.x * ppp).round() as i32;
    let t = (rect.min.y * ppp).round() as i32;
    let r = (rect.max.x * ppp).round() as i32;
    let b = (rect.max.y * ppp).round() as i32;
    Some((l, t, r, b))
}

fn work_area_for_window(l: i32, t: i32, r: i32, b: i32) -> (i32, i32, i32, i32) {
    // 窗口中心可能已在屏外；优先用光标所在显示器工作区
    if let Some((cx, cy)) = platform::cursor_screen_px() {
        return platform::work_area_containing_px(cx, cy);
    }
    let cx = (l + r) / 2;
    let cy = (t + b) / 2;
    platform::work_area_containing_px(cx, cy)
}

#[cfg(not(windows))]
fn work_area_for_ctx(ctx: &eframe::egui::Context, l: i32, t: i32, r: i32, b: i32) -> (i32, i32, i32, i32) {
    if let Some((cx, cy)) = platform::cursor_screen_px_from_ctx(ctx) {
        return platform::work_area_containing_px(cx, cy);
    }
    let _ = (l, t, r, b);
    platform::work_area_from_ctx(ctx)
}

/// 根据窗口矩形与工作区计算贴边状态。
pub fn dock_from_rects(
    win_l: i32,
    win_t: i32,
    win_r: i32,
    win_b: i32,
    work_l: i32,
    work_t: i32,
    work_r: i32,
    work_b: i32,
    eps: i32,
) -> DockState {
    DockState {
        left: (win_l - work_l).abs() <= eps,
        right: (win_r - work_r).abs() <= eps,
        top: (win_t - work_t).abs() <= eps,
        bottom: (win_b - work_b).abs() <= eps,
    }
}

/// 若靠近或已超出工作区边缘，将左上角吸附到贴边位置；返回新左上角与吸附后的状态。
pub fn snap_to_work_area(
    win_l: i32,
    win_t: i32,
    win_w: i32,
    win_h: i32,
    work_l: i32,
    work_t: i32,
    work_r: i32,
    work_b: i32,
    threshold_px: i32,
) -> (i32, i32, DockState) {
    let win_r = win_l + win_w;
    let win_b = win_t + win_h;
    let mut x = win_l;
    let mut y = win_t;

    // 出界（超出工作区）或内侧靠近阈值内 → 吸附；仅用 abs 会漏掉「拖到屏幕外」
    let past_or_near_left = win_l <= work_l + threshold_px;
    let past_or_near_right = win_r >= work_r - threshold_px;
    if past_or_near_left && past_or_near_right {
        let overflow_l = work_l - win_l;
        let overflow_r = win_r - work_r;
        if overflow_l >= overflow_r {
            x = work_l;
        } else {
            x = work_r - win_w;
        }
    } else if past_or_near_left {
        x = work_l;
    } else if past_or_near_right {
        x = work_r - win_w;
    }

    let past_or_near_top = win_t <= work_t + threshold_px;
    let past_or_near_bottom = win_b >= work_b - threshold_px;
    if past_or_near_top && past_or_near_bottom {
        let overflow_t = work_t - win_t;
        let overflow_b = win_b - work_b;
        if overflow_t >= overflow_b {
            y = work_t;
        } else {
            y = work_b - win_h;
        }
    } else if past_or_near_top {
        y = work_t;
    } else if past_or_near_bottom {
        y = work_b - win_h;
    }

    let dock = dock_from_rects(
        x,
        y,
        x + win_w,
        y + win_h,
        work_l,
        work_t,
        work_r,
        work_b,
        DOCKED_EPS_PX.max(threshold_px / 8),
    );
    (x, y, dock)
}

/// 读取当前宠窗贴边状态；失败则 [`DockState::FREE`]。
pub fn current_dock(hwnd: isize) -> DockState {
    let Some((l, t, r, b)) = window_rect_px(hwnd) else {
        return DockState::FREE;
    };
    let (wl, wt, wr, wb) = work_area_for_window(l, t, r, b);
    dock_from_rects(l, t, r, b, wl, wt, wr, wb, DOCKED_EPS_PX)
}

/// 非 Windows：用 egui 视口外接矩形算贴边。
#[cfg(not(windows))]
pub fn current_dock_ctx(ctx: &eframe::egui::Context) -> DockState {
    let Some((l, t, r, b)) = window_rect_from_ctx(ctx) else {
        return DockState::FREE;
    };
    let (wl, wt, wr, wb) = work_area_for_ctx(ctx, l, t, r, b);
    dock_from_rects(l, t, r, b, wl, wt, wr, wb, DOCKED_EPS_PX)
}

/// 松手：靠近或出界则吸附并返回新状态。
pub fn snap_on_release(hwnd: isize, threshold_points: f32, pixels_per_point: f32) -> DockState {
    let Some((l, t, r, b)) = window_rect_px(hwnd) else {
        return DockState::FREE;
    };
    let w = (r - l).max(1);
    let h = (b - t).max(1);
    let (wl, wt, wr, wb) = work_area_for_window(l, t, r, b);
    let thr = (threshold_points * pixels_per_point.max(0.01)).round() as i32;
    let thr = thr.max(8);
    let (nx, ny, dock) = snap_to_work_area(l, t, w, h, wl, wt, wr, wb, thr);
    if nx != l || ny != t {
        platform::move_window_screen(hwnd, nx, ny);
    }
    dock
}

/// 非 Windows：松手吸附（ViewportCommand 移动）。
#[cfg(not(windows))]
pub fn snap_on_release_ctx(
    ctx: &eframe::egui::Context,
    threshold_points: f32,
    pixels_per_point: f32,
) -> DockState {
    let Some((l, t, r, b)) = window_rect_from_ctx(ctx) else {
        return DockState::FREE;
    };
    let w = (r - l).max(1);
    let h = (b - t).max(1);
    let (wl, wt, wr, wb) = work_area_for_ctx(ctx, l, t, r, b);
    let ppp = pixels_per_point.max(0.01);
    let thr = (threshold_points * ppp).round() as i32;
    let thr = thr.max(8);
    let (nx, ny, dock) = snap_to_work_area(l, t, w, h, wl, wt, wr, wb, thr);
    if nx != l || ny != t {
        platform::move_viewport_points(ctx, nx as f32 / ppp, ny as f32 / ppp);
    }
    dock
}

/// 切宠改尺寸后：按原贴边边用**新尺寸**重新锚定（勿等 HWND 更新）。
///
/// 右/底贴边时只改 InnerSize 会让右下角离开工作区边缘，且可能超出吸附阈值。
pub fn reanchor_after_size_change(
    hwnd: isize,
    new_w_points: f32,
    new_h_points: f32,
    prefer: DockState,
    threshold_points: f32,
    pixels_per_point: f32,
) -> DockState {
    let Some((l, t)) = platform::window_screen_pos(hwnd) else {
        return DockState::FREE;
    };
    let ppp = pixels_per_point.max(0.01);
    let w = (new_w_points * ppp).round().max(1.0) as i32;
    let h = (new_h_points * ppp).round().max(1.0) as i32;
    let (wl, wt, wr, wb) = work_area_for_window(l, t, l + w, t + h);
    let thr = (threshold_points * ppp).round() as i32;
    let thr = thr.max(8);

    let (nx, ny, dock) = if prefer.is_free() {
        snap_to_work_area(l, t, w, h, wl, wt, wr, wb, thr)
    } else {
        reanchor_to_sides(l, t, w, h, wl, wt, wr, wb, prefer)
    };
    if nx != l || ny != t {
        platform::move_window_screen(hwnd, nx, ny);
    }
    dock
}

#[cfg(not(windows))]
pub fn reanchor_after_size_change_ctx(
    ctx: &eframe::egui::Context,
    new_w_points: f32,
    new_h_points: f32,
    prefer: DockState,
    threshold_points: f32,
    pixels_per_point: f32,
) -> DockState {
    let Some((l, t, _, _)) = window_rect_from_ctx(ctx) else {
        return DockState::FREE;
    };
    let ppp = pixels_per_point.max(0.01);
    let w = (new_w_points * ppp).round().max(1.0) as i32;
    let h = (new_h_points * ppp).round().max(1.0) as i32;
    let (wl, wt, wr, wb) = work_area_for_ctx(ctx, l, t, l + w, t + h);
    let thr = (threshold_points * ppp).round() as i32;
    let thr = thr.max(8);
    let (nx, ny, dock) = if prefer.is_free() {
        snap_to_work_area(l, t, w, h, wl, wt, wr, wb, thr)
    } else {
        reanchor_to_sides(l, t, w, h, wl, wt, wr, wb, prefer)
    };
    if nx != l || ny != t {
        platform::move_viewport_points(ctx, nx as f32 / ppp, ny as f32 / ppp);
    }
    dock
}

/// 强制贴到 `prefer` 指定的边（用于尺寸变化后保持贴边语义）。
pub fn reanchor_to_sides(
    win_l: i32,
    win_t: i32,
    win_w: i32,
    win_h: i32,
    work_l: i32,
    work_t: i32,
    work_r: i32,
    work_b: i32,
    prefer: DockState,
) -> (i32, i32, DockState) {
    let mut x = win_l;
    let mut y = win_t;
    if prefer.left {
        x = work_l;
    } else if prefer.right {
        x = work_r - win_w;
    }
    if prefer.top {
        y = work_t;
    } else if prefer.bottom {
        y = work_b - win_h;
    }
    let dock = dock_from_rects(
        x,
        y,
        x + win_w,
        y + win_h,
        work_l,
        work_t,
        work_r,
        work_b,
        DOCKED_EPS_PX,
    );
    (x, y, dock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_left_and_bottom_corner() {
        let (x, y, dock) = snap_to_work_area(
            12, 500, 100, 100, // near left+bottom of 0,0,800,600
            0, 0, 800, 600, 28,
        );
        assert_eq!(x, 0);
        assert_eq!(y, 500); // 500+100=600 → bottom
        assert!(dock.left);
        assert!(dock.bottom);
        assert!(dock.is_corner());
    }

    #[test]
    fn snap_when_dragged_outside() {
        let (x, y, dock) = snap_to_work_area(
            -80, -40, 100, 100, // fully past top-left
            0, 0, 800, 600, 28,
        );
        assert_eq!((x, y), (0, 0));
        assert!(dock.left);
        assert!(dock.top);
    }

    #[test]
    fn snap_when_past_right_bottom() {
        let (x, y, dock) = snap_to_work_area(
            750, 560, 100, 100, // past right+bottom
            0, 0, 800, 600, 28,
        );
        assert_eq!(x, 700); // 800-100
        assert_eq!(y, 500); // 600-100
        assert!(dock.right);
        assert!(dock.bottom);
    }

    #[test]
    fn free_when_far() {
        let (x, y, dock) = snap_to_work_area(200, 200, 100, 100, 0, 0, 800, 600, 28);
        assert_eq!((x, y), (200, 200));
        assert!(dock.is_free());
    }

    #[test]
    fn reanchor_right_after_shrink() {
        // 原 140 宽贴边：x=660；缩到 96 后应 x=704
        let prefer = DockState {
            left: false,
            right: true,
            top: false,
            bottom: false,
        };
        let (x, y, dock) = reanchor_to_sides(660, 100, 96, 96, 0, 0, 800, 600, prefer);
        assert_eq!((x, y), (704, 100));
        assert!(dock.right);
        assert!(!dock.left);
    }
}
