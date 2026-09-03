//! HUD 子窗口、布局边界和虚线动画绘制职责。

use super::{AdjustmentUnit, HudLayoutTarget, HudRenderItem, LayoutState, ShadowTarget};
use deskhud_engine::HudVisual;
use deskhud_ui::{HUD_SIZE_FACTOR_MAX, HUD_SIZE_FACTOR_MIN, MessageKey, UiPreferences};

const HUD_PADDING: f32 = 8.0;
const GRID_STEP: f32 = 0.05;
const ADJUST_PANEL_WIDTH: f32 = 440.0;
const ADJUST_ROW_HEIGHT: f32 = 32.0;
const ADJUST_ROW_GAP: f32 = 8.0;
const ADJUST_LABEL_INDENT: f32 = 36.0;
const ADJUST_LABEL_WIDTH: f32 = 108.0;
const ADJUST_VALUE_WIDTH: f32 = 82.0;
const RESIZE_EDGE_GRAB: f32 = 7.0;
const RESIZE_CORNER_GRAB: f32 = 14.0;
const HUD_BORDER_WIDTH_MAX: f32 = 6.0;
// Allow the radius to reach half of a HUD's short side even at 300% scale,
// so the maximum value can produce a true capsule rather than a rounded
// rectangle capped at the old 32 px threshold.
const HUD_CORNER_RADIUS_MAX: f32 = 160.0;

pub(super) struct DrawResult {
    pub(super) size: [f32; 2],
    pub(super) move_by: Option<[f32; 2]>,
    pub(super) changed: bool,
}

struct FrameResponse {
    body: egui::Response,
    size: egui::Vec2,
    resize_drag: Option<ResizeDrag>,
    resize_started: bool,
}

