//! Pet 图形绘制职责。

/// 绘制 Pet 小球及其动画效果。
pub(super) fn draw(ui: &mut egui::Ui, time: f32) {
    let pulse = (time * 3.0).sin();
    let radius = 46.0 + pulse * 4.0;
    let center = ui.max_rect().center();
    let painter = ui.painter();

    painter.circle_filled(
        center + egui::vec2(4.0, 6.0),
        radius + 2.0,
        egui::Color32::from_black_alpha(50),
    );
    painter.circle_filled(center, radius, egui::Color32::from_rgb(76, 145, 255));
    painter.circle_filled(
        center + egui::vec2(-14.0, -14.0),
        11.0,
        egui::Color32::from_rgba_unmultiplied(235, 246, 255, 220),
    );
}
