//! Settings 动画绘制职责。

/// 绘制 Settings 的渲染状态动画。
pub(super) fn draw_status(ui: &mut egui::Ui, time: f32) {
    ui.add_space(18.0);
    let (spinner_rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::hover());
    let center = spinner_rect.center();
    let pulse = 0.5 + 0.5 * (time * 3.0).sin();
    ui.painter().circle_stroke(
        center,
        18.0,
        egui::Stroke::new(2.0, egui::Color32::from_gray(180)),
    );
    for index in 0..8 {
        let angle = time * 4.0 + index as f32 * std::f32::consts::TAU / 8.0;
        let point = center + egui::vec2(angle.cos() * 18.0, angle.sin() * 18.0);
        let alpha = 70 + (((index as f32 + time * 4.0) % 8.0) / 8.0 * 185.0) as u8;
        ui.painter().circle_filled(
            point,
            3.0 + pulse * 1.5,
            egui::Color32::from_rgba_unmultiplied(70, 145, 255, alpha),
        );
    }
    ui.label("Settings render loop is active");
}