#[derive(Clone, Copy)]
struct ResizeEdges {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

struct ResizeDrag {
    edges: ResizeEdges,
    delta: egui::Vec2,
}

struct EditorOverlay {
    key: String,
    rect: egui::Rect,
    layer_id: egui::LayerId,
    corner_radius: f32,
}

struct ShadowControlRow<'a> {
    plugin: &'a str,
    contribution: &'a str,
    global: bool,
    master: bool,
    target: ShadowTarget,
    preview: Option<(f32, f32, f32, f32, [u8; 3])>,
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
    let mut editor_overlays = Vec::new();
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
        let base_size = egui::vec2(item.base_size.width, item.base_size.height);
        let preferred_size = egui::vec2(
            base_size.x * item.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
            base_size.y * item.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
        );
        let mut window = egui::Window::new(egui::RichText::new(&item.key).small())
            .id(egui::Id::new((
                "hud-item",
                &item.key,
                layout.adjust_session,
                layout.window_revision,
            )))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .movable(false)
            // The HUD visual owns the background. The egui window frame must
            // stay transparent, otherwise its padding/background becomes a
            // second rectangle around the actual HUD panel.
            .frame(egui::Frame::NONE)
            .fixed_pos(*position);
        if layout.layout_mode {
            // DeskHud owns all four resize edges so left/top resizing can
            // update both the slot position and size. egui disables those
            // edges for fixed, immovable windows.
            window = window.fixed_size(preferred_size);
        } else {
            // Layout mode may leave a remembered large Window rectangle in
            // egui memory. Compact HUDs must size themselves to their content
            // again when the editor is closed.
            window = window.auto_sized();
        }
        let response = window.show(ui.ctx(), |ui| draw_frame(ui, item, layout.layout_mode));
        let Some(response) = response else { continue };
        let Some(frame) = response.inner else {
            continue;
        };
        if layout.layout_mode
            && (frame.body.clicked() || frame.body.drag_started() || frame.resize_started)
        {
            layout.selected = Some(item.key.clone());
        }
        if layout.layout_mode && frame.body.secondary_clicked() {
            layout.selected = Some(item.key.clone());
            layout.adjust_open = true;
        }
        if layout.layout_mode && frame.resize_drag.is_none() && frame.body.dragged() {
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
            editor_overlays.push(EditorOverlay {
                key: item.key.clone(),
                rect: frame.body.rect,
                layer_id: frame.body.layer_id,
                corner_radius: item.corner_radius,
            });
            if let Some(resize) = frame.resize_drag
                && let Some(mut slot) = layout_slot(prefs, item)
            {
                let old_size = frame.size;
                let min_size = base_size * HUD_SIZE_FACTOR_MIN;
                let max_size = base_size * HUD_SIZE_FACTOR_MAX;
                let mut next_size = old_size;
                if resize.edges.left {
                    next_size.x = (old_size.x - resize.delta.x).clamp(min_size.x, max_size.x);
                    position.x += old_size.x - next_size.x;
                } else if resize.edges.right {
                    next_size.x = (old_size.x + resize.delta.x).clamp(min_size.x, max_size.x);
                }
                if resize.edges.top {
                    next_size.y = (old_size.y - resize.delta.y).clamp(min_size.y, max_size.y);
                    position.y += old_size.y - next_size.y;
                } else if resize.edges.bottom {
                    next_size.y = (old_size.y + resize.delta.y).clamp(min_size.y, max_size.y);
                }
                slot.width = (next_size.x / base_size.x.max(1.0))
                    .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
                slot.height = (next_size.y / base_size.y.max(1.0))
                    .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
                set_layout_slot(prefs, item, slot);
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

    if layout.layout_mode {
        draw_editor_overlays(ui, time, &editor_overlays, layout.selected.as_deref());
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
    let Some(mut slot) = layout_slot(prefs, item) else {
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
    let max_panel_height = layout
        .activity_size
        .map(|size| (size.y - 64.0).max(360.0))
        .unwrap_or(720.0);

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
            .map(|size| egui::pos2((size.x - ADJUST_PANEL_WIDTH - 24.0).max(24.0), 32.0))
            .unwrap_or(egui::pos2(24.0, 32.0)),
    )
    .default_width(ADJUST_PANEL_WIDTH)
    .default_height(max_panel_height.min(720.0))
    .min_width(ADJUST_PANEL_WIDTH)
    .max_width(ADJUST_PANEL_WIDTH)
    .min_height(320.0)
    .max_height(max_panel_height)
    .resizable([false, true])
    .collapsible(false)
    .open(&mut open)
    .show(ui.ctx(), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                changed |= draw_position_group(ui, layout, prefs, &mut slot);
                ui.add_space(8.0);
                let (size_changed, width_was_changed, height_was_changed) =
                    draw_size_group(ui, layout, prefs, &mut slot, item, initial_ratio);
                changed |= size_changed;
                width_changed = width_was_changed;
                height_changed = height_was_changed;
                ui.add_space(8.0);
                if let Some(source) = &item.source {
                    changed |= draw_effects_group(
                        ui,
                        layout,
                        prefs,
                        &source.plugin_id,
                        &source.contribution_id,
                        item,
                    );
                }
            });
    });
    if layout.shadow_open {
        changed |= draw_shadow_window(
            ui,
            layout,
            prefs,
            item,
            item.source
                .as_ref()
                .map(|source| source.plugin_id.as_str())
                .unwrap_or("hud.deskhud.group"),
            item.source
                .as_ref()
                .map(|source| source.contribution_id.as_str())
                .unwrap_or("group"),
            layout.shadow_target.unwrap_or(ShadowTarget::Global),
        );
    }
    layout.adjust_open = open;
    if layout.lock_ratio && !initial_lock_ratio {
        layout.locked_ratio = Some(initial_height / initial_width.max(0.001));
    } else if !layout.lock_ratio {
        layout.locked_ratio = None;
    }
    if layout.lock_ratio {
        let ratio = layout.locked_ratio.unwrap_or(initial_ratio).max(0.001);
        if width_changed && !height_changed {
            slot.height = (slot.width * ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            slot.width = (slot.height / ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            changed = true;
        } else if height_changed && !width_changed {
            slot.width = (slot.height / ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
            slot.height = (slot.width * ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
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
        set_layout_slot(prefs, item, slot);
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
    egui::Frame::group(ui.style())
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ADJUST_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustPosition,
                        ))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        changed |= draw_snap_grid_control(ui, layout, prefs.locale);
                        changed |= adjustment_unit(
                            ui,
                            "hud-position-unit",
                            &mut layout.position_unit,
                            prefs.locale,
                        );
                    });
                },
            );
            ui.separator();
            ui.add_space(4.0);
            let activity = layout.activity_size.unwrap_or(egui::vec2(1.0, 1.0));
            for (label, value, pixels) in [
                (MessageKey::HudAdjustX, &mut slot.x, activity.x),
                (MessageKey::HudAdjustY, &mut slot.y, activity.y),
            ] {
                let (label_rect, input_rect) = allocate_adjustment_input_row(ui);
                draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, label));
                let mut shown = adjustment_value(*value, layout.position_unit, pixels);
                if ui
                    .put(
                        input_rect,
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
                ui.add_space(ADJUST_ROW_GAP);
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
    let base = egui::vec2(item.base_size.width, item.base_size.height);
    egui::Frame::group(ui.style())
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ADJUST_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(
                        egui::RichText::new(deskhud_ui::i18n::t(
                            prefs.locale,
                            MessageKey::HudAdjustSize,
                        ))
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        changed |= draw_ratio_lock_control(ui, layout, prefs.locale);
                        changed |= adjustment_unit(
                            ui,
                            "hud-size-unit",
                            &mut layout.size_unit,
                            prefs.locale,
                        );
                    });
                },
            );
            ui.separator();
            ui.add_space(4.0);
            let width_max = if layout.lock_ratio {
                (HUD_SIZE_FACTOR_MAX / ratio.max(0.001))
                    .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX)
            } else {
                HUD_SIZE_FACTOR_MAX
            };
            {
                let (label_rect, input_rect) = allocate_adjustment_input_row(ui);
                draw_effect_label(
                    ui,
                    label_rect,
                    deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustWidth),
                );
                let mut shown = adjustment_value(slot.width, layout.size_unit, base.x);
                width_changed = ui
                    .put(
                        input_rect,
                        size_adjustment_drag_value(&mut shown, layout.size_unit, base.x, width_max),
                    )
                    .changed();
                if width_changed {
                    slot.width = (shown / adjustment_reference(layout.size_unit, base.x))
                        .clamp(HUD_SIZE_FACTOR_MIN, width_max);
                }
                changed |= width_changed;
                ui.add_space(ADJUST_ROW_GAP);
            }
            let height_max = if layout.lock_ratio {
                (HUD_SIZE_FACTOR_MAX * ratio).clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX)
            } else {
                HUD_SIZE_FACTOR_MAX
            };
            {
                let (label_rect, input_rect) = allocate_adjustment_input_row(ui);
                draw_effect_label(
                    ui,
                    label_rect,
                    deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustHeight),
                );
                let mut shown = adjustment_value(slot.height, layout.size_unit, base.y);
                height_changed = ui
                    .put(
                        input_rect,
                        size_adjustment_drag_value(
                            &mut shown,
                            layout.size_unit,
                            base.y,
                            height_max,
                        ),
                    )
                    .changed();
                if height_changed {
                    slot.height = (shown / adjustment_reference(layout.size_unit, base.y))
                        .clamp(HUD_SIZE_FACTOR_MIN, height_max);
                }
                changed |= height_changed;
                ui.add_space(ADJUST_ROW_GAP);
            }
        });
    (changed, width_changed, height_changed)
}

