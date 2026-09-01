//! HUD 子窗口、布局边界和虚线动画绘制职责。

use super::{AdjustmentUnit, HudRenderItem, LayoutState};
use deskhud_engine::HudVisual;
use deskhud_ui::{MessageKey, UiPreferences};

const HUD_PADDING: f32 = 8.0;
const EMPTY_FRAME_SIZE: egui::Vec2 = egui::Vec2::new(136.0, 72.0);
const GRID_STEP: f32 = 0.05;

pub(super) struct DrawResult {
    pub(super) size: [f32; 2],
    pub(super) move_by: Option<[f32; 2]>,
    pub(super) changed: bool,
}

struct FrameResponse {
    body: egui::Response,
    size: egui::Vec2,
}

/// 绘制 HUD 子窗口并返回根据子窗口计算出的 HUD 尺寸。
pub(super) fn draw(
    ui: &mut egui::Ui,
    time: f32,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
) -> DrawResult {
    let mut bounds = egui::Rect::NOTHING;
    let mut changed = false;
    if layout.layout_mode
        && layout.snap_to_grid
        && let Some(activity) = layout.activity_size
    {
        draw_alignment_grid(ui, activity);
    }
    for item in items {
        let position = layout
            .positions
            .entry(item.key.clone())
            .or_insert(item.initial_position);
        let base_size = frame_size(&item.frame.visuals);
        let preferred_size = egui::vec2(
            base_size.x * item.width.clamp(0.5, 3.0),
            base_size.y * item.height.clamp(0.5, 3.0),
        );
        let mut window = egui::Window::new(egui::RichText::new(&item.key).small())
            .id(egui::Id::new((
                "hud-item",
                &item.key,
                layout.adjust_session,
                layout.window_revision,
            )))
            .title_bar(false)
            .resizable(layout.layout_mode)
            .collapsible(false)
            .movable(false)
            // The HUD visual owns the background. The egui window frame must
            // stay transparent, otherwise its padding/background becomes a
            // second rectangle around the actual HUD panel.
            .frame(egui::Frame::NONE)
            .default_size(preferred_size)
            .min_size(base_size * 0.5)
            .fixed_pos(*position);
        if !layout.layout_mode {
            // Layout mode may leave a remembered large Window rectangle in
            // egui memory. Compact HUDs must size themselves to their content
            // again when the editor is closed.
            window = window.auto_sized();
        }
        let response = window.show(ui.ctx(), |ui| {
            draw_frame(
                ui,
                item,
                layout.layout_mode,
                layout.selected.as_deref() == Some(item.key.as_str()),
            )
        });
        let Some(response) = response else { continue };
        let Some(frame) = response.inner else {
            continue;
        };
        if layout.layout_mode && (frame.body.clicked() || frame.body.drag_started()) {
            layout.selected = Some(item.key.clone());
        }
        if layout.layout_mode && frame.body.secondary_clicked() {
            layout.selected = Some(item.key.clone());
            layout.adjust_open = true;
        }
        if layout.layout_mode && frame.body.dragged() {
            *position += frame.body.drag_delta();
            if layout.snap_to_grid
                && let Some(activity) = layout.activity_size
            {
                position.x = snap_coordinate(position.x, activity.x);
                position.y = snap_coordinate(position.y, activity.y);
            }
            changed = true;
        }
        if layout.layout_mode {
            let base = frame_size(&item.frame.visuals);
            let next_width = (frame.size.x / base.x.max(1.0)).clamp(0.5, 3.0);
            let next_height = (frame.size.y / base.y.max(1.0)).clamp(0.5, 3.0);
            if ((next_width - item.width).abs() > 0.0001
                || (next_height - item.height).abs() > 0.0001)
                && let Some(slot) = layout_slot(prefs, &item.key)
            {
                prefs.hud.set_slot_layout(
                    &slot.0,
                    &slot.1,
                    deskhud_ui::HudSlotLayout {
                        display: slot.2.display,
                        x: slot.2.x,
                        y: slot.2.y,
                        scale: slot.2.scale,
                        width: next_width,
                        height: next_height,
                    },
                );
                changed = true;
            }
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

    if layout.layout_mode && layout.adjust_open {
        changed |= draw_adjust_window(ui, layout, items, prefs);
    }

    if !bounds.is_positive() {
        return DrawResult {
            size: [160.0, 100.0],
            move_by: None,
            changed,
        };
    }

    // 布局模式是覆盖活动区的辅助界面：用户排列面板时暂时扩大原生 HUD 窗口，
    // 按 Escape 退出后，下面的紧凑布局分支会将窗口恢复为实际内容尺寸。
    if layout.layout_mode
        && let Some(activity_size) = layout.activity_size
    {
        draw_border(
            ui,
            time,
            egui::Rect::from_min_size(egui::Pos2::ZERO, activity_size),
        );
        return DrawResult {
            size: [activity_size.x, activity_size.y],
            move_by: None,
            changed: changed || sync_layouts(prefs, layout, items, activity_size),
        };
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
        changed: changed || sync_layouts(prefs, layout, items, egui::Vec2::ZERO),
    }
}

fn draw_adjust_window(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
) -> bool {
    let Some(key) = layout.selected.clone() else {
        return false;
    };
    let Some(item) = items.iter().find(|item| item.key == key) else {
        return false;
    };
    let Some((plugin, contribution, mut slot)) = layout_slot(prefs, &key) else {
        return false;
    };
    let initial_width = slot.width;
    let initial_height = slot.height;
    let initial_ratio = layout
        .locked_ratio
        .unwrap_or(initial_height / initial_width.max(0.001));
    let initial_lock_ratio = layout.lock_ratio;
    let mut changed = false;
    let mut width_changed = false;
    let mut height_changed = false;
    let mut open = layout.adjust_open;

    egui::Window::new(
        egui::RichText::new(deskhud_ui::i18n::t(
            prefs.locale,
            MessageKey::HudAdjustTitle,
        ))
        .strong(),
    )
    .id(egui::Id::new(("hud-adjust-window", layout.adjust_session)))
    .default_pos(
        layout
            .activity_size
            .map(|size| egui::pos2((size.x - 360.0).max(24.0), 32.0))
            .unwrap_or(egui::pos2(24.0, 32.0)),
    )
    // The adjustment panel has a stable content canvas. This prevents each
    // group from choosing a different shrink-to-fit width and also discards a
    // previously remembered full-screen-sized panel.
    .fixed_size(egui::vec2(460.0, 440.0))
    .resizable(false)
    .open(&mut open)
    .show(ui.ctx(), |ui| {
        changed |= draw_position_group(ui, layout, prefs, &mut slot);
        ui.add_space(8.0);
        let (size_changed, width_was_changed, height_was_changed) =
            draw_size_group(ui, layout, prefs, &mut slot, item, initial_ratio);
        changed |= size_changed;
        width_changed = width_was_changed;
        height_changed = height_was_changed;
        ui.add_space(8.0);
        changed |= draw_effects_group(ui, prefs, &plugin, &contribution, item);
    });
    layout.adjust_open = open;
    if layout.lock_ratio && !initial_lock_ratio {
        layout.locked_ratio = Some(initial_height / initial_width.max(0.001));
    } else if !layout.lock_ratio {
        layout.locked_ratio = None;
    }
    if layout.lock_ratio {
        let ratio = layout.locked_ratio.unwrap_or(initial_ratio).max(0.001);
        if width_changed && !height_changed {
            slot.height = (slot.width * ratio).clamp(0.5, 3.0);
            slot.width = (slot.height / ratio).clamp(0.5, 3.0);
            changed = true;
        } else if height_changed && !width_changed {
            slot.width = (slot.height / ratio).clamp(0.5, 3.0);
            slot.height = (slot.width * ratio).clamp(0.5, 3.0);
            changed = true;
        }
    }
    if changed {
        if (slot.width - initial_width).abs() > 0.0001
            || (slot.height - initial_height).abs() > 0.0001
        {
            layout.window_revision = layout.window_revision.wrapping_add(1);
        }
        let (slot_x, slot_y) = (slot.x, slot.y);
        prefs.hud.set_slot_layout(&plugin, &contribution, slot);
        if let Some(pos) = layout.positions.get_mut(&key)
            && let Some(activity) = layout.activity_size
        {
            pos.x = slot_x * activity.x;
            pos.y = slot_y * activity.y;
        }
    }
    changed
}

fn draw_position_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &UiPreferences,
    slot: &mut deskhud_ui::HudSlotLayout,
) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustPosition,
                ))
                .strong(),
            );
        });
        ui.separator();
        ui.add_space(4.0);
        let activity = layout.activity_size.unwrap_or(egui::vec2(1.0, 1.0));
        let row_width = ui.available_width();
        for (label, value, pixels, id) in [
            (
                MessageKey::HudAdjustX,
                &mut slot.x,
                activity.x,
                "hud-position-x",
            ),
            (
                MessageKey::HudAdjustY,
                &mut slot.y,
                activity.y,
                "hud-position-y",
            ),
        ] {
            ui.horizontal(|ui| {
                ui.set_width(row_width);
                ui.add_sized(
                    [72.0, 24.0],
                    egui::Label::new(deskhud_ui::i18n::t(prefs.locale, label)),
                );
                let mut shown = adjustment_value(*value, layout.position_unit, pixels);
                if ui
                    .add_sized(
                        [132.0, 24.0],
                        adjustment_drag_value(&mut shown, layout.position_unit, pixels, 1.0),
                    )
                    .changed()
                {
                    *value = (shown / adjustment_reference(layout.position_unit, pixels))
                        .clamp(0.0, 1.0);
                    if layout.snap_to_grid {
                        *value = snap_normalized(*value);
                    }
                    changed = true;
                }
                changed |= adjustment_unit(ui, id, &mut layout.position_unit, prefs.locale);
                if id == "hud-position-x" {
                    ui.add_space(8.0);
                    changed |= draw_snap_grid_control(ui, layout, prefs.locale);
                }
            });
        }
    });
    changed
}

