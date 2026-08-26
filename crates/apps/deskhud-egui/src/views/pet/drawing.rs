//! Pet 图形绘制职责。

/// 绘制 Pet 小球及其动画效果。
pub(super) fn draw(ui: &mut egui::Ui, paint: &deskhud_engine::PetPaint) {
    let radius = ui.max_rect().width().min(ui.max_rect().height()) * 0.32 * paint.bounce;
    let center = ui.max_rect().center();
    let painter = ui.painter();

    painter.circle_filled(
        center + egui::vec2(4.0, 6.0),
        radius + 2.0,
        egui::Color32::from_black_alpha(50),
    );
    let body = paint.body_rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8);
    painter.circle_filled(
        center,
        radius,
        egui::Color32::from_rgb(body[0], body[1], body[2]),
    );
    if paint.draw_eyes {
        let eye_open = paint.eye_open.clamp(0.0, 1.0);
        let eye = egui::Color32::from_rgb(
            (paint.eye_rgb[0].clamp(0.0, 1.0) * 255.0) as u8,
            (paint.eye_rgb[1].clamp(0.0, 1.0) * 255.0) as u8,
            (paint.eye_rgb[2].clamp(0.0, 1.0) * 255.0) as u8,
        );
        for x in [-14.0, 14.0] {
            let eye_center = center + egui::vec2(x, -12.0);
            painter.circle_filled(eye_center, 11.0, eye);
            if eye_open > 0.01 {
                painter.circle_filled(
                    eye_center + egui::vec2(paint.pupil_offset[0], paint.pupil_offset[1]),
                    4.5,
                    egui::Color32::from_gray(35),
                );
            }
        }
    }
}