fn draw_effects_group(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    plugin: &str,
    contribution: &str,
    item: &HudRenderItem,
) -> bool {
    let mut changed = false;
    egui::Frame::group(ui.style())
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
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

            draw_effect_section_title(ui, prefs.locale, MessageKey::HudAdjustGlobalEffects, false);
            let (shadow_toggle_changed, shadow_target) = draw_shadow_control_row(
                ui,
                prefs,
                ShadowControlRow {
                    plugin,
                    contribution,
                    global: item.shadow_enabled,
                    master: true,
                    target: ShadowTarget::Global,
                    preview: Some((
                        item.window_shadow,
                        item.window_shadow_blur,
                        item.window_shadow_distance,
                        item.window_shadow_angle,
                        item.window_shadow_color,
                    )),
                },
            );
            changed |= shadow_toggle_changed;
            if let Some(shadow_target) = shadow_target {
                layout.shadow_open = true;
                layout.shadow_target = Some(shadow_target);
            }

            draw_effect_section_title(ui, prefs.locale, MessageKey::HudAdjustWindowEffects, false);
            changed |= draw_effect_slider_row(
                ui,
                prefs,
                plugin,
                contribution,
                "corner_radius",
                MessageKey::HudAdjustCornerRadius,
                item.corner_radius,
                HUD_CORNER_RADIUS_MAX,
                " px",
            );
            let (mode_changed, mode_target) = draw_shadow_control_row(
                ui,
                prefs,
                ShadowControlRow {
                    plugin,
                    contribution,
                    global: item.window_shadow_global,
                    master: false,
                    target: ShadowTarget::Window,
                    preview: None,
                },
            );
            changed |= mode_changed;
            if let Some(mode_target) = mode_target {
                layout.shadow_open = true;
                layout.shadow_target = Some(mode_target);
            }

            draw_effect_section_title(ui, prefs.locale, MessageKey::HudAdjustContentEffects, true);
            changed |= draw_effect_color_row(
                ui,
                prefs,
                plugin,
                contribution,
                MessageKey::HudAdjustContentColor,
                item.content_color,
                ["content_red", "content_green", "content_blue"],
            );
            changed |= draw_effect_slider_row(
                ui,
                prefs,
                plugin,
                contribution,
                "content_opacity",
                MessageKey::HudAdjustContentOpacity,
                item.content_opacity,
                1.0,
                "",
            );
            let (mode_changed, mode_target) = draw_shadow_control_row(
                ui,
                prefs,
                ShadowControlRow {
                    plugin,
                    contribution,
                    global: item.content_shadow_global,
                    master: false,
                    target: ShadowTarget::Content,
                    preview: None,
                },
            );
            changed |= mode_changed;
            if let Some(mode_target) = mode_target {
                layout.shadow_open = true;
                layout.shadow_target = Some(mode_target);
            }

            let (toggle_changed, border_enabled) = draw_effect_toggle_section_title(
                ui,
                prefs,
                plugin,
                contribution,
                MessageKey::HudAdjustBorderEffects,
                "border_enabled",
                item.border_enabled,
                true,
            );
            changed |= toggle_changed;
            ui.add_enabled_ui(border_enabled, |ui| {
                changed |= draw_effect_slider_row(
                    ui,
                    prefs,
                    plugin,
                    contribution,
                    "border_width",
                    MessageKey::HudAdjustBorderWidth,
                    item.border_width,
                    HUD_BORDER_WIDTH_MAX,
                    " px",
                );
                changed |= draw_effect_color_row(
                    ui,
                    prefs,
                    plugin,
                    contribution,
                    MessageKey::HudAdjustBorderColor,
                    item.border_color,
                    ["border_red", "border_green", "border_blue"],
                );
                changed |= draw_effect_slider_row(
                    ui,
                    prefs,
                    plugin,
                    contribution,
                    "border_opacity",
                    MessageKey::HudAdjustBorderOpacity,
                    item.border_opacity,
                    1.0,
                    "",
                );
            });

            let (toggle_changed, background_enabled) = draw_effect_toggle_section_title(
                ui,
                prefs,
                plugin,
                contribution,
                MessageKey::HudAdjustBackgroundEffects,
                "background_enabled",
                item.background_enabled,
                true,
            );
            changed |= toggle_changed;
            ui.add_enabled_ui(background_enabled, |ui| {
                changed |= draw_effect_slider_row(
                    ui,
                    prefs,
                    plugin,
                    contribution,
                    "background_opacity",
                    MessageKey::HudAdjustBackgroundOpacity,
                    item.background_opacity,
                    1.0,
                    "",
                );
                changed |= draw_effect_slider_row(
                    ui,
                    prefs,
                    plugin,
                    contribution,
                    "background_blur",
                    MessageKey::HudAdjustBackgroundBlur,
                    item.background_blur,
                    1.0,
                    "",
                );
            });
        });
    changed
}

fn draw_effect_section_title(
    ui: &mut egui::Ui,
    locale: deskhud_ui::Locale,
    key: MessageKey,
    separated: bool,
) {
    if separated {
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
    }
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(deskhud_ui::i18n::t(locale, key))
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
    });
    ui.add_space(2.0);
}

fn draw_shadow_control_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    row: ShadowControlRow<'_>,
) -> (bool, Option<ShadowTarget>) {
    let ShadowControlRow {
        plugin,
        contribution,
        global,
        master,
        target,
        preview,
    } = row;
    let label_key = MessageKey::HudAdjustWindowShadow;
    let (mode_name, enable_name) = match target {
        ShadowTarget::Window => ("window_shadow_mode", "window_shadow_enabled"),
        ShadowTarget::Content => ("content_shadow_mode", "content_shadow_enabled"),
        ShadowTarget::Global => ("shadow_enabled", "shadow_enabled"),
    };
    let mut mode_global = global;
    let mut enabled = if master {
        global
    } else {
        prefs
            .hud
            .visual_value(plugin, contribution, enable_name, 1.0)
            >= 0.5
    };
    let before_mode_global = mode_global;
    let before_enabled = enabled;
    let right_width = if master { 42.0 } else { 82.0 };
    let row_width = ui.available_width();
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(row_width, ADJUST_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let label_rect = egui::Rect::from_min_size(
        row_rect.min + egui::vec2(ADJUST_LABEL_INDENT, 0.0),
        egui::vec2(ADJUST_LABEL_WIDTH - ADJUST_LABEL_INDENT, ADJUST_ROW_HEIGHT),
    );
    let value_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - right_width, row_rect.top()),
        egui::vec2(right_width, ADJUST_ROW_HEIGHT),
    );
    let control_rect = egui::Rect::from_min_max(
        egui::pos2(
            label_rect.right() + ui.spacing().item_spacing.x,
            row_rect.top(),
        ),
        egui::pos2(
            value_rect.left() - ui.spacing().item_spacing.x,
            row_rect.bottom(),
        ),
    );
    draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, label_key));
    let clicked = if master {
        let response = ui
            .interact(
                control_rect,
                ui.make_persistent_id(("hud-shadow-preview", plugin, contribution)),
                egui::Sense::click(),
            )
            .on_hover_text(deskhud_ui::i18n::t(
                prefs.locale,
                MessageKey::HudAdjustShadowSettings,
            ));
        if let Some((opacity, blur, distance, angle, color)) = preview {
            draw_shadow_preview_inline(ui, control_rect, opacity, blur, distance, angle, color);
        }
        if response.clicked() {
            Some(ShadowTarget::Global)
        } else {
            None
        }
    } else {
        let split = control_rect.width() * 0.5;
        let global_rect = egui::Rect::from_min_max(
            control_rect.min,
            egui::pos2(control_rect.left() + split, control_rect.bottom()),
        );
        let custom_rect = egui::Rect::from_min_max(
            egui::pos2(
                control_rect.left() + split + ui.spacing().item_spacing.x,
                control_rect.top(),
            ),
            control_rect.max,
        );
        let global_clicked = ui
            .put(
                global_rect,
                egui::Button::new(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustShadowGlobal,
                ))
                .selected(mode_global),
            )
            .clicked();
        let custom_clicked = ui
            .put(
                custom_rect,
                egui::Button::new(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustShadowCustom,
                ))
                .selected(!mode_global),
            )
            .clicked();
        if global_clicked {
            mode_global = true;
        } else if custom_clicked {
            mode_global = false;
        }
        if global_clicked {
            Some(ShadowTarget::Global)
        } else if custom_clicked {
            Some(target)
        } else {
            None
        }
    };
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(value_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    if !master {
        right_ui.label(deskhud_ui::i18n::t(
            prefs.locale,
            MessageKey::HudAdjustShadowGlobal,
        ));
    }
    let switch_rect = egui::Rect::from_min_size(
        egui::pos2(value_rect.right() - 42.0, value_rect.top() + 4.0),
        egui::vec2(42.0, 24.0),
    );
    crate::components::toggle_switch_with_id(
        &mut right_ui,
        switch_rect,
        &mut enabled,
        ("shadow", plugin, contribution, enable_name),
    );
    let mode_changed = !master && before_mode_global != mode_global;
    let enable_changed = before_enabled != enabled;
    if mode_changed {
        prefs.hud.set_visual_value(
            plugin,
            contribution,
            mode_name,
            if mode_global { 0.0 } else { 1.0 },
        );
    }
    if enable_changed {
        prefs.hud.set_visual_value(
            plugin,
            contribution,
            enable_name,
            if enabled { 1.0 } else { 0.0 },
        );
    }
    ui.add_space(ADJUST_ROW_GAP);
    (mode_changed || enable_changed, clicked)
}