fn draw_size_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &UiPreferences,
    slot: &mut deskhud_ui::HudSlotLayout,
    item: &HudRenderItem,
    ratio: f32,
) -> (bool, bool, bool) {
    let mut changed = false;
    let mut width_changed = false;
    let mut height_changed = false;
    let base = frame_size(&item.frame.visuals);
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustSize))
                .strong(),
        );
        ui.separator();
        ui.add_space(4.0);
        let group_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.set_width(group_width);
            ui.vertical(|ui| {
                ui.set_width((group_width - 48.0).max(180.0));
                let width_max = if layout.lock_ratio {
                    (3.0 / ratio.max(0.001)).clamp(0.5, 3.0)
                } else {
                    3.0
                };
                ui.horizontal(|ui| {
                    ui.set_width((group_width - 48.0).max(180.0));
                    ui.add_sized(
                        [72.0, 24.0],
                        egui::Label::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustWidth,
                        )),
                    );
                    let mut shown = adjustment_value(slot.width, layout.size_unit, base.x);
                    width_changed = ui
                        .add_sized(
                            [132.0, 24.0],
                            adjustment_drag_value(
                                &mut shown,
                                layout.size_unit,
                                base.x * width_max,
                                width_max,
                            ),
                        )
                        .changed();
                    if width_changed {
                        slot.width = (shown / adjustment_reference(layout.size_unit, base.x))
                            .clamp(0.5, width_max);
                    }
                    changed |= width_changed;
                    changed |=
                        adjustment_unit(ui, "hud-size-width", &mut layout.size_unit, prefs.locale);
                    ui.add_space(8.0);
                    changed |= draw_ratio_lock_control(ui, layout, prefs.locale);
                });
                ui.add_space(4.0);
                let height_max = if layout.lock_ratio {
                    (3.0 * ratio).clamp(0.5, 3.0)
                } else {
                    3.0
                };
                ui.horizontal(|ui| {
                    ui.set_width((group_width - 48.0).max(180.0));
                    ui.add_sized(
                        [72.0, 24.0],
                        egui::Label::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustHeight,
                        )),
                    );
                    let mut shown = adjustment_value(slot.height, layout.size_unit, base.y);
                    height_changed = ui
                        .add_sized(
                            [132.0, 24.0],
                            adjustment_drag_value(
                                &mut shown,
                                layout.size_unit,
                                base.y * height_max,
                                height_max,
                            ),
                        )
                        .changed();
                    if height_changed {
                        slot.height = (shown / adjustment_reference(layout.size_unit, base.y))
                            .clamp(0.5, height_max);
                    }
                    changed |= height_changed;
                    changed |=
                        adjustment_unit(ui, "hud-size-height", &mut layout.size_unit, prefs.locale);
                });
            });
        });
    });
    (changed, width_changed, height_changed)
}

