//! 宠物帧绘制（主宠窗）。

use eframe::egui::{self, Color32, CornerRadius, FontId, Galley, Pos2, Stroke, Vec2};
use deskhud_host::PetPaint;
use std::sync::Arc;

/// 在指定中心与基准半径绘制一帧宠物。
/// `window_w`：宠窗逻辑宽，用于气泡换行上限。
pub fn draw_pet_frame(
    painter: &egui::Painter,
    center: Pos2,
    base_radius: f32,
    paint: &PetPaint,
    pupil_smooth: [f32; 2],
    window_w: f32,
) {
    let radius = base_radius * paint.bounce;
    let body = Color32::from_rgb(
        (paint.body_rgb[0] * 255.0) as u8,
        (paint.body_rgb[1] * 255.0) as u8,
        (paint.body_rgb[2] * 255.0) as u8,
    );
    let eye_white = Color32::from_rgb(
        (paint.eye_rgb[0] * 255.0) as u8,
        (paint.eye_rgb[1] * 255.0) as u8,
        (paint.eye_rgb[2] * 255.0) as u8,
    );
    let pupil = Color32::from_rgb(28, 32, 40);

    painter.circle_filled(center, radius, body);

    if paint.draw_eyes {
        let eye_y = -radius * 0.12;
        let eye_x = radius * 0.28;
        let eye_r = radius * 0.16;
        let pupil_r = eye_r * 0.48;
        let pupil_d = Vec2::new(pupil_smooth[0], pupil_smooth[1]);
        let left = center + Vec2::new(-eye_x, eye_y);
        let right = center + Vec2::new(eye_x, eye_y);
        painter.circle_filled(left, eye_r, eye_white);
        painter.circle_filled(right, eye_r, eye_white);
        painter.circle_filled(left + pupil_d, pupil_r, pupil);
        painter.circle_filled(right + pupil_d, pupil_r, pupil);
    }

    if let Some(text) = paint.bubble_text.as_deref().filter(|t| !t.is_empty()) {
        let max_w = (window_w - 16.0).clamp(72.0, window_w.max(72.0));
        draw_speech_bubble(painter, center, radius, text, max_w);
    }
}

fn layout_bubble_text(
    painter: &egui::Painter,
    text: &str,
    font: FontId,
    color: Color32,
    max_w: f32,
    max_lines: usize,
) -> Arc<Galley> {
    let line_h = font.size * 1.35;
    let mut candidate = text.to_string();
    for _ in 0..64 {
        let galley = painter.layout(candidate.clone(), font.clone(), color, max_w);
        let lines = ((galley.size().y / line_h).round() as usize).max(1);
        if lines <= max_lines {
            return galley;
        }
        let keep = candidate.chars().count().saturating_sub(2).max(1);
        candidate = candidate.chars().take(keep).collect::<String>();
        while candidate.ends_with('+') || candidate.ends_with('…') {
            candidate.pop();
        }
        candidate.push('…');
    }
    painter.layout(candidate, font, color, max_w)
}

fn draw_speech_bubble(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    text: &str,
    max_bubble_w: f32,
) {
    let font = FontId::proportional(12.5);
    let color = Color32::from_rgb(35, 40, 48);
    let pad_x = 9.0;
    let pad_y = 5.0;
    let text_max_w = (max_bubble_w - pad_x * 2.0).max(40.0);
    let galley = layout_bubble_text(painter, text, font, color, text_max_w, 3);
    let tw = galley.size().x;
    let th = galley.size().y;
    let bw = (tw + pad_x * 2.0).clamp(36.0, max_bubble_w);
    let bh = th + pad_y * 2.0;
    let gap = 4.0;
    let bubble_center = Pos2::new(center.x, center.y - radius * 0.72 - bh * 0.15 - gap);
    let mut rect = egui::Rect::from_center_size(bubble_center, Vec2::new(bw, bh));

    let max_x = center.x + (max_bubble_w * 0.5) - 2.0;
    let min_x = center.x - (max_bubble_w * 0.5) + 2.0;
    if rect.center().x > max_x {
        rect = rect.translate(Vec2::new(max_x - rect.center().x, 0.0));
    }
    if rect.center().x < min_x {
        rect = rect.translate(Vec2::new(min_x - rect.center().x, 0.0));
    }
    // 避免顶出窗上沿
    if rect.top() < 2.0 {
        rect = rect.translate(Vec2::new(0.0, 2.0 - rect.top()));
    }

    let fill = Color32::from_rgba_unmultiplied(255, 255, 255, 235);
    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(40, 50, 70, 90));
    painter.rect(
        rect,
        CornerRadius::same(10),
        fill,
        stroke,
        egui::StrokeKind::Middle,
    );

    let tip = Pos2::new(center.x, (center.y - radius * 0.35).max(rect.bottom() + 1.0));
    let base_y = rect.bottom();
    let tri = vec![
        Pos2::new(rect.center().x - 6.0, base_y),
        Pos2::new(rect.center().x + 6.0, base_y),
        tip,
    ];
    painter.add(egui::Shape::convex_polygon(tri, fill, Stroke::NONE));

    let text_pos = Pos2::new(
        rect.center().x - tw * 0.5,
        rect.center().y - th * 0.5,
    );
    painter.galley(text_pos, galley, color);
}

/// 根据宠窗尺寸估算绘制半径。
pub fn pet_base_radius(window_w: f32, window_h: f32) -> f32 {
    window_w.min(window_h) * 0.38
}