fn draw_shadow_preview_inline(
    ui: &egui::Ui,
    rect: egui::Rect,
    opacity: f32,
    blur: f32,
    distance: f32,
    angle: f32,
    color: [u8; 3],
) {
    let painter = ui.painter();
    // Keep the preview's visible height aligned with position/size inputs;
    // only inset horizontally so the preview still has breathing room beside
    // the label and switch.
    let panel = rect.shrink2(egui::vec2(4.0, 0.0));
    if opacity > f32::EPSILON {
        paint_window_shadow(painter, panel, 6.0, opacity, blur, distance, angle, color);
    }
    painter.rect_filled(panel, 6.0, egui::Color32::from_rgb(48, 52, 62));
    painter.text(
        panel.center(),
        egui::Align2::CENTER_CENTER,
        "Aa",
        egui::FontId::proportional(16.0),
        egui::Color32::from_rgb(245, 247, 250),
    );
}

/*
fn draw_shadow_mode_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    plugin: &str,
    contribution: &str,
    global: bool,
    target: ShadowTarget,
) -> (bool, bool) {
    let mode_name = match target {
        ShadowTarget::Window => "window_shadow_mode",
        ShadowTarget::Content => "content_shadow_mode",
        ShadowTarget::Global => return (false, false),
    };
    let mut use_global = global;
    let mut clicked_custom = false;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(ADJUST_LABEL_WIDTH, ADJUST_ROW_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(deskhud_ui::i18n::t(
                    prefs.locale,
                    MessageKey::HudAdjustShadowMode,
                ));
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .selectable_label(
                    !use_global,
                    deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustShadowCustom),
                )
                .clicked()
            {
                use_global = false;
                clicked_custom = true;
            }
            if ui
                .selectable_label(
                    use_global,
                    deskhud_ui::i18n::t(prefs.locale, MessageKey::HudAdjustShadowGlobal),
                )
                .clicked()
            {
                use_global = true;
            }
        });
    });
    let changed = use_global != global;
    if changed {
        prefs.hud.set_visual_value(
            plugin,
            contribution,
            mode_name,
            if use_global { 0.0 } else { 1.0 },
        );
    }
    (changed, clicked_custom)
}
*/

fn draw_shadow_window(
    ui: &egui::Ui,
    layout: &mut LayoutState,
    prefs: &mut UiPreferences,
    item: &HudRenderItem,
    plugin: &str,
    contribution: &str,
    target: ShadowTarget,
) -> bool {
    let mut open = layout.shadow_open;
    let mut changed = false;
    let position = layout
        .activity_size
        .map(|size| egui::pos2((size.x - ADJUST_PANEL_WIDTH - 336.0).max(24.0), 32.0))
        .unwrap_or(egui::pos2(24.0, 32.0));
    let (
        title_key,
        opacity_name,
        blur_name,
        distance_name,
        angle_name,
        color_names,
        opacity,
        blur,
        distance,
        angle,
        color,
    ) = match target {
        ShadowTarget::Global => (
            MessageKey::HudAdjustGlobalShadow,
            "shadow_opacity",
            "shadow_blur",
            "shadow_distance",
            "shadow_angle",
            ["shadow_red", "shadow_green", "shadow_blue"],
            item.window_shadow,
            item.window_shadow_blur,
            item.window_shadow_distance,
            item.window_shadow_angle,
            item.window_shadow_color,
        ),
        ShadowTarget::Window => (
            MessageKey::HudAdjustCustomShadow,
            "window_shadow",
            "window_shadow_blur",
            "window_shadow_distance",
            "window_shadow_angle",
            [
                "window_shadow_red",
                "window_shadow_green",
                "window_shadow_blue",
            ],
            item.window_custom_shadow,
            item.window_custom_shadow_blur,
            item.window_custom_shadow_distance,
            item.window_custom_shadow_angle,
            item.window_custom_shadow_color,
        ),
        ShadowTarget::Content => (
            MessageKey::HudAdjustCustomShadow,
            "content_shadow",
            "content_shadow_blur",
            "content_shadow_distance",
            "content_shadow_angle",
            [
                "content_shadow_red",
                "content_shadow_green",
                "content_shadow_blue",
            ],
            item.content_custom_shadow,
            item.content_custom_shadow_blur,
            item.content_custom_shadow_distance,
            item.content_custom_shadow_angle,
            item.content_custom_shadow_color,
        ),
    };
    egui::Window::new(egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, title_key)).strong())
        .id(egui::Id::new(("hud-shadow-window", layout.adjust_session)))
        .default_pos(position)
        .default_width(320.0)
        .min_width(320.0)
        .max_width(320.0)
        .resizable(false)
        .collapsible(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    changed |= draw_effect_color_row(
                        ui,
                        prefs,
                        plugin,
                        contribution,
                        MessageKey::HudAdjustShadowColor,
                        color,
                        color_names,
                    );
                    changed |= draw_effect_slider_row(
                        ui,
                        prefs,
                        plugin,
                        contribution,
                        opacity_name,
                        MessageKey::HudAdjustShadowOpacity,
                        opacity,
                        1.0,
                        "",
                    );
                    changed |= draw_effect_slider_row(
                        ui,
                        prefs,
                        plugin,
                        contribution,
                        blur_name,
                        MessageKey::HudAdjustShadowBlur,
                        blur,
                        24.0,
                        " px",
                    );
                    changed |= draw_effect_slider_row(
                        ui,
                        prefs,
                        plugin,
                        contribution,
                        distance_name,
                        MessageKey::HudAdjustShadowDistance,
                        distance,
                        12.0,
                        " px",
                    );
                    changed |= draw_effect_slider_row(
                        ui,
                        prefs,
                        plugin,
                        contribution,
                        angle_name,
                        MessageKey::HudAdjustShadowAngle,
                        angle,
                        360.0,
                        "°",
                    );
                });
        });
    layout.shadow_open = open;
    changed
}

