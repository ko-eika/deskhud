//! HUD 子窗口、布局边界和虚线动画绘制职责。

use super::LayoutState;

const HUD_PADDING: f32 = 8.0;
const PANEL_MIN_SIZE: egui::Vec2 = egui::Vec2::new(136.0, 72.0);

pub(super) struct DrawResult {
    pub(super) size: [f32; 2],
    pub(super) move_by: Option<[f32; 2]>,
}

/// 绘制 HUD 子窗口并返回根据子窗口计算出的 HUD 尺寸。
pub(super) fn draw(ui: &mut egui::Ui, time: f32, layout: &mut LayoutState) -> DrawResult {
    let mut bounds = egui::Rect::NOTHING;
    for (index, position) in layout.positions.iter_mut().enumerate() {
        let title = if index == 0 {
            "HUD panel A"
        } else {
            "HUD panel B"
        };
        let response = egui::Area::new(egui::Id::new(("hud-panel", index)))
            .order(egui::Order::Middle)
            .movable(false)
            .fixed_pos(*position)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(18, 24, 36, 235))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(100, 160, 255, 180),
                    ))
                    .corner_radius(10)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_size(PANEL_MIN_SIZE);
                        ui.horizontal(|ui| {
                            let (dot, _) =
                                ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                dot.center(),
                                4.0,
                                if index == 0 {
                                    egui::Color32::from_rgb(92, 172, 255)
                                } else {
                                    egui::Color32::from_rgb(112, 220, 180)
                                },
                            );
                            ui.label(
                                egui::RichText::new(title)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Live HUD content")
                                .small()
                                .color(egui::Color32::from_gray(180)),
                        );
                    });
                ui.interact(
                    ui.max_rect(),
                    egui::Id::new(("hud-panel-drag", index)),
                    if layout.layout_mode {
                        egui::Sense::drag()
                    } else {
                        egui::Sense::hover()
                    },
                )
            });
        if layout.layout_mode && response.inner.dragged() {
            *position += response.inner.drag_delta();
        }
        if let Some(activity_size) = layout.activity_size {
            let panel_size = response.response.rect.size();
            position.x = position.x.clamp(
                HUD_PADDING,
                (activity_size.x - panel_size.x - HUD_PADDING).max(HUD_PADDING),
            );
            position.y = position.y.clamp(
                HUD_PADDING,
                (activity_size.y - panel_size.y - HUD_PADDING).max(HUD_PADDING),
            );
        }
        bounds = bounds.union(egui::Rect::from_min_size(
            *position,
            response.response.rect.size(),
        ));
    }

    if !bounds.is_positive() {
        return DrawResult {
            size: [160.0, 100.0],
            move_by: None,
        };
    }

    // 布局模式是覆盖活动区的辅助界面：用户排列面板时暂时扩大原生 HUD 窗口，
    // 按 Escape 退出后，下面的紧凑布局分支会将窗口恢复为实际内容尺寸。
    if layout.layout_mode {
        if let Some(activity_size) = layout.activity_size {
            draw_border(
                ui,
                time,
                egui::Rect::from_min_size(egui::Pos2::ZERO, activity_size),
            );
            return DrawResult {
                size: [activity_size.x, activity_size.y],
                move_by: None,
            };
        }
    }

    let offset = egui::vec2(HUD_PADDING, HUD_PADDING) - bounds.min.to_vec2();
    if offset != egui::Vec2::ZERO {
        for position in &mut layout.positions {
            *position += offset;
        }
    }
    let border_size = bounds.size() + egui::vec2(HUD_PADDING * 2.0, HUD_PADDING * 2.0);
    if layout.layout_mode {
        draw_border(
            ui,
            time,
            egui::Rect::from_min_size(egui::Pos2::ZERO, border_size),
        );
    }
    if layout.compact_pending {
        layout.compact_pending = false;
        layout.activity_size = None;
    }
    DrawResult {
        size: [border_size.x.max(160.0), border_size.y.max(100.0)],
        move_by: if offset == egui::Vec2::ZERO {
            None
        } else {
            Some([-offset.x, -offset.y])
        },
    }
}

/// 绘制沿边框移动的虚线动画。
pub(super) fn draw_border(ui: &mut egui::Ui, time: f32, rect: egui::Rect) {
    let dash_phase = (time * 42.0) % 16.0;
    let rect = rect.shrink(2.0);
    let stroke = egui::Stroke::new(
        2.0,
        egui::Color32::from_rgba_unmultiplied(76, 145, 255, 220),
    );
    let dash = 10.0;
    let gap = 6.0;
    let draw_dashes = |start: egui::Pos2, end: egui::Pos2| {
        let length = start.distance(end);
        let direction = (end - start) / length;
        let mut offset = -dash_phase;
        while offset < length {
            let dash_start = offset.max(0.0);
            let dash_end = (offset + dash).min(length);
            if dash_end > dash_start {
                ui.painter().line_segment(
                    [start + direction * dash_start, start + direction * dash_end],
                    stroke,
                );
            }
            offset += dash + gap;
        }
    };
    draw_dashes(rect.left_top(), rect.right_top());
    draw_dashes(rect.right_top(), rect.right_bottom());
    draw_dashes(rect.right_bottom(), rect.left_bottom());
    draw_dashes(rect.left_bottom(), rect.left_top());
}