fn draw_effects_group(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    plugin: &str,
    contribution: &str,
    item: &HudRenderItem,
) -> bool {
    let mut changed = false;
    ui.group(|ui| {
        ui.set_min_width(ui.available_width());
        ui.label(
            egui::RichText::new(deskhud_ui::i18n::t(
                prefs.locale,
                MessageKey::HudAdjustEffects,
            ))
            .strong(),
        );
        ui.separator();
        ui.add_space(4.0);
        let row_width = ui.available_width();
        for (name, message, default) in [
            (
                "background_opacity",
                MessageKey::HudAdjustBackgroundOpacity,
                item.background_opacity,
            ),
            (
                "background_blur",
                MessageKey::HudAdjustBackgroundBlur,
                item.background_blur,
            ),
            (
                "content_opacity",
                MessageKey::HudAdjustContentOpacity,
                item.content_opacity,
            ),
        ] {
            ui.horizontal(|ui| {
                ui.set_width(row_width);
                ui.add_sized(
                    [88.0, 24.0],
                    egui::Label::new(deskhud_ui::i18n::t(prefs.locale, message)),
                );
                let mut value = prefs.hud.visual_value(plugin, contribution, name, default);
                let value_width = 42.0;
                let slider_width = (ui.available_width() - value_width).max(80.0);
                if ui
                    .add_sized(
                        [slider_width, 24.0],
                        egui::Slider::new(&mut value, 0.0..=1.0).show_value(false),
                    )
                    .changed()
                {
                    prefs
                        .hud
                        .set_visual_value(plugin, contribution, name, value);
                    changed = true;
                }
                ui.add_sized([value_width, 24.0], egui::Label::new(format!("{value:.2}")));
            });
        }
    });
    changed
}