#[allow(clippy::too_many_arguments)]
fn draw_effect_toggle_section_title(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    plugin: &str,
    contribution: &str,
    key: MessageKey,
    name: &str,
    default_enabled: bool,
    separated: bool,
) -> (bool, bool) {
    if separated {
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
    }
    let mut enabled = prefs.hud.visual_value(
        plugin,
        contribution,
        name,
        if default_enabled { 1.0 } else { 0.0 },
    ) >= 0.5;
    let before = enabled;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(deskhud_ui::i18n::t(prefs.locale, key))
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(42.0, 24.0), egui::Sense::hover());
            crate::components::toggle_switch_with_id(
                ui,
                rect,
                &mut enabled,
                ("effect-section", plugin, contribution, name),
            );
        });
    });
    if before != enabled {
        prefs
            .hud
            .set_visual_value(plugin, contribution, name, if enabled { 1.0 } else { 0.0 });
    }
    ui.add_space(2.0);
    (before != enabled, enabled)
}

#[allow(clippy::too_many_arguments)]
fn draw_effect_slider_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    plugin: &str,
    contribution: &str,
    name: &str,
    message: MessageKey,
    default: f32,
    display_scale: f32,
    suffix: &str,
) -> bool {
    let mut changed = false;
    let (label_rect, control_rect, value_rect) = allocate_effect_row(ui);
    draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, message));
    let mut value = prefs.hud.visual_value(plugin, contribution, name, default);
    let mut slider_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-slider", plugin, contribution, name))
            .max_rect(control_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    slider_ui.spacing_mut().slider_width = control_rect.width();
    let slider_changed = slider_ui
        .add(
            egui::Slider::new(&mut value, 0.0..=1.0)
                .step_by(0.01 / display_scale as f64)
                .handle_shape(egui::style::HandleShape::Circle)
                .show_value(false),
        )
        .changed();
    let mut shown = value * display_scale;
    let mut value_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-value", plugin, contribution, name))
            .max_rect(value_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    let input_changed = value_ui
        .add(
            egui::DragValue::new(&mut shown)
                .fixed_decimals(2)
                .range(0.0..=display_scale)
                .speed((display_scale / 100.0).max(0.01))
                .suffix(suffix),
        )
        .changed();
    if input_changed {
        value = (shown / display_scale.max(f32::EPSILON)).clamp(0.0, 1.0);
    }
    if slider_changed || input_changed {
        prefs
            .hud
            .set_visual_value(plugin, contribution, name, value);
        changed = true;
    }
    ui.add_space(ADJUST_ROW_GAP);
    changed
}

#[allow(clippy::too_many_arguments)]
fn draw_effect_color_row(
    ui: &mut egui::Ui,
    prefs: &mut UiPreferences,
    plugin: &str,
    contribution: &str,
    message: MessageKey,
    mut color: [u8; 3],
    names: [&str; 3],
) -> bool {
    let mut changed = false;
    let (label_rect, control_rect, value_rect) = allocate_effect_row(ui);
    draw_effect_label(ui, label_rect, deskhud_ui::i18n::t(prefs.locale, message));
    let mut color_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-color", plugin, contribution, names[0]))
            .max_rect(control_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    color_ui.spacing_mut().interact_size = control_rect.size();
    let color_input_id =
        ui.make_persistent_id(("hud-effect-color-text", plugin, contribution, names[0]));
    let picker_changed = color_ui.color_edit_button_srgb(&mut color).changed();
    if picker_changed {
        for (name, channel) in names.into_iter().zip(color) {
            prefs
                .hud
                .set_visual_value(plugin, contribution, name, channel as f32 / 255.0);
        }
        changed = true;
    }
    let canonical = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
    let mut input = ui.ctx().data_mut(|data| {
        data.get_temp::<String>(color_input_id)
            .unwrap_or_else(|| canonical.clone())
    });
    if picker_changed {
        input.clone_from(&canonical);
    }
    let mut value_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("hud-effect-color-value", plugin, contribution, names[0]))
            .max_rect(value_rect)
            .layout(egui::Layout::centered_and_justified(
                egui::Direction::TopDown,
            )),
    );
    let response = value_ui.add(
        egui::TextEdit::singleline(&mut input)
            .id(color_input_id.with("edit"))
            .font(egui::TextStyle::Monospace)
            .horizontal_align(egui::Align::Center)
            .desired_width(value_rect.width()),
    );
    if response.changed()
        && let Some(parsed) = parse_hex_color(&input)
    {
        color = parsed;
        for (name, channel) in names.into_iter().zip(color) {
            prefs
                .hud
                .set_visual_value(plugin, contribution, name, channel as f32 / 255.0);
        }
        changed = true;
    }
    if response.lost_focus() && parse_hex_color(&input).is_none() {
        input = canonical;
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(color_input_id, input));
    ui.add_space(ADJUST_ROW_GAP);
    changed
}

