//! Non-Windows geometry fallback for the direct egui host.

/// A portable global-cursor backend has not been wired yet.
pub fn cursor_screen_px() -> Option<(i32, i32)> {
    None
}

/// Keep a popup inside a conservative primary-display work area.
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