#[allow(dead_code)]
fn draw_adjust_window_legacy(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    items: &[HudRenderItem],
    prefs: &mut UiPreferences,
) -> bool {
    let Some(key) = layout.selected.clone() else {
        return false;
    };
    let Some(item) = items.iter().find(|item| item.key == key) else {
        return false;
    };
    let Some((plugin, contribution, mut slot)) = layout_slot(prefs, &key) else {
        return false;
    };
    let initial_width = slot.width;
    let initial_height = slot.height;
    let initial_ratio = layout
        .locked_ratio
        .unwrap_or_else(|| initial_height / initial_width.max(0.001));
    let initial_lock_ratio = layout.lock_ratio;
    let mut changed = false;
    let mut width_changed = false;
    let mut height_changed = false;
    let mut open = layout.adjust_open;
    egui::Window::new(
        egui::RichText::new(deskhud_ui::i18n::t(
            prefs.locale,
            MessageKey::HudAdjustTitle,
        ))
        .strong(),
    )
    .id(egui::Id::new(("hud-adjust-window", layout.adjust_session)))
    .default_pos(
        layout
            .activity_size
            .map(|size| egui::pos2((size.x - 360.0).max(24.0), 32.0))
            .unwrap_or(egui::pos2(24.0, 32.0)),
    )
    .default_size(egui::vec2(336.0, 360.0))
    .resizable(false)
    .open(&mut open)
    .show(ui.ctx(), |ui| {
        egui::Grid::new("hud-adjust-grid")
            .num_columns(4)
            .spacing(egui::vec2(12.0, 10.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(deskhud_ui::i18n::t(
                        prefs.locale,
                        MessageKey::HudAdjustPosition,
                    ))
                    .strong(),
                );
                ui.label("");
                ui.label("");
                ui.label("");
                ui.end_row();
                ui.label(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustX));
                let activity = layout.activity_size.unwrap_or(egui::vec2(1.0, 1.0));
                let mut x = adjustment_value(slot.x, layout.position_unit, activity.x);
                if ui
                    .add(adjustment_drag_value(
                        &mut x,
                        layout.position_unit,
                        activity.x,
                        1.0,
                    ))
                    .changed()
                {
                    slot.x = (x / adjustment_reference(layout.position_unit, activity.x))
                        .clamp(0.0, 1.0);
                    changed = true;
                }
                changed |= adjustment_unit(
                    ui,
                    "hud-adjust-position-x",
                    &mut layout.position_unit,
                    prefs.locale,
                );
                ui.label("");
                ui.end_row();
                ui.label(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustY));
                let mut y = adjustment_value(slot.y, layout.position_unit, activity.y);
                if ui
                    .add(adjustment_drag_value(
                        &mut y,
                        layout.position_unit,
                        activity.y,
                        1.0,
                    ))
                    .changed()
                {
                    slot.y = (y / adjustment_reference(layout.position_unit, activity.y))
                        .clamp(0.0, 1.0);
                    changed = true;
                }
                changed |= adjustment_unit(
                    ui,
                    "hud-adjust-position-y",
                    &mut layout.position_unit,
                    prefs.locale,
                );
                ui.label("");
                ui.end_row();
                ui.label(deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustSize));
                ui.label("");
                ui.label("");
                ui.label("");
                ui.end_row();
                ui.label(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustWidth,
                ));
                let width_max = if layout.lock_ratio {
                    (3.0 / initial_ratio.max(0.001)).clamp(0.5, 3.0)
                } else {
                    3.0
                };
                let base = frame_size(&item.frame.visuals);
                let mut width = adjustment_value(slot.width, layout.size_unit, base.x);
                width_changed = ui
                    .add(adjustment_drag_value(
                        &mut width,
                        layout.size_unit,
                        base.x * width_max,
                        width_max,
                    ))
                    .changed();
                if width_changed {
                    slot.width = (width / adjustment_reference(layout.size_unit, base.x))
                        .clamp(0.5, width_max);
                }
                changed |= width_changed;
                changed |= adjustment_unit(
                    ui,
                    "hud-adjust-size-width",
                    &mut layout.size_unit,
                    prefs.locale,
                );
                ui.label("");
                ui.end_row();
                ui.label("");
                ui.label("");
                ui.label("");
                let (lock_rect, lock_response) =
                    ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
                if lock_response.clicked() {
                    layout.lock_ratio = !layout.lock_ratio;
                    changed = true;
                }
                let lock_color = if layout.lock_ratio {
                    ui.visuals().selection.stroke.color
                } else {
                    ui.visuals().widgets.inactive.fg_stroke.color
                };
                let line_x = lock_rect.center().x;
                ui.painter().line_segment(
                    [
                        egui::pos2(line_x, lock_rect.top() - 9.0),
                        egui::pos2(line_x, lock_rect.top()),
                    ],
                    egui::Stroke::new(1.0, lock_color),
                );
                ui.painter().line_segment(
                    [
                        egui::pos2(line_x, lock_rect.bottom()),
                        egui::pos2(line_x, lock_rect.bottom() + 9.0),
                    ],
                    egui::Stroke::new(1.0, lock_color),
                );
                ui.painter().rect_filled(
                    lock_rect,
                    4.0,
                    if layout.lock_ratio {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().widgets.inactive.bg_fill
                    },
                );
                crate::components::icons::paint(
                    ui,
                    "link",
                    lock_rect.shrink(4.0),
                    lock_color,
                    false,
                );
                lock_response.on_hover_text(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustLockRatio,
                ));
                ui.end_row();
                ui.label(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustHeight,
                ));
                let height_max = if layout.lock_ratio {
                    (3.0 * initial_ratio).clamp(0.5, 3.0)
                } else {
                    3.0
                };
                let mut height = adjustment_value(slot.height, layout.size_unit, base.y);
                height_changed = ui
                    .add(adjustment_drag_value(
                        &mut height,
                        layout.size_unit,
                        base.y * height_max,
                        height_max,
                    ))
                    .changed();
                if height_changed {
                    slot.height = (height / adjustment_reference(layout.size_unit, base.y))
                        .clamp(0.5, height_max);
                }
                changed |= height_changed;
                changed |= adjustment_unit(
                    ui,
                    "hud-adjust-size-height",
                    &mut layout.size_unit,
                    prefs.locale,
                );
                ui.label("");
                ui.end_row();
                ui.label(
                    egui::RichText::new(deskhud_ui::i18n::t(
                        prefs.locale,
                        MessageKey::HudAdjustEffects,
                    ))
                    .strong(),
                );
                ui.label("");
                ui.label("");
                ui.label("");
                ui.end_row();
                for (name, message, default) in [
                    (
                        "background_opacity",
                        MessageKey::HudAdjustBackgroundOpacity,
                        item.background_opacity,
                    ),
                    (
                        "background_blur",
                        MessageKey::HudAdjustBackgroundBlur,
                        item.background_blur,
                    ),
                    (
                        "content_opacity",
                        MessageKey::HudAdjustContentOpacity,
                        item.content_opacity,
                    ),
                ] {
                    ui.label(deskhud_ui::i18n::t(prefs.locale, message));
                    let mut value = prefs
                        .hud
                        .visual_value(&plugin, &contribution, name, default);
                    if ui.add(egui::Slider::new(&mut value, 0.0..=1.0)).changed() {
                        prefs
                            .hud
                            .set_visual_value(&plugin, &contribution, name, value);
                        changed = true;
                    }
                    ui.end_row();
                }
            });
    });
    layout.adjust_open = open;
    if layout.lock_ratio && !initial_lock_ratio {
        layout.locked_ratio = Some(initial_height / initial_width.max(0.001));
    } else if !layout.lock_ratio {
        layout.locked_ratio = None;
    }
    if layout.lock_ratio {
        let ratio = layout.locked_ratio.unwrap_or(initial_ratio).max(0.001);
        if width_changed && !height_changed {
            slot.height = (slot.width * ratio).clamp(0.5, 3.0);
            slot.width = (slot.height / ratio).clamp(0.5, 3.0);
            changed = true;
        } else if height_changed && !width_changed {
            slot.width = (slot.height / ratio).clamp(0.5, 3.0);
            slot.height = (slot.width * ratio).clamp(0.5, 3.0);
            changed = true;
        }
    }
    if changed {
        if (slot.width - initial_width).abs() > 0.0001
            || (slot.height - initial_height).abs() > 0.0001
        {
            layout.window_revision = layout.window_revision.wrapping_add(1);
        }
        let (slot_x, slot_y) = (slot.x, slot.y);
        prefs.hud.set_slot_layout(&plugin, &contribution, slot);
        if let Some(pos) = layout.positions.get_mut(&key)
            && let Some(activity) = layout.activity_size
        {
            pos.x = slot_x * activity.x;
            pos.y = slot_y * activity.y;
        }
    }
    changed
}