fn draw_effect_label(ui: &mut egui::Ui, rect: egui::Rect, text: &str) {
    let mut label_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    label_ui.label(text);
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

fn allocate_effect_row(ui: &mut egui::Ui) -> (egui::Rect, egui::Rect, egui::Rect) {
    let row_width = ui.available_width();
    let spacing = ui.spacing().item_spacing.x;
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(row_width, ADJUST_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let label_rect = egui::Rect::from_min_size(
        row_rect.min + egui::vec2(ADJUST_LABEL_INDENT, 0.0),
        egui::vec2(ADJUST_LABEL_WIDTH - ADJUST_LABEL_INDENT, ADJUST_ROW_HEIGHT),
    );
    let value_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.right() - ADJUST_VALUE_WIDTH, row_rect.top()),
        egui::vec2(ADJUST_VALUE_WIDTH, ADJUST_ROW_HEIGHT),
    );
    let control_rect = egui::Rect::from_min_max(
        egui::pos2(label_rect.right() + spacing, row_rect.top()),
        egui::pos2(value_rect.left() - spacing, row_rect.bottom()),
    );
    (label_rect, control_rect, value_rect)
}

fn allocate_adjustment_input_row(ui: &mut egui::Ui) -> (egui::Rect, egui::Rect) {
    let row_width = ui.available_width();
    let spacing = ui.spacing().item_spacing.x;
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(row_width, ADJUST_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let label_rect = egui::Rect::from_min_size(
        row_rect.min + egui::vec2(ADJUST_LABEL_INDENT, 0.0),
        egui::vec2(ADJUST_LABEL_WIDTH - ADJUST_LABEL_INDENT, ADJUST_ROW_HEIGHT),
    );
    let input_rect = egui::Rect::from_min_max(
        egui::pos2(label_rect.right() + spacing, row_rect.top()),
        row_rect.max,
    );
    (label_rect, input_rect)
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
        .fixed_decimals(2)
        .speed(if unit == AdjustmentUnit::Percent {
            0.01
        } else {
            0.1
        })
        .range(0.0..=max)
        .suffix(match unit {
            AdjustmentUnit::Percent => "%",
            AdjustmentUnit::Pixels => " px",
        })
}

fn size_adjustment_drag_value(
    value: &mut f32,
    unit: AdjustmentUnit,
    base_pixels: f32,
    max_factor: f32,
) -> egui::DragValue<'_> {
    let reference = adjustment_reference(unit, base_pixels);
    let min = HUD_SIZE_FACTOR_MIN * reference;
    let max = max_factor * reference;
    egui::DragValue::new(value)
        .fixed_decimals(2)
        .speed(if unit == AdjustmentUnit::Percent {
            0.01
        } else {
            0.1
        })
        .range(min..=max)
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
    ui.allocate_ui_with_layout(
        egui::vec2(84.0, 28.0),
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            egui::ComboBox::from_id_salt(id)
                .width(84.0)
                .selected_text(match before {
                    AdjustmentUnit::Percent => {
                        deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPercent)
                    }
                    AdjustmentUnit::Pixels => {
                        deskhud_ui::i18n::t(locale, MessageKey::HudAdjustPixels)
                    }
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
        },
    );
    before != *unit
}

fn draw_frame(ui: &mut egui::Ui, item: &HudRenderItem, layout_mode: bool) -> FrameResponse {
    let base_size = egui::vec2(item.base_size.width, item.base_size.height);
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
            base_size.x * item.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
            base_size.y * item.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX),
        )
    };
    let scale = (size.x / base_size.x.max(1.0))
        .min(size.y / base_size.y.max(1.0))
        .clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
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
    let window_radius = item.corner_radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX;
    let hud_painter = ui.ctx().layer_painter(response.layer_id);
    let (window_shadow, window_blur, window_distance, window_angle, window_color) =
        if item.window_shadow_global {
            (
                item.window_shadow,
                item.window_shadow_blur,
                item.window_shadow_distance,
                item.window_shadow_angle,
                item.window_shadow_color,
            )
        } else {
            (
                item.window_custom_shadow,
                item.window_custom_shadow_blur,
                item.window_custom_shadow_distance,
                item.window_custom_shadow_angle,
                item.window_custom_shadow_color,
            )
        };
    let (content_shadow, content_blur, content_distance, content_angle, content_color) =
        if item.content_shadow_global {
            (
                item.window_shadow,
                item.window_shadow_blur,
                item.window_shadow_distance,
                item.window_shadow_angle,
                item.window_shadow_color,
            )
        } else {
            (
                item.content_custom_shadow,
                item.content_custom_shadow_blur,
                item.content_custom_shadow_distance,
                item.content_custom_shadow_angle,
                item.content_custom_shadow_color,
            )
        };
    let window_shadow_enabled = if item.window_shadow_global {
        item.shadow_enabled && item.window_shadow_enabled
    } else {
        item.window_shadow_enabled && window_shadow > f32::EPSILON
    };
    let content_shadow_enabled = if item.content_shadow_global {
        item.shadow_enabled && item.content_shadow_enabled
    } else {
        item.content_shadow_enabled && content_shadow > f32::EPSILON
    };
    if window_shadow_enabled {
        paint_window_shadow(
            &hud_painter,
            rect,
            window_radius,
            window_shadow,
            window_blur,
            window_distance,
            window_angle,
            window_color,
        );
    }
    let scale_x = rect.width() / item.base_size.width.max(1.0);
    let scale_y = rect.height() / item.base_size.height.max(1.0);
    for layer in &item.layers {
        let child_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(layer.rect.x * scale_x, layer.rect.y * scale_y),
            egui::vec2(layer.rect.width * scale_x, layer.rect.height * scale_y),
        );
        let child_clip = egui::Rect::from_min_size(
            rect.min + egui::vec2(layer.clip.x * scale_x, layer.clip.y * scale_y),
            egui::vec2(layer.clip.width * scale_x, layer.clip.height * scale_y),
        )
        .intersect(rect);
        let child_painter = hud_painter.with_clip_rect(child_clip);
        for visual in &layer.frame.visuals {
            match visual {
                HudVisual::Panel {
                    width: _,
                    height: _,
                    radius: _,
                    color,
                } => {
                    if item.background_enabled {
                        paint_acrylic_background(
                            &child_painter,
                            child_rect,
                            window_radius,
                            *color,
                            item.background_opacity,
                            item.background_blur,
                        );
                    }
                }
                HudVisual::Text {
                    text,
                    font_size,
                    color,
                } => {
                    paint_hud_text(
                        &child_painter,
                        child_rect.center(),
                        text,
                        egui::FontId::proportional(
                            (font_size * scale * ui_font_scale).clamp(8.0, 96.0),
                        ),
                        item.content_color,
                        color[3],
                        item.content_opacity,
                        if content_shadow_enabled {
                            content_shadow
                        } else {
                            0.0
                        },
                        content_blur,
                        content_distance,
                        content_angle,
                        content_color,
                    );
                }
            }
        }
    }
    // The configured HUD border is part of the HUD visual itself and is
    // rendered identically in edit and runtime modes. Layout mode only adds
    // editor overlays after this frame has been painted.
    if item.border_enabled {
        paint_hud_border(
            &hud_painter,
            rect,
            item.border_opacity,
            item.border_width,
            item.corner_radius,
            egui::Color32::from_rgb(
                item.border_color[0],
                item.border_color[1],
                item.border_color[2],
            ),
        );
    }
    let (resize_drag, resize_started) = if layout_mode {
        hud_resize_interaction(ui, &item.key, rect)
    } else {
        (None, false)
    };
    FrameResponse {
        body: response,
        size,
        resize_drag,
        resize_started,
    }
}

