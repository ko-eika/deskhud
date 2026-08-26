//! HUD 子窗口、布局边界和虚线动画绘制职责。

use super::{HudRenderItem, LayoutState};
use deskhud_engine::HudVisual;

const HUD_PADDING: f32 = 8.0;
const EMPTY_FRAME_SIZE: egui::Vec2 = egui::Vec2::new(136.0, 72.0);

pub(super) struct DrawResult {
    pub(super) size: [f32; 2],
    pub(super) move_by: Option<[f32; 2]>,
}

/// 绘制 HUD 子窗口并返回根据子窗口计算出的 HUD 尺寸。
pub(super) fn draw(
    ui: &mut egui::Ui,
    time: f32,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
) -> DrawResult {
    let mut bounds = egui::Rect::NOTHING;
    for item in items {
        let position = layout
            .positions
            .entry(item.key.clone())
            .or_insert(item.initial_position);
        let response = egui::Area::new(egui::Id::new(("hud-item", &item.key)))
            .order(egui::Order::Middle)
            .movable(false)
            .fixed_pos(*position)
            .show(ui.ctx(), |ui| draw_frame(ui, item, layout.layout_mode));
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
        for item in items {
            if let Some(position) = layout.positions.get_mut(&item.key) {
                *position += offset;
            }
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

fn draw_frame(ui: &mut egui::Ui, item: &HudRenderItem, layout_mode: bool) -> egui::Response {
    let base_size = frame_size(&item.frame.visuals);
    let scale = item.scale.clamp(0.5, 3.0);
    let size = base_size * scale;
    let (rect, response) = ui.allocate_exact_size(
        size,
        if layout_mode {
            egui::Sense::drag()
        } else {
            egui::Sense::hover()
        },
    );
    let ui_font_scale =
        egui::TextStyle::Body.resolve(ui.style()).size / deskhud_ui::DEFAULT_UI_FONT_SIZE.max(1.0);
    for visual in &item.frame.visuals {
        match visual {
            HudVisual::Panel {
                width,
                height,
                radius,
                color,
            } => {
                let panel = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(width.max(1.0), height.max(1.0)) * scale,
                );
                ui.painter().rect_filled(
                    panel,
                    (*radius * scale).round().clamp(0.0, 255.0) as u8,
                    rgba(*color),
                );
            }
            HudVisual::Text {
                text,
                font_size,
                color,
            } => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(
                        (font_size * scale * ui_font_scale).clamp(8.0, 96.0),
                    ),
                    rgba(*color),
                );
            }
        }
    }
    response
}

fn frame_size(visuals: &[HudVisual]) -> egui::Vec2 {
    let mut size = egui::Vec2::ZERO;
    for visual in visuals {
        match visual {
            HudVisual::Panel { width, height, .. } => {
                size.x = size.x.max(*width);
                size.y = size.y.max(*height);
            }
            HudVisual::Text {
                text, font_size, ..
            } => {
                size.x = size
                    .x
                    .max(text.chars().count() as f32 * font_size * 0.62 + 20.0);
                size.y = size.y.max(font_size + 16.0);
            }
        }
    }
    if size == egui::Vec2::ZERO {
        EMPTY_FRAME_SIZE
    } else {
        size
    }
}

fn rgba([red, green, blue, alpha]: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

/// 绘制沿边框移动的虚线动画。
pub(super) fn draw_border(ui: &mut egui::Ui, time: f32, rect: egui::Rect) {
    let dash_phase = (time * 42.0) % 16.0;
    let rect = rect.shrink(2.0);
    let stroke = egui::Stroke::new(2.0, with_alpha(ui.visuals().selection.bg_fill, 220));
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

fn with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    let [red, green, blue, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}