fn adjustment_reference(unit: AdjustmentUnit, pixels: f32) -> f32 {
    match unit {
        AdjustmentUnit::Percent => 100.0,
        AdjustmentUnit::Pixels => pixels.max(1.0),
    }
}

fn adjustment_value(value: f32, unit: AdjustmentUnit, pixels: f32) -> f32 {
    value * adjustment_reference(unit, pixels)
}

fn adjustment_drag_value(
    value: &mut f32,
    unit: AdjustmentUnit,
    max_pixels: f32,
    max_factor: f32,
) -> egui::DragValue<'_> {
    let max = match unit {
        AdjustmentUnit::Percent => max_factor * 100.0,
        AdjustmentUnit::Pixels => max_pixels.max(1.0),
    };
    egui::DragValue::new(value)
        .speed(if unit == AdjustmentUnit::Percent {
            0.1
        } else {
            1.0
        })
        .range(0.0..=max)
        .suffix(match unit {
            AdjustmentUnit::Percent => "%",
            AdjustmentUnit::Pixels => " px",
        })
}

fn adjustment_unit(
    ui: &mut egui::Ui,
    id: &'static str,
    unit: &mut AdjustmentUnit,
    locale: deskhud_ui::Locale,
) -> bool {
    let before = *unit;
    egui::ComboBox::from_id_salt(id)
        .selected_text(match before {
            AdjustmentUnit::Percent => deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPercent),
            AdjustmentUnit::Pixels => deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPixels),
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                unit,
                AdjustmentUnit::Percent,
                deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPercent),
            );
            ui.selectable_value(
                unit,
                AdjustmentUnit::Pixels,
                deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPixels),
            );
        });
    before != *unit
}