fn hud_resize_interaction(
    ui: &mut egui::Ui,
    key: &str,
    rect: egui::Rect,
) -> (Option<ResizeDrag>, bool) {
    let edge = RESIZE_EDGE_GRAB
        .min(rect.width() * 0.25)
        .min(rect.height() * 0.25);
    let corner = RESIZE_CORNER_GRAB
        .min(rect.width() * 0.4)
        .min(rect.height() * 0.4);
    let sides = [
        (
            "left",
            egui::Rect::from_min_max(
                rect.left_top() + egui::vec2(0.0, corner),
                rect.left_bottom() + egui::vec2(edge, -corner),
            ),
            ResizeEdges {
                left: true,
                right: false,
                top: false,
                bottom: false,
            },
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            "right",
            egui::Rect::from_min_max(
                rect.right_top() + egui::vec2(-edge, corner),
                rect.right_bottom() + egui::vec2(0.0, -corner),
            ),
            ResizeEdges {
                left: false,
                right: true,
                top: false,
                bottom: false,
            },
            egui::CursorIcon::ResizeHorizontal,
        ),
        (
            "top",
            egui::Rect::from_min_max(
                rect.left_top() + egui::vec2(corner, 0.0),
                rect.right_top() + egui::vec2(-corner, edge),
            ),
            ResizeEdges {
                left: false,
                right: false,
                top: true,
                bottom: false,
            },
            egui::CursorIcon::ResizeVertical,
        ),
        (
            "bottom",
            egui::Rect::from_min_max(
                rect.left_bottom() + egui::vec2(corner, -edge),
                rect.right_bottom() + egui::vec2(-corner, 0.0),
            ),
            ResizeEdges {
                left: false,
                right: false,
                top: false,
                bottom: true,
            },
            egui::CursorIcon::ResizeVertical,
        ),
    ];
    let corners = [
        (
            "top-left",
            egui::Rect::from_min_size(rect.left_top(), egui::Vec2::splat(corner)),
            ResizeEdges {
                left: true,
                right: false,
                top: true,
                bottom: false,
            },
            egui::CursorIcon::ResizeNwSe,
        ),
        (
            "top-right",
            egui::Rect::from_min_max(
                rect.right_top() + egui::vec2(-corner, 0.0),
                rect.right_top() + egui::vec2(0.0, corner),
            ),
            ResizeEdges {
                left: false,
                right: true,
                top: true,
                bottom: false,
            },
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            "bottom-left",
            egui::Rect::from_min_max(
                rect.left_bottom() + egui::vec2(0.0, -corner),
                rect.left_bottom() + egui::vec2(corner, 0.0),
            ),
            ResizeEdges {
                left: true,
                right: false,
                top: false,
                bottom: true,
            },
            egui::CursorIcon::ResizeNeSw,
        ),
        (
            "bottom-right",
            egui::Rect::from_min_max(
                rect.right_bottom() - egui::Vec2::splat(corner),
                rect.right_bottom(),
            ),
            ResizeEdges {
                left: false,
                right: true,
                top: false,
                bottom: true,
            },
            egui::CursorIcon::ResizeNwSe,
        ),
    ];

    let mut drag = None;
    let mut started = false;
    for (name, hit_rect, edges, cursor) in sides.into_iter().chain(corners) {
        let response = ui
            .interact(
                hit_rect,
                ui.make_persistent_id(("hud-resize", key, name)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(cursor);
        started |= response.drag_started();
        if response.dragged() {
            drag = Some(ResizeDrag {
                edges,
                delta: response.drag_delta(),
            });
        }
    }
    (drag, started)
}

fn draw_editor_overlays(
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

#[allow(clippy::too_many_arguments)]
fn paint_window_shadow(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    opacity: f32,
    blur: f32,
    distance: f32,
    angle: f32,
    color: [u8; 3],
) {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity <= f32::EPSILON {
        return;
    }
    let blur = blur.clamp(0.0, 1.0);
    let distance = distance.clamp(0.0, 1.0) * 12.0;
    let angle = angle.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let offset = egui::vec2(angle.cos(), angle.sin()) * distance;
    for step in (1..=6).rev() {
        let spread = blur * step as f32 * 4.0;
        let alpha = (opacity * 80.0 / 6.0).round() as u8;
        painter.rect_filled(
            rect.translate(offset).expand(spread),
            (radius + spread).round().clamp(0.0, 255.0) as u8,
            egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], alpha),
        );
    }
}

fn paint_acrylic_background(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: f32,
    color: [u8; 4],
    opacity: f32,
    acrylic: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let acrylic = acrylic.clamp(0.0, 1.0);
    if acrylic > f32::EPSILON {
        // The portable Glow renderer cannot sample pixels behind the native
        // window. Layered tint, softened edges and an inner highlight provide
        // a stable acrylic-like treatment on every supported platform.
        for step in (1..=3).rev() {
            let spread = acrylic * step as f32 * 3.0;
            painter.rect_filled(
                rect.expand(spread),
                (radius + spread).round().clamp(0.0, 255.0) as u8,
                rgba_with_alpha(color, opacity * acrylic * (0.10 / step as f32)),
            );
        }
    }
    painter.rect_filled(
        rect,
        radius.round().clamp(0.0, 255.0) as u8,
        rgba_with_alpha(color, opacity),
    );
    if acrylic > f32::EPSILON {
        let [red, green, blue, _] = color;
        let luminance = red as f32 * 0.2126 + green as f32 * 0.7152 + blue as f32 * 0.0722;
        let tint = if luminance < 150.0 {
            egui::Color32::from_white_alpha((acrylic * opacity * 34.0).round() as u8)
        } else {
            egui::Color32::from_black_alpha((acrylic * opacity * 24.0).round() as u8)
        };
        painter.rect_filled(rect, radius.round().clamp(0.0, 255.0) as u8, tint);
        painter.rect_stroke(
            rect.shrink(0.5),
            radius,
            egui::Stroke::new(
                1.0,
                egui::Color32::from_white_alpha((acrylic * opacity * 64.0).round() as u8),
            ),
            egui::StrokeKind::Inside,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_hud_text(
    painter: &egui::Painter,
    position: egui::Pos2,
    text: &str,
    font: egui::FontId,
    color: [u8; 3],
    source_alpha: u8,
    opacity: f32,
    shadow_opacity: f32,
    shadow_blur: f32,
    shadow_distance: f32,
    shadow_angle: f32,
    shadow_color: [u8; 3],
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let shadow_opacity = shadow_opacity.clamp(0.0, 1.0);
    if shadow_opacity > f32::EPSILON {
        let blur = shadow_blur.clamp(0.0, 1.0);
        let distance = shadow_distance.clamp(0.0, 1.0) * 8.0;
        let angle = shadow_angle.clamp(0.0, 1.0) * std::f32::consts::TAU;
        let offset = egui::vec2(angle.cos(), angle.sin()) * distance;
        let steps = 5;
        for step in (0..steps).rev() {
            let angle = step as f32 * std::f32::consts::TAU / steps as f32;
            let spread = blur * 4.0;
            let delta = egui::vec2(angle.cos(), angle.sin()) * spread;
            let alpha = (source_alpha as f32 * opacity * shadow_opacity * 0.75 / steps as f32)
                .round() as u8;
            painter.text(
                position + offset + delta,
                egui::Align2::CENTER_CENTER,
                text,
                font.clone(),
                egui::Color32::from_rgba_unmultiplied(
                    shadow_color[0],
                    shadow_color[1],
                    shadow_color[2],
                    alpha,
                ),
            );
        }
    }
    painter.text(
        position,
        egui::Align2::CENTER_CENTER,
        text,
        font,
        egui::Color32::from_rgba_unmultiplied(
            color[0],
            color[1],
            color[2],
            (source_alpha as f32 * opacity).round() as u8,
        ),
    );
}

fn paint_hud_border(
    painter: &egui::Painter,
    rect: egui::Rect,
    opacity: f32,
    width: f32,
    radius: f32,
    color: egui::Color32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let width = width.clamp(0.0, 1.0) * HUD_BORDER_WIDTH_MAX;
    if opacity <= f32::EPSILON || width <= f32::EPSILON {
        return;
    }
    painter.rect_stroke(
        rect,
        radius.clamp(0.0, 1.0) * HUD_CORNER_RADIUS_MAX,
        egui::Stroke::new(width, with_alpha(color, (opacity * 255.0).round() as u8)),
        egui::StrokeKind::Inside,
    );
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
    // Match the combo-box row height; the icon itself remains 24 px and is
    // centered inside this 28 px interaction area.
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(28.0, ADJUST_ROW_HEIGHT), egui::Sense::click());
    if response.clicked() {
        layout.snap_to_grid = !layout.snap_to_grid;
    }
    let color = if layout.snap_to_grid {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    draw_toggle_icon(ui, rect, "grid", color, layout.snap_to_grid);
    let clicked = response.clicked();
    response.on_hover_text(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustSnapGrid));
    clicked
}

fn draw_ratio_lock_control(
    ui: &mut egui::Ui,
    layout: &mut LayoutState,
    locale: deskhud_ui::Locale,
) -> bool {
    // Match the combo-box row height; the icon itself remains 24 px and is
    // centered inside this 28 px interaction area.
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(28.0, ADJUST_ROW_HEIGHT), egui::Sense::click());
    if response.clicked() {
        layout.lock_ratio = !layout.lock_ratio;
    }
    let color = if layout.lock_ratio {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    draw_toggle_icon(ui, rect, "link", color, layout.lock_ratio);
    let clicked = response.clicked();
    response.on_hover_text(deskhud_ui::i18n::t(locale, MessageKey::HudAdjustLockRatio));
    clicked
}

fn draw_toggle_icon(
    ui: &egui::Ui,
    rect: egui::Rect,
    icon: &'static str,
    color: egui::Color32,
    active: bool,
) {
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(24.0, 24.0));
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
}

fn layout_slot(prefs: &UiPreferences, item: &HudRenderItem) -> Option<deskhud_ui::HudSlotLayout> {
    match &item.target {
        HudLayoutTarget::Instance(id) => prefs
            .hud
            .instances
            .iter()
            .find(|instance| &instance.id == id)
            .map(|instance| instance.layout.clone()),
        HudLayoutTarget::Group(id) => prefs
            .hud
            .groups
            .iter()
            .find(|group| &group.id == id)
            .map(|group| group.layout.clone()),
    }
}

fn set_layout_slot(
    prefs: &mut UiPreferences,
    item: &HudRenderItem,
    slot: deskhud_ui::HudSlotLayout,
) {
    let slot = slot.clamp01();
    match &item.target {
        HudLayoutTarget::Instance(id) => {
            if let Some(instance) = prefs
                .hud
                .instances
                .iter_mut()
                .find(|instance| &instance.id == id)
            {
                instance.layout = slot;
            }
        }
        HudLayoutTarget::Group(id) => {
            if let Some(group) = prefs.hud.groups.iter_mut().find(|group| &group.id == id) {
                group.layout = slot;
            }
        }
    }
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
            && let Some(mut slot) = layout_slot(prefs, item)
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
                set_layout_slot(prefs, item, slot);
                changed = true;
            }
        }
    }
    changed
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

fn with_alpha(color: egui::Color32, alpha: u8) -> egui::Color32 {
    let [red, green, blue, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}
