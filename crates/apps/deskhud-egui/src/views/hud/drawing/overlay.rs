//! HUD 布局编辑选中框与活动区域边界。

use super::*;

#[derive(Clone)]
pub(super) struct EditorOverlay {
    pub(super) key: String,
    pub(super) rect: egui::Rect,
    pub(super) layer_id: egui::LayerId,
    pub(super) corner_radius: f32,
}

#[derive(Clone, Copy)]
pub(super) enum GroupDropFeedback {
    Add,
    Remove,
}

pub(super) fn draw_group_drop_feedback(
    ui: &egui::Ui,
    time: f32,
    overlay: &EditorOverlay,
    feedback: GroupDropFeedback,
) {
    let color = match feedback {
        GroupDropFeedback::Add => to_egui_color(crate::views::theme::palette(ui.visuals()).success),
        GroupDropFeedback::Remove => {
            to_egui_color(crate::views::theme::palette(ui.visuals()).danger)
        }
    };
    let pulse = ((time * 5.0).sin() * 0.5 + 0.5) * 0.35 + 0.25;
    let painter = ui.ctx().layer_painter(overlay.layer_id);
    let radius = overlay.corner_radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX;
    painter.rect_filled(
        overlay.rect,
        radius,
        with_alpha(color, (pulse * 96.0).round() as u8),
    );
    painter.rect_stroke(
        overlay.rect.expand(3.0),
        radius + 3.0,
        egui::Stroke::new(3.0, with_alpha(color, 235)),
        egui::StrokeKind::Outside,
    );
}

pub(super) fn draw_editor_overlays(
    ui: &egui::Ui,
    time: f32,
    overlays: &[EditorOverlay],
    selected_key: Option<&str>,
) {
    for overlay in overlays {
        let selected = selected_key == Some(overlay.key.as_str());
        let painter = ui.ctx().layer_painter(overlay.layer_id);
        if selected {
            draw_animated_border(
                &painter,
                time,
                overlay.rect.expand(4.0),
                ui.visuals().selection.bg_fill,
                overlay.corner_radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX + 4.0,
            );
        }
    }
}

/// 绘制沿边框移动的虚线动画。
pub(super) fn draw_border(ui: &mut egui::Ui, time: f32, rect: egui::Rect) {
    draw_animated_border(
        ui.painter(),
        time,
        rect,
        ui.visuals().selection.bg_fill,
        0.0,
    );
}

/// Draws the compact preview background used as the layout editor's virtual
/// root group. It follows the active egui theme for light/dark mode legibility.
pub(super) fn draw_preview_background(ui: &egui::Ui, rect: egui::Rect) {
    let fill = ui.visuals().window_fill();
    let [red, green, blue, _] = fill.to_array();
    ui.painter().rect_filled(
        rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(red, green, blue, 8),
    );
}

pub(super) fn draw_preview_border(ui: &egui::Ui, rect: egui::Rect) {
    let color = ui
        .visuals()
        .widgets
        .noninteractive
        .fg_stroke
        .color
        .gamma_multiply(0.9);
    ui.painter().rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(2.0, with_alpha(color, 160)),
        egui::StrokeKind::Outside,
    );
}

fn draw_animated_border(
    painter: &egui::Painter,
    time: f32,
    rect: egui::Rect,
    color: egui::Color32,
    corner_radius: f32,
) {
    let dash_phase = (time * 42.0) % 16.0;
    let rect = rect.shrink(2.0);
    let dash = 10.0;
    let gap = 6.0;
    let path = rounded_rect_path(rect, corner_radius);
    let guard_color = animated_border_guard_color(color);
    // Paint a wider contrast guard first. It follows the exact same animated
    // dash path, so the selection remains legible over user-selected borders,
    // translucent HUD backgrounds, and arbitrary desktop colors.
    painter.extend(egui::Shape::dashed_line_with_offset(
        &path,
        egui::Stroke::new(5.0, guard_color),
        &[dash],
        &[gap],
        dash_phase,
    ));
    painter.extend(egui::Shape::dashed_line_with_offset(
        &path,
        egui::Stroke::new(2.0, with_alpha(color, 240)),
        &[dash],
        &[gap],
        dash_phase,
    ));
}

fn animated_border_guard_color(color: egui::Color32) -> egui::Color32 {
    let [red, green, blue, _] = color.to_array();
    let luminance = red as f32 * 0.2126 + green as f32 * 0.7152 + blue as f32 * 0.0722;
    if luminance < 140.0 {
        egui::Color32::from_white_alpha(190)
    } else {
        egui::Color32::from_black_alpha(190)
    }
}

fn rounded_rect_path(rect: egui::Rect, corner_radius: f32) -> Vec<egui::Pos2> {
    let radius = corner_radius
        .max(0.0)
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5);
    if radius <= f32::EPSILON {
        return vec![
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
            rect.left_top(),
        ];
    }

    let mut points = vec![
        egui::pos2(rect.left() + radius, rect.top()),
        egui::pos2(rect.right() - radius, rect.top()),
    ];
    let arcs = [
        (
            egui::pos2(rect.right() - radius, rect.top() + radius),
            -std::f32::consts::FRAC_PI_2,
        ),
        (
            egui::pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
        ),
        (
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            std::f32::consts::FRAC_PI_2,
        ),
        (
            egui::pos2(rect.left() + radius, rect.top() + radius),
            std::f32::consts::PI,
        ),
    ];
    let connectors = [
        egui::pos2(rect.right(), rect.bottom() - radius),
        egui::pos2(rect.left() + radius, rect.bottom()),
        egui::pos2(rect.left(), rect.top() + radius),
    ];
    for (index, (center, start_angle)) in arcs.into_iter().enumerate() {
        for step in 1..=4 {
            let angle = start_angle + std::f32::consts::FRAC_PI_2 * step as f32 / 4.0;
            points.push(center + egui::vec2(angle.cos(), angle.sin()) * radius);
        }
        if let Some(connector) = connectors.get(index) {
            points.push(*connector);
        }
    }
    points
}

pub(super) fn with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    let [red, green, blue, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

fn to_egui_color(color: deskhud_engine::OverlayColor) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.red, color.green, color.blue, color.alpha)
}