fn draw_frame(
    ui: &mut egui::Ui,
    item: &HudRenderItem,
    layout_mode: bool,
    selected: bool,
) -> FrameResponse {
    let base_size = frame_size(&item.frame.visuals);
    let available = ui.available_size_before_wrap();
    let size = if layout_mode
        && available.x.is_finite()
        && available.y.is_finite()
        && available.x > 1.0
        && available.y > 1.0
    {
        available
    } else {
        egui::vec2(
            base_size.x * item.width.clamp(0.5, 3.0),
            base_size.y * item.height.clamp(0.5, 3.0),
        )
    };
    let scale = (size.x / base_size.x.max(1.0))
        .min(size.y / base_size.y.max(1.0))
        .clamp(0.5, 3.0);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if layout_mode {
            egui::Sense::click_and_drag()
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
                let panel = if layout_mode {
                    // A resizable egui Window defines the editable HUD size;
                    // the panel must follow its client rect exactly.
                    rect
                } else {
                    egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(width.max(1.0) * item.width, height.max(1.0) * item.height),
                    )
                };
                // egui has no portable backdrop-filter. A small stack of
                // translucent expanded panels gives the same readable soft
                // edge on every renderer while keeping the value persistent.
                let blur = item.background_blur.clamp(0.0, 1.0) * 10.0;
                if blur > 0.0 {
                    for step in 1..=3 {
                        let spread = blur * step as f32 / 3.0;
                        ui.painter().rect_filled(
                            panel.expand(spread),
                            (*radius * scale + spread).round().clamp(0.0, 255.0) as u8,
                            rgba_with_alpha(*color, item.background_opacity * (0.12 / step as f32)),
                        );
                    }
                }
                ui.painter().rect_filled(
                    panel,
                    (*radius * scale).round().clamp(0.0, 255.0) as u8,
                    rgba_with_alpha(*color, item.background_opacity),
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
                    rgba_with_alpha(*color, item.content_opacity),
                );
            }
        }
    }
    if layout_mode {
        // Paint the editor border after the visual so it cannot be covered by
        // a full-size panel background.
        ui.painter().rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().widgets.noninteractive.fg_stroke.color
                },
            ),
            egui::StrokeKind::Outside,
        );
    }
    if selected {
        ui.painter().rect_stroke(
            rect.expand(4.0),
            6.0,
            egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
            egui::StrokeKind::Outside,
        );
    }
    FrameResponse {
        body: response,
        size,
    }
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

fn rgba_with_alpha([red, green, blue, alpha]: [u8; 4], opacity: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        red,
        green,
        blue,
        ((alpha as f32) * opacity.clamp(0.0, 1.0)) as u8,
    )
}

fn snap_normalized(value: f32) -> f32 {
    (value / GRID_STEP).round() * GRID_STEP
}

fn snap_coordinate(value: f32, activity: f32) -> f32 {
    snap_normalized(value / activity.max(1.0)) * activity
}

fn draw_alignment_grid(ui: &egui::Ui, activity: egui::Vec2) {
    let color = with_alpha(ui.visuals().widgets.noninteractive.fg_stroke.color, 48);
    let stroke = egui::Stroke::new(1.0, color);
    let divisions = (1.0 / GRID_STEP) as i32;
    for index in 1..divisions {
        let x = activity.x * index as f32 / divisions as f32;
        let y = activity.y * index as f32 / divisions as f32;
        ui.painter()
            .line_segment([egui::pos2(x, 0.0), egui::pos2(x, activity.y)], stroke);
        ui.painter()
            .line_segment([egui::pos2(0.0, y), egui::pos2(activity.x, y)], stroke);
    }
}

fn draw_snap_grid_control(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    locale: deskhud_ui::Locale,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(48.0, 24.0), egui::Sense::click());
    if response.clicked() {
        layout.snap_to_grid = !layout.snap_to_grid;
    }
    let color = if layout.snap_to_grid {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    draw_bracket_icon(ui, rect, "grid", color, layout.snap_to_grid);
    let clicked = response.clicked();
    response.on_hover_text(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustSnapGrid));
    clicked
}

fn draw_ratio_lock_control(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    locale: deskhud_ui::Locale,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(48.0, 24.0), egui::Sense::click());
    if response.clicked() {
        layout.lock_ratio = !layout.lock_ratio;
    }
    let color = if layout.lock_ratio {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    draw_bracket_icon(ui, rect, "link", color, layout.lock_ratio);
    let clicked = response.clicked();
    response.on_hover_text(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustLockRatio));
    clicked
}

fn draw_bracket_icon(
    ui: &egui::Ui,
    rect: egui::Rect,
    icon: &'static str,
    color: egui::Color32,
    active: bool,
) {
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 12.0, rect.center().y),
        egui::vec2(24.0, 24.0),
    );
    ui.painter().rect_filled(
        icon_rect,
        4.0,
        if active {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        },
    );
    crate::components::icons::paint(ui, icon, icon_rect.shrink(4.0), color, false);

    // A bracket-shaped connector sits to the left of the icon, matching the
    // width/height relationship shown by the layout editor reference.
    let bracket_x = icon_rect.left() - 5.0;
    let top = icon_rect.top() + 3.0;
    let bottom = icon_rect.bottom() - 3.0;
    let stroke = egui::Stroke::new(1.0, color);
    ui.painter().line_segment(
        [egui::pos2(rect.left(), top), egui::pos2(bracket_x, top)],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(bracket_x, top), egui::pos2(bracket_x, bottom)],
        stroke,
    );
    ui.painter().line_segment(
        [
            egui::pos2(bracket_x, bottom),
            egui::pos2(rect.left(), bottom),
        ],
        stroke,
    );
}

fn layout_slot(
    prefs: &UiPreferences,
    key: &str,
) -> Option<(String, String, deskhud_ui::HudSlotLayout)> {
    let (plugin, contribution) = key.split_once('/')?;
    Some((
        plugin.to_owned(),
        contribution.to_owned(),
        prefs.hud.slot_layout(plugin, contribution, 0),
    ))
}

fn sync_layouts(
    prefs: &mut UiPreferences,
    layout: &LayoutState,
    items: &[HudRenderItem],
    activity: egui::Vec2,
) -> bool {
    if !layout.layout_mode || activity.x <= 0.0 || activity.y <= 0.0 {
        return false;
    }
    let mut changed = false;
    for item in items {
        if let Some(pos) = layout.positions.get(&item.key)
            && let Some((plugin, contribution, mut slot)) = layout_slot(prefs, &item.key)
        {
            let x = (pos.x / activity.x).clamp(0.0, 1.0);
            let y = (pos.y / activity.y).clamp(0.0, 1.0);
            let x = if layout.snap_to_grid {
                snap_normalized(x)
            } else {
                x
            };
            let y = if layout.snap_to_grid {
                snap_normalized(y)
            } else {
                y
            };
            if (slot.x - x).abs() > 0.0001 || (slot.y - y).abs() > 0.0001 {
                slot.x = x;
                slot.y = y;
                prefs.hud.set_slot_layout(&plugin, &contribution, slot);
                changed = true;
            }
        }
    }
    changed
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
